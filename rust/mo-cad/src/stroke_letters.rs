//! Turkish letters in the engine's own single-stroke style.
//!
//! The engine's drafting font ("CADCraft Stroke", `cadcraft_fonts` `stroke.rs`) has ASCII only
//! and is compiled in: there is no way to add glyphs to it. Every other character is drawn from
//! a fallback font (the FONTFALLBACK list first), as a filled outline sized so its font's cap
//! height is the text height, advancing by the glyph's own advance. So Ç Ğ İ Ö Ş Ü ç ğ ı ö ş ü
//! came out as filled Roboto letters next to thin single-stroke ones and looked bold.
//!
//! This module builds a small TrueType font in memory whose glyphs are those letters made the
//! way the drafting font would make them: its base letter (the same strokes, copied from the
//! engine's public-domain font) plus the accent strokes (cedilla, breve, dot, dieresis,
//! circumflex), on its grid, with its advance (glyph width + letter gap) and its cap height.
//! Placed by the engine's fallback code, a glyph lands exactly where a drafting-font glyph would
//! (rotation, width factor, oblique, alignment and MTEXT layout are the engine's own).
//!
//! A TrueType glyph is filled, not stroked, so each stroke segment is a sliver: a rectangle
//! about 1/1900 of the cap height wide. Filled, a sliver would be a faint sub-pixel line, so the
//! frame builder draws a filled shape whose every triangle is thinner than a pixel as hairlines
//! along its triangles ([`crate::hairline_tris`]), and the letter comes out like its neighbours.

/// Font units per drafting-font grid unit (cap height = 6 units): the grid's tenths and its
/// letter gap (1.6) are whole font units.
const UNIT: f64 = 320.0;
const UPEM: u16 = 2560;
const CAP: f64 = 6.0;
/// The drafting font's gap after each letter, in grid units (`stroke::GAP`).
const GAP: f64 = 1.6;

/// The name the font is registered under (and put first in FONTFALLBACK as).
pub(crate) const NAME: &str = "mo-stroke-letters";

/// A drafting-font stroke spec (`stroke.rs`): points of an x digit and a y letter (`c` =
/// baseline, `i` = cap height), strokes separated by spaces.
fn spec(s: &str) -> Strokes {
    s.split(' ')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.as_bytes()
                .chunks(2)
                .filter_map(|p| Some((f64::from(p.first()?.checked_sub(b'0')?), f64::from(p.get(1)?.checked_sub(b'a')?) - 2.0)))
                .collect()
        })
        .collect()
}

/// The base letters, as the engine's drafting font draws them: (width, strokes).
fn base(c: char) -> Option<(f64, &'static str)> {
    Some(match c {
        'A' => (4.0, "0c2i4c 1f3f"),
        'C' => (4.0, "4h3i1i0h0d1c3c4d"),
        'G' => (4.0, "4h3i1i0h0d1c3c4d4f2f"),
        'I' => (2.0, "0i2i 1i1c 0c2c"),
        'O' => (4.0, "1c0d0h1i3i4h4d3c1c"),
        'S' => (4.0, "4h3i1i0h0g1f3f4e4d3c1c0d"),
        'U' => (4.0, "0i0d1c3c4d4i"),
        'a' => (4.0, "4g4c 4f3g1g0f0d1c3c4d"),
        'c' => (4.0, "4f3g1g0f0d1c3c4d"),
        'g' => (4.0, "4g4b3a1a0b 4f3g1g0f0d1c3c4d"),
        'o' => (4.0, "1c0d0f1g3g4f4d3c1c"),
        's' => (4.0, "4f3g1g0f1e3e4d3c1c0d"),
        'u' => (4.0, "0g0d1c3c4d 4g4c"),
        // The dotless stem of `i` and `I` (whose dot the drafting font draws as a short stroke).
        'ı' => (1.0, "0c0g"),
        _ => return None,
    })
}

/// Accents, centred on x = `m`, for a capital (above the cap height) or a small letter (above
/// the x-height, where the drafting font puts the dot of `i`). Grid units.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Accent {
    Cedilla,
    Breve,
    Dot,
    Dieresis,
    Circumflex,
}

