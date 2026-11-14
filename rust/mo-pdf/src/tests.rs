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

/// `bytes` with the `startxref` offset pointing at the start of the file instead of the index.
fn broken_xref_offset(mut bytes: Vec<u8>) -> Vec<u8> {
    let at = bytes.windows(9).rposition(|w| w == b"startxref").expect("startxref") + 9;
    let digits = bytes[at..].iter().skip_while(|b| b.is_ascii_whitespace()).take_while(|b| b.is_ascii_digit()).count();
    let start = at + bytes[at..].iter().take_while(|b| b.is_ascii_whitespace()).count();
    assert!(digits > 0);
    // Same length, so nothing else moves: "0000…" leads to the %PDF- header, not an index.
    for b in &mut bytes[start..start + digits] {
        *b = b'0';
    }
    bytes
}

#[test]
fn tells_what_opening_a_damaged_file_repaired() {
    assert!(reopened(pdf(2, "Sayfa")).open_notes().is_empty(), "a sound file needs no repair");
    let doc = reopened(broken_xref_offset(pdf(2, "Sayfa")));
    assert!(!doc.open_notes().is_empty(), "the broken index offset is reported");
    assert!(doc.open_notes().iter().any(|l| l.contains("rebuilt cross-reference index")), "{:?}", doc.open_notes());
    // The repaired document still has its pages, and what is saved from it needs no repair.
    assert_eq!(doc.page_count().unwrap(), 2);
    let saved = reopened(doc.save().unwrap());
    assert!(saved.open_notes().is_empty(), "{:?}", saved.open_notes());
    assert_eq!(saved.page_count().unwrap(), 2);
}

