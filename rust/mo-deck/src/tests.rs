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
    if let Ok(dir) = std::env::var("MO_INTEROP_DIR") {
        std::fs::write(std::path::Path::new(&dir).join("deck.pptx"), &saved).unwrap();
    }
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
    // Moved by (−36, 18), then doubled about its centre. The rows' own heights double (36 → 72
    // pt each); the text, which kept its size, now fits them, so the table is drawn 144 pt tall.
    let cx = table.x - 36.0 + table.w / 2.0;
    assert!((t.w - table.w * 2.0).abs() < 1.0 && (t.h - 144.0).abs() < 1.0, "{t:?}");
    assert!((t.x + t.w / 2.0 - cx).abs() < 1.0 && (t.y - (table.y + 18.0 - table.h / 2.0)).abs() < 1.0, "{t:?}");
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
    d.style_object(0, box_id, ObjectStyle::Font("Georgia".into())).unwrap();
    // A picture has no text to style.
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    assert!(saved.text_is_bold(0, box_id));
    let shape = saved.pres.slides[0].shapes.iter().find(|s| s.id.0 == box_id).unwrap();
    let run = &shape.text.as_ref().unwrap().paragraphs[0].runs[0].props;
    assert_eq!(run.size, Some(28.0));
    assert_eq!(run.font.as_deref(), Some("Georgia"));
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

#[test]
fn changes_can_be_taken_back_and_made_again() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    assert!(!d.can_undo() && !d.undo());
    let before = d.objects(0).unwrap();
    let id = d.add_shape(0, "star5", 0xFFC000).unwrap();
    d.place_object(0, id, 40.0, 0.0, 1.0, 0.0).unwrap();
    d.new_slide(0).unwrap();
    assert_eq!(d.slide_count(), 2);
    assert!(d.undo());
    assert_eq!(d.slide_count(), 1);
    assert!(d.undo());
    let star = d.objects(0).unwrap().into_iter().find(|o| o.id == id).unwrap();
    assert!(d.undo());
    assert_eq!(d.objects(0).unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo() && d.redo());
    // Back to the star after its move.
    let again = d.objects(0).unwrap().into_iter().find(|o| o.id == id).unwrap();
    assert_eq!((again.x, again.y), (star.x + 40.0, star.y));
    // A new change forgets what was taken back.
    d.delete_object(0, id).unwrap();
    assert!(!d.can_redo());
    // A failed change is not remembered and changes nothing.
    let now = d.objects(0).unwrap();
    assert!(d.delete_object(0, 999_999).is_err());
    assert_eq!(d.objects(0).unwrap(), now);
    assert!(d.undo());
    assert!(d.objects(0).unwrap().iter().any(|o| o.id == id));
}

#[test]
fn a_new_presentation_has_a_title_slide() {
    let mut d = DeckDoc::blank().unwrap();
    assert_eq!(d.slide_count(), 1);
    let texts = d.text_shapes(0).unwrap();
    assert!(!texts.is_empty(), "title placeholders to type into");
    d.set_text(0, texts[0].id, "Yeni sunum").unwrap();
    let again = DeckDoc::open(&d.save("Yeni.pptx").unwrap()).unwrap();
    assert_eq!(again.slide_count(), 1);
    assert!(again.text_shapes(0).unwrap().iter().any(|t| t.text == "Yeni sunum"));
}

#[test]
fn speaker_notes_can_be_written() {
    let mut d = DeckDoc::open(&sample_pptx()).unwrap();
    d.set_notes(1, "Önce teşekkür et.\nSonra grafiği göster.").unwrap();
    let again = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    assert_eq!(again.notes(1).unwrap(), "Önce teşekkür et.\nSonra grafiği göster.");
    assert!(d.undo());
    assert_ne!(d.notes(1).unwrap(), "Önce teşekkür et.\nSonra grafiği göster.");
}

