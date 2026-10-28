use super::*;

/// The engine's own sample handbook, written out as DOCX bytes: a real multi-page document.
fn sample_docx() -> Vec<u8> {
    wordcraft_engine::io::save_bytes("sample.docx", &wordcraft_engine::sample::sample_document()).unwrap()
}

fn ink(r: &Raster) -> usize {
    r.pixels.as_chunks::<4>().0.iter().filter(|p| p[0] < 128 && p[1] < 128 && p[2] < 128).count()
}

#[test]
fn docx_lays_out_into_pages_and_renders() {
    let doc = WordDoc::open("sample.docx", &sample_docx(), None).unwrap();
    assert!(doc.page_count() >= 2, "{}", doc.page_count());
    let (w, h) = doc.page_size(0).unwrap();
    assert!(w > 500.0 && h > 700.0, "{w}×{h}");
    let r = doc.render_page(0, 1.0).unwrap();
    assert_eq!((r.width, r.height), (w.ceil() as u32, h.ceil() as u32));
    assert_eq!(r.pixels.len(), (r.width * r.height * 4) as usize);
    assert!(ink(&r) > 1000, "page 1 has text");
}

#[test]
fn a_tile_matches_the_same_area_of_the_whole_page() {
    let doc = WordDoc::open("sample.docx", &sample_docx(), None).unwrap();
    let whole = doc.render_page(0, 2.0).unwrap();
    let tile = doc.render_tile(0, 2.0, 200.0, 300.0, 64, 48).unwrap();
    assert_eq!((tile.width, tile.height), (64, 48));
    let mut differ = 0;
    for y in 0..48 {
        for x in 0..64 {
            let t = &tile.pixels[((y * 64 + x) * 4) as usize..][..4];
            let w = &whole.pixels[(((300 + y) * whole.width + 200 + x) * 4) as usize..][..4];
            differ += t.iter().zip(w).any(|(a, b)| a.abs_diff(*b) > 8) as u32;
        }
    }
    assert!(differ < 64 * 48 / 50, "{differ} pixels differ");
}

#[test]
fn turkish_text_renders_with_bundled_fonts() {
    let md = "# Dilekçe\n\nİstanbul Şişli Belediyesi'ne; ğüşıöç ĞÜŞİÖÇ.\n";
    let doc = WordDoc::open("dilekce.md", md.as_bytes(), None).unwrap();
    assert_eq!(doc.page_count(), 1);
    assert!(ink(&doc.render_page(0, 1.0).unwrap()) > 200);
}

#[test]
fn bad_input_and_bad_pages_are_errors() {
    assert!(matches!(WordDoc::open("x.docx", b"PK\x03\x04 broken", None), Err(WordError::Open(_))));
    let doc = WordDoc::open("a.txt", b"merhaba", None).unwrap();
    assert!(matches!(doc.render_page(5, 1.0), Err(WordError::NoPage(5))));
}
