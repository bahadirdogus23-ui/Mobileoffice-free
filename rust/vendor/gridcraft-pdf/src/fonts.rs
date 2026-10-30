//! TrueType faces embedded in place of the standard 14 fonts (MobileOffice patch, see PATCHES.md).
//!
//! The standard fonts are not embedded, so a viewer draws them with whatever font it has, and
//! some lack letters such as Turkish İ. With faces registered through [`set_text_fonts`], every
//! text font is embedded as a simple TrueType font in the same single-byte encoding
//! (`TEXT_ENCODING`), and widths for layout come from the embedded face.

use std::sync::OnceLock;

/// One face to embed: its bytes and the metrics a PDF font dictionary needs.
pub(crate) struct Face {
    pub data: &'static [u8],
    pub name: String,
    /// Advance widths in 1/1000 em for codes 32..=255.
    pub widths: [u16; 224],
    pub bbox: [i32; 4],
    pub ascent: i32,
    pub descent: i32,
    pub cap_height: i32,
    pub italic_angle: f32,
    pub flags: u32,
    compressed: OnceLock<Vec<u8>>,
}

impl Face {
    /// The font file, zlib-compressed once on first use.
    pub fn compressed(&self) -> &[u8] {
        self.compressed.get_or_init(|| miniz_oxide::deflate::compress_to_vec_zlib(self.data, 6))
    }
}

static FACES: OnceLock<Vec<Face>> = OnceLock::new();

/// The registered face for standard font slot `index` (the order of `Font::ALL`), if any.
pub(crate) fn face(index: usize) -> Option<&'static Face> {
    FACES.get()?.get(index)
}

/// Registers TrueType faces for the standard fonts, in the order of `Font::ALL` (Helvetica,
/// -Bold, -Oblique, -BoldOblique, Times…, Courier…). Later PDFs embed them. The first successful
/// call wins; returns false when a face cannot be read (nothing is registered then).
pub fn set_text_fonts(faces: [&'static [u8]; 12]) -> bool {
    if FACES.get().is_some() {
        return true;
    }
    let parsed: Option<Vec<Face>> = faces.iter().enumerate().map(|(i, data)| read(data, i)).collect();
    match parsed {
        Some(v) => {
            let _ = FACES.set(v);
            true
        }
        None => false,
    }
}

/// Whether faces are registered.
pub fn text_fonts_set() -> bool {
    FACES.get().is_some()
}

fn read(data: &'static [u8], slot: usize) -> Option<Face> {
    let f = ttf_parser::Face::parse(data, 0).ok()?;
    let upem = f32::from(f.units_per_em().max(1));
    let em = |v: f32| (v * 1000.0 / upem).round() as i32;
    let mut widths = [0u16; 224];
    for (k, w) in widths.iter_mut().enumerate() {
        let code = (k + 32) as u8;
        let adv = super::code_char(code).and_then(|c| f.glyph_index(c)).and_then(|g| f.glyph_hor_advance(g));
        *w = adv.map_or(0, |a| em(f32::from(a)).clamp(0, 4000) as u16);
    }
    let b = f.global_bounding_box();
    let name = f
        .names()
        .into_iter()
        .find(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
        .and_then(|n| n.to_string())
        .unwrap_or_else(|| format!("Embedded{slot}"));
    let name: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    // Bit 6 nonsymbolic (the font's own Unicode cmap is used through glyph names), 7 italic,
    // 1 fixed pitch, 2 serif.
    let italic = slot % 4 >= 2;
    let mut flags = 32;
    if italic {
        flags |= 64;
    }
    if f.is_monospaced() {
        flags |= 1;
    }
    if (4..8).contains(&slot) {
        flags |= 2;
    }
    Some(Face {
        data,
        name,
        widths,
        bbox: [em(f32::from(b.x_min)), em(f32::from(b.y_min)), em(f32::from(b.x_max)), em(f32::from(b.y_max))],
        ascent: em(f32::from(f.ascender())),
        descent: em(f32::from(f.descender())),
        cap_height: em(f32::from(f.capital_height().unwrap_or(f.ascender()))),
        italic_angle: f.italic_angle(),
        flags,
        compressed: OnceLock::new(),
    })
}