fn accent(a: Accent, m: f64, capital: bool) -> Strokes {
    // The dot of `i` runs from 5 to 6 over an x-height of 4: one unit, a unit clear. Capitals
    // get the same mark a unit over the cap height, a little shorter so the line keeps clear of
    // the one above.
    let (lo, hi) = if capital { (6.9, 7.7) } else { (5.0, 6.0) };
    match a {
        Accent::Cedilla => {
            vec![vec![(m, 0.0), (m, -0.45), (m + 0.45, -0.55), (m + 0.7, -0.85), (m + 0.55, -1.25), (m + 0.1, -1.45), (m - 0.6, -1.5)]]
        }
        Accent::Breve => vec![vec![(m - 1.0, hi), (m - 0.75, lo + 0.25), (m, lo), (m + 0.75, lo + 0.25), (m + 1.0, hi)]],
        Accent::Dot => vec![vec![(m, lo), (m, hi)]],
        Accent::Dieresis => vec![vec![(m - 1.0, lo), (m - 1.0, hi)], vec![(m + 1.0, lo), (m + 1.0, hi)]],
        Accent::Circumflex => vec![vec![(m - 1.0, lo), (m, hi), (m + 1.0, lo)]],
    }
}

/// The letters this font has: (character, base letter, accent).
const LETTERS: [(char, char, Option<Accent>); 18] = [
    ('Ç', 'C', Some(Accent::Cedilla)),
    ('ç', 'c', Some(Accent::Cedilla)),
    ('Ğ', 'G', Some(Accent::Breve)),
    ('ğ', 'g', Some(Accent::Breve)),
    ('İ', 'I', Some(Accent::Dot)),
    ('ı', 'ı', None),
    ('Ö', 'O', Some(Accent::Dieresis)),
    ('ö', 'o', Some(Accent::Dieresis)),
    ('Ş', 'S', Some(Accent::Cedilla)),
    ('ş', 's', Some(Accent::Cedilla)),
    ('Ü', 'U', Some(Accent::Dieresis)),
    ('ü', 'u', Some(Accent::Dieresis)),
    // Circumflexed vowels of Turkish loanwords ("kâğıt", "hâlâ", "Kur'ân").
    ('Â', 'A', Some(Accent::Circumflex)),
    ('â', 'a', Some(Accent::Circumflex)),
    ('Î', 'I', Some(Accent::Circumflex)),
    ('î', 'ı', Some(Accent::Circumflex)),
    ('Û', 'U', Some(Accent::Circumflex)),
    ('û', 'u', Some(Accent::Circumflex)),
];

/// Strokes: runs of grid points.
type Strokes = Vec<Vec<(f64, f64)>>;

/// The strokes of letter `c` in grid units and its width, if this font has it.
pub(crate) fn strokes(c: char) -> Option<(f64, Strokes)> {
    let &(_, b, a) = LETTERS.iter().find(|l| l.0 == c)?;
    let (w, s) = base(b)?;
    let mut out = spec(s);
    if let Some(a) = a {
        out.extend(accent(a, w / 2.0, b.is_uppercase()));
    }
    Some((w, out))
}

/// One glyph: its contours (each a closed run of on-curve points) in font units.
struct Glyph {
    advance: u16,
    contours: Vec<[(i16, i16); 4]>,
}

/// Font units of grid point `p`.
fn units(p: (f64, f64)) -> (i32, i32) {
    ((p.0 * UNIT).round() as i32, (p.1 * UNIT).round() as i32)
}

