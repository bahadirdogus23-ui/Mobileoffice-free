use super::*;

fn csv_1254(text: &str) -> Vec<u8> {
    encoding_rs::WINDOWS_1254.encode(text).0.into_owned()
}

#[test]
fn turkish_windows_1254_csv_keeps_its_letters() {
    let bytes = csv_1254("İl;İlçe;Tutar\nİstanbul;Şişli;1250\nİzmir;Karşıyaka;980\n");
    assert!(looks_like_windows_1254(&bytes));
    let doc = GridDoc::open("satislar.csv", &bytes, None).unwrap();
    assert_eq!(doc.used_size(0).unwrap(), UsedSize { rows: 3, cols: 3 });
    let texts = doc.cell_texts(0, 0, 0, 3, 3).unwrap();
    assert_eq!(texts[0], "İl");
    assert_eq!(texts[1], "İlçe");
    assert_eq!(texts[4], "Şişli");
    assert_eq!(texts[7], "Karşıyaka");
    assert!(doc.warnings().iter().any(|w| w.contains("Windows-1254")));
}

#[test]
fn utf8_and_western_text_are_left_alone() {
    assert!(!looks_like_windows_1254("Şişli;ğüşıöç".as_bytes()));
    // Windows-1252 "café" (0xE9) has no Turkish-only byte.
    assert!(!looks_like_windows_1254(b"caf\xe9;1\n"));
}

