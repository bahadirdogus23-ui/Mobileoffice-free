//! The Office stand-in fonts the app ships as assets (for the slides), handed to WordCraft's
//! process-wide font database as the app does it (FFI `register_word_fonts`). Its own test
//! binary: what the global database holds would otherwise change the other tests' layouts.

fn assets() -> Vec<Vec<u8>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../android/app/src/main/assets/fonts");
    let mut paths: Vec<_> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "ttf")).collect();
    paths.sort();
    paths.iter().map(|p| std::fs::read(p).unwrap()).collect()
}

const TURKISH: &str = "İıŞşĞğÇçÖöÜü";

#[test]
fn documents_lay_out_office_fonts_in_their_bundled_twins() {
    let files = assets();
    // A machine with the same fonts installed (Linux desktops) finds them in its font folders,
    // which the global database always scans; a phone has none.
    let installed = ["Carlito", "Caladea", "Liberation Sans", "Liberation Serif"].iter().any(|f| wordcraft_fonts::FontDb::global().has_family(f));
    if !installed {
        // Looked up before the fonts are there: the lookup must not be remembered afterwards.
        let before = wordcraft_fonts::word::resolve("Calibri", false, false);
        assert_ne!(before.face.get().family, "Carlito");
    }
    assert_eq!(mo_word::register_word_fonts(files.clone()), 16);
    assert_eq!(mo_word::register_word_fonts(files.clone()), 0, "calling again adds nothing");
    // (family, bold, italic) → (family, style drawn).
    let cases = [
        ("Calibri", false, false, "Carlito", "Regular"),
        ("Calibri", true, false, "Carlito", "Bold"),
        ("Calibri", false, true, "Carlito", "Italic"),
        ("Calibri", true, true, "Carlito", "Bold Italic"),
        ("Cambria", true, false, "Caladea", "Bold"),
        ("Arial", false, false, "Liberation Sans", "Regular"),
        ("Arial", true, true, "Liberation Sans", "Bold Italic"),
        ("Times New Roman", false, false, "Liberation Serif", "Regular"),
        ("Times New Roman", false, true, "Liberation Serif", "Italic"),
        ("Times New Roman", true, false, "Liberation Serif", "Bold"),
        // Missing sans and serif families take WordCraft's generic lists, now headed by the twins.
        ("Tahoma", false, false, "Carlito", "Regular"),
        ("Georgia", false, false, "Caladea", "Regular"),
    ];
    for (family, bold, italic, want, style) in cases {
        let r = wordcraft_fonts::word::resolve(family, bold, italic);
        let face = r.face.get();
        assert_eq!((face.family.as_str(), face.style.as_str()), (want, style), "{family} bold={bold} italic={italic}");
        assert!(!r.synth_bold && !r.synth_italic, "{family}: a real face, not a slanted or emboldened one");
        assert!(files.iter().any(|f| f.as_slice() == face.data()), "{family}: drawn from a shipped file, byte for byte");
        for c in TURKISH.chars() {
            assert_ne!(face.glyph_for(c), 0, "{family} → {want} has no {c}");
        }
    }
    // Courier New keeps WordCraft's bundled monospaced face (Liberation Mono is not shipped).
    if !wordcraft_fonts::FontDb::global().has_family("Liberation Mono") {
        assert_eq!(wordcraft_fonts::word::resolve("Courier New", false, false).face.get().family, "JetBrains Mono");
    }

    // A document in Calibri exported to PDF embeds a subset of Carlito.
    let page = mo_word::TextPage { width_pt: 595.0, height_pt: 842.0, landscape: false, margin_pt: 72.0, font: "Calibri".into(), size_pt: 11.0 };
    let doc = mo_word::WordDoc::from_text("Çağrı Işık, Şişli Göğüs İstanbul".as_bytes(), &page);
    let pdf = doc.export("belge.pdf").unwrap();
    assert!(pdf.windows(7).any(|w| w == b"Carlito"), "the PDF names the embedded Carlito");
}
