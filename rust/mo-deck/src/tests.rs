use super::*;

/// The engine's sample deck ("Northern Lights"), written out as PPTX bytes.
fn sample_pptx() -> Vec<u8> {
    let mut s = deckcraft_engine::Session::default();
    deckcraft_engine::sample::open_sample(&mut s).unwrap();
    deckcraft_pptx::export(&s.doc().unwrap().doc).unwrap()
}

fn ink(r: &Raster) -> usize {
    r.pixels.as_chunks::<4>().0.iter().filter(|p| p[3] > 0 && (p[0] < 200 || p[1] < 200 || p[2] < 200)).count()
}

#[test]
fn pptx_opens_and_slides_render() {
    let deck = DeckDoc::open(&sample_pptx()).unwrap();
    assert!(deck.slide_count() >= 5, "{}", deck.slide_count());
    let (w, h) = deck.slide_size();
    assert!((w / h - 16.0 / 9.0).abs() < 0.01, "{w}×{h}");
    let r = deck.render_slide(0, 0.5).unwrap();
    assert_eq!((r.width, r.height), ((w * 0.5).round() as u32, (h * 0.5).round() as u32));
    assert_eq!(r.pixels.len(), (r.width * r.height * 4) as usize);
    assert!(ink(&r) > 1000, "title slide is drawn");
    assert!(deck.notes(0).unwrap().contains("Welcome"));
}

#[test]
fn a_tile_matches_the_same_area_of_the_whole_slide() {
    let deck = DeckDoc::open(&sample_pptx()).unwrap();
    let whole = deck.render_slide(0, 1.0).unwrap();
    let tile = deck.render_tile(0, 1.0, 100.0, 120.0, 64, 48).unwrap();
    assert_eq!((tile.width, tile.height), (64, 48));
    let mut differ = 0;
    for y in 0..48 {
        for x in 0..64 {
            let t = &tile.pixels[((y * 64 + x) * 4) as usize..][..4];
            let w = &whole.pixels[(((120 + y) * whole.width + 100 + x) * 4) as usize..][..4];
            differ += t.iter().zip(w).any(|(a, b)| a.abs_diff(*b) > 8) as u32;
        }
    }
    assert!(differ < 64 * 48 / 50, "{differ} pixels differ");
}

#[test]
fn non_pptx_input_is_an_error() {
    assert!(matches!(DeckDoc::open(b"not a deck"), Err(DeckError::Open(_))));
    let deck = DeckDoc::from_presentation(Presentation::default());
    assert!(matches!(deck.render_slide(3, 1.0), Err(DeckError::NoSlide(3))));
}

#[test]
fn exports_a_pdf() {
    let deck = DeckDoc::open(&sample_pptx()).unwrap();
    let pdf = deck.export_pdf().unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1000, "{} bytes", pdf.len());
}

#[test]
fn edits_text_and_slides_and_saves_pptx() {
    let mut doc = DeckDoc::open(&sample_pptx()).unwrap();
    let n = doc.slide_count();
    let shapes = doc.text_shapes(0).unwrap();
    let first = shapes.iter().find(|s| !s.text.is_empty()).expect("a slide with text").clone();
    assert!(first.w > 10.0 && first.h > 5.0, "{first:?}");
    let old = first.text.clone();
    let new = format!("{old} — İğdır'da güneşli");
    doc.set_text(0, first.id, &new).unwrap();
    assert!(doc.is_dirty());
    let dup = doc.duplicate_slide(0).unwrap();
    assert_eq!((dup, doc.slide_count()), (1, n + 1));
    let added = doc.new_slide(1).unwrap();
    assert_eq!((added, doc.slide_count()), (2, n + 2));
    doc.move_slide(2, 0).unwrap();
    doc.delete_slide(0).unwrap();
    assert_eq!(doc.slide_count(), n + 1);
    let saved = doc.save("a.pptx").unwrap();
    assert!(!doc.is_dirty());
    // Read back: the new text on the first two slides (original and copy), ids not clashing.
    let back = DeckDoc::open(&saved).unwrap();
    assert_eq!(back.slide_count(), n + 1);
    for i in 0..2 {
        assert!(back.text_shapes(i).unwrap().iter().any(|s| s.text == new), "slide {i}");
    }
    let ids: Vec<u32> = (0..2).flat_map(|i| back.text_shapes(i).unwrap().into_iter().map(|s| s.id)).collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "the copy has its own shape ids");
    let mut one = DeckDoc::open(&saved).unwrap();
    while one.slide_count() > 1 {
        one.delete_slide(0).unwrap();
    }
    assert!(one.delete_slide(0).is_err(), "the last slide stays");
}
