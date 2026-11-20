use pdfcraft_cos::{Dict, Document, ImageBudget, Object, SaveOptions, Stream};

use crate::{PdfDoc, PdfError};

const W: u32 = 300;
const H: u32 = 420;

/// A smooth colour picture, as a scanned page is.
fn scan_pixels() -> Vec<u8> {
    (0..H).flat_map(|y| (0..W).flat_map(move |x| [(60 + x * 150 / W) as u8, (80 + y * 150 / H) as u8, 200])).collect()
}

fn jpeg() -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90).encode(&scan_pixels(), W, H, image::ExtendedColorType::Rgb8).unwrap();
    out
}

fn image_dict(w: u32, h: u32, cs: &str, filter: Option<&str>) -> Dict {
    let mut d = Dict::new();
    d.set(b"Type".to_vec(), Object::name("XObject"));
    d.set(b"Subtype".to_vec(), Object::name("Image"));
    d.set(b"Width".to_vec(), Object::Int(i64::from(w)));
    d.set(b"Height".to_vec(), Object::Int(i64::from(h)));
    d.set(b"ColorSpace".to_vec(), Object::name(cs));
    d.set(b"BitsPerComponent".to_vec(), Object::Int(8));
    if let Some(f) = filter {
        d.set(b"Filter".to_vec(), Object::name(f));
    }
    d
}

/// One A4 page as a scanner makes it: `scan` drawn over the whole page, a small grey picture (a
/// stamp) at the bottom right, and a picture with a soft mask (a badge) at the top right.
fn scanned_page(scan: Stream) -> Vec<u8> {
    let mut doc = pdfcraft_create::blank(595.0, 842.0, 1).unwrap();
    let page = pdfcraft_annot::page_refs(&doc).unwrap()[0];
    let scan = doc.add(Object::Stream(scan));
    let stamp: Vec<u8> = (0..40 * 40).map(|i| (i % 251) as u8).collect();
    let stamp = doc.add(Object::Stream(Stream::flate(image_dict(40, 40, "DeviceGray", None), &stamp)));
    let alpha: Vec<u8> = (0..20 * 20).map(|i| 100 + (i % 100) as u8).collect();
    let alpha = doc.add(Object::Stream(Stream::flate(image_dict(20, 20, "DeviceGray", None), &alpha)));
    let mut badge = image_dict(20, 20, "DeviceRGB", None);
    badge.set(b"SMask".to_vec(), Object::Ref(alpha));
    let badge = doc.add(Object::Stream(Stream::flate(badge, &[0x80; 20 * 20 * 3])));
    let content = "q 595 0 0 842 0 0 cm /Scan Do Q q 80 0 0 80 500 20 cm /Stamp Do Q q 100 0 0 100 495 742 cm /Badge Do Q\n";
    let content = doc.add(Object::Stream(Stream::flate(Dict::new(), content.as_bytes())));
    let mut xobjects = Dict::new();
    for (name, r) in [("Scan", scan), ("Stamp", stamp), ("Badge", badge)] {
        xobjects.set(name.as_bytes().to_vec(), Object::Ref(r));
    }
    let mut resources = Dict::new();
    resources.set(b"XObject".to_vec(), Object::Dict(xobjects));
    doc.update_dict(page, |d| {
        d.set(b"Resources".to_vec(), Object::Dict(resources));
        d.set(b"Contents".to_vec(), Object::Ref(content));
    })
    .unwrap();
    pdfcraft_cos::write_full(&doc, &SaveOptions::default()).unwrap()
}

