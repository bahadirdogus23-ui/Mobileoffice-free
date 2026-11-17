//! Slide fonts on a phone. DeckCraft looks for installed fonts only in desktop font folders and the
//! app is built without its optional craft-fonts faces, so on Android every family fell back to
//! its one compiled-in face (Ubuntu Light), bold text included.
//!
//! Two sources fill DeckCraft's font database, both unmodified (the Original Version under the
//! OFL 1.1: no `name` table is touched, so no Reserved Font Name question arises):
//!
//! - **The Office stand-ins the app ships as assets** (`android/app/src/main/assets/fonts/`):
//!   Carlito, Caladea, Liberation Sans and Liberation Serif, Regular/Bold/Italic/Bold Italic each.
//!   DeckCraft already asks for exactly these families when a slide names an Office font
//!   (`deckcraft_fonts::substitutes`: Calibri → Carlito, Cambria → Caladea, Arial → Liberation
//!   Sans, Times New Roman → Liberation Serif) and they keep those fonts' advance widths, so text
//!   breaks into the same lines as in PowerPoint. Any family a phone lacks and that has no
//!   stand-in draws in Calibri, as PowerPoint draws it (`FontDb::document_face`), so Carlito.
//!   The app reads them on the first slide open, off the UI thread, and hands them over through
//!   [`register_files`] (FFI `register_slide_fonts`). Exported PDFs embed subsets of them, which
//!   the OFL allows without renaming (a document using the font, not a Modified Version).
//! - **Inter**, which WordCraft compiles in (`wordcraft_fonts::bundled`): DeckCraft's own default
//!   family, the fallback for what has no stand-in when Carlito is not there (the tests, and
//!   Courier New: Liberation Mono is not shipped). Added on every open by [`register_global`].
//!
//! The other faces WordCraft compiles in (Source Sans 3, Source Serif 4, JetBrains Mono) are not
//! used for slides: DeckCraft only finds a face under a family it asks for and has no alias API,
//! and renaming them would make a Modified Version that exported PDFs distribute (see
//! docs/TURKCE-METIN-DENETIMI.md, open issue 1).

use deckcraft_fonts::FontDb;

pub(crate) use mo_convert::sfnt::family;

/// Bundled families handed to DeckCraft, as they are.
const FAMILIES: &[&str] = &["Inter"];

/// The Office stand-ins the app ships, by family. Files of any other family are refused, so a
/// stray asset cannot take over a family DeckCraft resolves by name.
pub const OFFICE_FAMILIES: &[&str] = &["Carlito", "Caladea", "Liberation Sans", "Liberation Serif", "Liberation Mono"];

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

/// Adds the Office stand-in font files (the app's assets) to `db`, as they are: files of a family
/// in [`OFFICE_FAMILIES`] only; faces already there are skipped. Returns the number of faces added.
pub fn register_files(db: &FontDb, files: &[Vec<u8>]) -> usize {
    files.iter().filter(|data| family(data).is_some_and(|f| OFFICE_FAMILIES.contains(&f.as_str()))).map(|data| db.add_font(data.clone())).sum()
}
