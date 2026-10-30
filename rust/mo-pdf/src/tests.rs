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

#[test]
fn signs_a_page_where_it_was_drawn() {
    let mut doc = PdfDoc::open(pdf(2, "Sayfa"), None).unwrap();
    // A small zigzag near the bottom right of the second page.
    doc.sign(1, &[vec![[0.6, 0.8], [0.7, 0.85], [0.8, 0.8]], vec![]]).unwrap();
    let saved = reopened(doc.save().unwrap());
    let found = pdfcraft_annot::summaries(&saved.doc);
    assert_eq!(found.len(), 1);
    let s = &found[0];
    assert_eq!((s.page, s.subtype.as_str()), (1, "Ink"));
    // 0.6–0.8 of 595 wide, 0.8–0.85 down an 842 high page (y up), plus the stroke's width.
    let [x0, y0, x1, y1] = s.rect;
    assert!((350.0..357.0).contains(&x0) && (476.0..483.0).contains(&x1), "{:?}", s.rect);
    assert!((120.0..127.0).contains(&y0) && (165.0..172.0).contains(&y1), "{:?}", s.rect);
    assert!(doc.sign(0, &[vec![]]).is_err());
    assert!(doc.sign(5, &[vec![[0.5, 0.5]]]).is_err());
}

#[test]
fn display_fractions_follow_the_page_rotation() {
    let crop = [0.0, 0.0, 100.0, 200.0];
    // The displayed top left corner, for each /Rotate.
    assert_eq!(user_point(crop, 0, [0.0, 0.0]), [0.0, 200.0]);
    assert_eq!(user_point(crop, 90, [0.0, 0.0]), [0.0, 0.0]);
    assert_eq!(user_point(crop, 180, [0.0, 0.0]), [100.0, 0.0]);
    assert_eq!(user_point(crop, 270, [0.0, 0.0]), [100.0, 200.0]);
    assert_eq!(user_point(crop, -90, [1.0, 1.0]), [0.0, 0.0]);
}