#[test]
fn text_boxes_resize_like_design_apps() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let id = d.add_text_box(0, "Büyüyen yazı").unwrap();
    let size = |d: &DeckDoc| d.pres.slides[0].shapes.iter().find(|s| s.id.0 == id).unwrap().text.as_ref().unwrap().paragraphs[0].runs[0].props.size;
    // From a corner: the letters grow with the box (18 pt by default, doubled).
    d.place_object(0, id, 0.0, 0.0, 2.0, 0.0).unwrap();
    assert_eq!(size(&d), Some(36.0));
    d.place_object(0, id, 0.0, 0.0, 0.5, 0.0).unwrap();
    assert_eq!(size(&d), Some(18.0));
    // From a side: only the box changes; the letters keep their size.
    let o = d.objects(0).unwrap().into_iter().find(|o| o.id == id).unwrap();
    d.set_object_box(0, id, o.x as f64, o.y as f64, o.w as f64 * 0.5, o.h as f64).unwrap();
    let after = d.objects(0).unwrap().into_iter().find(|o| o.id == id).unwrap();
    assert!((after.w - o.w * 0.5).abs() < 0.5 && after.x == o.x, "{after:?}");
    assert_eq!(size(&d), Some(18.0));
    assert!(d.set_object_box(0, id, 0.0, 0.0, 0.0, 10.0).is_err());
    let saved = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    assert!(saved.objects(0).unwrap().iter().any(|o| o.id == id && (o.w - after.w).abs() < 0.5));
}

#[test]
fn opendocument_presentations_open() {
    // Made with Python's zipfile as LibreOffice lays an .odp out: two pages, a title, a list, an
    // ellipse with text, a picture, an embedded object it cannot show, and speaker notes.
    let mut d = DeckDoc::open(include_bytes!("../testdata/sample.odp")).unwrap();
    assert_eq!(d.slide_count(), 2);
    let (w, h) = d.slide_size();
    assert!((w - 793.7).abs() < 0.5 && (h - 446.5).abs() < 0.5, "{w}×{h}");
    let texts: Vec<String> = d.text_shapes(0).unwrap().into_iter().map(|t| t.text).collect();
    assert_eq!(texts, ["Çalışma Raporu 2026", "Birinci madde\nİkinci  madde", "Şekil yazısı"]);
    let title = d.text_shapes(0).unwrap().into_iter().next().unwrap();
    assert!((title.x - 56.7).abs() < 0.5 && (title.y - 28.3).abs() < 0.5, "{title:?}");
    assert_eq!(d.notes(0).unwrap(), "Önce teşekkür et.");
    let pics: Vec<_> = d.objects(1).unwrap().into_iter().filter(|o| o.kind == ObjectKind::Picture).collect();
    assert_eq!(pics.len(), 1);
    assert!(d.warnings().iter().any(|w| w.contains("gösterilemedi")), "{:?}", d.warnings());
    assert!(d.render_slide(0, 0.5).is_ok());
    // Saved as PowerPoint (the format DeckCraft writes), it opens again with its text.
    let again = DeckDoc::open(&d.save("x.pptx").unwrap()).unwrap();
    assert_eq!(again.slide_count(), 2);
    assert!(again.text_shapes(0).unwrap().iter().any(|t| t.text == "Çalışma Raporu 2026"));
}

