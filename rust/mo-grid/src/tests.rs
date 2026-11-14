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
    if let Ok(dir) = std::env::var("MO_INTEROP_DIR") {
        std::fs::write(std::path::Path::new(&dir).join("grid.xlsx"), &bytes).unwrap();
    }
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
    assert_eq!(looks[1], CellLook { align: 3, ..CellLook::default() });
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
fn general_alignment_uses_values_instead_of_the_first_display_character() {
    let mut doc = GridDoc::blank();
    for (row, input) in ["1250.5", "'123 dosya", "=\"123\"", "=TRUE()", "-42", "'-açıklama", "=1/0"].iter().enumerate() {
        doc.set_input(row as u32, 0, input).unwrap();
    }
    let looks = doc.cell_looks(0, 0, 0, 7, 1).unwrap();
    assert_eq!(looks.iter().map(|l| l.align).collect::<Vec<_>>(), vec![3, 0, 0, 2, 3, 0, 2]);
    doc.execute("home.alignLeft", r#"{"range":"A1"}"#).unwrap();
    assert_eq!(doc.cell_looks(0, 0, 0, 1, 1).unwrap()[0].align, 1);
    let saved = doc.save("alignment.xlsx").unwrap();
    let again = GridDoc::open("alignment.xlsx", &saved, None).unwrap();
    assert_eq!(again.cell_looks(0, 1, 0, 6, 1).unwrap().iter().map(|l| l.align).collect::<Vec<_>>(), vec![0, 0, 2, 3, 0, 2]);
}

#[test]
fn inserting_and_deleting_selected_lines_preserves_surrounding_cells() {
    let mut doc = GridDoc::blank();
    doc.execute("cell.set", r#"{"cell":"A1","input":"Header"}"#).unwrap();
    doc.execute("cell.set", r#"{"cell":"B2","input":"Keep me"}"#).unwrap();
    doc.execute("home.insertRows", r#"{"rows":"2:3"}"#).unwrap();
    assert_eq!(doc.cell_texts(0, 3, 1, 1, 1).unwrap(), vec!["Keep me"]);
    assert_eq!(doc.cell_texts(0, 1, 1, 2, 1).unwrap(), vec!["", ""]);
    doc.execute("home.deleteRows", r#"{"rows":"2:3"}"#).unwrap();
    assert_eq!(doc.cell_texts(0, 1, 1, 1, 1).unwrap(), vec!["Keep me"]);
    doc.execute("home.insertColumns", r#"{"cols":"B:C"}"#).unwrap();
    assert_eq!(doc.cell_texts(0, 1, 3, 1, 1).unwrap(), vec!["Keep me"]);
    doc.execute("home.deleteColumns", r#"{"cols":"B:C"}"#).unwrap();
    assert_eq!(doc.cell_texts(0, 1, 1, 1, 1).unwrap(), vec!["Keep me"]);
    assert_eq!(doc.cell_texts(0, 0, 0, 1, 1).unwrap(), vec!["Header"]);
}

#[test]
fn native_paste_keeps_formulas_and_can_replace_them_with_a_copied_blank() {
    let mut doc = GridDoc::blank();
    doc.set_input(0, 0, "12").unwrap();
    doc.set_input(0, 1, "=A1*2").unwrap();
    doc.execute("home.bold", r#"{"range":"B1"}"#).unwrap();
    doc.execute("edit.copy", r#"{"range":"B1"}"#).unwrap();
    doc.execute("edit.paste", r#"{"at":"B2"}"#).unwrap();
    assert_eq!(doc.edit_text(0, 1, 1).unwrap(), "=A2*2");
    assert!(doc.cell_looks(0, 1, 1, 1, 1).unwrap()[0].bold);
    let blank: serde_json::Value = serde_json::from_str(&doc.execute("edit.copy", r#"{"range":"C1"}"#).unwrap()).unwrap();
    assert!(blank["text"].as_str().unwrap().trim().is_empty());
    doc.execute("edit.paste", r#"{"at":"B2"}"#).unwrap();
    assert_eq!(doc.cell_texts(0, 1, 1, 1, 1).unwrap(), vec![""]);
}

#[test]
fn text_from_another_workbook_pastes_without_a_native_clipboard() {
    let mut source = GridDoc::blank();
    source.set_input(0, 0, "İzmir").unwrap();
    source.set_input(0, 1, "1250.5").unwrap();
    let copied: serde_json::Value = serde_json::from_str(&source.execute("edit.copy", r#"{"range":"A1:B1"}"#).unwrap()).unwrap();
    let mut target = GridDoc::blank();
    target.execute("edit.paste", &serde_json::json!({"at":"B2", "text":copied["text"]}).to_string()).unwrap();
    assert_eq!(target.cell_texts(0, 1, 1, 1, 2).unwrap(), vec!["İzmir", "1250.5"]);
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

#[test]
fn columns_and_rows_take_sizes_dragged_in_points() {
    // Sizes go in and come back in the same unit (Excel's pixels), the one the app's grid draws in.
    let mut doc = GridDoc::blank();
    doc.execute("home.columnWidth", r#"{"cols": "B:B", "width": 133.333}"#).unwrap();
    doc.execute("home.rowHeight", r#"{"rows": "3:3", "height": 40}"#).unwrap();
    let s = doc.sizes(0, 3, 4).unwrap();
    assert!((s.col_widths[1] - 133.333).abs() < 0.5, "{:?}", s.col_widths);
    assert!((s.row_heights[2] - 40.0).abs() < 0.5, "{:?}", s.row_heights);
}

#[test]
fn autosum_adds_up_the_numbers_above() {
    let mut doc = GridDoc::blank();
    doc.set_turkish(true);
    for (r, v) in ["10", "20,5", "30"].iter().enumerate() {
        doc.set_input(r as u32, 0, v).unwrap();
    }
    doc.execute("formulas.autoSum", r#"{"range": "A4"}"#).unwrap();
    let typed = doc.edit_text(0, 3, 0).unwrap();
    assert!(typed.starts_with("=TOPLA(") && typed.contains("A1:A3"), "{typed}");
    assert_eq!(doc.cell_texts(0, 3, 0, 1, 1).unwrap(), ["60,5"]);
}

#[test]
fn panes_freeze_above_and_left_of_a_cell() {
    let mut doc = GridDoc::blank();
    doc.execute("view.freezePanes", r#"{"cell": "B3"}"#).unwrap();
    let s = doc.sizes(0, 2, 2).unwrap();
    assert_eq!((s.frozen_rows, s.frozen_cols), (2, 1));
    doc.execute("view.unfreezePanes", "{}").unwrap();
    let s = doc.sizes(0, 2, 2).unwrap();
    assert_eq!((s.frozen_rows, s.frozen_cols), (0, 0));
}

#[test]
fn rows_and_columns_hide_and_show_again() {
    let mut doc = GridDoc::blank();
    doc.execute("home.hideRows", r#"{"rows": "2:3"}"#).unwrap();
    doc.execute("home.hideColumns", r#"{"cols": "B:B"}"#).unwrap();
    let s = doc.sizes(0, 3, 4).unwrap();
    assert_eq!((s.row_heights[1], s.row_heights[2], s.col_widths[1]), (0.0, 0.0, 0.0));
    doc.execute("home.unhideRows", r#"{"rows": "1:1048576"}"#).unwrap();
    doc.execute("home.unhideColumns", r#"{"cols": "A:XFD"}"#).unwrap();
    let s = doc.sizes(0, 3, 4).unwrap();
    assert!(s.row_heights.iter().all(|h| *h > 0.0) && s.col_widths.iter().all(|w| *w > 0.0), "{s:?}");
}

#[test]
fn conditional_formats_show_on_the_grid() {
    let mut doc = GridDoc::blank();
    for (r, v) in ["5", "50", "500"].iter().enumerate() {
        doc.set_input(r as u32, 0, v).unwrap();
        doc.set_input(r as u32, 1, v).unwrap();
        doc.set_input(r as u32, 2, v).unwrap();
    }
    // Over 100: light red fill with dark red text.
    let rule = r#"{"range": "A1:A3", "rule": {"type": "cellIs", "operator": "greaterThan", "value": "100", "preset": "lightRedFillDarkRedText"}}"#;
    doc.execute("home.conditionalFormat", rule).unwrap();
    doc.execute("home.conditionalFormat", r#"{"range": "B1:B3", "rule": {"type": "colorScale"}}"#).unwrap();
    doc.execute("home.conditionalFormat", r##"{"range": "C1:C3", "rule": {"type": "dataBar", "color": "#638EC6"}}"##).unwrap();
    let a = doc.cell_looks(0, 0, 0, 3, 1).unwrap();
    assert_eq!(a[0].fill, 0, "5 is not over 100");
    assert_ne!(a[2].fill, 0, "500 is: {:?}", a[2]);
    assert_ne!(a[2].color, 0);
    let b = doc.cell_looks(0, 0, 1, 3, 1).unwrap();
    assert!(b.iter().all(|l| l.fill != 0) && b[0].fill != b[2].fill, "{b:?}");
    let c = doc.cell_looks(0, 0, 2, 3, 1).unwrap();
    assert!(c[2].bar > c[0].bar && c[2].bar_color != 0, "{c:?}");
}

#[test]
fn the_apps_conditional_format_rules_are_accepted() {
    // The rules the sheet's menu sends: duplicates on names in A, the rest on numbers in B.
    let mut doc = GridDoc::blank();
    for (r, (name, n)) in [("a", "10"), ("b", "2000"), ("a", "30")].iter().enumerate() {
        doc.set_input(r as u32, 0, name).unwrap();
        doc.set_input(r as u32, 1, n).unwrap();
    }
    doc.execute("home.conditionalFormat", r#"{"range": "A1:A3", "rule": {"type": "duplicate", "preset": "lightRedFillDarkRedText"}}"#).unwrap();
    for rule in [
        r##"{"type": "colorScale", "colors": ["#F8696B", "#FFEB84", "#63BE7B"]}"##,
        r##"{"type": "dataBar", "color": "#638EC6"}"##,
        r##"{"type": "cellIs", "operator": "greaterThan", "value": "1250.5", "preset": "lightRedFillDarkRedText"}"##,
    ] {
        doc.execute("home.conditionalFormat", &format!(r#"{{"range": "B1:B3", "rule": {rule}}}"#)).unwrap();
    }
    let dup = doc.cell_looks(0, 0, 0, 3, 1).unwrap();
    assert!(dup[0].fill != 0 && dup[2].fill != 0 && dup[1].fill == 0, "{dup:?}");
    let nums = doc.cell_looks(0, 0, 1, 3, 1).unwrap();
    assert!(nums.iter().all(|l| l.fill != 0) && nums[1].bar > nums[0].bar, "{nums:?}");
    doc.execute("home.clearRules", r#"{"range": "A1:B3"}"#).unwrap();
    assert!(doc.cell_looks(0, 0, 0, 3, 2).unwrap().iter().all(|l| l.fill == 0 && l.bar == 0.0));
}

#[test]
fn icon_sets_show_on_the_grid() {
    let mut doc = GridDoc::blank();
    for (r, v) in ["1", "50", "100"].iter().enumerate() {
        doc.set_input(r as u32, 0, v).unwrap();
    }
    doc.execute("home.conditionalFormat", r#"{"range": "A1:A3", "rule": {"type": "iconSet", "set": "3Arrows"}}"#).unwrap();
    let l = doc.cell_looks(0, 0, 0, 3, 1).unwrap();
    assert!(l.iter().all(|c| c.icon_kind == 1 && c.icon_count == 3), "{l:?}");
    assert_eq!((l[0].icon_index, l[2].icon_index), (0, 2));
}

#[test]
fn notes_are_listed_added_and_removed() {
    let mut doc = GridDoc::blank();
    assert!(doc.notes(0).unwrap().is_empty());
    doc.execute("review.newNote", r#"{"cell": "B2", "text": "Kontrol edilecek: Şubat", "author": "Bahadır"}"#).unwrap();
    let notes = doc.notes(0).unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!((notes[0].row, notes[0].col), (1, 1));
    assert!(notes[0].text.contains("Şubat"), "{notes:?}");
    doc.execute("review.deleteComment", r#"{"cell": "B2"}"#).unwrap();
    assert!(doc.notes(0).unwrap().is_empty());
    // Kept in the file.
    doc.execute("review.newNote", r#"{"cell": "A1", "text": "Kalıcı not"}"#).unwrap();
    let again = GridDoc::open("x.xlsx", &doc.save("x.xlsx").unwrap(), None).unwrap();
    assert!(again.notes(0).unwrap().iter().any(|n| n.text.contains("Kalıcı not")));
}

#[test]
fn drop_down_lists_offer_their_choices() {
    let mut doc = GridDoc::blank();
    for (r, v) in ["Ankara", "İzmir", "Şanlıurfa"].iter().enumerate() {
        doc.set_input(r as u32, 4, v).unwrap();
    }
    doc.execute("data.validation", r#"{"range": "A1:A5", "type": "list", "formula1": "Evet,Hayır"}"#).unwrap();
    doc.execute("data.validation", r#"{"range": "B1", "type": "list", "formula1": "=$E$1:$E$3"}"#).unwrap();
    assert_eq!(doc.list_choices(2, 0), ["Evet", "Hayır"]);
    assert_eq!(doc.list_choices(0, 1), ["Ankara", "İzmir", "Şanlıurfa"]);
    assert!(doc.list_choices(0, 2).is_empty());
}

#[test]
fn charts_are_placed_and_drawn_with_their_words() {
    let mut doc = GridDoc::blank();
    for (r, row) in [["Ay", "Satış"], ["Ocak", "10"], ["Şubat", "30"], ["Mart", "20"]].iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            doc.set_input(r as u32, c as u32, v).unwrap();
        }
    }
    doc.execute("insert.chart", r#"{"range": "A1:B4", "type": "column", "title": "Aylık satış", "at": "D2"}"#).unwrap();
    let charts = doc.charts(0).unwrap();
    assert_eq!(charts.len(), 1);
    assert_eq!((charts[0].row, charts[0].col), (1, 3));
    assert!(charts[0].width > 50.0 && charts[0].height > 50.0, "{charts:?}");
    let pic = doc.render_chart(0, 0, 2.0).unwrap();
    assert_eq!((pic.width as f32, pic.height as f32), ((charts[0].width * 2.0).round(), (charts[0].height * 2.0).round()));
    assert_eq!(pic.pixels.len(), (pic.width * pic.height * 4) as usize);
    // Columns are drawn in colour, not only white.
    let coloured = pic.pixels.as_chunks::<4>().0.iter().filter(|p| !(p[0] > 240 && p[1] > 240 && p[2] > 240)).count();
    assert!(coloured > 500, "{coloured}");
    assert!(pic.texts.iter().any(|t| t.text == "Aylık satış"), "{:?}", pic.texts.iter().map(|t| &t.text).collect::<Vec<_>>());
    assert!(pic.texts.iter().any(|t| t.text == "Şubat"));
    // Moved, retitled and turned into a line chart; then deleted.
    doc.change_chart(0, 0, r#"{"at": "F8", "dx": 0, "dy": 0, "title": "Satışlar", "type": "line"}"#).unwrap();
    let moved = doc.charts(0).unwrap();
    assert_eq!((moved[0].row, moved[0].col), (7, 5));
    assert!(doc.render_chart(0, 0, 1.0).unwrap().texts.iter().any(|t| t.text == "Satışlar"));
    doc.change_chart(0, 0, r#"{"delete": true}"#).unwrap();
    assert!(doc.charts(0).unwrap().is_empty());
    assert!(doc.change_chart(0, 0, "{}").is_err());
}

#[test]
fn pictures_are_listed_with_their_bytes() {
    let mut doc = GridDoc::blank();
    // A 1×1 PNG.
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC";
    doc.execute("insert.picture", &serde_json::json!({ "base64": png, "mime": "image/png", "at": "C3", "width": 120, "height": 80 }).to_string())
        .unwrap();
    let pics = doc.pictures(0).unwrap();
    assert_eq!(pics.len(), 1);
    assert_eq!((pics[0].row, pics[0].col), (2, 2));
    assert!((pics[0].width - 120.0).abs() < 1.0, "{pics:?}");
    assert!(doc.picture_bytes(0, 0).unwrap().starts_with(&[0x89, b'P', b'N', b'G']));
    // Kept in the file.
    let again = GridDoc::open("x.xlsx", &doc.save("x.xlsx").unwrap(), None).unwrap();
    assert_eq!(again.pictures(0).unwrap().len(), 1);
}

#[test]
fn a_chart_from_one_cell_takes_the_used_area() {
    // With one cell selected the app sends the used area (a one-cell range charts that cell alone).
    let mut doc = GridDoc::blank();
    for (r, row) in [["Ay", "Satış"], ["Ocak", "10"], ["Şubat", "30"]].iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            doc.set_input(r as u32, c as u32, v).unwrap();
        }
    }
    doc.execute("insert.chart", r#"{"type": "pie", "range": "A1:B3", "at": "D1"}"#).unwrap();
    let pic = doc.render_chart(0, 0, 1.0).unwrap();
    assert!(pic.texts.iter().any(|t| t.text.contains("Ocak")), "{:?}", pic.texts.iter().map(|t| &t.text).collect::<Vec<_>>());
}

/// The lossless-save corpus (scripts/office_corpus.py): every `*.xlsx` in `MO_CORPUS_DIR` is
/// opened, cell B3 of its "Veri" sheet (an input of its formulas) is set to 75 through the engine
/// and it is saved as `*.saved.xlsx`, which the independent reader then compares with the
/// original. Nothing to do without the variable.
#[test]
fn corpus_round_trip() {
    let Ok(dir) = std::env::var("MO_CORPUS_DIR") else { return };
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".xlsx").filter(|s| !s.ends_with(".saved")) else { continue };
        let mut doc = GridDoc::open(&name, &std::fs::read(&path).unwrap(), None).unwrap();
        // Everything in the corpus is written back, so it must save over its file.
        assert!(!doc.opened_incomplete(), "{name}: losses {:?}, warnings {:?}", doc.losses(), doc.warnings());
        let sheet = doc.sheet_names().iter().position(|s| s == "Veri").expect("a Veri sheet");
        doc.set_active_sheet(sheet as u32).unwrap();
        doc.execute("cell.set", r#"{"cell":"B3","input":"75"}"#).unwrap();
        std::fs::write(path.with_file_name(format!("{stem}.saved.xlsx")), doc.save(&name).unwrap()).unwrap();
        std::fs::write(path.with_file_name(format!("{stem}.xlsx.warnings.txt")), doc.warnings().join("\n")).unwrap();
        seen += 1;
    }
    assert!(seen > 0, "no .xlsx in {dir}");
}

#[test]
fn reads_the_engines_damaged_sheet_warning() {
    assert_eq!(
        parse_truncation("sheet \"Satışlar\" is damaged (the part holds more than 1000000 elements); kept 49995 rows"),
        Some(("Satışlar", 49995))
    );
    assert_eq!(
        parse_truncation("sheet \"a \"b\" is damaged (x; kept 3 rows) y\" is damaged (z); kept 7 rows"),
        Some(("a \"b\" is damaged (x; kept 3 rows) y", 7))
    );
    assert_eq!(parse_truncation("sheet \"A\": 3 cells or rows outside the 1,048,576 × 16,384 grid were dropped"), None);
    assert_eq!(parse_truncation("shared strings are damaged (x); kept 5 strings"), None);
    assert_eq!(parse_strings_cut("shared strings are damaged (the part holds more than 1000000 elements); kept 999999 strings"), Some(999_999));
    assert_eq!(parse_strings_cut("shared strings: 2 phonetic runs were dropped (bad sb/eb or no reading)"), None);
}

/// GridCraft's XLSX reader used to stop after 1,000,000 XML elements in one sheet, counting the
/// cells of rows it had already streamed, so a plain 60,000 × 10 sheet was cut near row 50,000
/// (docs/upstream-notlar/gridcraft-satir-siniri.md; fixed upstream in 0b7deaa3). A 120,000 × 10
/// sheet must open with every row and no truncation reported; the warning parser above still
/// guards against cuts the engine reports for other reasons.
#[test]
fn a_sheet_beyond_a_million_xml_elements_opens_whole() {
    const ROWS: usize = 120_000;
    let mut csv = String::with_capacity(ROWS * 70);
    csv.push_str("A,B,C,D,E,F,G,H,I,J\n");
    for r in 1..ROWS {
        for c in 0..10 {
            if c > 0 {
                csv.push(',');
            }
            csv.push_str(&(r * 10 + c).to_string());
        }
        csv.push('\n');
    }
    let xlsx = GridDoc::open("liste.csv", csv.as_bytes(), None).unwrap().save("liste.xlsx").unwrap();

    let doc = GridDoc::open("liste.xlsx", &xlsx, None).unwrap();
    assert!(doc.truncated().is_empty(), "warnings: {:?}", doc.warnings());
    assert_eq!(doc.strings_kept(), None);
    assert!(!doc.opened_incomplete(), "warnings: {:?}", doc.warnings());
    let size = doc.used_size(0).unwrap();
    assert_eq!((size.rows as usize, size.cols), (ROWS, 10));
    let last = ROWS - 1;
    assert_eq!(doc.cell_texts(0, last as u32, 9, 1, 1).unwrap(), [(last * 10 + 9).to_string()]);

    let small = GridDoc::open("liste.csv", "a,b\n1,2\n".as_bytes(), None).unwrap().save("k.xlsx").unwrap();
    assert!(!GridDoc::open("k.xlsx", &small, None).unwrap().opened_incomplete());
    assert!(!GridDoc::blank().opened_incomplete());
}

#[test]
fn reads_the_engines_unread_sheet_and_part_warnings() {
    assert_eq!(parse_unread_sheet("sheet \"Özet\" could not be read: XML error: x"), Some("Özet"));
    assert_eq!(parse_unread_sheet("sheet \"Makro\" is a macrosheet and was skipped"), Some("Makro"));
    assert_eq!(parse_unread_sheet("chart sheet \"Grafik 1\" was skipped and is not kept on save"), Some("Grafik 1"));
    assert_eq!(parse_unread_sheet("sheet \"A\" has no part; skipped"), None);
    assert_eq!(parse_unread_sheet("sheet \"A\" is damaged (x); kept 3 rows"), None);
    assert_eq!(parse_skipped_part("skipped xl/styles.xml: XML error: unexpected end"), Some("xl/styles.xml"));
    assert_eq!(parse_skipped_part("skipped 3 cells: x"), None);
}

/// `bytes` (a zip) with entry `name` replaced by `data`, the other entries as they were.
fn with_entry(bytes: &[u8], name: &str, data: &[u8]) -> Vec<u8> {
    use std::io::{Read, Write};
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut found = false;
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).unwrap();
        let entry = f.name().to_string();
        let mut old = Vec::new();
        f.read_to_end(&mut old).unwrap();
        out.start_file(entry.as_str(), opts).unwrap();
        found |= entry == name;
        out.write_all(if entry == name { data } else { &old }).unwrap();
    }
    assert!(found, "no {name}");
    out.finish().unwrap().into_inner()
}

/// A worksheet or styles part GridCraft cannot parse is only a warning: the sheet opens empty, the
/// cells lose their formats. Saving that over the original would make it permanent.
#[test]
fn a_sheet_or_part_the_engine_could_not_read_blocks_saving_over_the_file() {
    let xlsx = GridDoc::open("k.csv", "a,b\n1,2\n".as_bytes(), None).unwrap().save("k.xlsx").unwrap();
    assert!(!GridDoc::open("k.xlsx", &xlsx, None).unwrap().opened_incomplete());

    let doc = GridDoc::open("k.xlsx", &with_entry(&xlsx, "xl/worksheets/sheet1.xml", b"\x00\x01 not xml"), None).unwrap();
    assert!(doc.opened_incomplete(), "warnings: {:?}", doc.warnings());
    // Reported as a sheet cut at row 0 (see `truncated`).
    assert_eq!(doc.truncated().len(), 1, "warnings: {:?}", doc.warnings());

    let doc = GridDoc::open("k.xlsx", &with_entry(&xlsx, "xl/styles.xml", b"\x00\x01 not xml"), None).unwrap();
    assert!(doc.opened_incomplete(), "warnings: {:?}", doc.warnings());
    assert_eq!(doc.skipped_parts(), ["xl/styles.xml"], "warnings: {:?}", doc.warnings());
}

// ---------------------------------------------------------------------------------------------
// What saving an XLSX back would lose (losses.rs).

const NS_X14: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const NS_XM: &str = "http://schemas.microsoft.com/office/excel/2006/main";
const NS_X15: &str = "http://schemas.microsoft.com/office/spreadsheetml/2010/11/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const SHEET: &str = "xl/worksheets/sheet1.xml";
const SHEET_RELS: &str = "xl/worksheets/_rels/sheet1.xml.rels";
const WORKBOOK: &str = "xl/workbook.xml";
const WORKBOOK_RELS: &str = "xl/_rels/workbook.xml.rels";

/// The parts of an XLSX package, to build test files from GridCraft's own output.
struct Pkg(Vec<(String, Vec<u8>)>);

impl Pkg {
    /// A two-row sheet "k" as GridCraft writes it (no sheet relationships).
    fn base() -> Pkg {
        Pkg::of(&GridDoc::open("k.csv", "a,b\n1,2\n".as_bytes(), None).unwrap().save("k.xlsx").unwrap())
    }

    fn of(bytes: &[u8]) -> Pkg {
        use std::io::Read;
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        Pkg((0..z.len())
            .map(|i| {
                let mut f = z.by_index(i).unwrap();
                let mut data = Vec::new();
                f.read_to_end(&mut data).unwrap();
                (f.name().to_string(), data)
            })
            .collect())
    }

    fn text(&self, part: &str) -> Option<String> {
        self.0.iter().find(|(n, _)| n.eq_ignore_ascii_case(part)).map(|(_, d)| String::from_utf8_lossy(d).into_owned())
    }

    fn has_prefix(&self, prefix: &str) -> bool {
        self.0.iter().any(|(n, _)| n.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()))
    }

    /// Adds or replaces `part`.
    fn put(mut self, part: &str, data: &str) -> Pkg {
        self.0.retain(|(n, _)| n != part);
        self.0.push((part.to_string(), data.as_bytes().to_vec()));
        self
    }

    /// `text` inserted in `part` before the first `before`.
    fn insert(self, part: &str, before: &str, text: &str) -> Pkg {
        let old = self.text(part).unwrap_or_else(|| panic!("no {part}"));
        let at = old.find(before).unwrap_or_else(|| panic!("no {before} in {part}"));
        let new = format!("{}{text}{}", &old[..at], &old[at..]);
        self.put(part, &new)
    }

    /// A relationship of kind `ty` (full type URI) from the relationships part `rels`.
    fn rel(self, rels: &str, id: &str, ty: &str, target: &str, external: bool) -> Pkg {
        let mode = if external { " TargetMode=\"External\"" } else { "" };
        let r = format!("<Relationship Id=\"{id}\" Type=\"{ty}\" Target=\"{target}\"{mode}/>");
        match self.text(rels) {
            Some(_) => self.insert(rels, "</Relationships>", &r),
            None => self.put(
                rels,
                &format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{r}</Relationships>"),
            ),
        }
    }

    fn content_type(self, part: &str, ct: &str) -> Pkg {
        self.insert("[Content_Types].xml", "</Types>", &format!("<Override PartName=\"/{part}\" ContentType=\"{ct}\"/>"))
    }

    /// `xml` added to the sheet right after its cells.
    fn after_cells(self, xml: &str) -> Pkg {
        self.insert(SHEET, "<pageMargins", xml)
    }

    fn sheet_ext(self, xml: &str) -> Pkg {
        self.insert(SHEET, "</worksheet>", &format!("<extLst>{xml}</extLst>"))
    }

    fn bytes(&self) -> Vec<u8> {
        use std::io::Write;
        let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in &self.0 {
            out.start_file(name.as_str(), opts).unwrap();
            out.write_all(data).unwrap();
        }
        out.finish().unwrap().into_inner()
    }
}

/// Opens `pkg` as `name`, saves it back under the same name and reopens it.
fn round_trip(pkg: &Pkg, name: &str) -> (GridDoc, Pkg) {
    let doc = GridDoc::open(name, &pkg.bytes(), None).unwrap();
    let saved = doc.save(name).unwrap();
    (doc, Pkg::of(&saved))
}

/// What GridCraft writes back is all there: links to other workbooks, custom XML, data
/// connections, custom document properties, hyperlinks, data validation, conditional formats,
/// sparklines, merged cells, sheet and workbook protection and defined names. Such a workbook
/// must still save over its file.
#[test]
fn what_the_engine_writes_back_does_not_block_saving_over_the_file() {
    let pkg = Pkg::base()
        .after_cells(
            "<sheetProtection sheet=\"1\" objects=\"1\" scenarios=\"1\"/><mergeCells count=\"1\"><mergeCell ref=\"A3:B3\"/></mergeCells>\
             <conditionalFormatting sqref=\"A2:B2\"><cfRule type=\"colorScale\" priority=\"1\"><colorScale><cfvo type=\"min\"/><cfvo type=\"max\"/>\
             <color rgb=\"FFF8696B\"/><color rgb=\"FF63BE7B\"/></colorScale></cfRule></conditionalFormatting>\
             <dataValidations count=\"1\"><dataValidation type=\"list\" allowBlank=\"1\" sqref=\"A2\"><formula1>\"Evet,Hayır\"</formula1></dataValidation></dataValidations>\
             <hyperlinks><hyperlink ref=\"A1\" r:id=\"rId1\"/><hyperlink ref=\"B1\" location=\"k!A2\" display=\"A2\"/></hyperlinks>",
        )
        .sheet_ext(&format!(
            "<ext uri=\"{{05C60535-1F16-4fd2-B633-F4F36F0B64E0}}\" xmlns:x14=\"{NS_X14}\"><x14:sparklineGroups xmlns:xm=\"{NS_XM}\">\
             <x14:sparklineGroup type=\"column\"><x14:colorSeries rgb=\"FF376092\"/><x14:sparklines><x14:sparkline><xm:f>k!A2:B2</xm:f><xm:sqref>C2</xm:sqref>\
             </x14:sparkline></x14:sparklines></x14:sparklineGroup></x14:sparklineGroups></ext>"
        ))
        .rel(SHEET_RELS, "rId1", &format!("{REL}/hyperlink"), "https://example.org/rapor", true)
        .insert(WORKBOOK, "<bookViews>", "<workbookProtection lockStructure=\"1\"/>")
        .insert(
            WORKBOOK,
            "<calcPr",
            "<externalReferences><externalReference r:id=\"rId10\"/></externalReferences><definedNames><definedName name=\"Gelir\">k!$B$2</definedName></definedNames>",
        )
        .rel(WORKBOOK_RELS, "rId10", &format!("{REL}/externalLink"), "externalLinks/externalLink1.xml", false)
        .put(
            "xl/externalLinks/externalLink1.xml",
            "<externalLink xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
             <externalBook r:id=\"rId1\"><sheetNames><sheetName val=\"Veri\"/></sheetNames></externalBook></externalLink>",
        )
        .content_type("xl/externalLinks/externalLink1.xml", "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml")
        .rel("xl/externalLinks/_rels/externalLink1.xml.rels", "rId1", &format!("{REL}/externalLinkPath"), "kaynak.xlsx", true)
        .rel(WORKBOOK_RELS, "rId11", &format!("{REL}/customXml"), "../customXml/item1.xml", false)
        .put("customXml/item1.xml", "<kök xmlns=\"urn:test\">Şişli</kök>")
        .rel(WORKBOOK_RELS, "rId12", &format!("{REL}/connections"), "connections.xml", false)
        .put(
            "xl/connections.xml",
            "<connections xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><connection id=\"1\" name=\"Sorgu\" type=\"5\" refreshedVersion=\"6\">\
             <dbPr connection=\"Provider=x\" command=\"SELECT 1\"/></connection></connections>",
        )
        .content_type("xl/connections.xml", "application/vnd.openxmlformats-officedocument.spreadsheetml.connections+xml")
        .rel("_rels/.rels", "rId9", &format!("{REL}/custom-properties"), "docProps/custom.xml", false)
        .put(
            "docProps/custom.xml",
            "<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/custom-properties\" xmlns:vt=\"http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes\">\
             <property fmtid=\"{D5CDD505-2E9C-101B-9397-08002B2CF9AE}\" pid=\"2\" name=\"Proje\"><vt:lpwstr>Bütçe</vt:lpwstr></property></Properties>",
        )
        .content_type("docProps/custom.xml", "application/vnd.openxmlformats-officedocument.custom-properties+xml");

    let (doc, saved) = round_trip(&pkg, "k.xlsx");
    assert!(!doc.opened_incomplete(), "losses {:?}, warnings {:?}", doc.losses(), doc.warnings());
    for part in ["xl/externalLinks/", "customXml/item1.xml", "xl/connections.xml", "docProps/custom.xml"] {
        assert!(saved.has_prefix(part), "{part} kept: {:?}", saved.0.iter().map(|(n, _)| n).collect::<Vec<_>>());
    }
    let wb = saved.text(WORKBOOK).unwrap();
    for x in ["<externalReference ", "<workbookProtection", "<definedName name=\"Gelir\""] {
        assert!(wb.contains(x), "{x} in {wb}");
    }
    let sheet = saved.text(SHEET).unwrap();
    for x in ["<sheetProtection", "<mergeCell ", "<conditionalFormatting", "<dataValidation ", "<hyperlink ", "sparklineGroup"] {
        assert!(sheet.contains(x), "{x} in {sheet}");
    }
    assert!(saved.text(SHEET_RELS).is_some_and(|r| r.contains("https://example.org/rapor")));
    // What was saved saves over itself too.
    let again = GridDoc::open("k.xlsx", &saved.bytes(), None).unwrap();
    assert!(!again.opened_incomplete(), "losses {:?}, warnings {:?}", again.losses(), again.warnings());
}

/// Content GridCraft skips without a warning: each blocks saving over the file, and each is
/// indeed missing from what the engine would write.
#[test]
fn content_the_engine_skips_silently_blocks_saving_over_the_file() {
    let slicer = |p: Pkg| {
        p.sheet_ext(&format!(
            "<ext uri=\"{{A8765BA9-456A-4dab-B4F3-ACF838C121DE}}\" xmlns:x14=\"{NS_X14}\"><x14:slicerList><x14:slicer r:id=\"rId7\"/></x14:slicerList></ext>"
        ))
        .rel(SHEET_RELS, "rId7", "http://schemas.microsoft.com/office/2007/relationships/slicer", "../slicers/slicer1.xml", false)
        .put("xl/slicers/slicer1.xml", &format!("<slicers xmlns=\"{NS_X14}\"><slicer name=\"Ay\" cache=\"Slicer_Ay\" caption=\"Ay\" rowHeight=\"241300\"/></slicers>"))
        .content_type("xl/slicers/slicer1.xml", "application/vnd.ms-excel.slicer+xml")
    };
    let ink = |p: Pkg| {
        p.insert(SHEET, "</worksheet>", "<drawing r:id=\"rId5\"/>")
            .rel(SHEET_RELS, "rId5", &format!("{REL}/drawing"), "../drawings/drawing1.xml", false)
            .put(
                "xl/drawings/drawing1.xml",
                "<xdr:wsDr xmlns:xdr=\"http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing\" xmlns:xdr14=\"http://schemas.microsoft.com/office/excel/2010/spreadsheetDrawing\" \
                 xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><xdr:twoCellAnchor><xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff>\
                 <xdr:row>0</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:to><xdr:col>2</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>3</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>\
                 <xdr14:contentPart r:id=\"rId1\"/><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>",
            )
            .content_type("xl/drawings/drawing1.xml", "application/vnd.openxmlformats-officedocument.drawing+xml")
            .rel("xl/drawings/_rels/drawing1.xml.rels", "rId1", &format!("{REL}/customXml"), "../ink/ink1.xml", false)
            .put("xl/ink/ink1.xml", "<inkml:ink xmlns:inkml=\"http://www.w3.org/2003/InkML\"><inkml:trace>0 0, 10 10</inkml:trace></inkml:ink>")
            .content_type("xl/ink/ink1.xml", "application/inkml+xml")
    };
    type Edit = Box<dyn Fn(Pkg) -> Pkg>;
    let cases: Vec<(&str, Edit, Loss, &str, &str)> = vec![
        (
            "what-if scenario",
            Box::new(|p: Pkg| {
                p.after_cells("<scenarios><scenario name=\"İyi\" count=\"1\" user=\"x\"><inputCells r=\"A2\" val=\"5\"/></scenario></scenarios>")
            }),
            Loss::Other,
            SHEET,
            "<scenario",
        ),
        (
            "custom view",
            Box::new(|p: Pkg| {
                p.after_cells("<customSheetViews><customSheetView guid=\"{11111111-2222-3333-4444-555555555555}\" scale=\"80\"><selection activeCell=\"A1\" sqref=\"A1\"/></customSheetView></customSheetViews>")
            }),
            Loss::Other,
            SHEET,
            "customSheetView",
        ),
        (
            "date filter grouping",
            Box::new(|p: Pkg| {
                p.insert(SHEET, "<pageMargins", "<autoFilter ref=\"A1:B2\"><filterColumn colId=\"0\"><filters><dateGroupItem year=\"2024\" dateTimeGrouping=\"year\"/></filters></filterColumn></autoFilter>")
            }),
            Loss::Filters,
            SHEET,
            "dateGroupItem",
        ),
        ("slicer", Box::new(slicer), Loss::Slicers, SHEET, "slicer"),
        (
            "data model",
            Box::new(|p: Pkg| {
                p.insert(
                    WORKBOOK,
                    "</workbook>",
                    &format!("<extLst><ext uri=\"{{FCE2AD5D-F65C-4FA6-A056-5C36A1767C68}}\" xmlns:x15=\"{NS_X15}\"><x15:dataModel><x15:modelTables><x15:modelTable id=\"T\" name=\"T\" connection=\"Sorgu\"/></x15:modelTables></x15:dataModel></ext></extLst>"),
                )
            }),
            Loss::Other,
            WORKBOOK,
            "dataModel",
        ),
        (
            "password to modify",
            Box::new(|p: Pkg| {
                p.insert(WORKBOOK, "<workbookPr", "<fileSharing readOnlyRecommended=\"1\" userName=\"Ayşe\" reservationPassword=\"CC3D\"/>")
            }),
            Loss::Other,
            WORKBOOK,
            "fileSharing",
        ),
        (
            "sheet custom property",
            Box::new(|p: Pkg| {
                p.insert(SHEET, "</worksheet>", "<customProperties><customPr name=\"Kod\" r:id=\"rId3\"/></customProperties>")
                    .rel(SHEET_RELS, "rId3", &format!("{REL}/customProperty"), "../customProperty1.bin", false)
                    .put("xl/customProperty1.bin", "K\0o\0d\0")
            }),
            Loss::Other,
            SHEET,
            "customPr",
        ),
        ("ink", Box::new(ink), Loss::Drawings, "xl/drawings/drawing1.xml", "contentPart"),
    ];
    for (what, edit, loss, part, marker) in cases {
        let pkg = edit(Pkg::base());
        assert!(pkg.text(part).unwrap().contains(marker), "{what}: test file holds {marker}");
        let (doc, saved) = round_trip(&pkg, "k.xlsx");
        assert!(doc.losses().contains(&loss), "{what}: losses {:?}, warnings {:?}", doc.losses(), doc.warnings());
        assert!(doc.opened_incomplete(), "{what}");
        assert!(!saved.text(part).unwrap_or_default().contains(marker), "{what}: the engine does write {marker} back");
    }
}

/// A VBA project is written back only to a macro-enabled type (write.rs:422): in an `.xlsx` it
/// would be lost, in an `.xlsm` it is kept.
#[test]
fn macros_block_saving_over_an_xlsx_but_not_an_xlsm() {
    let pkg = Pkg::base()
        .rel(WORKBOOK_RELS, "rId8", "http://schemas.microsoft.com/office/2006/relationships/vbaProject", "vbaProject.bin", false)
        .put("xl/vbaProject.bin", "not really an OLE file")
        .insert("[Content_Types].xml", "</Types>", "<Default Extension=\"bin\" ContentType=\"application/vnd.ms-office.vbaProject\"/>");
    let (doc, saved) = round_trip(&pkg, "k.xlsx");
    assert_eq!(doc.losses(), [Loss::Macros], "warnings {:?}", doc.warnings());
    assert!(doc.opened_incomplete());
    assert!(!saved.has_prefix("xl/vbaProject.bin"));

    let (doc, saved) = round_trip(&pkg, "k.xlsm");
    assert!(!doc.opened_incomplete(), "losses {:?}, warnings {:?}", doc.losses(), doc.warnings());
    assert!(saved.text("xl/vbaProject.bin").is_some_and(|b| b == "not really an OLE file"));
}

/// Losses the engine reports as warnings, from real files: hyperlinks past its cap, an Excel 2010
/// conditional format it cannot keep, a date filter, a what-if data table.
#[test]
fn losses_the_engine_warns_about_block_saving_over_the_file() {
    let links: String = (1..=10_001).map(|r| format!("<hyperlink ref=\"C{r}\" location=\"k!A1\" display=\"A1\"/>")).collect();
    let cases: Vec<(&str, Pkg, Loss)> = vec![
        ("hyperlinks past the cap", Pkg::base().after_cells(&format!("<hyperlinks>{links}</hyperlinks>")), Loss::Links),
        (
            "Excel 2010 conditional format",
            Pkg::base().sheet_ext(&format!(
                "<ext uri=\"{{78C0D931-6437-407d-A8EE-F0AAD7539E65}}\" xmlns:x14=\"{NS_X14}\"><x14:conditionalFormattings>\
                 <x14:conditionalFormatting xmlns:xm=\"{NS_XM}\"><x14:cfRule type=\"iconSet\" priority=\"1\" id=\"{{00000000-0000-0000-0000-000000000001}}\">\
                 <x14:iconSet iconSet=\"3Stars\" custom=\"1\"><x14:cfvo type=\"percent\"><xm:f>0</xm:f></x14:cfvo></x14:iconSet></x14:cfRule>\
                 <xm:sqref>A2:B2</xm:sqref></x14:conditionalFormatting></x14:conditionalFormattings></ext>"
            )),
            Loss::ConditionalFormats,
        ),
        (
            "date filter",
            Pkg::base().insert(SHEET, "<pageMargins", "<autoFilter ref=\"A1:B2\"><filterColumn colId=\"0\"><dynamicFilter type=\"thisMonth\"/></filterColumn></autoFilter>"),
            Loss::Filters,
        ),
        (
            "what-if data table",
            Pkg::base().insert(SHEET, "<v>2</v>", "<f t=\"dataTable\" ref=\"B2\" dt2D=\"0\" dtr=\"0\" r1=\"A1\"/>"),
            Loss::Formulas,
        ),
    ];
    for (what, pkg, loss) in cases {
        let doc = GridDoc::open("k.xlsx", &pkg.bytes(), None).unwrap();
        assert_eq!(doc.losses(), [loss], "{what}: warnings {:?}", doc.warnings());
        assert!(doc.opened_incomplete(), "{what}");
    }
}

/// GridCraft's warning wording (crates/xlsx/src at c0ee120), as the bridge reads it. A change of
/// wording upstream fails here.
#[test]
fn reads_the_engines_loss_warnings() {
    use losses::warning_loss as w;
    let cases: &[(&str, Option<Loss>)] = &[
        // Losses.
        ("sheet \"Satış\": hyperlinks longer than 8192 characters were dropped", Some(Loss::Links)),
        ("sheet \"Satış\": hyperlinks past the first 10000 cells were dropped", Some(Loss::Links)),
        ("sheet \"Satış\": hyperlink targets past 8388608 bytes were dropped", Some(Loss::Links)),
        ("links to external workbooks were dropped: a link has no part", Some(Loss::Links)),
        ("connections part connections.xml was dropped: it is too large", Some(Loss::Links)),
        ("conditional format type \"x\" is not supported", Some(Loss::ConditionalFormats)),
        ("conditional formats past 4000000 kept range cells were dropped", Some(Loss::ConditionalFormats)),
        ("some Excel 2010 conditional format extensions were skipped", Some(Loss::ConditionalFormats)),
        ("PivotTables on sheet \"Özet\" could not be read: XML error", Some(Loss::PivotTables)),
        ("PivotTables with a consolidation data source are not supported; kept as plain cells", Some(Loss::PivotTables)),
        ("xl/pivotCache/x.xml is not a pivot cache definition; its PivotTable was kept as plain cells", Some(Loss::PivotTables)),
        ("calculated PivotTable fields are not supported", Some(Loss::PivotTables)),
        ("PivotTable grouping of \"Tarih\" is not supported; the field is ungrouped", Some(Loss::PivotTables)),
        ("PivotTable function stdDevp is not supported; using Sum", Some(Loss::PivotTables)),
        ("macros (VBA project) could not be read and were dropped: x", Some(Loss::Macros)),
        ("what-if data tables are not supported; kept their values", Some(Loss::Formulas)),
        ("shared formulas expand to too much data; kept the cached values of the remaining cells that share them", Some(Loss::Formulas)),
        ("a shared formula is longer than 8192 characters; kept the cached values of the cells that share it", Some(Loss::Formulas)),
        ("formula-generated cell pictures were imported as static pictures; their formulas were not retained", Some(Loss::Formulas)),
        ("the Python part is damaged (x); Python cells kept only their values", Some(Loss::Formulas)),
        ("the Python part holds more than 10000 scripts; 3 were not read", Some(Loss::Formulas)),
        ("2 Python cell(s) name code the file's Python part doesn't hold (or holds too much of); they kept only their values", Some(Loss::Formulas)),
        ("date filters in AutoFilters are not supported", Some(Loss::Filters)),
        ("some date cells could not be read", Some(Loss::CellValues)),
        ("some numeric cells could not be read", Some(Loss::CellValues)),
        ("a linked (not embedded) picture was skipped", Some(Loss::Drawings)),
        ("chart xl/charts/chart1.xml has no supported plot and was skipped", Some(Loss::Drawings)),
        ("an embedded object (SmartArt, OLE…) was skipped (xl/drawings/drawing1.xml, object 2)", Some(Loss::Drawings)),
        ("grouped shapes are not supported and were skipped (xl/drawings/drawing1.xml, object 1)", Some(Loss::Drawings)),
        ("cell-picture data could not be read (x); scalar fallbacks were retained", Some(Loss::Drawings)),
        ("missing rich-value data relationship; cell pictures were not loaded", Some(Loss::Drawings)),
        ("some cells have unsupported or missing value metadata; kept their scalar fallback (cell pictures may be missing)", Some(Loss::Drawings)),
        ("sheet \"A\": controls and embedded objects were dropped: it is too large", Some(Loss::Drawings)),
        ("sheet \"A\": header and footer pictures were dropped: x", Some(Loss::Drawings)),
        ("sheet \"A\": oleObjects were dropped: its XML is damaged", Some(Loss::Drawings)),
        ("unmodelled content of a Strict Open XML workbook (custom XML, links, controls…) is not kept on save", Some(Loss::Other)),
        ("custom-properties was dropped: it is too large", Some(Loss::Other)),
        ("customXml part ../customXml/item1.xml was dropped: x", Some(Loss::Other)),
        ("sheet \"A\": relationships could not be read (x); unmodelled content was dropped", Some(Loss::Other)),
        ("sheet \"A\": queryTable was dropped: x", Some(Loss::Other)),
        ("sheet \"A\": printer settings were dropped: x", Some(Loss::Other)),
        ("shared strings: 2 phonetic readings were dropped because another <si> with the same text already had one", Some(Loss::Other)),
        // A sheet name that reads like a harmless warning does not hide a loss.
        ("sheet \"missing part \": hyperlinks past the first 10000 cells were dropped", Some(Loss::Links)),
        // Not losses.
        ("missing part xl/comments1.xml", None),
        ("missing picture xl/media/image1.png", None),
        ("a chart without its chart part was skipped (xl/drawings/drawing1.xml, object 1)", None),
        ("chart xl/charts/chart1.xml is missing from the package and was skipped (xl/drawings/drawing1.xml, object 1)", None),
        ("sheet \"A\" has no part; skipped", None),
        ("macros (VBA project) were kept but not executed", None),
        ("macros (VBA project) reference a missing part and were dropped", None),
        ("sheet \"A\": 3 cells or rows outside the 1,048,576 × 16,384 grid were dropped", None),
        ("shared strings: 2 phonetic runs were dropped (bad sb/eb or no reading)", None),
        ("links to external workbooks were kept; they are not followed or refreshed", None),
        ("sheet \"A\": embedded objects and form controls are not shown; they are kept for saving", None),
        ("sheet \"A\": pictures in the header and footer are not shown; they are kept for saving", None),
        ("a PivotTable's source is in another workbook; its source reference may not resolve", None),
        ("a shared formula refers to a missing master cell; kept the cached value", None),
        ("some cells refer to missing shared strings", None),
        ("no styles part", None),
        ("the workbook has no worksheets; added an empty sheet", None),
        // Reported by unread_sheets / skipped_parts / truncated instead.
        ("chart sheet \"Grafik\" was skipped and is not kept on save", None),
        ("chart sheet \"Grafik\" was dropped: it is too large", None),
        ("sheet \"Makro\" is a macrosheet and was skipped", None),
        ("skipped xl/styles.xml: XML error", None),
        ("sheet \"A\" is damaged (x); kept 3 rows", None),
    ];
    for (warning, loss) in cases {
        assert_eq!(w(warning), *loss, "{warning}");
    }
    assert_eq!(parse_unread_sheet("chart sheet \"Grafik 1\" was dropped: it is too large"), Some("Grafik 1"));
}
