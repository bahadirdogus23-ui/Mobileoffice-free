//! Slide fonts on a phone. DeckCraft looks for installed fonts only in desktop font folders and the
//! app is built without its optional craft-fonts faces, so on Android every family fell back to
//! its one compiled-in face (Ubuntu Light), bold text included.
//!
//! DeckCraft's own default family is Inter, and every family it cannot find ends there (Calibri
//! via Carlito → Liberation Sans → Arial → Inter, Arial, Aptos, Times New Roman and Courier New
//! via the default list, see `deckcraft_fonts::FontDb::face`). The app already carries Inter
//! Regular, Medium and SemiBold among WordCraft's bundled faces (`wordcraft_fonts::bundled`), so
//! they are handed to DeckCraft's font database here, unmodified: no bytes are added to the app and
//! the font files stay the Original Version under their licence (OFL 1.1, no Reserved Font Name).
//! Bold draws in SemiBold; italic is slanted by the renderer, as DeckCraft does for any family
//! without an italic face.
//!
//! The other bundled faces are not used for slides. DeckCraft only finds a face under a family it
//! asks for (Carlito, Liberation Serif, Liberation Mono, Arial, Times New Roman…), and there is no
//! alias API, so using Source Serif 4 or JetBrains Mono would mean renaming them in their `name`
//! table: a Modified Version under the OFL that exported PDFs would distribute, either under
//! Adobe's Reserved Font Name "Source" or under another font's name (Carlito and Liberation are
//! themselves Reserved Font Names). See docs/TURKCE-METIN-DENETIMI.md, open issue 1.

use deckcraft_fonts::FontDb;

/// Bundled families handed to DeckCraft, as they are.
const FAMILIES: &[&str] = &["Inter"];

/// Hands the bundled faces to DeckCraft's process-wide font database, once.
pub(crate) fn register_global() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        register(FontDb::global());
    });
}

/// Adds the bundled faces slides use to `db`, unmodified. Returns the number of faces added.
pub(crate) fn register(db: &FontDb) -> usize {
    wordcraft_fonts::bundled()
        .iter()
        .filter(|data| family(data).is_some_and(|f| FAMILIES.contains(&f.as_str())))
        .map(|data| db.add_font(data.to_vec()))
        .sum()
}

/// The typographic family of a font file (`name` ID 16, else 1; Windows Unicode records).
fn family(data: &[u8]) -> Option<String> {
    let n = be16(data, 4)? as usize;
    let r = (0..n).map(|i| 12 + i * 16).find(|r| data.get(*r..*r + 4) == Some(b"name"))?;
    let (off, len) = (be32(data, r + 8)? as usize, be32(data, r + 12)? as usize);
    let t = data.get(off..off.checked_add(len)?)?;
    let (count, strings) = (be16(t, 2)? as usize, be16(t, 4)? as usize);
    let get = |want: u16| {
        (0..count).find_map(|i| {
            let r = 6 + i * 12;
            let (platform, encoding, id) = (be16(t, r)?, be16(t, r + 2)?, be16(t, r + 6)?);
            if platform != 3 || !matches!(encoding, 1 | 10) || id != want {
                return None;
            }
            let (l, o) = (be16(t, r + 8)? as usize, be16(t, r + 10)? as usize);
            let units: Vec<u16> = t.get(strings + o..strings + o + l)?.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            Some(String::from_utf16_lossy(&units)).filter(|s| !s.is_empty())
        })
    };
    get(16).or_else(|| get(1))
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
