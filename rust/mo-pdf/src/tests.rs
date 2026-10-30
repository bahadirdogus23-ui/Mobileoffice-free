use super::*;

/// A PDF of `n` A4 pages reading "Sayfa 1" … "Sayfa n", from the spreadsheet's PDF writer.
fn pdf(n: usize, label: &str) -> Vec<u8> {
    let mut d = gridcraft_pdf::PdfDoc::new();
    for i in 1..=n {
        let p = d.add_page(595.0, 842.0);
        p.text(72.0, 100.0, 14.0, gridcraft_pdf::Font::Helvetica, [0, 0, 0], &format!("{label} {i}"));
    }
    d.finish()
}

fn reopened(bytes: Vec<u8>) -> PdfDoc {
    PdfDoc::open(bytes, None).unwrap()
}

#[test]
fn rotates_deletes_and_moves_pages() {
    let mut doc = PdfDoc::open(pdf(4, "Sayfa"), None).unwrap();
    assert_eq!(doc.page_count().unwrap(), 4);
    doc.rotate(&[1], 90).unwrap();
    assert_eq!(doc.rotation(1).unwrap(), 90);
    doc.delete(&[0]).unwrap();
    assert_eq!(doc.page_count().unwrap(), 3);
    // The rotated page (was 2nd) is now first; move it to the end.
    assert_eq!(doc.rotation(0).unwrap(), 90);
    doc.move_pages(&[0], 2).unwrap();
    assert_eq!(doc.rotation(2).unwrap(), 90);
    let saved = reopened(doc.save().unwrap());
    assert_eq!(saved.page_count().unwrap(), 3);
    assert_eq!(saved.rotation(2).unwrap(), 90);
}

#[test]
fn never_deletes_every_page() {
    let mut doc = PdfDoc::open(pdf(2, "Sayfa"), None).unwrap();
    assert!(doc.delete(&[0, 1]).is_err());
    assert_eq!(doc.page_count().unwrap(), 2);
}

#[test]
fn combines_files_and_extracts_pages() {
    let mut doc = PdfDoc::open(pdf(2, "Birinci"), None).unwrap();
    doc.append(pdf(3, "İkinci"), None).unwrap();
    assert_eq!(doc.page_count().unwrap(), 5);
    let part = reopened(doc.extract(&[4, 0]).unwrap());
    assert_eq!(part.page_count().unwrap(), 2);
    // The source is untouched.
    assert_eq!(doc.page_count().unwrap(), 5);
    assert!(doc.extract(&[9]).is_err());
}

#[test]
fn bad_input_is_an_error() {
    assert!(matches!(PdfDoc::open(b"%PDF-1.7 broken".to_vec(), None), Err(PdfError::Open(_))));
}
