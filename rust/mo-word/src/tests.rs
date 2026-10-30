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
    let r = doc.render_page(0, 1.0, false).unwrap();
    assert_eq!((r.width, r.height), (w.ceil() as u32, h.ceil() as u32));
    assert_eq!(r.pixels.len(), (r.width * r.height * 4) as usize);
    assert!(ink(&r) > 1000, "page 1 has text");
}

#[test]
fn a_tile_matches_the_same_area_of_the_whole_page() {
    let doc = WordDoc::open("sample.docx", &sample_docx(), None).unwrap();
    let whole = doc.render_page(0, 2.0, false).unwrap();
    let tile = doc.render_tile(0, 2.0, 200.0, 300.0, 64, 48, false).unwrap();
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
    assert!(ink(&doc.render_page(0, 1.0, false).unwrap()) > 200);
}

#[test]
fn bad_input_and_bad_pages_are_errors() {
    assert!(matches!(WordDoc::open("x.docx", b"PK\x03\x04 broken", None), Err(WordError::Open(_))));
    let doc = WordDoc::open("a.txt", b"merhaba", None).unwrap();
    assert!(matches!(doc.render_page(5, 1.0, false), Err(WordError::NoPage(5))));
}

#[test]
fn a_dark_page_has_dark_paper_and_light_text() {
    let doc = WordDoc::open("a.txt", "Gece modu: ğüşıöç İĞÜŞÖÇ".as_bytes(), None).unwrap();
    let light = doc.render_page(0, 1.0, false).unwrap();
    let dark = doc.render_page(0, 1.0, true).unwrap();
    let corner = |r: &Raster| r.pixels[..3].to_vec();
    assert!(corner(&light).iter().all(|&v| v > 240), "{:?}", corner(&light));
    assert!(corner(&dark).iter().all(|&v| v < 80), "{:?}", corner(&dark));
    let bright = dark.pixels.as_chunks::<4>().0.iter().filter(|p| p[0] > 180 && p[1] > 180 && p[2] > 180).count();
    assert!(bright > 50, "text turns light: {bright}");
}

#[test]
fn finds_text_ignoring_turkish_letter_forms() {
    let md = "# İstanbul\n\nIŞIK ve ışık aynı kelime. Çağrı merkezi.\n";
    let doc = WordDoc::open("a.md", md.as_bytes(), None).unwrap();
    let hits = doc.find("isik");
    let matches: std::collections::BTreeSet<u32> = hits.iter().map(|h| h.index).collect();
    assert_eq!(matches.len(), 2, "{hits:?}");
    assert!(hits.iter().all(|h| h.page == 0 && h.w > 1.0 && h.h > 1.0), "{hits:?}");
    assert_eq!(doc.find("ISTANBUL").iter().map(|h| h.index).max(), Some(0));
    assert!(!doc.find("cagri").is_empty());
    assert!(doc.find("yok böyle bir şey").is_empty());
    assert!(doc.find("   ").is_empty());
}

#[test]
fn matches_sit_where_the_text_is_drawn() {
    let doc = WordDoc::open("sample.docx", &sample_docx(), None).unwrap();
    let hits = doc.find("the");
    assert!(hits.len() > 5);
    let (w, h) = doc.page_size(hits[0].page).unwrap();
    for hit in &hits {
        assert!(hit.x >= 0.0 && hit.y >= 0.0 && hit.x + hit.w <= w + 1.0 && hit.y + hit.h <= h + 1.0, "{hit:?}");
    }
}

#[test]
fn page_texts_follow_the_pages() {
    let doc = WordDoc::open("sample.docx", &sample_docx(), None).unwrap();
    let texts = doc.page_texts();
    assert_eq!(texts.len(), doc.page_count() as usize);
    assert!(texts[0].len() > 100, "{}", texts[0]);
    assert!(texts.iter().skip(1).any(|t| !t.is_empty()));
    let one = WordDoc::open("a.md", "Merhaba dünya.\n\nİkinci paragraf.\n".as_bytes(), None).unwrap();
    assert_eq!(one.page_texts(), vec!["Merhaba dünya.\nİkinci paragraf.".to_string()]);
}

#[test]
fn exports_pdf_and_other_word_formats() {
    let doc = WordDoc::open("a.md", "# Başlık\n\nİçerik ğüşıöç.\n".as_bytes(), None).unwrap();
    assert!(doc.export("a.pdf").unwrap().starts_with(b"%PDF"));
    assert!(doc.export("a.docx").unwrap().starts_with(b"PK"));
    assert!(doc.export("a.rtf").unwrap().starts_with(b"{\\rtf"));
    assert!(String::from_utf8(doc.export("a.txt").unwrap()).unwrap().contains("ğüşıöç"));
    let html = String::from_utf8(doc.export("a.html").unwrap()).unwrap();
    assert!(html.contains("<h1") && html.contains("ğüşıöç"), "{html}");
    let md = String::from_utf8(doc.export("a.md").unwrap()).unwrap();
    assert!(md.contains("# Başlık"), "{md}");
    assert!(doc.export("a.odt").unwrap().starts_with(b"PK"));
}

#[test]
fn text_takes_the_page_setup_it_is_given() {
    let page = TextPage { width_pt: 595.3, height_pt: 841.9, landscape: true, margin_pt: 36.0, font: "Source Serif 4".into(), size_pt: 14.0 };
    let doc = WordDoc::from_text("Birinci satır ğüşıöç\nİkinci satır\n".as_bytes(), &page);
    let (w, h) = doc.page_size(0).unwrap();
    assert!((w - 841.9).abs() < 1.0 && (h - 595.3).abs() < 1.0, "landscape A4: {w}×{h}");
    assert!(doc.export("a.pdf").unwrap().starts_with(b"%PDF"));
    let docx = doc.export("a.docx").unwrap();
    // The DOCX carries the chosen font and the 14 pt size (28 half-points) in its Normal style.
    let styles = zip_entry(&docx, "word/styles.xml");
    assert!(styles.contains("Source Serif 4") && styles.contains("w:val=\"28\""), "{}", &styles[..styles.len().min(400)]);
    let portrait = WordDoc::from_text(b"x", &TextPage { landscape: false, ..page });
    let (w, h) = portrait.page_size(0).unwrap();
    assert!(w < h);
}

fn zip_entry(bytes: &[u8], name: &str) -> String {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut z.by_name(name).unwrap(), &mut s).unwrap();
    s
}
