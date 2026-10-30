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
