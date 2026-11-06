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

#[test]
fn text_inside_groups_can_be_edited() {
    // Made with python-pptx: a title, a two-line body and a text box inside a group.
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let shapes: Vec<_> = d.text_shapes(0).unwrap().into_iter().filter(|s| s.cell.is_none()).collect();
    let texts: Vec<&str> = shapes.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, ["Başlık", "Birinci madde\nİkinci madde", "Gruptaki yazı"]);
    // The grouped box is where it shows: 1 inch from the left, 6 down (72, 432 points).
    let g = shapes.iter().find(|s| s.text == "Gruptaki yazı").unwrap();
    assert_eq!((g.x, g.y, g.w, g.h), (72.0, 432.0, 216.0, 36.0));
    d.set_text(0, g.id, "Değişti").unwrap();
    d.set_text(0, shapes[1].id, "Yeni içerik").unwrap();
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    let texts: Vec<String> = saved.text_shapes(0).unwrap().into_iter().filter(|s| s.cell.is_none()).map(|s| s.text).collect();
    assert_eq!(texts, ["Başlık", "Yeni içerik", "Değişti"]);
}

#[test]
fn table_cells_on_slides_can_be_edited() {
    // The same python-pptx slide has a 2×2 table at (5, 6) inches, 3 × 1 inches, "Tablo hücresi" in A1.
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let cells: Vec<_> = d.text_shapes(0).unwrap().into_iter().filter(|s| s.cell.is_some()).collect();
    assert_eq!(cells.len(), 4);
    let a1 = &cells[0];
    assert_eq!((a1.cell, a1.text.as_str()), (Some((0, 0)), "Tablo hücresi"));
    assert_eq!((a1.x, a1.y), (360.0, 432.0));
    assert!((a1.w - 108.0).abs() < 0.5 && (a1.h - 36.0).abs() < 0.5, "{} {}", a1.w, a1.h);
    let b2 = cells.iter().find(|c| c.cell == Some((1, 1))).unwrap();
    assert!((b2.x - 468.0).abs() < 0.5 && (b2.y - 468.0).abs() < 0.5, "{} {}", b2.x, b2.y);
    d.set_cell_text(0, b2.id, 1, 1, "ARGUVAN").unwrap();
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    let again: Vec<String> = saved.text_shapes(0).unwrap().into_iter().filter(|s| s.cell == Some((1, 1))).map(|s| s.text).collect();
    assert_eq!(again, ["ARGUVAN"]);
}

#[test]
fn objects_can_be_moved_scaled_and_deleted() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let objs = d.objects(0).unwrap();
    // Title, body, the group and the table.
    assert_eq!(objs.len(), 4, "{objs:?}");
    let table = objs.iter().find(|o| o.kind == ObjectKind::Table).unwrap().clone();
    assert_eq!((table.x, table.y), (360.0, 432.0));
    d.place_object(0, table.id, -36.0, 18.0, 1.0, 0.0).unwrap();
    d.place_object(0, table.id, 0.0, 0.0, 2.0, 0.0).unwrap();
    let title = objs.iter().find(|o| o.kind == ObjectKind::Text).unwrap().clone();
    d.delete_object(0, title.id).unwrap();
    assert!(d.delete_object(0, title.id).is_err());
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    let after = saved.objects(0).unwrap();
    assert_eq!(after.len(), 3);
    let t = after.iter().find(|o| o.id == table.id).unwrap();
    // Moved by (−36, 18), then doubled about its centre.
    let (cx, cy) = (table.x - 36.0 + table.w / 2.0, table.y + 18.0 + table.h / 2.0);
    assert!((t.w - table.w * 2.0).abs() < 1.0 && (t.h - table.h * 2.0).abs() < 1.0, "{t:?}");
    assert!((t.x + t.w / 2.0 - cx).abs() < 1.0 && (t.y + t.h / 2.0 - cy).abs() < 1.0, "{t:?}");
    assert!(!saved.text_shapes(0).unwrap().iter().any(|s| s.text == "Başlık"));
    // The cells grew with the table.
    let a1 = saved.text_shapes(0).unwrap().into_iter().find(|s| s.cell == Some((0, 0))).unwrap();
    assert!((a1.w - 216.0).abs() < 1.0 && (a1.h - 72.0).abs() < 1.0, "{a1:?}");
}

