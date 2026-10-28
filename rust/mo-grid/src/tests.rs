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
