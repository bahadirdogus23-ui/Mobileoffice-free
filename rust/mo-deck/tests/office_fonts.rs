//! The Office stand-in fonts the app ships as assets, handed to DeckCraft's process-wide font
//! database as the app does it (FFI `register_slide_fonts`). Its own test binary: what the
//! global database holds would otherwise change the other tests' layouts.

use mo_deck::{DeckDoc, ObjectStyle};

fn assets() -> Vec<Vec<u8>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../android/app/src/main/assets/fonts");
    let mut paths: Vec<_> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "ttf")).collect();
    paths.sort();
    paths.iter().map(|p| std::fs::read(p).unwrap()).collect()
}

#[test]
fn slides_draw_and_export_office_fonts_in_their_bundled_twins() {
    let files = assets();
    assert_eq!(mo_deck::register_slide_fonts(&files), 16);
    assert_eq!(mo_deck::register_slide_fonts(&files), 0, "calling again adds nothing");
    let db = deckcraft_fonts::FontDb::global();
    assert_eq!(db.document_face("Calibri", "Bold").family, "Carlito");
    assert_eq!(db.document_face("Times New Roman", "Italic").style, "Italic");

    // A text box in Calibri, exported: the PDF embeds a subset of Carlito (allowed by the OFL
    // without renaming), not Inter.
    let mut d = DeckDoc::blank().unwrap();
    let id = d.add_text_box(0, "Çağrı Işık, Şişli Göğüs İstanbul").unwrap();
    d.style_object(0, id, ObjectStyle::Font("Calibri".into())).unwrap();
    d.style_object(0, id, ObjectStyle::Bold(true)).unwrap();
    let pdf = d.export_pdf().unwrap();
    let has = |name: &[u8]| pdf.windows(name.len()).any(|w| w == name);
    assert!(has(b"Carlito-Bold"), "the PDF names the embedded Carlito Bold");
}