#[test]
fn a_missing_header_is_reported_and_later_edits_do_not_add_notes() {
    let bytes = pdf(1, "Sayfa");
    let at = bytes.windows(5).position(|w| w == b"%PDF-").unwrap();
    let mut headerless = bytes.clone();
    headerless[at..at + 5].copy_from_slice(b"%XXX-");
    let mut doc = reopened(headerless);
    let notes = doc.open_notes().to_vec();
    assert!(notes.iter().any(|l| l.contains("header")), "{notes:?}");
    doc.rotate(&[0], 90).unwrap();
    assert_eq!(doc.open_notes(), &notes[..]);
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

/// A page with personal data on it, as a spreadsheet PDF would carry it.
fn personal() -> Vec<u8> {
    let mut d = gridcraft_pdf::PdfDoc::new();
    let p = d.add_page(595.0, 842.0);
    let lines = [
        "Ad Soyad: Şule Yılmaz",
        "TC Kimlik No: 10000000146",
        "IBAN: TR33 0006 1005 1978 6457 8413 26",
        "Telefon: 0532 123 45 67",
        "E-posta: sule@ornek.com.tr",
        "İSTANBUL",
    ];
    for (i, l) in lines.iter().enumerate() {
        p.text(72.0, 100.0 + 24.0 * i as f32, 12.0, gridcraft_pdf::Font::Helvetica, [0, 0, 0], l);
    }
    d.finish()
}

#[test]
fn finds_text_on_pages_ignoring_turkish_i() {
    let mut doc = PdfDoc::open(personal(), None).unwrap();
    let hits = doc.find_text("istanbul", 10).unwrap();
    assert_eq!(hits.len(), 1);
    let r = hits[0].rects[0];
    // x from 72 pt of 595, y around the 6th line (220 pt of 842, y down).
    assert!((0.11..0.13).contains(&r[0]) && r[1] > 0.22 && r[3] < 0.28, "{r:?}");
    assert_eq!(doc.find_text("yilmaz", 10).unwrap().len(), 1);
    assert!(doc.page_text(0).unwrap().contains("Şule Yılmaz"));
}

#[test]
fn blacks_out_personal_data_for_good() {
    let mut doc = PdfDoc::open(personal(), None).unwrap();
    let marked = doc.mark_redactions(&[Sensitive::TurkishId, Sensitive::Iban, Sensitive::Phone, Sensitive::Email], &["Şule".into()]).unwrap();
    assert_eq!(marked.len(), 5);
    assert_eq!(doc.redaction_marks(), 5);
    let mut undone = PdfDoc::open(doc.save().unwrap(), None).unwrap();
    assert_eq!(undone.clear_redaction_marks().unwrap(), 5);
    assert_eq!(undone.redaction_marks(), 0);
    assert!(undone.page_text(0).unwrap().contains("10000000146"));
    // Marked only: the text is still there until applied.
    assert!(doc.page_text(0).unwrap().contains("10000000146"));
    assert_eq!(doc.apply_redactions().unwrap(), 5);
    if let Ok(path) = std::env::var("DUMP") {
        std::fs::write(path, doc.save().unwrap()).unwrap();
    }
    let mut saved = reopened(doc.save().unwrap());
    let text = saved.page_text(0).unwrap();
    for gone in ["10000000146", "TR33", "0532", "sule@ornek", "Şule"] {
        assert!(!text.contains(gone), "{gone} still in {text:?}");
    }
    assert!(text.contains("Yılmaz") && text.contains("İSTANBUL"), "{text:?}");
    assert_eq!(saved.redaction_marks(), 0);
}

#[test]
fn redacts_all_58_ids_after_reopening_marks() {
    // Synthetic checksum-valid numbers, not a corpus of real people's identities.
    let ids: Vec<String> = (100_000_000..100_000_058)
        .map(|seed| {
            let prefix = seed.to_string();
            let digits: Vec<i32> = prefix.bytes().map(|b| i32::from(b - b'0')).collect();
            let odd: i32 = digits.iter().step_by(2).sum();
            let even: i32 = digits.iter().skip(1).step_by(2).sum();
            let tenth = (7 * odd - even).rem_euclid(10);
            let last = (digits.iter().sum::<i32>() + tenth) % 10;
            format!("{prefix}{tenth}{last}")
        })
        .collect();
    let mut pdf = gridcraft_pdf::PdfDoc::new();
    for chunk in ids.chunks(20) {
        let page = pdf.add_page(595.0, 842.0);
        page.text(72.0, 50.0, 12.0, gridcraft_pdf::Font::Helvetica, [0, 0, 0], "KEEP THIS TEXT");
        for (row, id) in chunk.iter().enumerate() {
            let shown = if row % 2 == 0 { id.clone() } else { format!("{} {} {} {}", &id[..3], &id[3..6], &id[6..9], &id[9..]) };
            page.text(72.0, 100.0 + row as f32 * 24.0, 12.0, gridcraft_pdf::Font::Helvetica, [0, 0, 0], &format!("TC: {shown}"));
        }
    }
    let original = pdf.finish();
    let mut doc = reopened(original.clone());
    assert_eq!(doc.page_count().unwrap(), 3);
    assert_eq!(doc.mark_redactions(&[Sensitive::TurkishId], &[]).unwrap().len(), 58);
    let mut marked = reopened(doc.save().unwrap());
    assert_eq!(marked.redaction_marks(), 58);
    assert_eq!(marked.apply_redactions().unwrap(), 58);
    let bytes = marked.save().unwrap();
    if let Ok(dir) = std::env::var("MO_REDACTION_CORPUS_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::write(dir.join("before.pdf"), original).unwrap();
        std::fs::write(dir.join("after.pdf"), &bytes).unwrap();
        std::fs::write(dir.join("ids.txt"), ids.join("\n")).unwrap();
    }
    let mut saved = reopened(bytes);
    assert_eq!(saved.redaction_marks(), 0);
    for page in 0..3 {
        let text = saved.page_text(page).unwrap();
        assert!(text.contains("KEEP THIS TEXT"), "page {page}: {text}");
        assert!(!text.chars().any(|c| c.is_ascii_digit()), "page {page} leaks digits: {text}");
    }
}

/// A small RGB PNG, `w` × `h`.
fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut e = png::Encoder::new(&mut out, w, h);
    e.set_color(png::ColorType::Rgb);
    e.set_depth(png::BitDepth::Eight);
    let mut wr = e.write_header().unwrap();
    wr.write_image_data(&vec![200u8; (w * h * 3) as usize]).unwrap();
    wr.finish().unwrap();
    out
}

#[test]
fn makes_a_pdf_from_pictures() {
    let pics = vec![("dikey.png".to_string(), png(300, 400)), ("yatay.png".to_string(), png(800, 200))];
    let own = reopened(images_to_pdf(&pics, false).unwrap());
    assert_eq!(own.page_count().unwrap(), 2);
    let a4 = reopened(images_to_pdf(&pics, true).unwrap());
    let boxes = organize::page_boxes(&a4.doc).unwrap();
    let size = |b: [f64; 4]| ((b[2] - b[0]).round(), (b[3] - b[1]).round());
    assert_eq!(size(boxes[0][0]), (595.0, 842.0));
    assert_eq!(size(boxes[1][0]), (842.0, 595.0), "a wide picture gets a landscape page");
    let info = a4.doc.trailer().get(b"Info").map(|i| a4.doc.resolve(i)).and_then(|i| i.as_dict().and_then(|d| d.get(b"Producer").cloned()));
    assert_eq!(info.and_then(|p| p.as_string().map(|s| s.to_text())), Some("Mobile Office Free".to_string()));
    assert!(images_to_pdf(&[], true).is_err());
    assert!(images_to_pdf(&[("x.png".into(), b"not a picture".to_vec())], true).is_err());
}