/// A record of the PowerPoint 97–2003 binary format: header (version/instance, type, length) and body.
fn ppt_record(ver: u16, instance: u16, rec_type: u16, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(ver | instance << 4).to_le_bytes());
    out.extend_from_slice(&rec_type.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn ppt_text(text_type: u32, text: &str, wide: bool) -> Vec<u8> {
    let mut out = ppt_record(0, 0, 0x0F9F, &text_type.to_le_bytes());
    if wide {
        let chars: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        out.extend(ppt_record(0, 0, 0x0FA0, &chars));
    } else {
        // TextBytesAtom: the low byte of each UTF-16 unit, so only U+0000–U+00FF ([MS-PPT] 2.9.42).
        let bytes: Vec<u8> = text.chars().map(|c| u8::try_from(u32::from(c)).expect("Latin-1 only")).collect();
        out.extend(ppt_record(0, 0, 0x0FA8, &bytes));
    }
    out
}

#[test]
fn old_powerpoint_text_atoms_keep_turkish_letters() {
    // PowerPoint writes a run in a TextBytesAtom (one byte a letter, Latin-1) when every letter
    // fits, else in a TextCharsAtom (UTF-16): "Çiçek Ölçüsü" can be narrow, "İğne Şişesi" cannot.
    let persist = ppt_record(0, 0, 0x03F3, &[0u8; 20]);
    let mut slides = persist;
    slides.extend(ppt_text(0, "Çiçek Ölçüsü", false));
    slides.extend(ppt_text(1, "İğne Şişesi ığüşöç", true));
    let stream = ppt_record(0x0F, 0, 0x03E8, &ppt_record(0x0F, 0, 0x0FF0, &slides));
    let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    std::io::Write::write_all(&mut file.create_stream("/PowerPoint Document").unwrap(), &stream).unwrap();
    let d = DeckDoc::open(&file.into_inner().into_inner()).unwrap();
    let texts: Vec<String> = d.text_shapes(0).unwrap().into_iter().map(|t| t.text).collect();
    assert_eq!(texts, ["Çiçek Ölçüsü", "İğne Şişesi ığüşöç"]);
}

#[test]
fn slide_fonts_the_phone_lacks_fall_back_to_a_face_with_turkish_letters() {
    // DeckCraft finds no installed fonts on Android (it looks in desktop folders) and the app is
    // built without the optional craft-fonts faces, so only its compiled-in face is there.
    let db = deckcraft_fonts::FontDb::with_font_dirs(Vec::new());
    for family in ["Calibri", "Calibri Light", "Times New Roman", "Arial", "Aptos", "Tahoma", "Comic Sans MS"] {
        for style in ["Regular", "Bold", "Italic"] {
            let face = db.document_face(family, style);
            for c in "İıŞşĞğÇçÖöÜü".chars() {
                assert_ne!(face.glyph_for(c), 0, "{family} {style} → {} has no {c}", face.family);
            }
        }
    }
}

#[test]
fn old_powerpoint_files_open_with_their_words() {
    // A .ppt built record by record ([MS-PPT]): a 10×7.5 inch deck, two slides and a note.
    let mut doc_atom = Vec::new();
    doc_atom.extend_from_slice(&5760i32.to_le_bytes());
    doc_atom.extend_from_slice(&4320i32.to_le_bytes());
    doc_atom.extend_from_slice(&[0u8; 32]);
    let persist = ppt_record(0, 0, 0x03F3, &[0u8; 20]);
    let mut slides = persist.clone();
    slides.extend(ppt_text(0, "Yıllık Değerlendirme", true));
    slides.extend(ppt_text(1, "Birinci madde\rİkinci madde", true));
    slides.extend(persist.clone());
    slides.extend(ppt_text(6, "Second slide", false));
    let mut notes = persist;
    notes.extend(ppt_text(2, "Süreyi kontrol et", true));
    let mut container = ppt_record(1, 0, 0x03E9, &doc_atom);
    container.extend(ppt_record(0x0F, 0, 0x0FF0, &slides));
    container.extend(ppt_record(0x0F, 2, 0x0FF0, &notes));
    let stream = ppt_record(0x0F, 0, 0x03E8, &container);
    let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    {
        use std::io::Write;
        let mut s = file.create_stream("/PowerPoint Document").unwrap();
        s.write_all(&stream).unwrap();
    }
    let bytes = file.into_inner().into_inner();

    let d = DeckDoc::open(&bytes).unwrap();
    assert_eq!(d.slide_count(), 2);
    assert_eq!(d.slide_size(), (720.0, 540.0));
    let first: Vec<String> = d.text_shapes(0).unwrap().into_iter().map(|t| t.text).collect();
    assert_eq!(first, ["Yıllık Değerlendirme", "Birinci madde\nİkinci madde"]);
    let second: Vec<String> = d.text_shapes(1).unwrap().into_iter().map(|t| t.text).collect();
    assert_eq!(second, ["Second slide"]);
    assert_eq!(d.notes(0).unwrap(), "Süreyi kontrol et");
    assert!(d.warnings().iter().any(|w| w.contains(".ppt")));
    assert!(d.render_slide(0, 0.5).is_ok());
}

#[test]
fn a_narrowed_table_grows_down_only_as_far_as_its_text_needs() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let t = d.objects(0).unwrap().into_iter().find(|o| o.kind == ObjectKind::Table).unwrap();
    d.set_cell_text(0, t.id, 0, 0, "Uzun bir hücre metni, dar sütunda birkaç satıra bölünür").unwrap();
    let before = d.objects(0).unwrap().into_iter().find(|o| o.id == t.id).unwrap();
    // Pulled narrower from a side, with the box as it is drawn.
    let narrow = |d: &mut DeckDoc, o: &SlideObject| d.set_object_box(0, o.id, o.x as f64, o.y as f64, o.w as f64 * 0.5, o.h as f64).unwrap();
    narrow(&mut d, &before);
    let after = d.objects(0).unwrap().into_iter().find(|o| o.id == t.id).unwrap();
    assert!((after.w - before.w * 0.5).abs() < 0.5, "{after:?}");
    // The frame is as tall as the table is drawn: the wrapped text made it taller.
    assert!(after.h >= before.h, "{after:?} vs {before:?}");
    // Asking again with the same width does not make it grow further.
    d.set_object_box(0, after.id, after.x as f64, after.y as f64, after.w as f64, after.h as f64).unwrap();
    let again = d.objects(0).unwrap().into_iter().find(|o| o.id == t.id).unwrap();
    assert!((again.h - after.h).abs() < 0.5, "{again:?} vs {after:?}");
}

