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

#[test]
fn protects_with_a_password_and_shows_without_one() {
    let mut doc = PdfDoc::open(pdf(2, "Gizli"), None).unwrap();
    assert!(!doc.is_protected());
    assert!(doc.protect("", [7; 32]).is_err());
    doc.protect("şifre123", [7; 32]).unwrap();
    assert!(doc.is_protected());
    let locked = doc.save().unwrap();
    assert_eq!(PdfDoc::open(locked.clone(), None).err(), Some(PdfError::NeedsPassword));
    assert_eq!(PdfDoc::open(locked.clone(), Some("yanlış")).err(), Some(PdfError::WrongPassword));
    let mut opened = PdfDoc::open(locked, Some("şifre123")).unwrap();
    assert!(opened.is_protected() && !opened.is_restricted());
    // The on-screen copy needs no password; pages taken out keep it.
    assert_eq!(reopened(opened.display_copy().unwrap()).page_count().unwrap(), 2);
    assert_eq!(PdfDoc::open(opened.extract(&[1]).unwrap(), None).err(), Some(PdfError::NeedsPassword));
    opened.rotate(&[0], 90).unwrap();
    opened.unprotect().unwrap();
    let open = reopened(opened.save().unwrap());
    assert!(!open.is_protected());
    assert_eq!(open.rotation(0).unwrap(), 90);
}

#[test]
fn respects_the_owners_restrictions() {
    let mut cos = Document::open(Arc::new(pdf(1, "Kısıtlı"))).unwrap();
    let params = pdfcraft_cos::NewEncryption {
        algorithm: pdfcraft_cos::Algorithm::Aes128,
        user_password: "",
        owner_password: "sahip",
        permissions: -4 & !(1 << 3), // no modifying
        encrypt_metadata: true,
        seed: [3; 32],
    };
    cos.set_encryption(&params).unwrap();
    let bytes = pdfcraft_cos::write_full(&cos, &SaveOptions::default()).unwrap();
    // An empty user password opens it at once, with the restrictions.
    let mut doc = PdfDoc::open(bytes.clone(), None).unwrap();
    assert!(doc.is_restricted());
    assert!(doc.rotate(&[0], 90).is_err());
    assert!(doc.unprotect().is_err());
    assert_eq!(reopened(doc.display_copy().unwrap()).page_count().unwrap(), 1);
    // The owner's password lifts them.
    let mut owner = PdfDoc::open(bytes, Some("sahip")).unwrap();
    assert!(!owner.is_restricted());
    owner.rotate(&[0], 90).unwrap();
}

#[test]
fn compresses_a_photo_heavy_file() {
    // A 1500 × 1500 px picture in a 200 pt box (540 ppi), written uncompressed.
    let mut d = gridcraft_pdf::PdfDoc::new();
    let p = d.add_page(595.0, 842.0);
    let n = 1500u32;
    let rgba: Vec<u8> = (0..n * n).flat_map(|i| [(i % n) as u8, (i / n) as u8, ((i * 7) % 251) as u8, 255]).collect();
    p.image_rgba(100.0, 100.0, 200.0, 200.0, &rgba, n, n);
    let big = d.finish();
    let doc = PdfDoc::open(big.clone(), None).unwrap();
    let small = doc.compress(false).unwrap();
    let smaller = doc.compress(true).unwrap();
    assert!(small.len() * 5 < big.len(), "{} → {}", big.len(), small.len());
    assert!(smaller.len() <= small.len());
    assert_eq!(reopened(small).page_count().unwrap(), 1);
}
#[test]
fn fills_a_form_with_turkish_letters() {
    use pdfcraft_forms as f;
    let mut doc = PdfDoc::open(pdf(1, "Form"), None).unwrap();
    assert!(doc.fields().is_empty());
    f::add_field(&mut doc.doc, 0, [72.0, 600.0, 400.0, 630.0], &f::NewField::Text { multiline: false }, Some("ad")).unwrap();
    f::add_field(&mut doc.doc, 0, [72.0, 560.0, 400.0, 590.0], &f::NewField::Text { multiline: false }, Some("il")).unwrap();
    f::add_field(&mut doc.doc, 0, [72.0, 520.0, 92.0, 540.0], &f::NewField::CheckBox, Some("onay")).unwrap();
    doc.set_text("ad", "Şükrü Güneş İĞDIR ığüşöç").unwrap();
    doc.set_text("il", "Ankara").unwrap();
    doc.set_check("onay", true).unwrap();
    assert!(doc.set_text("yok", "x").is_err());
    let saved = reopened(doc.save().unwrap());
    let fields = saved.fields();
    let by = |n: &str| fields.iter().find(|x| x.name == n).unwrap().clone();
    assert_eq!(by("ad").value, "Şükrü Güneş İĞDIR ığüşöç");
    assert_eq!((by("ad").kind, by("ad").page), (FieldKind::Text, Some(0)));
    assert_eq!(by("onay").kind, FieldKind::CheckBox);
    assert!(!by("onay").value.is_empty());
    if let Ok(path) = std::env::var("DUMP") {
        std::fs::write(path, saved.save().unwrap()).unwrap();
    }
    // Drawn by us in the embedded font, not left for the viewer to draw.
    assert!(!pdfcraft_forms::need_appearances(&saved.doc));
    let da = |n: &str| pdfcraft_forms::fields(&saved.doc).into_iter().find(|x| x.name == n).unwrap().da;
    assert!(da("ad").contains("/MoTr"), "{}", da("ad"));
    assert!(!da("il").contains("/MoTr"), "plain text keeps its font");
}

#[test]
fn draws_highlights_notes_and_undoes_only_its_own() {
    let mut doc = PdfDoc::open(pdf(1, "Not"), None).unwrap();
    // A comment the file already had stays whatever is undone.
    doc.sign(0, &[vec![[0.1, 0.1], [0.2, 0.2]]]).unwrap();
    let mut doc = reopened(doc.save().unwrap());
    assert!(!doc.undo_annotation().unwrap());
    doc.draw(0, &[vec![[0.1, 0.5], [0.9, 0.5]]], [1.0, 0.9, 0.0], 12.0, 0.4).unwrap();
    doc.note(0, [0.5, 0.3], "Şunu kontrol et: ığüşöç İ").unwrap();
    let kinds = |d: &PdfDoc| {
        let mut k = pdfcraft_annot::summaries(&d.doc).into_iter().map(|s| s.subtype).collect::<Vec<_>>();
        k.sort();
        k
    };
    assert_eq!(kinds(&doc), ["Ink", "Ink", "Text"]);
    let saved = reopened(doc.save().unwrap());
    let note = pdfcraft_annot::summaries(&saved.doc).into_iter().find(|s| s.subtype == "Text").unwrap();
    assert_eq!(note.contents.as_deref(), Some("Şunu kontrol et: ığüşöç İ"));
    assert!(doc.undo_annotation().unwrap());
    assert!(doc.undo_annotation().unwrap());
    assert!(!doc.undo_annotation().unwrap());
    // The note's popup goes with it.
    assert_eq!(kinds(&doc), ["Ink"]);
    let page = pdfcraft_annot::page_refs(&doc.doc).unwrap()[0];
    let annots = doc.doc.get(page).as_dict().and_then(|d| d.get(b"Annots").cloned()).map(|a| doc.doc.resolve(&a).as_array().map_or(0, Vec::len));
    assert_eq!(annots, Some(1));
}