#[test]
fn text_boxes_and_pictures_can_be_added() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let before = d.objects(0).unwrap().len();
    let text = d.add_text_box(0, "Yeni kutu: ğüşıöç").unwrap();
    // A 4×2 red PNG.
    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, 4, 2);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header().unwrap().write_image_data(&[255, 0, 0, 255].repeat(8)).unwrap();
    }
    let pic = d.add_picture(0, png.clone(), 4, 2).unwrap();
    assert!(d.add_picture(0, b"not a picture".to_vec(), 4, 2).is_err());
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    let objs = saved.objects(0).unwrap();
    assert_eq!(objs.len(), before + 2, "{objs:?}");
    let t = objs
        .iter()
        .find(|o| o.kind == ObjectKind::Text && saved.text_shapes(0).unwrap().iter().any(|s| s.id == o.id && s.text.starts_with("Yeni kutu")))
        .unwrap();
    let p = objs.iter().find(|o| o.kind == ObjectKind::Picture).unwrap();
    let (sw, sh) = saved.slide_size();
    // The picture keeps its 2:1 shape and sits in the middle.
    assert!((p.w / p.h - 2.0).abs() < 0.01, "{p:?}");
    assert!((p.x + p.w / 2.0 - sw / 2.0).abs() < 1.0 && (p.y + p.h / 2.0 - sh / 2.0).abs() < 1.0, "{p:?}");
    assert!(t.w > 0.0);
    let _ = (text, pic);
    // And it is drawn: red in the middle of the slide.
    let r = saved.render_slide(0, 0.5).unwrap();
    let (cx, cy) = (r.width / 2, r.height / 2);
    let px = &r.pixels[((cy * r.width + cx) * 4) as usize..][..4];
    assert!(px[0] > 200 && px[1] < 60, "{px:?}");
}

#[test]
fn objects_turn_and_can_be_drawn_left_out() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, 4, 2);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header().unwrap().write_image_data(&[255, 0, 0, 255].repeat(8)).unwrap();
    }
    let pic = d.add_picture(0, png, 4, 2).unwrap();
    d.place_object(0, pic, 0.0, 0.0, 1.0, 30.0).unwrap();
    d.place_object(0, pic, 0.0, 0.0, 1.0, -45.0).unwrap();
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    let o = saved.objects(0).unwrap().into_iter().find(|o| o.kind == ObjectKind::Picture).unwrap();
    assert!((o.rot - 345.0).abs() < 0.01, "{o:?}");
    // Left out, the red picture is gone from the middle of the slide.
    let red = |r: &Raster| {
        let (cx, cy) = (r.width / 2, r.height / 2);
        let px = &r.pixels[((cy * r.width + cx) * 4) as usize..][..4];
        px[0] > 200 && px[1] < 60
    };
    assert!(red(&saved.render_slide(0, 0.5).unwrap()));
    assert!(!red(&saved.render_slide_without(0, o.id, 0.5).unwrap()));
    assert!(red(&saved.render_slide(0, 0.5).unwrap()), "the deck itself is untouched");
}

#[test]
fn objects_are_copied_arranged_and_restyled() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let box_id = d.add_text_box(0, "Merhaba").unwrap();
    let copy = d.duplicate_object(0, box_id).unwrap();
    let objs = d.objects(0).unwrap();
    let (a, b) = (objs.iter().find(|o| o.id == box_id).unwrap(), objs.iter().find(|o| o.id == copy).unwrap());
    assert_eq!((b.x - a.x, b.y - a.y), (18.0, 18.0));
    assert_eq!(objs.last().unwrap().id, copy);
    d.arrange_object(0, copy, false).unwrap();
    assert_eq!(d.objects(0).unwrap()[0].id, copy);
    assert!(!d.text_is_bold(0, box_id));
    d.style_object(0, box_id, ObjectStyle::Bold(true)).unwrap();
    d.style_object(0, box_id, ObjectStyle::TextSize(10.0)).unwrap();
    d.style_object(0, box_id, ObjectStyle::TextColor(0xC00000)).unwrap();
    d.style_object(0, box_id, ObjectStyle::Align(1)).unwrap();
    d.style_object(0, box_id, ObjectStyle::Fill(Some(0xFFFF00))).unwrap();
    // A picture has no text to style.
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    assert!(saved.text_is_bold(0, box_id));
    let shape = saved.pres.slides[0].shapes.iter().find(|s| s.id.0 == box_id).unwrap();
    let run = &shape.text.as_ref().unwrap().paragraphs[0].runs[0].props;
    assert_eq!(run.size, Some(28.0));
    assert!(matches!(shape.fill, Some(deckcraft_model::Fill::Solid { .. })));
    assert_eq!(shape.text.as_ref().unwrap().paragraphs[0].props.align, Some(deckcraft_model::text::Align::Center));
    // The copy is untouched.
    assert!(!saved.text_is_bold(0, copy));
}

#[test]
fn shapes_can_be_added_and_typed_into() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let id = d.add_shape(0, "ellipse", 0x0070C0).unwrap();
    assert!(d.add_shape(0, "spaceship", 0).is_err());
    d.set_text(0, id, "İçinde yazı").unwrap();
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    let o = saved.objects(0).unwrap().into_iter().find(|o| o.id == id).unwrap();
    assert_eq!(o.kind, ObjectKind::Text);
    assert!(saved.text_shapes(0).unwrap().iter().any(|t| t.id == id && t.text == "İçinde yazı"));
    // Blue in the middle of the slide (a little above its text).
    let r = saved.render_slide(0, 0.5).unwrap();
    let y = r.height / 2 - r.height / 10;
    let px = &r.pixels[((y * r.width + r.width / 2) * 4) as usize..][..4];
    assert!(px[2] > 150 && px[0] < 60, "{px:?}");
}