#[test]
fn an_object_moves_to_another_slide_where_it_was() {
    let mut d = DeckDoc::open(include_bytes!("../testdata/grouped-text.pptx")).unwrap();
    let to = d.new_slide(0).unwrap();
    let id = d.add_text_box(0, "Taşınan kutu").unwrap();
    let before = d.objects(0).unwrap().into_iter().find(|o| o.id == id).unwrap();
    let new_id = d.move_object_to_slide(0, id, to).unwrap();
    assert!(!d.objects(0).unwrap().iter().any(|o| o.id == id));
    let after = d.objects(to).unwrap().into_iter().find(|o| o.id == new_id).unwrap();
    assert_eq!((after.x, after.y, after.w, after.h), (before.x, before.y, before.w, before.h));
    assert!(d.text_shapes(to).unwrap().iter().any(|s| s.text == "Taşınan kutu"));
    assert!(d.move_object_to_slide(0, new_id, 99).is_err());
    // One step back puts it where it was.
    assert!(d.undo());
    assert!(d.objects(0).unwrap().iter().any(|o| o.id == id));
}

/// The lossless-save corpus (scripts/office_corpus.py): every `*.pptx` in `MO_CORPUS_DIR` is
/// opened, " (düzenlendi)" is added to the first text on its first slide through the bridge's
/// editing and it is saved as `*.saved.pptx`, which the independent reader then compares with the
/// original. Nothing to do without the variable.
#[test]
fn corpus_round_trip() {
    let Ok(dir) = std::env::var("MO_CORPUS_DIR") else { return };
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".pptx").filter(|s| !s.ends_with(".saved")) else { continue };
        let mut doc = DeckDoc::open(&std::fs::read(&path).unwrap()).unwrap();
        let first = doc.text_shapes(0).unwrap().into_iter().find(|s| s.cell.is_none() && !s.text.is_empty()).expect("text on slide 1");
        doc.set_text(0, first.id, &format!("{} (düzenlendi)", first.text)).unwrap();
        std::fs::write(path.with_file_name(format!("{stem}.saved.pptx")), doc.save(&name).unwrap()).unwrap();
        std::fs::write(path.with_file_name(format!("{stem}.pptx.warnings.txt")), doc.warnings().join("\n")).unwrap();
        seen += 1;
    }
    assert!(seen > 0, "no .pptx in {dir}");
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

fn entry(bytes: &[u8], name: &str) -> Vec<u8> {
    use std::io::Read;
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    zin.by_name(name).unwrap().read_to_end(&mut out).unwrap();
    out
}

