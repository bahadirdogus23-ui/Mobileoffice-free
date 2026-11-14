//! Slide fonts on a phone. DeckCraft looks for installed fonts only in desktop font folders and the
//! app is built without its optional craft-fonts faces, so on Android every family fell back to
//! its one compiled-in face (Ubuntu Light), bold and serif text included.
//!
//! The app already carries WordCraft's bundled OFL faces (Source Sans 3, Source Serif 4, Inter,
//! JetBrains Mono; see `wordcraft_fonts::bundled`). They are handed to DeckCraft's font database
//! here, so slides use them at no cost in app size. DeckCraft only ever asks for its own default
//! family (Inter) and the open stand-ins for Office fonts (Carlito for Calibri, Liberation Serif
//! for Times New Roman and Cambria, Liberation Mono for Courier New…; `deckcraft_fonts::substitutes`),
//! so each face keeps its own family name and gets the stand-in's name as its Windows family
//! (`name` IDs 1 and 2), the name DeckCraft also finds faces by. A real Carlito or Liberation
//! font, where one is installed, still wins: Windows family names are only tried when no face of
//! that typographic family exists.
//!
//! | Asked for | Stand-in name | Drawn with | Why |
//! |---|---|---|---|
//! | Calibri, and any other missing family (PowerPoint draws those in Calibri: Tahoma, Georgia…) | Carlito | Source Sans 3 | humanist sans about as narrow as Calibri, with bold and italic faces |
//! | Times New Roman, Cambria | Liberation Serif | Source Serif 4 | the only serif; all four styles |
//! | Courier New | Liberation Mono | JetBrains Mono | monospaced, 0.6 em like Courier New |
//! | Arial, Helvetica, Aptos (an Office cloud font) | (Inter, DeckCraft's default) | Inter | about as wide as Arial |
//!
//! Inter comes in Regular, Medium and SemiBold only: bold draws in SemiBold and italic is slanted by
//! the renderer, as DeckCraft does for any family without those faces.

use deckcraft_fonts::FontDb;

/// The stand-in family DeckCraft asks for that each bundled family answers to.
const STAND_INS: &[(&str, &str)] = &[("Source Sans 3", "Carlito"), ("Source Serif 4", "Liberation Serif"), ("JetBrains Mono", "Liberation Mono")];

/// Bundled families added under their own name only.
const AS_THEMSELVES: &[&str] = &["Inter"];

/// Hands the bundled faces to DeckCraft's process-wide font database, once.
pub(crate) fn register_global() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        register(FontDb::global());
    });
}

/// Adds the bundled faces to `db`. Returns the number of faces added.
pub(crate) fn register(db: &FontDb) -> usize {
    wordcraft_fonts::bundled().iter().filter_map(|data| face_for_slides(data)).map(|bytes| db.add_font(bytes)).sum()
}

/// The font file as DeckCraft should see it, or none when it is not one slides use (the
/// Semibold faces: their Windows family is a family of its own, "Source Sans 3 Semibold").
fn face_for_slides(data: &[u8]) -> Option<Vec<u8>> {
    let names = Names::read(data)?;
    let family = names.get(16).or_else(|| names.get(1))?;
    let style = names.get(17).or_else(|| names.get(2)).unwrap_or_else(|| "Regular".into());
    if AS_THEMSELVES.contains(&family.as_str()) {
        return Some(data.to_vec());
    }
    let (_, stand_in) = STAND_INS.iter().find(|(f, _)| *f == family)?;
    let win_style = names.get(2)?;
    if names.get(1)? != family || !matches!(win_style.as_str(), "Regular" | "Bold" | "Italic" | "Bold Italic") {
        return None;
    }
    with_names(data, &[(1, stand_in), (2, &win_style), (16, &family), (17, &style)])
}

/// The Windows Unicode names of a font file (`name` table, platform 3, encoding 1 or 10),
/// English (United States) first.
struct Names(Vec<(u16, u16, String)>);

impl Names {
    fn read(data: &[u8]) -> Option<Names> {
        let (off, len) = table(data, b"name")?;
        let t = data.get(off..off.checked_add(len)?)?;
        let count = be16(t, 2)? as usize;
        let strings = be16(t, 4)? as usize;
        let mut v = Vec::new();
        for i in 0..count {
            let r = 6 + i * 12;
            let (platform, encoding, language, id) = (be16(t, r)?, be16(t, r + 2)?, be16(t, r + 4)?, be16(t, r + 6)?);
            if platform != 3 || !matches!(encoding, 1 | 10) {
                continue;
            }
            let (l, o) = (be16(t, r + 8)? as usize, be16(t, r + 10)? as usize);
            let Some(raw) = t.get(strings + o..strings + o + l) else { continue };
            let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            v.push((id, language, String::from_utf16_lossy(&units)));
        }
        Some(Names(v))
    }