/// A stroke segment as a sliver: the segment and its copy one font unit to the side.
fn sliver(a: (i32, i32), b: (i32, i32)) -> Option<[(i16, i16); 4]> {
    let (dx, dy) = (f64::from(b.0 - a.0), f64::from(b.1 - a.1));
    let len = dx.hypot(dy);
    if len < 1.0 {
        return None;
    }
    // The unit normal, rounded to whole units: never parallel to the segment.
    let (nx, ny) = ((-dy / len).round() as i32, (dx / len).round() as i32);
    let p = |x: i32, y: i32| Some((i16::try_from(x).ok()?, i16::try_from(y).ok()?));
    Some([p(a.0, a.1)?, p(b.0, b.1)?, p(b.0 + nx, b.1 + ny)?, p(a.0 + nx, a.1 + ny)?])
}

fn glyph(c: char) -> Option<Glyph> {
    let (w, strokes) = strokes(c)?;
    let mut contours = Vec::new();
    for st in &strokes {
        for seg in st.windows(2) {
            contours.extend(sliver(units(seg[0]), units(seg[1])));
        }
    }
    Some(Glyph { advance: ((w + GAP) * UNIT).round() as u16, contours })
}

/// Big-endian table writer.
#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn i16(&mut self, v: i16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn zeros(&mut self, n: usize) -> &mut Self {
        self.0.resize(self.0.len() + n, 0);
        self
    }
    fn pad4(&mut self) {
        while !self.0.len().is_multiple_of(4) {
            self.0.push(0);
        }
    }
}

fn checksum(b: &[u8]) -> u32 {
    b.chunks(4).fold(0u32, |s, c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        s.wrapping_add(u32::from_be_bytes(w))
    })
}