/// DeckCraft reports only the slides it cannot read: a damaged master, layout or notes page is
/// left out, and a slide cut short is read up to the cut, without a word. Saving over the
/// original would make that loss permanent, so the deck is flagged.
#[test]
fn what_deckcraft_leaves_out_on_opening_is_flagged() {
    let pptx = sample_pptx();
    let whole = DeckDoc::open(&pptx).unwrap();
    assert!(!whole.opened_incomplete(), "{:?}", whole.warnings());

    let master = entry(&pptx, "ppt/slideMasters/slideMaster1.xml");
    let deck = DeckDoc::open(&with_entry(&pptx, "ppt/slideMasters/slideMaster1.xml", &master[..master.len() / 2])).unwrap();
    assert!(deck.warnings().is_empty(), "DeckCraft says nothing: {:?}", deck.warnings());
    assert!(deck.opened_incomplete());

    // Cut after a tag: the elements still open are closed and the slide shows what came before.
    let slide = entry(&pptx, "ppt/slides/slide2.xml");
    let cut = slide[..slide.len() * 2 / 3].iter().rposition(|&b| b == b'>').unwrap() + 1;
    let deck = DeckDoc::open(&with_entry(&pptx, "ppt/slides/slide2.xml", &slide[..cut])).unwrap();
    assert_eq!(deck.slide_count(), whole.slide_count(), "the cut slide is still shown");
    assert!(deck.warnings().is_empty(), "{:?}", deck.warnings());
    assert!(deck.opened_incomplete());

    // A slide that does not parse at all is one DeckCraft reports itself.
    let deck = DeckDoc::open(&with_entry(&pptx, "ppt/slides/slide2.xml", b"<p:sld")).unwrap();
    assert!(!deck.warnings().is_empty());
    assert!(deck.opened_incomplete());

    // Another format read into a PPTX model.
    assert!(DeckDoc::open(include_bytes!("../testdata/sample.odp")).unwrap().opened_incomplete());
    assert!(!DeckDoc::blank().unwrap().opened_incomplete());
}

/// The Turkish letters every slide font must draw.
const TURKISH: &str = "İıŞşĞğÇçÖöÜü";

#[test]
fn slide_fonts_resolve_to_the_app_s_bundled_faces_without_system_fonts() {
    // As on a phone: no font folder DeckCraft knows, no craft-fonts faces.
    let db = deckcraft_fonts::FontDb::with_font_dirs(Vec::new());
    let before = db.document_face("Calibri", "Bold");
    assert_eq!(before.family, deckcraft_fonts::LAST_RESORT_FAMILY, "without the bundled faces everything is Ubuntu Light");
    assert!(crate::fonts::register(&db) >= 10);
    // (family, style asked for) → (family drawn, weight at least, italic face).
    let cases = [
        ("Calibri", "Regular", "Source Sans 3", 400.0, false),
        ("Calibri", "Bold", "Source Sans 3", 700.0, false),
        ("Calibri", "Italic", "Source Sans 3", 400.0, true),
        ("Calibri Light", "Regular", "Source Sans 3", 400.0, false),
        ("Comic Sans MS", "Regular", "Source Sans 3", 400.0, false),
        ("Times New Roman", "Regular", "Source Serif 4", 400.0, false),
        ("Times New Roman", "Bold Italic", "Source Serif 4", 700.0, true),
        ("Cambria", "Bold", "Source Serif 4", 700.0, false),
        // A missing font without a metric twin draws as Calibri, as in PowerPoint (Aptos, an
        // Office cloud font, keeps its own stand-in).
        ("Georgia", "Italic", "Source Sans 3", 400.0, true),
        ("Courier New", "Regular", "JetBrains Mono", 400.0, false),
        ("Arial", "Regular", "Inter", 400.0, false),
        ("Arial", "Bold", "Inter", 600.0, false),
        // Inter has no italic face here: the renderer slants the upright one.
        ("Arial", "Italic", "Inter", 400.0, false),
        ("Aptos", "Regular", "Inter", 400.0, false),
        ("Tahoma", "Regular", "Source Sans 3", 400.0, false),
    ];
    let mut ids = std::collections::HashSet::new();
    for (family, style, drawn, weight, italic) in cases {
        let face = db.document_face(family, style);
        assert_eq!((face.family.as_str(), face.italic), (drawn, italic), "{family} {style} → {} {}", face.family, face.style);
        assert!(face.weight >= weight && face.weight < weight + 150.0, "{family} {style} → {} {} ({})", face.family, face.style, face.weight);
        let glyphs = deckcraft_fonts::shape(&face, TURKISH, &[], |c| c);
        assert_eq!(glyphs.len(), TURKISH.chars().count());
        assert!(glyphs.iter().all(|g| g.gid != 0), "{family} {style} → {} draws .notdef", face.family);
        ids.insert(face.id());
    }
    assert!(ids.len() >= 8, "only {} different faces", ids.len());
    // Registering again adds nothing.
    assert_eq!(crate::fonts::register(&db), 0);
}

