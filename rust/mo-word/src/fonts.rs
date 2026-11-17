//! Word fonts on a phone. WordCraft finds no installed fonts on Android, so a document set in
//! Calibri, Cambria, Arial or Times New Roman drew in its compiled-in Source Sans 3 / Source
//! Serif 4: Turkish letters were all there, but lines broke where Word's do not.
//!
//! The app ships Carlito, Caladea, Liberation Sans and Liberation Serif (four faces each,
//! unmodified, SIL OFL 1.1) as assets for the slides (see `mo-deck/src/fonts.rs`), and WordCraft
//! asks for the same families first when a document names an Office font
//! (`wordcraft_fonts::word`: Calibri and any other missing sans family → Carlito, Cambria and
//! missing serif families → Caladea, Arial → Liberation Sans, Times New Roman → Liberation
//! Serif). They keep those fonts' advance widths, so pages break as in Word.
//!
//! WordCraft's run-time API for fonts that are not installed is
//! `FontDb::add_document_fonts`, the one it uses for fonts a document embeds: each file is added
//! under the family its own `name` table gives (passed back unchanged, so nothing is renamed) and
//! lookups made before are forgotten (`FontDb::generation`), so a document opened earlier lays
//! out with them the next time it is laid out. The fonts are not recorded or stored anywhere.

use wordcraft_fonts::FontDb;

/// The Office stand-ins the app ships, by family; files of any other family are refused.
const OFFICE_FAMILIES: &[&str] = &["Carlito", "Caladea", "Liberation Sans", "Liberation Serif", "Liberation Mono"];

/// Adds the Office stand-in font files to `db` under their own family names, unmodified. Faces
/// already there are skipped. Returns the number of faces added.
pub fn register_files(db: &FontDb, files: Vec<Vec<u8>>) -> usize {
    let named: Vec<(Vec<u8>, String)> = files
        .into_iter()
        .filter_map(|data| {
            let family = mo_convert::sfnt::family(&data).filter(|f| OFFICE_FAMILIES.contains(&f.as_str()))?;
            Some((data, family))
        })
        .collect();
    if named.is_empty() { 0 } else { db.add_document_fonts(named) }
}