#[test]
fn shows_signatures_and_typed_text_to_a_renderer_that_draws_no_annotations() {
    let mut doc = reopened(pdf(1, "Form"));
    assert!(!doc.has_drawn_annotations());
    doc.sign(0, &[vec![[0.2, 0.8], [0.4, 0.75], [0.6, 0.85]]]).unwrap();
    doc.add_text(0, [0.1, 0.1], "Şişli Ağaç İğne\nıöüç", 18.0, TextFont::Serif, [0.8, 0.0, 0.0]).unwrap();
    assert!(doc.has_drawn_annotations());

    // The saved file keeps them as annotations; the copy shown has them in the page.
    let saved = reopened(doc.save().unwrap());
    assert!(saved.has_drawn_annotations());
    let shown = reopened(doc.display_copy().unwrap());
    assert!(!shown.has_drawn_annotations());
    let mut shown = shown;
    let text = shown.page_text(0).unwrap();
    assert!(text.contains("Şişli Ağaç İğne"), "{text}");

    // Undo takes the text away again.
    assert!(doc.undo_annotation().unwrap());
    let mut back = reopened(doc.display_copy().unwrap());
    assert!(!back.page_text(0).unwrap().contains("Şişli"));
}

#[test]
fn reads_forms_made_by_browser_pdf_libraries() {
    // jsPDF's AcroForm plugin (what browser-made forms such as Gemini's often use) and pdf-lib.
    for bytes in [&include_bytes!("../testdata/jspdf-form.pdf")[..], &include_bytes!("../testdata/pdflib-form.pdf")[..]] {
        let doc = reopened(bytes.to_vec());
        let kinds: Vec<FieldKind> = doc.fields().iter().map(|f| f.kind).collect();
        assert_eq!(kinds, vec![FieldKind::Text, FieldKind::CheckBox, FieldKind::Choice, FieldKind::Radio]);
        assert!(doc.form_diagnosis().starts_with("AcroForm: var"), "{}", doc.form_diagnosis());
    }
    assert!(reopened(pdf(1, "x")).form_diagnosis().starts_with("AcroForm: yok"));
}

#[test]
fn finds_the_fields_of_a_form_printed_from_a_browser() {
    // An HTML form printed to PDF by Chromium (as AI assistants' "fillable" forms often are):
    // the boxes are drawn, rounded, and the fields are gone.
    let mut doc = reopened(include_bytes!("../testdata/browser-printed-form.pdf").to_vec());
    assert!(doc.fields().is_empty());
    assert_eq!(doc.detect_fields().unwrap(), 5);
    let fields: Vec<(String, FieldKind)> = doc.fields().into_iter().map(|f| (f.name, f.kind)).collect();
    assert!(fields.contains(&("Adı Soyadı".into(), FieldKind::Text)), "{fields:?}");
    assert!(fields.contains(&("Onaylıyorum".into(), FieldKind::CheckBox)), "{fields:?}");
    // The text area's label is above it.
    assert!(fields.contains(&("Açıklama".into(), FieldKind::Text)), "{fields:?}");
    doc.set_text("Adı Soyadı", "Şükrü Ağaoğlu").unwrap();
    doc.set_check("Onaylıyorum", true).unwrap();
    // Shown and saved with the value; finding again adds nothing.
    let mut shown = reopened(doc.display_copy().unwrap());
    assert!(shown.page_text(0).unwrap().contains("Şükrü Ağaoğlu"));
    let mut saved = reopened(doc.save().unwrap());
    assert_eq!(saved.fields().len(), 5);
    assert_eq!(saved.detect_fields().unwrap(), 0);
}