#[test]
fn slide_text_lays_out_in_bold_italic_and_serif_faces() {
    use deckcraft_model::text::{Paragraph, Run, RunProps, TextBody};
    let d = DeckDoc::blank().unwrap();
    let props = |font: &str, bold: bool, italic: bool| RunProps {
        font: Some(font.into()),
        bold: Some(bold),
        italic: Some(italic),
        size: Some(24.0),
        ..Default::default()
    };
    let runs = [
        ("Calibri ", props("Calibri", false, false)),
        ("Çağrı Işık ", props("Calibri", true, false)),
        ("Şişli Göğüs ", props("Times New Roman", false, false)),
        ("İstanbul Üsküdar", props("Arial", false, true)),
    ];
    let body = TextBody {
        paragraphs: vec![Paragraph { runs: runs.iter().map(|(t, p)| Run::with(*t, p.clone())).collect(), ..Default::default() }],
        ..Default::default()
    };
    let shape =
        deckcraft_model::Shape { xfrm: Some(deckcraft_model::Xfrm::new(0.0, 0.0, 900.0, 400.0)), text: Some(body.clone()), ..Default::default() };
    let ctx = deckcraft_model::resolve::Ctx::for_slide(&d.pres, &d.pres.slides[0]).unwrap();
    let opts = deckcraft_text::Opts {
        rect: deckcraft_model::Rect::new(0.0, 0.0, 900.0, 400.0),
        fields: &deckcraft_text::NoFields,
        prompt_color: None,
        no_shrink: true,
    };
    let laid = deckcraft_text::layout(&ctx, &shape, &body, &opts);
    let text: String = runs.iter().map(|(t, _)| *t).collect();
    let chars: Vec<char> = text.chars().collect();
    // Where each run starts, in characters of the paragraph.
    let mut starts = Vec::new();
    let mut n = 0;
    for (t, _) in &runs {
        starts.push(n);
        n += t.chars().count();
    }
    let run_of = |ch: usize| starts.iter().rposition(|s| *s <= ch).unwrap();
    let mut faces = std::collections::HashSet::new();
    let mut seen = [false; 4];
    for r in &laid.runs {
        for ((gid, _, _), (ch, c)) in r.glyphs.iter().zip(&r.cells) {
            assert_eq!(chars[*ch], *c);
            if c.is_whitespace() {
                continue;
            }
            assert_ne!(*gid, 0, "{c} drawn as .notdef in {}", r.face.family);
            let i = run_of(*ch);
            seen[i] = true;
            match i {
                1 => assert!(r.face.weight >= 600.0 || r.fake_bold, "bold Calibri in {} {}", r.face.family, r.face.style),
                3 => assert!(r.face.italic || r.fake_italic, "italic Arial in {} {}", r.face.family, r.face.style),
                _ => {}
            }
            faces.insert((r.face.family.clone(), r.face.style.clone()));
        }
    }
    assert_eq!(seen, [true; 4], "every run is laid out");
    assert!(faces.len() >= 3, "{faces:?}");
    assert!(faces.iter().all(|(f, _)| f != deckcraft_fonts::LAST_RESORT_FAMILY), "{faces:?}");
}