/// The font file: a TrueType font with glyph 0 (empty) and [`LETTERS`].
pub(crate) fn font() -> Vec<u8> {
    let glyphs: Vec<(char, Glyph)> = LETTERS.iter().filter_map(|l| Some((l.0, glyph(l.0)?))).collect();
    let n = u16::try_from(glyphs.len() + 1).unwrap_or(u16::MAX);
    let (mut x_min, mut y_min, mut x_max, mut y_max) = (0i16, 0i16, 0i16, 0i16);
    let (mut max_points, mut max_contours) = (0u16, 0u16);

    // glyf and loca (long offsets).
    let mut glyf = Buf::default();
    let mut loca = Buf::default();
    loca.u32(0).u32(0);
    for (_, g) in &glyphs {
        let pts: Vec<(i16, i16)> = g.contours.iter().flatten().copied().collect();
        let (gx0, gy0) = pts.iter().fold((i16::MAX, i16::MAX), |m, p| (m.0.min(p.0), m.1.min(p.1)));
        let (gx1, gy1) = pts.iter().fold((i16::MIN, i16::MIN), |m, p| (m.0.max(p.0), m.1.max(p.1)));
        (x_min, y_min, x_max, y_max) = (x_min.min(gx0), y_min.min(gy0), x_max.max(gx1), y_max.max(gy1));
        let contours = u16::try_from(g.contours.len()).unwrap_or(0);
        max_contours = max_contours.max(contours);
        max_points = max_points.max(contours.saturating_mul(4));
        glyf.i16(contours as i16).i16(gx0).i16(gy0).i16(gx1).i16(gy1);
        for i in 0..contours {
            glyf.u16(i * 4 + 3);
        }
        glyf.u16(0); // no instructions
        glyf.0.extend(std::iter::repeat_n(1u8, pts.len())); // on-curve, two-byte deltas
        let mut last = (0i16, 0i16);
        for p in &pts {
            glyf.i16(p.0.wrapping_sub(last.0));
            last.0 = p.0;
        }
        for p in &pts {
            glyf.i16(p.1.wrapping_sub(last.1));
            last.1 = p.1;
        }
        glyf.pad4();
        loca.u32(glyf.0.len() as u32);
    }

    let cap = (CAP * UNIT) as i16;
    let ascent = (8.0 * UNIT) as i16;
    let descent = (-2.0 * UNIT) as i16;
    let adv_max = glyphs.iter().map(|g| g.1.advance).max().unwrap_or(0);

    let mut head = Buf::default();
    head.u32(0x0001_0000).u32(0x0001_0000).u32(0).u32(0x5F0F_3CF5).u16(0x000B).u16(UPEM).zeros(16);
    head.i16(x_min).i16(y_min).i16(x_max).i16(y_max).u16(0).u16(8).i16(2).i16(1).i16(0);

    let mut hhea = Buf::default();
    hhea.u32(0x0001_0000).i16(ascent).i16(descent).i16(0).u16(adv_max).i16(x_min).i16(0).i16(x_max);
    hhea.i16(1).i16(0).i16(0).zeros(8).i16(0).u16(n);

    let mut maxp = Buf::default();
    maxp.u32(0x0001_0000).u16(n).u16(max_points).u16(max_contours).zeros(4).u16(1).zeros(16);

    let mut hmtx = Buf::default();
    hmtx.u16(0).i16(0);
    for (_, g) in &glyphs {
        hmtx.u16(g.advance).i16(0);
    }

    // cmap: format 4, one segment per letter (sorted), then the 0xFFFF end segment.
    let mut map: Vec<(u16, u16)> = glyphs.iter().enumerate().map(|(i, (c, _))| (*c as u16, i as u16 + 1)).collect();
    map.sort();
    let segs = map.len() as u16 + 1;
    let mut sub = Buf::default();
    let len = 16 + 8 * segs;
    let pow = 1u16 << (15 - segs.leading_zeros()).min(15);
    sub.u16(4).u16(len).u16(0).u16(segs * 2).u16(pow * 2).u16((15 - segs.leading_zeros()) as u16).u16(segs * 2 - pow * 2);
    for &(c, _) in &map {
        sub.u16(c);
    }
    sub.u16(0xFFFF).u16(0);
    for &(c, _) in &map {
        sub.u16(c);
    }
    sub.u16(0xFFFF);
    for &(c, g) in &map {
        sub.u16(g.wrapping_sub(c));
    }
    sub.u16(1);
    sub.zeros(2 * usize::from(segs));
    let mut cmap = Buf::default();
    cmap.u16(0).u16(1).u16(3).u16(1).u32(12);
    cmap.0.extend_from_slice(&sub.0);

    let mut os2 = Buf::default();
    os2.u16(4).i16(adv_max as i16).u16(400).u16(5).u16(0).zeros(20).i16(0).zeros(10).zeros(16).zeros(4);
    os2.u16(0x40).u16(map.first().map_or(0, |m| m.0)).u16(map.last().map_or(0, |m| m.0));
    os2.i16(ascent).i16(descent).i16(0).u16(ascent as u16).u16(descent.unsigned_abs()).zeros(8);
    os2.i16((4.0 * UNIT) as i16).i16(cap).u16(0).u16(32).u16(0);

    let mut post = Buf::default();
    post.u32(0x0003_0000).zeros(28);

    let family: Vec<u16> = "MO Stroke Letters".encode_utf16().collect();
    let mut name = Buf::default();
    name.u16(0).u16(1).u16(18).u16(3).u16(1).u16(0x409).u16(1).u16(family.len() as u16 * 2).u16(0);
    for u in family {
        name.u16(u);
    }

    let tables: [(&[u8; 4], Vec<u8>); 10] = [
        (b"OS/2", os2.0),
        (b"cmap", cmap.0),
        (b"glyf", glyf.0),
        (b"head", head.0),
        (b"hhea", hhea.0),
        (b"hmtx", hmtx.0),
        (b"loca", loca.0),
        (b"maxp", maxp.0),
        (b"name", name.0),
        (b"post", post.0),
    ];
    let mut out = Buf::default();
    out.u32(0x0001_0000).u16(tables.len() as u16).u16(128).u16(3).u16(32);
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in &tables {
        out.0.extend_from_slice(*tag);
        out.u32(checksum(data)).u32(offset as u32).u32(data.len() as u32);
        offset += data.len().next_multiple_of(4);
    }
    for (_, data) in &tables {
        out.0.extend_from_slice(data);
        out.pad4();
    }
    out.0
}