#[test]
fn failed_text_update_preserves_original_text_and_undo_history() {
    let mut doc = reopened(pdf(1, "Sayfa"));
    doc.add_text(0, [0.1, 0.1], "Korunacak metin", 14.0, TextFont::Sans, [0.0, 0.0, 0.0]).unwrap();
    let before = doc.typed_texts(0).unwrap();
    let look = TextLook { size: 24.0, font: TextFont::SerifBold, color: [0.8, 0.0, 0.0] };
    assert!(doc.update_text(0, before[0].index, [0.5, 0.6], " \n\t ", look).is_err());
    let after = doc.typed_texts(0).unwrap();
    assert_eq!(after.len(), 1, "failed edit must not delete the original annotation");
    assert_eq!(after[0].text, before[0].text);
    assert_eq!(after[0].rect, before[0].rect);
    assert_eq!(after[0].size, before[0].size);
    assert_eq!(reopened(doc.save().unwrap()).typed_texts(0).unwrap()[0].text, "Korunacak metin");
    assert!(doc.undo_annotation().unwrap(), "the original undo entry must survive a failed edit");
    assert!(doc.typed_texts(0).unwrap().is_empty());
}

#[test]
fn typed_text_can_be_moved_restyled_and_removed() {
    let mut doc = reopened(pdf(1, "Sayfa"));
    doc.add_text(0, [0.1, 0.1], "Taşınacak yazı", 14.0, TextFont::Sans, [0.0, 0.0, 0.0]).unwrap();
    let t = doc.typed_texts(0).unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].text.as_str(), t[0].size, t[0].font), ("Taşınacak yazı", 14.0, TextFont::Sans));
    // Its box starts where it was put (fractions of the page, y down).
    assert!((t[0].rect[0] - 0.1).abs() < 0.01 && (t[0].rect[1] - 0.1).abs() < 0.01, "{:?}", t[0].rect);

    let look = TextLook { size: 24.0, font: TextFont::SerifBold, color: [0.8, 0.0, 0.0] };
    doc.update_text(0, t[0].index, [0.5, 0.6], "Taşındı ve büyüdü", look).unwrap();
    let t = doc.typed_texts(0).unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].text.as_str(), t[0].size, t[0].font), ("Taşındı ve büyüdü", 24.0, TextFont::SerifBold));
    assert!((t[0].rect[0] - 0.5).abs() < 0.01 && (t[0].rect[1] - 0.6).abs() < 0.01, "{:?}", t[0].rect);
    // Kept after saving; the shown page has the new text in its place.
    let saved = reopened(doc.save().unwrap());
    assert_eq!(saved.typed_texts(0).unwrap()[0].text, "Taşındı ve büyüdü");
    assert!(reopened(doc.display_copy().unwrap()).page_text(0).unwrap().contains("Taşındı ve büyüdü"));
    // Undo takes back the moved text (one step); deleting removes it.
    assert!(doc.undo_annotation().unwrap());
    assert!(doc.typed_texts(0).unwrap().is_empty());
    doc.add_text(0, [0.2, 0.2], "Silinecek", 12.0, TextFont::Mono, [0.0, 0.0, 0.0]).unwrap();
    let i = doc.typed_texts(0).unwrap()[0].index;
    doc.delete_text(0, i).unwrap();
    assert!(doc.typed_texts(0).unwrap().is_empty());
    assert!(!doc.undo_annotation().unwrap());
}

#[test]
fn text_is_selected_between_two_points() {
    let mut doc = reopened(pdf(1, "Seçilecek metin burada"));
    // "Seçilecek metin burada 1" is at x 72, y 100 on a 595 × 842 page (y up, so 742 from the top).
    let hit = doc.find_text("Seçilecek", 1).unwrap();
    let r = hit[0].rects[0];
    let mid = ((r[1] + r[3]) / 2.0) as f64;
    let (text, rects) = doc.text_between(0, [r[0] as f64 + 0.001, mid], [0.9, mid]).unwrap();
    assert!(text.starts_with("Seçilecek metin burada"), "{text:?}");
    assert_eq!(rects.len(), 1);
    // Backwards works the same.
    let (back, _) = doc.text_between(0, [0.9, mid], [r[0] as f64 + 0.001, mid]).unwrap();
    assert_eq!(back, text);
}

#[test]
fn fields_with_default_names_take_their_printed_labels() {
    // Made with ReportLab: "Full name:" before text_1, "Address" above Text2, "I agree" after
    // the check box "undefined", and "Telefon" (a real name) after "Phone:".
    let mut doc = reopened(include_bytes!("../testdata/generic-names-form.pdf").to_vec());
    let labels: Vec<(String, String)> = doc.labelled_fields().into_iter().map(|f| (f.name, f.label)).collect();
    let label = |n: &str| labels.iter().find(|(name, _)| name == n).map(|(_, l)| l.as_str()).unwrap_or_default().to_string();
    assert_eq!(label("text_1"), "Full name", "{labels:?}");
    assert_eq!(label("Text2"), "Address", "{labels:?}");
    assert_eq!(label("undefined"), "I agree", "{labels:?}");
    assert_eq!(label("Telefon"), "Telefon", "{labels:?}");
}