#[test]
fn blank_workbook_edits_through_commands_and_round_trips_as_xlsx() {
    let mut doc = GridDoc::blank();
    assert_eq!(doc.used_size(0).unwrap(), UsedSize { rows: 0, cols: 0 });
    doc.execute("cell.set", r#"{"cell":"A1","input":"Gelir"}"#).unwrap();
    doc.execute("cell.set", r#"{"cell":"B1","input":"1200"}"#).unwrap();
    doc.execute("cell.set", r#"{"cell":"B2","input":"=B1*2"}"#).unwrap();
    assert!(doc.is_dirty());
    assert_eq!(doc.cell_texts(0, 0, 0, 2, 2).unwrap(), vec!["Gelir", "1200", "", "2400"]);
    assert_eq!(doc.edit_text(0, 1, 1).unwrap(), "=B1*2");

    let bytes = doc.save("rapor.xlsx").unwrap();
    let back = GridDoc::open("rapor.xlsx", &bytes, None).unwrap();
    assert_eq!(back.cell_texts(0, 0, 0, 2, 2).unwrap(), vec!["Gelir", "1200", "", "2400"]);
}

#[test]
fn bad_input_is_an_error_not_a_crash() {
    assert!(matches!(GridDoc::open("x.xlsx", b"PK\x03\x04 not really a zip", None), Err(GridError::Open(_))));
    let mut doc = GridDoc::blank();
    assert!(matches!(doc.execute("no.such.command", "{}"), Err(GridError::Command(_))));
    assert!(matches!(doc.execute("cell.set", "{not json"), Err(GridError::Command(_))));
    assert_eq!(doc.used_size(9), Err(GridError::NoSheet(9)));
}

#[test]
fn exports_a_pdf() {
    let bytes = "Ad;Tutar\nŞişli İzmir;1250\n".as_bytes();
    let mut doc = GridDoc::open("a.csv", bytes, None).unwrap();
    let pdf = doc.export_pdf(true, None, false).unwrap();
    assert!(pdf.starts_with(b"%PDF"), "{:?}", &pdf[..8.min(pdf.len())]);
    // Turkish letters are single-byte codes of an embedded TrueType face (vendor/gridcraft-pdf),
    // not a CID font phones do not have: "Şişli" is \336i\376li in the page text.
    let text = String::from_utf8_lossy(&pdf);
    assert!(!text.contains("STSong"));
    assert!(text.contains("\\336i\\376li"));
    assert!(text.contains("/Subtype /TrueType /BaseFont /SourceSans3"), "fonts embedded");
    assert!(text.contains("/FontFile2"));
}

#[test]
fn pdf_follows_page_setup_commands_and_range() {
    let bytes = "A;B;C\n1;2;3\n4;5;6\n".as_bytes();
    let mut doc = GridDoc::open("a.csv", bytes, None).unwrap();
    doc.execute("pageLayout.orientation", r#"{"orientation":"landscape"}"#).unwrap();
    doc.execute("pageLayout.size", r#"{"paper":"A4"}"#).unwrap();
    doc.execute("pageLayout.margins", r#"{"preset":"narrow"}"#).unwrap();
    doc.execute("pageLayout.printGridlines", r#"{"on":true}"#).unwrap();
    let pdf = doc.export_pdf(false, Some("A1:B2"), true).unwrap();
    let text = String::from_utf8_lossy(&pdf);
    // A4 landscape: 842 × 595 points.
    assert!(text.contains("/MediaBox [0 0 841.89 595.28]"), "{}", &text[..text.len().min(2000)]);
    assert!(doc.export_pdf(false, Some("bad range"), false).is_err());
}

#[test]
fn formats_cells_and_inserts_rows_through_commands() {
    let mut doc = GridDoc::blank();
    doc.execute("cell.set", r#"{"cell":"A1","input":"Başlık"}"#).unwrap();
    doc.set_turkish(true);
    doc.set_input(1, 0, "1.250,5").unwrap();
    assert_eq!(doc.cell_texts(0, 1, 0, 1, 1).unwrap()[0], "1250,5");
    assert_eq!(doc.edit_text(0, 1, 0).unwrap(), "1250,5");
    doc.set_input(2, 0, "=TOPLA(A2;10,5)").unwrap();
    assert_eq!(doc.cell_texts(0, 2, 0, 1, 1).unwrap()[0], "1261");
    assert_eq!(doc.edit_text(0, 2, 0).unwrap(), "=TOPLA(A2;10,5)");
    doc.set_input(3, 0, "31.10.2026").unwrap();
    assert!(doc.cell_texts(0, 3, 0, 1, 1).unwrap()[0].contains("2026"), "a date stays a date");
    doc.execute("home.bold", r#"{"range":"A1"}"#).unwrap();
    doc.execute("home.fillColor", r##"{"range":"A1","color":"#FFFF00"}"##).unwrap();
    doc.execute("home.alignCenter", r#"{"range":"A1"}"#).unwrap();
    let looks = doc.cell_looks(0, 0, 0, 2, 1).unwrap();
    // Saved files keep the English forms other programs read.
    assert!(doc.execute("cell.get", r#"{"cell":"A3"}"#).unwrap().contains("SUM(A2,10.5)"));
    assert!(looks[0].bold && looks[0].align == 2);
    assert_eq!(looks[0].fill, 0xFFFF_FF00);
    assert_eq!(looks[1], CellLook::default());
    // A row inserted above the number moves it down a row.
    doc.execute("home.insertRows", r#"{"rows":"2:2"}"#).unwrap();
    assert_eq!(doc.cell_texts(0, 1, 0, 2, 1).unwrap()[0], "");
    assert!(!doc.cell_texts(0, 2, 0, 1, 1).unwrap()[0].is_empty());
    doc.execute("edit.undo", "{}").unwrap();
    assert!(!doc.cell_texts(0, 1, 0, 1, 1).unwrap()[0].is_empty());
    doc.execute("home.percent", r#"{"range":"A2"}"#).unwrap();
    assert_eq!(doc.cell_texts(0, 1, 0, 1, 1).unwrap()[0], "125050%");
}

#[test]
fn sorts_and_manages_sheets() {
    let mut doc = GridDoc::blank();
    doc.set_turkish(true);
    for (r, (name, n)) in [("Şule", "3"), ("Ahmet", "1"), ("Çiğdem", "2"), ("Zeynep", "10")].iter().enumerate() {
        doc.set_input(r as u32, 0, name).unwrap();
        doc.set_input(r as u32, 1, n).unwrap();
    }
    doc.sort_column(1, false).unwrap();
    assert_eq!(doc.cell_texts(0, 0, 0, 4, 1).unwrap(), ["Zeynep", "Şule", "Çiğdem", "Ahmet"]);
    doc.sort_column(0, true).unwrap();
    // Turkish alphabet order: Ç after C, Ş after S, all before Z; numbers move with their names.
    assert_eq!(doc.cell_texts(0, 0, 0, 4, 1).unwrap(), ["Ahmet", "Çiğdem", "Şule", "Zeynep"]);
    assert_eq!(doc.cell_texts(0, 0, 1, 4, 1).unwrap(), ["1", "2", "3", "10"]);
    doc.execute("home.insertSheet", "{}").unwrap();
    assert_eq!(doc.sheet_names().len(), 2);
    let added = doc.active_sheet();
    doc.execute("sheet.rename", &serde_json::json!({ "sheet": added, "name": "Gelir Gider" }).to_string()).unwrap();
    assert!(doc.sheet_names().contains(&"Gelir Gider".to_string()));
    doc.execute("home.deleteSheet", &serde_json::json!({ "sheet": added }).to_string()).unwrap();
    assert_eq!(doc.sheet_names().len(), 1);
    // The delete dialog promises that Undo brings the sheet back.
    doc.execute("edit.undo", "{}").unwrap();
    assert!(doc.sheet_names().contains(&"Gelir Gider".to_string()));
}

#[test]
fn reports_column_widths_and_row_heights() {
    let mut doc = GridDoc::blank();
    doc.execute("home.columnWidth", r#"{"cols":"B:B","width":30}"#).unwrap();
    let s = doc.sizes(0, 3, 2).unwrap();
    assert_eq!(s.col_widths.len(), 3);
    assert_eq!(s.col_widths[0], s.default_col);
    assert!(s.col_widths[1] != s.default_col, "{s:?}");
    assert_eq!(s.row_heights, vec![s.default_row; 2]);
    assert!(s.default_col > 20.0 && s.default_row > 10.0, "{s:?}");
    assert_eq!((s.frozen_rows, s.frozen_cols), (0, 0));
    doc.execute("view.freezeTopRow", "{}").unwrap();
    assert_eq!(doc.sizes(0, 1, 1).unwrap().frozen_rows, 1);
}

#[test]
fn reports_merged_ranges() {
    let mut doc = GridDoc::blank();
    doc.execute("cell.set", r#"{"cell":"B2","input":"Başlık"}"#).unwrap();
    doc.execute("home.mergeCenter", r#"{"range":"B2:D3"}"#).unwrap();
    assert_eq!(doc.merges(0).unwrap(), vec![[1, 1, 2, 3]]);
}

#[test]
fn reports_cell_borders() {
    let mut doc = GridDoc::blank();
    doc.execute("home.borders", r#"{"range":"B2","preset":"thickOutside"}"#).unwrap();
    doc.execute("home.borders", r#"{"range":"C3","preset":"bottom"}"#).unwrap();
    let l = doc.cell_looks(0, 1, 1, 2, 2).unwrap();
    assert_eq!(l[0].borders, 0x0303_0303, "{:x}", l[0].borders);
    assert_eq!(l[3].borders, 0x0100_0000, "{:x}", l[3].borders);
    assert_eq!(l[1].borders, 0);
}