    fn get(&self, id: u16) -> Option<String> {
        let mut found = self.0.iter().filter(|(i, _, s)| *i == id && !s.is_empty());
        let first = found.clone().next();
        found.find(|(_, lang, _)| *lang == 0x0409).or(first).map(|(_, _, s)| s.clone())
    }
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Byte range of a table in a single-font file: (offset, length).
fn table(data: &[u8], tag: &[u8; 4]) -> Option<(usize, usize)> {
    let n = be16(data, 4)? as usize;
    (0..n)
        .map(|i| 12 + i * 16)
        .find(|r| data.get(*r..*r + 4) == Some(tag))
        .and_then(|r| Some((be32(data, r + 8)? as usize, be32(data, r + 12)? as usize)))
}

/// A copy of the font whose `name` table has every record for the given IDs replaced by one
/// Windows English (United States) record with the new text. The new table is appended and the
/// table directory pointed at it; the old one stays in the file, unused.
fn with_names(data: &[u8], set: &[(u16, &str)]) -> Option<Vec<u8>> {
    let (off, len) = table(data, b"name")?;
    let t = data.get(off..off.checked_add(len)?)?;
    let count = be16(t, 2)? as usize;
    let strings = be16(t, 4)? as usize;
    // (platform, encoding, language, id, string bytes); format 1 language tags are dropped with
    // the records that use them (languages 0x8000 and up).
    let mut records: Vec<(u16, u16, u16, u16, Vec<u8>)> = Vec::new();
    for i in 0..count {
        let r = 6 + i * 12;
        let (platform, encoding, language, id) = (be16(t, r)?, be16(t, r + 2)?, be16(t, r + 4)?, be16(t, r + 6)?);
        if set.iter().any(|(s, _)| *s == id) || language >= 0x8000 {
            continue;
        }
        let (l, o) = (be16(t, r + 8)? as usize, be16(t, r + 10)? as usize);
        records.push((platform, encoding, language, id, t.get(strings + o..strings + o + l)?.to_vec()));
    }
    for (id, text) in set {
        records.push((3, 1, 0x0409, *id, text.encode_utf16().flat_map(u16::to_be_bytes).collect()));
    }
    records.sort_by_key(|r| (r.0, r.1, r.2, r.3));
    let header = 6 + records.len() * 12;
    let mut name = Vec::new();
    name.extend_from_slice(&0u16.to_be_bytes());
    name.extend_from_slice(&u16::try_from(records.len()).ok()?.to_be_bytes());
    name.extend_from_slice(&u16::try_from(header).ok()?.to_be_bytes());
    let mut pool: Vec<u8> = Vec::new();
    for (p, e, l, id, s) in &records {
        for v in [*p, *e, *l, *id, u16::try_from(s.len()).ok()?, u16::try_from(pool.len()).ok()?] {
            name.extend_from_slice(&v.to_be_bytes());
        }
        pool.extend_from_slice(s);
    }
    name.extend_from_slice(&pool);

    let mut out = data.to_vec();
    out.resize(out.len().next_multiple_of(4), 0);
    let new_off = u32::try_from(out.len()).ok()?;
    out.extend_from_slice(&name);
    let mut padded = name.clone();
    padded.resize(name.len().div_ceil(4) * 4, 0);
    let checksum = padded.chunks_exact(4).fold(0u32, |a, c| a.wrapping_add(u32::from_be_bytes([c[0], c[1], c[2], c[3]])));
    let n = be16(data, 4)? as usize;
    let r = (0..n).map(|i| 12 + i * 16).find(|r| data.get(*r..*r + 4) == Some(b"name"))?;
    out.get_mut(r + 4..r + 8)?.copy_from_slice(&checksum.to_be_bytes());
    out.get_mut(r + 8..r + 12)?.copy_from_slice(&new_off.to_be_bytes());
    out.get_mut(r + 12..r + 16)?.copy_from_slice(&u32::try_from(name.len()).ok()?.to_be_bytes());
    out.resize(out.len().next_multiple_of(4), 0);
    Some(out)
}