/// Registers the font and puts it first in FONTFALLBACK, so the letters it has come from it
/// instead of an installed font; other characters still fall back as before.
pub(crate) fn register() {
    cadcraft_fonts::ttf::register(NAME, font());
    let list = cadcraft_fonts::ttf::fallback_fonts();
    if !list.split(',').any(|n| n.trim().eq_ignore_ascii_case(NAME)) {
        cadcraft_fonts::ttf::set_fallback_fonts(&if list.is_empty() { NAME.to_string() } else { format!("{NAME}, {list}") });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn face() -> cadcraft_fonts::ttf::Face {
        cadcraft_fonts::ttf::faces(&Arc::new(font())).into_iter().next().expect("the font parses")
    }

    #[test]
    fn the_font_parses_and_has_every_letter() {
        let f = face();
        for (c, _, _) in LETTERS {
            assert!(f.has_glyph(c), "{c}");
        }
        for c in ['S', 'i', 'Я', 'ß'] {
            assert!(!f.has_glyph(c), "{c}");
        }
    }

    /// Ş is the drafting font's S with a cedilla hanging from the middle of its bottom.
    #[test]
    fn s_cedilla_is_s_with_a_hook_below() {
        let (w, s) = strokes('Ş').unwrap();
        assert_eq!(w, 4.0);
        assert_eq!(s[0], spec("4h3i1i0h0g1f3f4e4d3c1c0d")[0]);
        let hook = s.last().unwrap();
        assert_eq!(hook[0], (2.0, 0.0));
        assert!(hook.iter().all(|p| p.1 <= 0.0) && hook.iter().any(|p| p.1 < -1.0), "{hook:?}");
        // ı is the stem of i without its dot; İ is I with a dot over the cap height.
        assert_eq!(strokes('ı').unwrap().1, vec![vec![(0.0, 0.0), (0.0, 4.0)]]);
        let dot = strokes('İ').unwrap().1.pop().unwrap();
        assert!(dot.iter().all(|p| p.0 == 1.0 && p.1 > 6.5), "{dot:?}");
        // Ö and ü carry two marks, ğ a curve, all above their letter.
        assert_eq!(strokes('Ö').unwrap().1.len(), spec("1c0d0h1i3i4h4d3c1c").len() + 2);
        assert!(strokes('ğ').unwrap().1.last().unwrap().iter().all(|p| p.1 >= 5.0));
    }

    /// The engine places a glyph of this font like a drafting-font letter: the same advance,
    /// strokes as slivers inside the letter's box.
    #[test]
    fn the_engine_places_the_letters_like_drafting_font_letters() {
        cadcraft_fonts::ttf::register("mo-stroke-letters-test", font());
        let f = cadcraft_fonts::ttf::find("mo-stroke-letters-test").unwrap();
        // "Ş" in this font is as wide as "S" in the drafting font (both without the last gap).
        let s = cadcraft_fonts::ttf::width(&f, "Ş", 6.0, 1.0).unwrap();
        assert!((s - (4.0 + GAP)).abs() < 1e-3, "{s}");
        let run = cadcraft_fonts::ttf::layout_line(&f, "Ş", 6.0, 1.0, 0.0).unwrap();
        let pts: Vec<_> = run.strokes.iter().flatten().collect();
        assert!(pts.iter().all(|p| p.x > -0.01 && p.x < 4.01 && p.y > -1.51 && p.y < 6.01), "{pts:?}");
        // The cedilla reaches below the baseline under the middle of the letter.
        assert!(pts.iter().any(|p| p.y < -1.0 && (p.x - 2.0).abs() < 1.0));
        // Each stroke segment is a closed sliver a 1900th (a 1350th slanted) of the cap height wide.
        assert!(run.strokes.len() > 12);
        for c in &run.strokes {
            assert_eq!(c.len(), 5);
            let area = c.windows(2).map(|w| w[0].x * w[1].y - w[1].x * w[0].y).sum::<f64>().abs() / 2.0;
            let len = c.windows(2).map(|w| w[0].dist(w[1])).fold(0.0, f64::max);
            assert!(area / len < 6.0 / 1000.0, "{c:?}");
        }
    }
}