/// The page's pictures.
fn pictures(doc: &Document) -> Vec<Stream> {
    let page = pdfcraft_annot::page_refs(doc).unwrap()[0];
    let res = super::page_resources(doc, page).unwrap();
    let xo = doc.resolve(res.get(b"XObject").unwrap()).as_dict().cloned().unwrap();
    xo.iter()
        .filter_map(|(_, v)| match &*doc.resolve(v) {
            Object::Stream(s) if s.dict.name(b"Subtype") == Some(b"Image".as_slice()) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

fn picture_of(doc: &Document, width: i64) -> Stream {
    pictures(doc).into_iter().find(|s| s.dict.int(b"Width") == Some(width)).expect("the picture is still on the page")
}

fn soft_mask(doc: &Document, s: &Stream) -> Stream {
    match &*doc.resolve(s.dict.get(b"SMask").expect("a soft mask")) {
        Object::Stream(m) => m.clone(),
        _ => panic!("the soft mask is not a stream"),
    }
}

/// A scan with the hidden text a recognised page carries; the word `word` at `at` (fractions of
/// the page, y down) is marked for blacking out.
fn marked(scan: Stream, word: &str, at: [f64; 4]) -> PdfDoc {
    let mut doc = PdfDoc::open(scanned_page(scan), None).unwrap();
    let words = vec![(word.to_string(), at), ("Kalacak".to_string(), [0.40, 0.60, 0.70, 0.65])];
    doc.add_ocr_words(0, &words).unwrap();
    assert_eq!(doc.mark_redactions(&[], &[word.to_lowercase()]).unwrap().len(), 1);
    doc
}

fn jpeg_scan(cs: &str) -> Stream {
    Stream::from_raw(image_dict(W, H, cs, Some("DCTDecode")), jpeg())
}

#[test]
fn blacking_out_a_corner_of_a_scan_keeps_the_rest_of_the_page() {
    let jpeg = jpeg();
    let mut doc = marked(Stream::from_raw(image_dict(W, H, "DeviceRGB", Some("DCTDecode")), jpeg.clone()), "Gizli", [0.05, 0.05, 0.30, 0.10]);
    let before = doc.doc.clone();
    assert_eq!(doc.apply_redactions().unwrap(), 1);
    let saved = doc.save().unwrap();
    let mut again = PdfDoc::open(saved.clone(), None).unwrap();
    // The text under the mark is gone, the rest stays.
    let text = again.page_text(0).unwrap();
    assert!(!text.contains("Gizli") && text.contains("Kalacak"), "{text:?}");
    assert_eq!(pictures(&again.doc).len(), 3, "every picture is still drawn");

    // The scan is still there, the same size, a JPEG again.
    let scan = picture_of(&again.doc, i64::from(W));
    assert_eq!(scan.dict.int(b"Height"), Some(i64::from(H)));
    assert_eq!(scan.dict.name(b"Filter"), Some(b"DCTDecode".as_slice()));
    assert!(!scan.dict.contains(super::PROBE));
    let px = pdfcraft_optimize::decode_samples(&again.doc, &scan, &mut ImageBudget::new(&again.doc)).unwrap();
    assert_eq!((px.width, px.height, px.components), (W, H, 3));
    let at = |x: u32, y: u32| px.data[((y * W + x) * 3) as usize..((y * W + x) * 3 + 3) as usize].to_vec();
    // The middle of the marked word (x 5–30 %, y 5–10 % from the top): black.
    assert!(at(52, 31).iter().all(|&v| v < 24), "{:?}", at(52, 31));
    // Far from it: the picture as it was, within what storing a JPEG again changes.
    let original = scan_pixels();
    for (x, y) in [(225, 300), (150, 210), (20, 400), (280, 10)] {
        let o = &original[((y * W + x) * 3) as usize..((y * W + x) * 3 + 3) as usize];
        for (a, b) in at(x, y).iter().zip(o) {
            assert!(a.abs_diff(*b) <= 8, "pixel ({x}, {y}) is {:?}, was {o:?}", at(x, y));
        }
    }

    // Pictures the mark doesn't touch are the very same bytes.
    for width in [40, 20] {
        let (old, new) = (picture_of(&before, width), picture_of(&again.doc, width));
        assert_eq!(old.raw, new.raw);
    }
    assert_eq!(soft_mask(&before, &picture_of(&before, 20)).raw, soft_mask(&again.doc, &picture_of(&again.doc, 20)).raw);
    // Nothing of the original scan is left in the file (its pixels under the mark included).
    let middle = &jpeg[jpeg.len() / 2..jpeg.len() / 2 + 64];
    assert!(!saved.windows(middle.len()).any(|w| w == middle), "the original scan is still in the file");
}

#[test]
fn a_picture_with_a_soft_mask_keeps_its_uncovered_part() {
    // The badge spans x 495–595, y 742–842 pt; the mark covers its left part, y 775–825.
    let scan = Stream::flate(image_dict(W, H, "DeviceRGB", None), &scan_pixels());
    let mut doc = marked(scan, "Rozet", [0.80, 0.02, 0.88, 0.08]);
    assert_eq!(doc.apply_redactions().unwrap(), 1);
    let again = PdfDoc::open(doc.save().unwrap(), None).unwrap();
    let badge = picture_of(&again.doc, 20);
    let mut budget = ImageBudget::new(&again.doc);
    let px = pdfcraft_optimize::decode_samples(&again.doc, &badge, &mut budget).unwrap();
    let alpha = pdfcraft_optimize::decode_samples(&again.doc, &soft_mask(&again.doc, &badge), &mut budget).unwrap();
    // Badge pixel (0, 6) lies under the mark: black and opaque; (18, 10) beyond it: as it was.
    let (under, beyond) = (6 * 20, 10 * 20 + 18);
    assert_eq!(&px.data[under * 3..under * 3 + 3], &[0, 0, 0]);
    assert_eq!(alpha.data[under], 255);
    assert_eq!(&px.data[beyond * 3..beyond * 3 + 3], &[0x80; 3]);
    assert_eq!(alpha.data[beyond], 100 + (beyond % 100) as u8);
    // The scan below it keeps its size too.
    assert_eq!(picture_of(&again.doc, i64::from(W)).dict.int(b"Height"), Some(i64::from(H)));
}

#[test]
fn a_picture_that_cannot_be_read_refuses_the_black_out_and_changes_nothing() {
    // A JPEG in CMYK: its pixels aren't read here, so the covered part can't be blackened.
    let mut doc = marked(jpeg_scan("DeviceCMYK"), "Gizli", [0.05, 0.05, 0.30, 0.10]);
    let before = doc.save().unwrap();
    assert_eq!(doc.apply_redactions(), Err(PdfError::Unredactable(1)));
    assert_eq!(doc.redaction_marks(), 1);
    assert!(doc.page_text(0).unwrap().contains("Gizli"));
    assert_eq!(doc.save().unwrap(), before);
}

#[test]
fn a_picture_the_mark_covers_whole_is_removed() {
    // A box drawn over the stamp at the bottom right (x 500–580, y 20–100 pt).
    let mut doc = PdfDoc::open(scanned_page(jpeg_scan("DeviceRGB")), None).unwrap();
    let quads = vec![[490.0, 110.0, 590.0, 110.0, 490.0, 10.0, 590.0, 10.0]];
    let shape = pdfcraft_annot::Shape::Redact { quads, overlay: String::new(), look: Default::default() };
    let style = pdfcraft_annot::Style::default_for(&shape);
    doc.annotate(0, shape, style, String::new()).unwrap();
    assert_eq!(doc.apply_redactions().unwrap(), 1);
    let again = PdfDoc::open(doc.save().unwrap(), None).unwrap();
    let widths: Vec<i64> = pictures(&again.doc).iter().filter_map(|s| s.dict.int(b"Width")).collect();
    assert!(!widths.contains(&40), "{widths:?}");
    assert!(widths.contains(&i64::from(W)), "the scan under it stays: {widths:?}");
}

#[test]
fn coarse_stand_ins_cover_every_pixel_they_touch() {
    // A 4 × 2 stand-in with its pixel (1, 0) covered, laid over a 10 × 5 grid.
    let mut mask = vec![0xFF; 8];
    mask[1] = 0;
    let cover = super::Cover { mask: &mask, pw: 4, ph: 2 };
    let mut rows = Vec::new();
    cover.rows(10, 5, |j, row| rows.push((j, row.iter().enumerate().filter(|(_, c)| **c).map(|(i, _)| i).collect::<Vec<_>>())));
    // Columns 2–4 overlap stand-in column 1 (2.5–5), rows 0–2 its row 0 (0–2.5).
    assert_eq!(rows, vec![(0, vec![2, 3, 4]), (1, vec![2, 3, 4]), (2, vec![2, 3, 4])]);
}
