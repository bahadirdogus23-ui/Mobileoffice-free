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

/// A UDF as UYAP's editor writes it: content.xml in a ZIP, offsets into the CDATA text.
fn sample_udf(signed: bool) -> Vec<u8> {
    let text = "T.C.\nİSTANBUL 1. ASLİYE HUKUK MAHKEMESİ\nDava: Alacak\tKarar No: 2026/15\nAd\tSoyad\n";
    // Offsets in UTF-16 units (all of these letters are one unit each).
    let at = |s: &str| {
        text.encode_utf16()
            .collect::<Vec<_>>()
            .windows(s.encode_utf16().count())
            .position(|w| w == s.encode_utf16().collect::<Vec<_>>().as_slice())
            .unwrap()
    };
    let len = |s: &str| s.encode_utf16().count();
    let line2 = "İSTANBUL 1. ASLİYE HUKUK MAHKEMESİ";
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" ?>
<template format_id="1.8">
<content><![CDATA[{text}]]></content>
<properties><pageFormat mediaSizeName="1" leftMargin="42.5" rightMargin="42.5" topMargin="42.5" bottomMargin="42.5" paperOrientation="1"/></properties>
<elements resolver="hvl-default">
<paragraph Alignment="1"><content startOffset="0" length="5" bold="true"/></paragraph>
<paragraph Alignment="1" size="14"><content startOffset="{a2}" length="{l2}" bold="true" underline="true"/></paragraph>
<paragraph Alignment="3" LeftIndent="20"><content startOffset="{a3}" length="12"/><tab startOffset="{t3}" length="1"/><content startOffset="{a4}" length="18" italic="true"/></paragraph>
<table columnCount="2" columnSpans="100,300" border="borderCell"><row><cell><paragraph><content startOffset="{a5}" length="2"/></paragraph></cell><cell><paragraph><content startOffset="{a6}" length="5"/></paragraph></cell></row></table>
</elements>
<styles><style name="hvl-default" family="Times New Roman" size="12"/></styles>
</template>"#,
        a2 = at(line2),
        l2 = len(line2),
        a3 = at("Dava: Alacak"),
        t3 = at("\tKarar"),
        a4 = at("Karar No"),
        a5 = at("Ad\t"),
        a6 = at("Soyad"),
    );
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut out);
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("content.xml", opts).unwrap();
        std::io::Write::write_all(&mut z, xml.as_bytes()).unwrap();
        z.start_file("documentproperties.xml", opts).unwrap();
        std::io::Write::write_all(&mut z, b"<properties/>").unwrap();
        if signed {
            z.start_file("sign.sgn", opts).unwrap();
            std::io::Write::write_all(&mut z, b"signature").unwrap();
        }
        z.finish().unwrap();
    }
    out.into_inner()
}

#[test]
fn reads_uyap_udf_documents() {
    assert!(udf::is_udf(&sample_udf(false)));
    let doc = WordDoc::open("karar.udf", &sample_udf(false), None).unwrap();
    assert_eq!(doc.page_count(), 1);
    assert!(!doc.is_signed());
    // Text, Turkish letters and the table all arrive.
    let texts = doc.page_texts().join("\n");
    for s in ["T.C.", "İSTANBUL 1. ASLİYE HUKUK MAHKEMESİ", "Dava: Alacak", "Karar No: 2026/15", "Ad", "Soyad"] {
        assert!(texts.contains(s), "{s} in {texts}");
    }
    assert!(!doc.find("asliye").is_empty());
    assert!(ink(&doc.render_page(0, 1.0, false).unwrap()) > 500);
    // A4 from the page format, and the bold/underline formatting survives into DOCX.
    let (w, h) = doc.page_size(0).unwrap();
    assert!((w - 595.3).abs() < 1.0 && (h - 841.9).abs() < 1.0, "{w}×{h}");
    let docx = doc.export("karar.docx").unwrap();
    let body = zip_entry(&docx, "word/document.xml");
    assert!(body.contains("<w:b/>") || body.contains("<w:b "), "bold kept");
    assert!(body.contains("<w:tbl>"), "table kept");
    // Column proportions 100:300 from columnSpans.
    let grid: Vec<f32> = body.split("<w:gridCol w:w=\"").skip(1).filter_map(|s| s.split('"').next()?.parse().ok()).collect();
    assert!(grid.len() == 2 && (grid[1] / grid[0] - 3.0).abs() < 0.1, "{grid:?}");
    assert!(doc.export("karar.pdf").unwrap().starts_with(b"%PDF"));
}

#[test]
fn notices_signed_udf_and_rejects_broken_ones() {
    assert!(WordDoc::open("imzali.udf", &sample_udf(true), None).unwrap().is_signed());
    assert!(matches!(WordDoc::open("bozuk.udf", b"PK\x03\x04 nope", None), Err(WordError::Open(_))));
}