#[test]
fn generic_tooltips_do_not_hide_printed_labels_or_meaningful_names() {
    use pdfcraft_cos::{Object, PdfString};
    let mut doc = reopened(include_bytes!("../testdata/generic-names-form.pdf").to_vec());
    for field in pdfcraft_forms::fields(&doc.doc) {
        let tooltip = if field.name == "Text2" { "Postal address" } else { "text" };
        doc.doc
            .update_dict(field.obj, |d| {
                d.set(b"TU".to_vec(), Object::String(PdfString::literal(tooltip.as_bytes().to_vec())));
            })
            .unwrap();
    }
    let mut doc = reopened(doc.save().unwrap());
    let before = doc.fields();
    let labelled = doc.labelled_fields();
    let label = |name: &str| labelled.iter().find(|f| f.name == name).unwrap().label.as_str();
    assert_eq!(label("text_1"), "Full name");
    assert_eq!(label("Text2"), "Postal address");
    assert_eq!(label("undefined"), "I agree");
    assert_eq!(label("Telefon"), "Telefon");
    // Display labels must not rewrite field identities or tooltip metadata in the source.
    assert_eq!(doc.fields(), before);
}

#[test]
fn selected_text_can_be_highlighted_underlined_and_struck() {
    let mut doc = reopened(pdf(1, "İşaretlenecek satır"));
    let hit = doc.find_text("İşaretlenecek", 1).unwrap();
    let r = hit[0].rects[0];
    let mid = ((r[1] + r[3]) / 2.0) as f64;
    let (_, rects) = doc.text_between(0, [r[0] as f64 + 0.001, mid], [0.9, mid]).unwrap();
    let rects: Vec<[f64; 4]> = rects.iter().map(|q| q.map(f64::from)).collect();
    assert!(!doc.has_drawn_annotations());
    doc.mark_text(0, &rects, 0, [1.0, 0.9, 0.0]).unwrap();
    doc.mark_text(0, &rects, 1, [0.0, 0.0, 1.0]).unwrap();
    doc.mark_text(0, &rects, 2, [1.0, 0.0, 0.0]).unwrap();
    let saved = doc.save().unwrap();
    let mut again = reopened(saved);
    assert!(again.has_drawn_annotations());
    // The text is still there to read and find.
    assert!(again.page_text(0).unwrap().contains("İşaretlenecek"));
}

#[test]
fn recognised_words_make_a_scanned_page_searchable_without_changing_its_look() {
    let mut doc = PdfDoc::open(pdf(1, "x"), None).unwrap();
    let words = vec![
        ("İSTANBUL".to_string(), [0.12, 0.30, 0.40, 0.34]),
        ("Şirketi".to_string(), [0.42, 0.30, 0.60, 0.34]),
        ("ığdır".to_string(), [0.12, 0.40, 0.30, 0.44]),
    ];
    assert_eq!(doc.add_ocr_words(0, &words).unwrap(), 3);
    // Found, Turkish letters included, and kept in the saved file.
    let saved = reopened(doc.save().unwrap());
    let mut saved = saved;
    assert_eq!(saved.find_text("istanbul", 10).unwrap().len(), 1);
    assert_eq!(saved.find_text("şirketi", 10).unwrap().len(), 1);
    assert!(saved.page_text(0).unwrap().contains("ığdır"), "{}", saved.page_text(0).unwrap());
    // Invisible: written in text render mode 3 (neither filled nor stroked).
    let page_ref = annot::page_refs(&saved.doc).unwrap()[0];
    let contents = saved.doc.get(page_ref).as_dict().and_then(|d| d.get(b"Contents").cloned()).unwrap();
    use pdfcraft_cos::Object;
    let top = saved.doc.resolve(&contents);
    let streams: Vec<std::sync::Arc<Object>> = match top.as_ref() {
        Object::Array(a) => a.iter().map(|o| saved.doc.resolve(o)).collect(),
        _ => vec![top.clone()],
    };
    let plain = streams
        .iter()
        .filter_map(|o| match o.as_ref() {
            Object::Stream(st) => Some(st),
            _ => None,
        })
        .filter_map(|st| st.decoded().ok())
        .any(|d| d.windows(4).any(|w| w == b"3 Tr"));
    assert!(plain, "the words are drawn invisibly");
}
