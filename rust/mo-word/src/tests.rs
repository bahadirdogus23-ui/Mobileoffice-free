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
fn one_page_text_matches_all_page_texts() {
    let doc = WordDoc::open("sample.docx", &sample_docx(), None).unwrap();
    let texts = doc.page_texts();
    assert!(texts.len() > 1);
    for (i, t) in texts.iter().enumerate() {
        assert_eq!(&doc.page_text(i as u32).unwrap(), t, "page {i}");
    }
    assert!(matches!(doc.page_text(texts.len() as u32), Err(WordError::NoPage(_))));
    let one = WordDoc::open("a.md", "Merhaba dünya.\n\nİkinci paragraf.\n".as_bytes(), None).unwrap();
    assert_eq!(one.page_text(0).unwrap(), "Merhaba dünya.\nİkinci paragraf.");
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
    if let Ok(dir) = std::env::var("MO_INTEROP_DIR") {
        std::fs::write(std::path::Path::new(&dir).join("word.docx"), doc.export("a.docx").unwrap()).unwrap();
    }
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
    // Tabs stay tabs (labels line up on tab stops), and paragraphs are single-spaced.
    assert!(texts.contains("Dava: Alacak\tKarar No"), "{texts}");
    assert!(ink(&doc.render_page(0, 1.0, false).unwrap()) > 500);
    // A4 from the page format, and the bold/underline formatting survives into DOCX.
    let (w, h) = doc.page_size(0).unwrap();
    assert!((w - 595.3).abs() < 1.0 && (h - 841.9).abs() < 1.0, "{w}×{h}");
    let docx = doc.export("karar.docx").unwrap();
    if let Ok(dir) = std::env::var("MO_INTEROP_DIR") {
        std::fs::write(std::path::Path::new(&dir).join("udf.docx"), &docx).unwrap();
    }
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

#[test]
fn types_formats_and_undoes_where_the_page_was_tapped() {
    let mut doc = WordDoc::open("a.md", "Birinci paragraf.\n\nİkinci paragraf.\n".as_bytes(), None).unwrap();
    assert!(!doc.is_dirty());
    // A tap at the start of the first line of text puts the caret there.
    let first = doc.find("Birinci")[0].clone();
    doc.tap(first.page, first.x + 0.5, first.y + first.h / 2.0, false).unwrap();
    let caret = doc.caret().unwrap();
    assert_eq!(caret.page, first.page);
    assert!((caret.x - first.x).abs() < 2.0 && caret.height > 5.0, "{caret:?} vs {first:?}");
    doc.execute("text.insert", r#"{"text": "Şimdi İĞDIR ığüş "}"#).unwrap();
    assert!(doc.is_dirty());
    assert!(doc.caret().unwrap().x > caret.x + 20.0, "the caret moves on with the text");
    assert_eq!(doc.find("şimdi iğdir").len(), 1);
    // Select the new word and make it bold.
    let hit = doc.find("Şimdi")[0].clone();
    doc.tap(hit.page, hit.x + 0.5, hit.y + hit.h / 2.0, false).unwrap();
    doc.tap(hit.page, hit.x + hit.w - 0.5, hit.y + hit.h / 2.0, true).unwrap();
    assert_eq!(doc.selected_text(), "Şimdi");
    assert!(!doc.selection().is_empty());
    doc.execute("format.bold", "").unwrap();
    let state: serde_json::Value = serde_json::from_str(&doc.execute("format.state", "").unwrap()).unwrap();
    assert_eq!(state["bold"], true);
    // New paragraph and backspace, then undo all the way back.
    doc.execute("select.collapse", r#"{"end": true}"#).unwrap();
    doc.execute("text.newParagraph", "").unwrap();
    doc.execute("text.backspace", "").unwrap();
    let docx = doc.export("a.docx").unwrap();
    let body = zip_entry(&docx, "word/document.xml");
    assert!(body.contains("Şimdi") && body.contains("<w:b/>"), "bold kept in DOCX");
    for _ in 0..10 {
        let _ = doc.execute("edit.undo", "");
    }
    assert!(doc.find("Şimdi").is_empty(), "undo takes the typing back");
    assert!(doc.execute("no.such.command", "").is_err());
    assert!(doc.execute("text.insert", "not json").is_err());
}

#[test]
fn selects_by_dragging_pastes_and_sizes_text() {
    let mut doc = WordDoc::open("a.md", "Bir iki üç dört.\n".as_bytes(), None).unwrap();
    let a = doc.find("iki")[0].clone();
    let b = doc.find("üç")[0].clone();
    // Long press on "iki", drag to the end of "üç": the selection runs from one to the other.
    doc.tap(a.page, a.x + 0.5, a.y + a.h / 2.0, false).unwrap();
    doc.tap(b.page, b.x + b.w - 0.2, b.y + b.h / 2.0, true).unwrap();
    assert_eq!(doc.selected_text(), "iki üç");
    doc.execute("format.size", r#"{"size": 20}"#).unwrap();
    let state: serde_json::Value = serde_json::from_str(&doc.execute("format.state", "").unwrap()).unwrap();
    assert_eq!(state["size"], 20.0);
    // Cut is copy + delete in the app; paste puts plain or HTML text at the caret.
    doc.execute("text.delete", "").unwrap();
    assert!(doc.find("iki").is_empty());
    doc.execute("edit.paste", r#"{"text": "Yapıştırılan", "html": "<b>Yapıştırılan</b>"}"#).unwrap();
    assert_eq!(doc.find("yapistirilan").len(), 1);
    let body = zip_entry(&doc.export("a.docx").unwrap(), "word/document.xml");
    assert!(body.contains("Yapıştırılan") && body.contains("<w:b/>"), "HTML paste keeps bold");
}

#[test]
fn inserts_tables_pictures_breaks_headings_and_replaces() {
    let mut doc = WordDoc::open("a.md", "Başlık olacak\n\nMetin burada. Metin bitti.\n".as_bytes(), None).unwrap();
    let pages = doc.page_count();
    let h = doc.find("Başlık")[0].clone();
    doc.tap(h.page, h.x + 0.5, h.y + h.h / 2.0, false).unwrap();
    doc.execute("para.heading1", "").unwrap();
    let state: serde_json::Value = serde_json::from_str(&doc.execute("format.state", "").unwrap()).unwrap();
    assert!(state.to_string().contains("Heading") || state.to_string().contains("Başlık"), "{state}");
    doc.execute("caret.docEnd", "").unwrap();
    doc.execute("text.pageBreak", "").unwrap();
    assert_eq!(doc.page_count(), pages + 1);
    doc.execute("insert.table", r#"{"rows": 2, "cols": 3}"#).unwrap();
    // A 1×1 red PNG.
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC";
    doc.execute("insert.picture", &serde_json::json!({ "data": png, "width": 100.0, "alt": "Kırmızı nokta" }).to_string()).unwrap();
    let out: serde_json::Value = serde_json::from_str(&doc.execute("edit.replaceAll", r#"{"text": "Metin", "with": "Yazı"}"#).unwrap()).unwrap();
    assert!(doc.find("metin").is_empty(), "{out}");
    assert_eq!(doc.find("yazı").len(), 2);
    let body = zip_entry(&doc.export("a.docx").unwrap(), "word/document.xml");
    assert!(body.contains("<w:tbl>") && body.contains("<w:drawing>") && body.contains("w:type=\"page\""), "table, picture and page break in DOCX");
}

#[test]
fn a_tap_on_a_picture_selects_it_for_sizing() {
    let mut doc = WordDoc::open("a.md", "Önce bir satır.\n".as_bytes(), None).unwrap();
    doc.execute("caret.docEnd", "").unwrap();
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC";
    doc.execute("insert.picture", &serde_json::json!({ "data": png, "width": 100.0 }).to_string()).unwrap();
    // Find the picture on the first page by probing a grid of points.
    let (w, h) = doc.page_size(0).unwrap();
    let mut at = None;
    'find: for yi in 0..200 {
        for xi in 0..100 {
            let (x, y) = (xi as f32 * w / 100.0, yi as f32 * h / 200.0);
            if doc.layout.object_at(0, x, y, 0.0).is_some() {
                at = Some((x, y));
                break 'find;
            }
        }
    }
    let (x, y) = at.expect("the picture is on the page");
    assert!(!doc.object_selected());
    assert!(doc.selected_object_box().is_none());
    doc.tap(0, x, y, false).unwrap();
    assert!(doc.object_selected());
    let b = doc.selected_object_box().expect("the picked picture's frame");
    // Inserted 100 pixels wide: 75 points.
    assert!(b.page == 0 && (b.w - 75.0).abs() < 1.0 && b.x <= x && x <= b.x + b.w && b.y <= y && y <= b.y + b.h, "{b:?}");
    doc.execute("picture.size", r#"{"width": 200}"#).unwrap();
    assert!((doc.selected_object_box().unwrap().w - 200.0).abs() < 1.0);
    let hit = doc.layout.object_at(0, x + 1.0, y + 1.0, 0.0).expect("still there");
    assert!((hit.rect.w - 200.0).abs() < 1.0, "{:?}", hit.rect);
    // Typing with the picture selected (the app first steps past it) keeps the picture.
    doc.execute("caret.right", "").unwrap();
    assert!(!doc.object_selected());
    doc.execute("text.insert", r#"{"text": "yazı"}"#).unwrap();
    assert!(doc.layout.object_at(0, x + 1.0, y + 1.0, 0.0).is_some(), "the picture is still there");
    // A tap on text puts the caret there instead.
    let t = doc.find("satır")[0].clone();
    doc.tap(t.page, t.x + 1.0, t.y + t.h / 2.0, false).unwrap();
    assert!(!doc.object_selected());
}

/// A DOCX as Word writes it: only the styles it uses, so no "Table Grid".
fn docx_without_table_grid() -> Vec<u8> {
    use std::io::Write;
    let mut out = std::io::Cursor::new(Vec::new());
    let mut z = zip::ZipWriter::new(&mut out);
    let opts = zip::write::SimpleFileOptions::default();
    let parts = [
        (
            "[Content_Types].xml",
            r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        (
            "word/_rels/document.xml.rels",
            r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#,
        ),
        (
            "word/styles.xml",
            r#"<?xml version="1.0"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/></w:style></w:styles>"#,
        ),
        (
            "word/document.xml",
            r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Merhaba</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#,
        ),
    ];
    for (name, body) in parts {
        z.start_file(name, opts).unwrap();
        z.write_all(body.as_bytes()).unwrap();
    }
    z.finish().unwrap();
    out.into_inner()
}

#[test]
fn a_new_table_has_lines_in_a_file_without_table_grid() {
    let mut doc = WordDoc::open("a.docx", &docx_without_table_grid(), None).unwrap();
    doc.execute("caret.docEnd", "").unwrap();
    doc.execute("insert.table", r#"{"rows": 2, "cols": 3}"#).unwrap();
    let r = doc.render_page(0, 1.0, false).unwrap();
    // Below the first line (from 1.25 in), the table's grid draws dark pixels.
    let dark = (90..140u32).flat_map(|y| (0..r.width).map(move |x| (y * r.width + x) as usize * 4)).filter(|&i| r.pixels[i] < 128).count();
    assert!(dark > 100, "the table's lines are drawn ({dark} dark pixels)");
    let styles = zip_entry(&doc.export("a.docx").unwrap(), "word/styles.xml");
    assert!(styles.contains("TableGrid"), "the style is saved with the file");
}

#[test]
fn contacts_calendars_and_mails_open_as_documents() {
    let vcf = "BEGIN:VCARD\nVERSION:3.0\nFN:Ahmet Yılmaz\nTEL;TYPE=CELL:0532 111 22 33\nEND:VCARD\n";
    let doc = WordDoc::open("rehber.vcf", vcf.as_bytes(), None).unwrap();
    assert_eq!(doc.find("ahmet yilmaz").len(), 1);
    assert!(doc.find("0532 111 22 33").len() == 1);
    let eml = "From: a@b.c\r\nSubject: Merhaba\r\nContent-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nŞirket notu.\r\n--x\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=\"fatura.pdf\"\r\n\r\n%PDF-1.4\r\n--x--\r\n";
    let doc = WordDoc::open("ileti.eml", eml.as_bytes(), None).unwrap();
    assert_eq!(doc.find("şirket notu").len(), 1);
    assert_eq!(doc.attachment_names(), ["fatura.pdf"]);
    assert!(doc.attachment(0).unwrap().starts_with(b"%PDF"));
    assert!(!doc.notes().is_empty());
}

/// The lossless-save corpus (scripts/office_corpus.py): every `*.docx` in `MO_CORPUS_DIR` is
/// opened, a word is typed at its start through the engine and it is saved as `*.saved.docx`,
/// which the independent reader then compares with the original. Nothing to do without the
/// variable.
#[test]
fn corpus_round_trip() {
    let Ok(dir) = std::env::var("MO_CORPUS_DIR") else { return };
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".docx").filter(|s| !s.ends_with(".saved")) else { continue };
        let mut doc = WordDoc::open(&name, &std::fs::read(&path).unwrap(), None).unwrap();
        doc.execute("caret.docStart", "").unwrap();
        doc.execute("text.insert", r#"{"text": "Düzenlendi "}"#).unwrap();
        std::fs::write(path.with_file_name(format!("{stem}.saved.docx")), doc.export(&name).unwrap()).unwrap();
        std::fs::write(path.with_file_name(format!("{stem}.docx.warnings.txt")), doc.notes().join("\n")).unwrap();
        seen += 1;
    }
    assert!(seen > 0, "no .docx in {dir}");
}

/// `bytes` (a zip) with entry `name` replaced by `data`, the other entries as they were.
fn with_entry(bytes: &[u8], name: &str, data: &[u8]) -> Vec<u8> {
    use std::io::{Read, Write};
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).unwrap();
        let entry = f.name().to_string();
        let mut old = Vec::new();
        f.read_to_end(&mut old).unwrap();
        out.start_file(entry.as_str(), opts).unwrap();
        out.write_all(if entry == name { data } else { &old }).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// WordCraft skips a DOCX part it cannot parse (styles, notes, comments…) with only a log line
/// and opens the rest; saving that over the original would delete the part for good.
#[test]
fn a_docx_part_the_engine_skips_is_reported() {
    let docx = sample_docx();
    let whole = WordDoc::open("el-kitabi.docx", &docx, None).unwrap();
    assert!(whole.losses().is_empty(), "{:?}", whole.losses());
    assert!(!whole.opened_incomplete());

    let styles = zip_entry(&docx, "word/styles.xml");
    let cut = with_entry(&docx, "word/styles.xml", &styles.as_bytes()[..styles.len() / 2]);
    let doc = WordDoc::open("el-kitabi.docx", &cut, None).expect("the engine opens it without the styles");
    assert_eq!(doc.losses(), [Loss::Damaged]);
    assert!(doc.opened_incomplete());

    // A damaged entry (its checksum no longer matches) is skipped the same way.
    let mut bad = with_entry(&docx, "docProps/app.xml", b"<Properties>0123456789abcdef</Properties>");
    let at = bad.windows(16).position(|w| w == b"0123456789abcdef").unwrap();
    bad[at] = b'X';
    assert_eq!(WordDoc::open("el-kitabi.docx", &bad, None).unwrap().losses(), [Loss::Damaged]);

    // Another format under a Word name is opened by content; saving would convert it.
    let rtf = WordDoc::open("rapor.docx", br"{\rtf1\ansi Merhaba\par}", None).unwrap();
    assert_eq!(rtf.losses(), [Loss::OtherFormat]);
}

/// The ODT reader keeps paragraphs, lists, tables and pictures only: notes, comments, text
/// boxes, shapes and form controls are not read.
#[test]
fn what_the_odt_reader_skips_is_reported() {
    let odt = WordDoc::open("el-kitabi.docx", &sample_docx(), None).unwrap().export("el-kitabi.odt").unwrap();
    assert!(WordDoc::open("el-kitabi.odt", &odt, None).unwrap().losses().is_empty());

    let content = zip_entry(&odt, "content.xml");
    let at = content.find("</text:p>").unwrap();
    let note = r#"<text:note text:id="n1" text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>Dipnot</text:p></text:note-body></text:note>"#;
    let with_note = format!("{}{note}{}", &content[..at], &content[at..]);
    let doc = WordDoc::open("el-kitabi.odt", &with_entry(&odt, "content.xml", with_note.as_bytes()), None).unwrap();
    assert_eq!(doc.losses(), [Loss::Notes]);

    let comment = r#"<office:annotation><text:p>Not</text:p></office:annotation>"#;
    let with_comment = format!("{}{comment}{}", &content[..at], &content[at..]);
    let doc = WordDoc::open("el-kitabi.odt", &with_entry(&odt, "content.xml", with_comment.as_bytes()), None).unwrap();
    assert_eq!(doc.losses(), [Loss::Comments]);

    // Content cut short: the reader stops where the XML breaks.
    let cut = with_entry(&odt, "content.xml", &content.as_bytes()[..content.len() * 2 / 3]);
    assert!(WordDoc::open("el-kitabi.odt", &cut, None).unwrap().losses().contains(&Loss::Damaged));

    assert_eq!(losses::find("x.odt", b"{\\rtf1 x}"), [Loss::OtherFormat]);
}

#[test]
fn what_the_rtf_reader_skips_is_reported() {
    let plain = br"{\rtf1\ansi{\fonttbl{\f0 Arial;}}{\*\generator Word;}Merhaba \b d\'fcnya\b0\par}";
    assert!(WordDoc::open("a.rtf", plain, None).unwrap().losses().is_empty());
    let note = WordDoc::open("a.rtf", br"{\rtf1\ansi Metin{\super\chftn}{\footnote\pard\plain\chftn Dipnot}\par}", None).unwrap();
    assert_eq!(note.losses(), [Loss::Notes]);
    assert_eq!(losses::find("a.rtf", br"{\rtf1 a{\*\atnid AY}{\*\annotation{\pard yorum}}\par}"), [Loss::Comments]);
    assert_eq!(losses::find("a.rtf", br"{\rtf1 {\object\objemb{\*\objdata 0102}{\result x}}\par}"), [Loss::Objects]);
    assert_eq!(losses::find("a.rtf", br"{\rtf1 {\shp{\*\shpinst{\sp{\sn fillColor}{\sv 255}}}}\par}"), [Loss::Shapes]);
    // A Windows metafile is dropped, unless it is only the fallback beside a PNG.
    assert_eq!(losses::find("a.rtf", br"{\rtf1 {\pict\wmetafile8\picw10\pich10 0102}\par}"), [Loss::Pictures]);
    assert!(losses::find("a.rtf", br"{\rtf1 {\*\shppict{\pict\pngblip 0102}}{\nonshppict{\pict\wmetafile8 0102}}\par}").is_empty());
    // Binary data is skipped unread, whatever it looks like.
    assert!(losses::find("a.rtf", b"{\\rtf1 {\\pict\\pngblip\\bin9 \\footnote}\\par}").is_empty());
    assert_eq!(losses::find("a.rtf", b"<html>x</html>"), [Loss::OtherFormat]);
}

#[test]
fn what_web_pages_and_markdown_lose_is_reported() {
    let page = "<html><body><h1>Başlık</h1><p>Metin <b>kalın</b></p></body></html>";
    assert!(WordDoc::open("a.html", page.as_bytes(), None).unwrap().losses().is_empty());
    assert!(losses::find("a.html", b"<p><img src=\"data:image/png;base64,AAAA\" alt=\"x\"></p>").is_empty());
    assert_eq!(losses::find("a.html", b"<p><IMG SRC='foto.jpg'></p>"), [Loss::WebContent]);
    assert_eq!(losses::find("a.htm", b"<head><style>p{color:red}</style></head><p>x</p>"), [Loss::WebContent]);
    assert_eq!(losses::find("a.html", b"<p>x</p><script>alert(1)</script>"), [Loss::WebContent]);
    // `<strong>` is not `<script>`, nor `<s>` a style sheet.
    assert!(losses::find("a.html", b"<p><strong>x</strong><s>y</s><small>z</small></p>").is_empty());

    let md = "# Başlık\n\nMetin **kalın** ve [bağlantı](https://ornek.org).\n";
    assert!(WordDoc::open("a.md", md.as_bytes(), None).unwrap().losses().is_empty());
    assert_eq!(losses::find("a.md", b"# Logo\n\n![logo](logo.png)\n"), [Loss::Pictures]);
    assert_eq!(losses::find("a.md", b"---\ntitle: x\n---\n# A\n"), [Loss::Other]);
    assert_eq!(losses::find("a.md", b"<div align=\"center\">\n<img src=\"a.png\">\n</div>\n"), [Loss::WebContent]);
    assert!(losses::find("a.md", b"```html\n<script>x</script>\n```\n").is_empty());
    // Formats not saved over the opened file are not checked.
    assert!(losses::find("a.doc", b"anything").is_empty());
}
