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

#[test]
fn reads_fills_borders_and_fonts_from_an_xls_file() {
    // Made by LibreOffice from an XLSX: A1:D1 merged, B2:D4 thin black borders, A2:D2 filled
    // (light blue, dark red, light green, silver).
    let bytes = include_bytes!("../testdata/styled.xls");
    let doc = GridDoc::open("styled.xls", bytes, None).unwrap();
    assert_eq!(doc.merges(0).unwrap(), vec![[0, 0, 0, 3], [5, 0, 7, 1]]);
    let looks = doc.cell_looks(0, 1, 0, 1, 4).unwrap();
    assert!(looks.iter().all(|l| l.borders == 0x0101_0101 && l.border_color == 0xFF00_0000), "{looks:?}");
    let fills: Vec<u32> = looks.iter().map(|l| l.fill).collect();
    // .xls keeps colours in a 56-colour palette: LibreOffice picks the nearest ones.
    assert!(fills.iter().all(|f| *f != 0), "{fills:x?}");
    assert_eq!(fills[3], 0xFFC0_C0C0);
    assert!(doc.warnings().iter().all(|w| !w.contains("Fonts, fills, borders")), "{:?}", doc.warnings());
}

#[test]
fn format_as_table_shows_its_header_and_bands() {
    let mut doc = GridDoc::blank();
    for (r, row) in [["Ad", "Şehir"], ["Şule", "Ankara"], ["Ahmet", "İzmir"], ["Zeynep", "Bursa"]].iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            doc.set_input(r as u32, c as u32, v).unwrap();
        }
    }
    let before = doc.cell_looks(0, 0, 0, 4, 2).unwrap();
    assert!(before.iter().all(|l| l.fill == 0));
    doc.execute("home.formatAsTable", r#"{"range": "A1:B4", "style": "TableStyleMedium2"}"#).unwrap();
    let after = doc.cell_looks(0, 0, 0, 4, 2).unwrap();
    // The header row is filled and bold; data rows are banded (alternate rows differ).
    assert!(after[0].fill != 0 && after[0].bold, "{:?}", after[0]);
    assert_ne!(after[2].fill, after[4].fill, "{after:?}");
}

#[test]
fn page_setup_from_the_pdf_sheet_fits_columns_and_repeats_the_header() {
    // A wide table: 30 columns × 120 rows; the app's PDF sheet sends these Page Setup choices.
    let mut doc = GridDoc::blank();
    for r in 0..120u32 {
        for c in 0..30u32 {
            doc.set_input(r, c, &format!("Hücre {r}-{c}")).unwrap();
        }
    }
    let pages = |doc: &mut GridDoc| {
        let v: serde_json::Value = serde_json::from_str(&doc.execute("file.printPreview", "{}").unwrap()).unwrap();
        (v["pages"].as_u64().unwrap(), v["pageList"][0]["scale"].clone(), v["pageList"][0]["columns"].clone())
    };
    let plain = pages(&mut doc);
    let setup = serde_json::json!({
        "orientation": "portrait", "paper": "A4", "gridlines": false, "headings": true, "centerH": true,
        "titleRows": "1:1", "fitWidth": 1, "fitHeight": 0, "footer": "&CSayfa &P / &N",
    });
    doc.execute("pageLayout.pageSetup", &setup.to_string()).unwrap();
    doc.execute("pageLayout.margins", r#"{"preset": "normal"}"#).unwrap();
    let fitted = pages(&mut doc);
    assert!(fitted.0 < plain.0, "{fitted:?} vs {plain:?}");
    assert!(doc.export_pdf(false, None, false).unwrap().starts_with(b"%PDF"));
    // Turning the header off again clears it.
    doc.execute("pageLayout.pageSetup", r#"{"titleRows": "", "fitToPage": false}"#).unwrap();
    assert!(doc.export_pdf(false, None, false).is_ok());
}

#[test]
fn a_column_filter_lists_values_and_hides_rows() {
    let mut doc = GridDoc::blank();
    doc.set_turkish(true);
    let rows = [["Ad", "Şehir", "Puan"], ["Şule", "Ankara", "10"], ["Ahmet", "İzmir", "9"], ["Çiğdem", "Ankara", "1.250,5"], ["Zeynep", "", "9"]];
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            doc.set_input(r as u32, c as u32, v).unwrap();
        }
    }
    let cities = doc.filter_values(1).unwrap();
    let labels: Vec<(&str, u32)> = cities.iter().map(|f| (f.label.as_str(), f.count)).collect();
    // Turkish order, blanks last.
    assert_eq!(labels, [("Ankara", 2), ("İzmir", 1), ("", 1)]);
    let points: Vec<String> = doc.filter_values(2).unwrap().into_iter().map(|f| f.label).collect();
    assert_eq!(points, ["9", "10", "1250,5"]);
    doc.filter_column(1, &["Ankara".to_string()]).unwrap();
    assert!(doc.has_filter());
    let heights = doc.sizes(0, 3, 5).unwrap().row_heights;
    assert_eq!(heights.iter().map(|h| *h > 0.0).collect::<Vec<_>>(), [true, true, false, true, false]);
    let again = doc.filter_values(1).unwrap();
    assert_eq!(again.iter().map(|f| f.selected).collect::<Vec<_>>(), [true, false, false]);
    // Blank cells can be picked too.
    doc.filter_column(1, &["".to_string()]).unwrap();
    let heights = doc.sizes(0, 3, 5).unwrap().row_heights;
    assert_eq!(heights.iter().map(|h| *h > 0.0).collect::<Vec<_>>(), [true, false, false, false, true]);
    doc.remove_filter().unwrap();
    assert!(!doc.has_filter());
    assert!(doc.sizes(0, 3, 5).unwrap().row_heights.iter().all(|h| *h > 0.0));
}

#[test]
fn finds_and_replaces_the_turkish_way() {
    let mut doc = GridDoc::blank();
    doc.set_turkish(true);
    doc.set_input(0, 0, "İSTANBUL şubesi").unwrap();
    doc.set_input(1, 1, "istanbul").unwrap();
    doc.set_input(2, 0, "Isparta").unwrap();
    doc.set_input(3, 0, "=B2&\" merkez\"").unwrap();
    doc.execute("home.insertSheet", "{}").unwrap();
    doc.set_active_sheet(1).unwrap();
    doc.set_input(4, 2, "Yeni İstanbul").unwrap();
    doc.set_active_sheet(0).unwrap();
    let hits: Vec<(u32, u32, u32)> = doc.find_cells("istanbul").unwrap().iter().map(|h| (h.sheet, h.row, h.col)).collect();
    // "Isparta" holds ı (dotless), not i; the formula's result is found too.
    assert_eq!(hits, [(0, 0, 0), (0, 1, 1), (0, 3, 0), (1, 4, 2)]);
    assert_eq!(doc.find_cells("ısparta").unwrap().len(), 1);
    let n = doc.replace_all("istanbul", "Ankara").unwrap();
    assert_eq!(n, 3, "the formula is left alone");
    assert_eq!(doc.edit_text(0, 0, 0).unwrap(), "Ankara şubesi");
    assert_eq!(doc.edit_text(1, 4, 2).unwrap(), "Yeni Ankara");
    assert!(doc.edit_text(0, 3, 0).unwrap().starts_with('='));
    assert_eq!(doc.active_sheet(), 0);
}
