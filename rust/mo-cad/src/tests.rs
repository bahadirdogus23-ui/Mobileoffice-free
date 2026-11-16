use super::*;

fn apartment() -> CadDoc {
    CadDoc::open("apartment.dxf", include_bytes!("../tests/data/apartment.dxf")).unwrap()
}

fn ink(r: &Raster, background: [u8; 3]) -> usize {
    r.pixels.as_chunks::<4>().0.iter().filter(|p| p[..3] != background).count()
}

#[test]
fn dxf_opens_with_spaces_and_layers() {
    let doc = apartment();
    assert_eq!(doc.space_names()[0], "Model");
    // Its paper layouts have nothing on them: they are not offered.
    assert_eq!(doc.space_names(), ["Model"]);
    let layers = doc.layers();
    assert!(layers.len() >= 3, "{layers:?}");
    assert!(layers.iter().any(|l| l.visible));
}

#[test]
fn whole_drawing_renders_on_dark_and_white() {
    let mut doc = apartment();
    let e = doc.extents(0).unwrap();
    let (w, h) = (400u32, 300u32);
    let scale = (w as f64 / (e.max_x - e.min_x)).min(h as f64 / (e.max_y - e.min_y)) * 0.95;
    let (cx, cy) = ((e.min_x + e.max_x) / 2.0, (e.min_y + e.max_y) / 2.0);
    let dark = doc.render(0, cx, cy, scale, w, h, true).unwrap();
    assert_eq!((dark.width, dark.height, dark.pixels.len()), (w, h, (w * h * 4) as usize));
    assert!(ink(&dark, [33, 40, 48]) > 2000, "{}", ink(&dark, [33, 40, 48]));
    let white = doc.render(0, cx, cy, scale, w, h, false).unwrap();
    assert!(ink(&white, [255, 255, 255]) > 2000, "{}", ink(&white, [255, 255, 255]));
}

#[test]
fn hiding_every_layer_leaves_an_empty_sheet() {
    let mut doc = apartment();
    let e = doc.extents(0).unwrap();
    let (cx, cy) = ((e.min_x + e.max_x) / 2.0, (e.min_y + e.max_y) / 2.0);
    let scale = 200.0 / (e.max_x - e.min_x);
    let before = ink(&doc.render(0, cx, cy, scale, 200, 200, false).unwrap(), [255, 255, 255]);
    for l in doc.layers() {
        doc.set_layer_visible(&l.name, false).unwrap();
    }
    let after = ink(&doc.render(0, cx, cy, scale, 200, 200, false).unwrap(), [255, 255, 255]);
    assert!(before > 500 && after < before / 10, "{before} → {after}");
}

#[test]
fn bad_input_is_an_error() {
    assert!(CadDoc::open("x.dwg", b"AC1032 broken").is_err());
    let mut doc = apartment();
    assert!(matches!(doc.render(9, 0.0, 0.0, 1.0, 10, 10, true), Err(CadError::NoSpace(9))));
    assert!(matches!(doc.set_layer_visible("yok", false), Err(CadError::NoLayer(_))));
}

/// A minimal R12 DXF: one line from (0, 0) to (100, 0) and a circle of radius 10 at (50, 40).
const LINE_AND_CIRCLE: &str = "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n30\n0\n11\n100\n21\n0\n31\n0\n0\nCIRCLE\n8\n0\n10\n50\n20\n40\n30\n0\n40\n10\n0\nENDSEC\n0\nEOF\n";

#[test]
fn measuring_snaps_to_ends_middles_and_centres() {
    let doc = CadDoc::open("snap.dxf", LINE_AND_CIRCLE.as_bytes()).unwrap();
    let end = doc.snap(0, 98.5, 1.0, 5.0).unwrap().unwrap();
    assert_eq!((end.x, end.y, end.kind.as_str()), (100.0, 0.0, "Endpoint"));
    let mid = doc.snap(0, 51.0, 0.5, 3.0).unwrap().unwrap();
    assert_eq!((mid.x, mid.y, mid.kind.as_str()), (50.0, 0.0, "Midpoint"));
    let centre = doc.snap(0, 50.5, 40.5, 2.0).unwrap().unwrap();
    assert_eq!((centre.x, centre.y), (50.0, 40.0), "{centre:?}");
    assert!(doc.snap(0, 25.0, 20.0, 2.0).unwrap().is_none());
}

/// A 40×40 grid of 10-unit cells, each with a square outline and a circle of radius 3 on layer
/// "C": 3200 objects, enough for the engine's spatial index.
fn grid() -> CadDoc {
    let mut s = String::from("0\nSECTION\n2\nENTITIES\n");
    for i in 0..40 {
        for j in 0..40 {
            let (x, y) = (f64::from(i) * 10.0, f64::from(j) * 10.0);
            let (x2, y2) = (x + 8.0, y + 8.0);
            s += &format!("0\nLWPOLYLINE\n8\n0\n90\n4\n70\n1\n10\n{x}\n20\n{y}\n10\n{x2}\n20\n{y}\n10\n{x2}\n20\n{y2}\n10\n{x}\n20\n{y2}\n");
            s += &format!("0\nCIRCLE\n8\nC\n10\n{}\n20\n{}\n40\n3\n", x + 4.0, y + 4.0);
        }
    }
    s += "0\nENDSEC\n0\nEOF\n";
    CadDoc::open("grid.dxf", s.as_bytes()).unwrap()
}

#[test]
fn close_up_views_draw_from_a_detail_list_of_the_objects_around_them() {
    let mut doc = grid();
    // 40 pixels per unit: one cell fills 400 pixels, far past what the overview serves.
    let r = doc.render(0, 4.0, 4.0, 40.0, 400, 400, false).unwrap();
    let sl = &doc.lists[&0];
    assert!(40.0 > sl.overview_max_scale);
    let d = sl.detail.as_ref().expect("a detail list");
    // Only the objects around the view are in it, not the whole drawing.
    assert!(d.list.prims.len() * 20 < sl.overview.prims.len(), "{} of {}", d.list.prims.len(), sl.overview.prims.len());
    assert!(ink(&r, [255, 255, 255]) > 1000, "{}", ink(&r, [255, 255, 255]));
    // Panning within the window keeps the list; leaving it builds another.
    let window = d.window;
    doc.render(0, 4.5, 4.0, 40.0, 400, 400, false).unwrap();
    assert_eq!(doc.lists[&0].detail.as_ref().unwrap().window, window);
    doc.render(0, 300.0, 300.0, 40.0, 400, 400, false).unwrap();
    assert_ne!(doc.lists[&0].detail.as_ref().unwrap().window, window);
    // Zooming back out frees it.
    doc.render(0, 200.0, 200.0, 1.0, 400, 400, false).unwrap();
    assert!(doc.lists[&0].detail.is_none());
}

#[test]
fn close_up_circles_are_smooth() {
    let mut doc = grid();
    // A circle of radius 3 at 100 pixels per unit: 300 pixels of radius. Its outline must stay
    // within a pixel of the true circle everywhere, so the ring of inked pixels is thin.
    let r = doc.render(0, 4.0, 4.0, 100.0, 700, 700, false).unwrap();
    let mut worst: f64 = 0.0;
    for (i, p) in r.pixels.as_chunks::<4>().0.iter().enumerate() {
        if p[..3] == [255, 255, 255] {
            continue;
        }
        let (x, y) = ((i % 700) as f64 + 0.5 - 350.0, (i / 700) as f64 + 0.5 - 350.0);
        let d = (x.hypot(y) - 300.0).abs();
        // The cell's square outline is farther out than 400 pixels' diagonal; ignore it.
        if d < 60.0 {
            worst = worst.max(d);
        }
    }
    assert!(worst > 0.0 && worst < 2.0, "{worst}");
}

#[test]
fn hidden_layers_stay_hidden_close_up() {
    let mut doc = grid();
    let before = ink(&doc.render(0, 4.0, 4.0, 40.0, 400, 400, false).unwrap(), [255, 255, 255]);
    doc.set_layer_visible("C", false).unwrap();
    let after = ink(&doc.render(0, 4.0, 4.0, 40.0, 400, 400, false).unwrap(), [255, 255, 255]);
    assert!(after < before * 3 / 4, "{before} → {after}");
}

#[test]
fn a_frame_keeps_what_the_view_shows_and_one_dot_per_pixel() {
    let mut doc = grid();
    let list = doc.lists(0).unwrap().overview.clone();
    let (mut frame, mut dots) = (DisplayList::default(), Vec::new());
    // The whole grid in 40×40 pixels: every object is under a pixel, at most one dot per pixel.
    let whole = View { center: Vec2::new(200.0, 200.0), scale: 0.1, width: 40, height: 40 };
    frame_list(&list, &whole, &mut frame, &mut dots);
    assert!(frame.prims.len() <= 40 * 40 && frame.prims.len() > 200, "{}", frame.prims.len());
    assert!(frame.prims.iter().all(|p| p.kind == Kind::Polyline && p.len == 2));
    // Close up, only the objects on screen remain.
    let close = View { center: Vec2::new(4.0, 4.0), scale: 20.0, width: 200, height: 200 };
    frame_list(&list, &close, &mut frame, &mut dots);
    assert!(!frame.prims.is_empty() && frame.prims.len() <= 8, "{}", frame.prims.len());
    // Off the drawing, nothing.
    let away = View { center: Vec2::new(-1000.0, -1000.0), scale: 20.0, width: 200, height: 200 };
    frame_list(&list, &away, &mut frame, &mut dots);
    assert!(frame.prims.is_empty());
}

#[test]
fn whole_drawing_still_shows_its_objects() {
    let mut doc = grid();
    let r = doc.render(0, 200.0, 200.0, 1.0, 420, 420, true).unwrap();
    assert!(ink(&r, [33, 40, 48]) > 20_000, "{}", ink(&r, [33, 40, 48]));
}

/// An ASCII DXF of `version` declaring `code_page`, with a layer (red), a TEXT and an MTEXT on it
/// whose name and strings are the raw bytes given.
fn coded_dxf(version: &str, code_page: &str, layer: &[u8], text: &[u8], mtext: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    let mut put = |code: i32, v: &[u8]| {
        b.extend_from_slice(format!("{code:>3}\r\n").as_bytes());
        b.extend_from_slice(v);
        b.extend_from_slice(b"\r\n");
    };
    let header = [
        (0, &b"SECTION"[..]),
        (2, b"HEADER"),
        (9, b"$ACADVER"),
        (1, version.as_bytes()),
        (9, b"$DWGCODEPAGE"),
        (3, code_page.as_bytes()),
        (0, b"ENDSEC"),
    ];
    let tables = [
        (0, &b"SECTION"[..]),
        (2, b"TABLES"),
        (0, b"TABLE"),
        (2, b"LAYER"),
        (0, b"LAYER"),
        (2, layer),
        (70, b"0"),
        (62, b"1"),
        (6, b"CONTINUOUS"),
        (0, b"ENDTAB"),
        (0, b"ENDSEC"),
    ];
    let entities = [(0, &b"SECTION"[..]), (2, b"ENTITIES"), (0, b"TEXT"), (8, layer), (10, b"0"), (20, b"0"), (40, b"1"), (1, text)];
    let end = [(0, &b"MTEXT"[..]), (8, layer), (10, b"0"), (20, b"5"), (40, b"1"), (1, mtext), (0, b"ENDSEC"), (0, b"EOF")];
    for (c, v) in header.into_iter().chain(tables).chain(entities).chain(end) {
        put(c, v);
    }
    b
}

/// The drawing's layer names (without "0" and "Defpoints") and its TEXT values and MTEXT
/// contents; asserts that every text is on a layer the drawing defines.
fn strings(doc: &CadDoc) -> (Vec<String>, Vec<String>) {
    let layers: Vec<String> = doc.layers().into_iter().map(|l| l.name).filter(|n| n != "0" && n != "Defpoints").collect();
    let mut texts = Vec::new();
    for e in doc.drawing.model.iter() {
        let s = match &e.kind {
            cadcraft_doc::EntityKind::Text(t) => t.value.clone(),
            cadcraft_doc::EntityKind::MText(t) => t.contents.clone(),
            _ => continue,
        };
        assert!(layers.contains(&e.common.layer), "{s} is on undefined layer {}", e.common.layer);
        texts.push(s);
    }
    (layers, texts)
}

/// The same drawing as DWG files of 2000, 2004 and 2018, written by the engine's DWG writer.
fn as_dwgs(dxf: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
    ["AC1015", "AC1018", "AC1032"].into_iter().map(|v| (v, cadcraft_dwg::dxf_to_dwg_as(dxf, v).unwrap())).collect()
}

/// "İstanbul ışık Şğ" in Windows-1254.
const ISTANBUL_1254: &[u8] = b"\xDDstanbul \xFD\xFE\xFDk \xDE\xF0";

/// Drawings in the Turkish code page (R12 and R2000 DXF, and the DWGs made from them) read
/// right, `\U+` escapes too (with lower-case hex digits).
#[test]
fn turkish_code_page_drawings_read_right() {
    for version in ["AC1009", "AC1015"] {
        let dxf = coded_dxf(version, "ANSI_1254", b"KAPI\xDD", ISTANBUL_1254, b"\\U+0130\\U+0131 \\U+015f");
        let expected = (vec!["KAPIİ".to_string()], vec!["İstanbul ışık Şğ".to_string(), "İı ş".to_string()]);
        assert_eq!(strings(&CadDoc::open("a.dxf", &dxf).unwrap()), expected, "{version}");
        for (v, dwg) in as_dwgs(&dxf) {
            // The DWG's code page (file header byte 0x13) is ANSI_1254, AutoCAD's index 33.
            assert_eq!(dwg[0x13], 33);
            assert_eq!(strings(&CadDoc::open("a.dwg", &dwg).unwrap()), expected, "{version} → {v}");
        }
    }
}

/// Turkish text in a drawing that says it is ANSI_1252 read as Ý ý Þ þ Ð ð; now it reads as
/// İ ı Ş ş Ğ ğ, in DXF and DWG, and its objects still find their (renamed the same way) layer.
#[test]
fn turkish_text_in_a_western_code_page_reads_as_turkish() {
    let dxf = coded_dxf("AC1015", "ANSI_1252", b"KAPI\xDD", ISTANBUL_1254, b"\xE7\xF6\xFC \xFD");
    let raw = cadcraft_io::read_dxf(&dxf).unwrap();
    assert!(raw.layers.iter().any(|l| l.name == "KAPIÝ"), "the engine alone reads it as 1252");
    let expected = (vec!["KAPIİ".to_string()], vec!["İstanbul ışık Şğ".to_string(), "çöü ı".to_string()]);
    assert_eq!(strings(&CadDoc::open("a.dxf", &dxf).unwrap()), expected);
    for (v, dwg) in as_dwgs(&dxf) {
        assert_eq!(dwg[0x13], 30, "ANSI_1252");
        let doc = CadDoc::open("a.dwg", &dwg).unwrap();
        assert_eq!(strings(&doc), expected, "{v}");
        assert_eq!(doc.layers().iter().find(|l| l.name == "KAPIİ").unwrap().color, (255, 0, 0), "{v}");
    }
    // A UTF-8 file (2007 and later) with those letters baked in by an earlier conversion.
    let utf8 = coded_dxf("AC1032", "ANSI_1252", "KAPIÝ".as_bytes(), "Ýstanbul ýþýk Þð".as_bytes(), "çöü ý".as_bytes());
    assert_eq!(strings(&CadDoc::open("a.dxf", &utf8).unwrap()), expected);
}

/// Icelandic text has Ý ý Þ þ Ð ð too, along with á é í ó ú æ: it is left as it is.
#[test]
fn icelandic_text_is_left_alone() {
    let text = b"\xDEetta er \xFEa\xF0 sem \xE9g vil \xE1 \xCDslandi, \xE9g f\xF3r \xED b\xEDl \xFDmsum";
    let dxf = coded_dxf("AC1015", "ANSI_1252", b"Hl\xF6\xF0", text, b"x");
    assert!(text::prepare(&dxf).is_none());
    let (layers, texts) = strings(&CadDoc::open("a.dxf", &dxf).unwrap());
    assert_eq!(layers, ["Hlöð"]);
    assert_eq!(texts[0], "Þetta er það sem ég vil á Íslandi, ég fór í bíl ýmsum");
}

/// `%%nnn` is character nnn of the drawing's code page: in a Turkish drawing %%221 is İ, not Ý.
#[test]
fn character_codes_follow_the_turkish_code_page() {
    let dxf = coded_dxf("AC1015", "ANSI_1254", b"%%221", b"%%221stanbul %%253 %%%221 %%d %%199", b"%%254");
    let (layers, texts) = strings(&CadDoc::open("a.dxf", &dxf).unwrap());
    // Only text values change: a layer name is a name, not text with control codes.
    assert_eq!(layers, ["%%221"]);
    assert_eq!(texts, ["İstanbul ı %%%221 %%d %%199", "ş"]);
    // A Western drawing whose only Turkish letters are such codes is a Turkish one.
    let dxf = coded_dxf("AC1015", "ANSI_1252", b"L", b"KAPI %%221", b"x");
    assert_eq!(strings(&CadDoc::open("a.dxf", &dxf).unwrap()).1[0], "KAPI İ");
    // Other code pages keep the engine's reading.
    let dxf = coded_dxf("AC1015", "ANSI_1251", b"L", b"%%221", b"x");
    assert!(text::prepare(&dxf).is_none());
    // A Turkish UTF-8 drawing with nothing to change is passed on as it is.
    let dxf = coded_dxf("AC1032", "ANSI_1254", b"KAPI", "İstanbul ışık".as_bytes(), b"x");
    assert!(text::prepare(&dxf).is_none());
    assert_eq!(strings(&CadDoc::open("a.dxf", &dxf).unwrap()).1[0], "İstanbul ışık");
}

/// DOS857 (Turkish DOS) is a code page the engine's DXF reader doesn't know: its DXF files read
/// as Windows-1252, and so did its DWGs (the DWG reader decodes them, then hands them on as
/// DOS857 DXF). Both read right now.
#[test]
fn turkish_dos_code_page_reads_right() {
    let dxf = coded_dxf("AC1015", "DOS857", b"KAPI\x98", b"\x98stanbul \x8D\x9F\x8Dk \x9E\xA7 \x80\x87\x94\x81", b"x");
    let raw = cadcraft_io::read_dxf(&dxf).unwrap();
    assert!(!raw.layers.iter().any(|l| l.name == "KAPIİ"), "the engine alone misreads it");
    let expected = (vec!["KAPIİ".to_string()], vec!["İstanbul ışık Şğ Ççöü".to_string(), "x".to_string()]);
    assert_eq!(strings(&CadDoc::open("a.dxf", &dxf).unwrap()), expected);
    for (v, dwg) in as_dwgs(&dxf) {
        assert_eq!(strings(&CadDoc::open("a.dwg", &dwg).unwrap()), expected, "{v}");
    }
}

/// The Turkish sample line of the stroke-letter tests.
const TURKISH_SAMPLE: &str = "İSTANBUL Şişli Çağrı Ölçü Ünvanı ığüşöçİĞÜŞÖÇ";

/// A UTF-8 (2007) DXF of TEXT entities, `(text, x, y, height, rotation°, width factor,
/// oblique°, horizontal alignment)`, and an MTEXT at (0, -40), all in the default (SHX) style.
type Text<'a> = (&'a str, f64, f64, f64, f64, f64, f64, i32);
fn text_dxf(texts: &[Text], mtext: &str) -> Vec<u8> {
    let mut s = String::from("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1021\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n");
    for &(t, x, y, h, rot, wf, obl, align) in texts {
        s += &format!(
            "0\nTEXT\n8\n0\n10\n{x}\n20\n{y}\n30\n0\n40\n{h}\n1\n{t}\n50\n{rot}\n41\n{wf}\n51\n{obl}\n72\n{align}\n11\n{x}\n21\n{y}\n31\n0\n"
        );
    }
    if !mtext.is_empty() {
        s += &format!("0\nMTEXT\n8\n0\n10\n0\n20\n-40\n30\n0\n40\n4\n41\n200\n1\n{mtext}\n");
    }
    s += "0\nENDSEC\n0\nEOF\n";
    s.into_bytes()
}

/// The sample drawing at several heights, a rotation, a width factor with an oblique angle, a
/// centred alignment and an MTEXT.
fn turkish_sample() -> CadDoc {
    let t = TURKISH_SAMPLE;
    let dxf = text_dxf(
        &[
            (t, 0.0, 0.0, 2.5, 0.0, 1.0, 0.0, 0),
            (t, 0.0, 6.0, 5.0, 0.0, 1.0, 0.0, 0),
            (t, 0.0, 16.0, 8.0, 0.0, 1.0, 0.0, 0),
            ("ASCII: Istanbul Sisli Cagri Olcu", 0.0, -10.0, 4.0, 0.0, 1.0, 0.0, 0),
            ("Döndürülmüş ŞİĞ", 150.0, -30.0, 4.0, 30.0, 1.0, 0.0, 0),
            ("Dar İĞNE çöp", 0.0, -20.0, 4.0, 0.0, 0.7, 15.0, 0),
            ("Ortalı ÇİĞ ışık", 90.0, -20.0, 4.0, 0.0, 1.0, 0.0, 1),
        ],
        "MTEXT: Kâğıt İŞÇİ ölçüsü\\Pikinci satır ğüşıöç",
    );
    CadDoc::open("tr.dxf", &dxf).unwrap()
}

/// Writes premultiplied RGBA views of the sample to `$MO_CAD_PNG_DIR`, to look at by eye
/// (nothing without it).
#[test]
fn turkish_sample_views() {
    let Some(dir) = std::env::var_os("MO_CAD_PNG_DIR") else { return };
    let mut doc = turkish_sample();
    let views = [
        ("whole", 95.0, -6.0, 6.5, 1400u32, 600u32),
        ("close", 45.0, 19.0, 22.0, 1400, 400),
        ("close2", 135.0, 19.0, 16.0, 1400, 400),
        ("turned", 165.0, -22.0, 20.0, 1000, 500),
        ("small", 40.0, 1.0, 12.0, 1000, 240),
    ];
    for (name, cx, cy, scale, w, h) in views {
        let r = doc.render(0, cx, cy, scale, w, h, false).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}-{w}x{h}.rgba")), &r.pixels).unwrap();
    }
}

/// The display list of model space and the frame drawn from it at `scale`, centred on `c`.
fn text_frame(doc: &mut CadDoc, c: Vec2, scale: f64) -> (DisplayList, DisplayList) {
    let list = doc.lists(0).unwrap().overview.clone();
    let (mut frame, mut dots) = (DisplayList::default(), Vec::new());
    frame_list(&list, &View { center: c, scale, width: 400, height: 400 }, &mut frame, &mut dots);
    (list, frame)
}

/// "Ş" is drawn like the drafting font's letters: the strokes of its "S" plus a cedilla below
/// the middle of it, as hairlines (no filled outline left), at every zoom.
#[test]
fn turkish_letters_are_drawn_in_strokes() {
    let one = |t: &str| CadDoc::open("a.dxf", &text_dxf(&[(t, 0.0, 0.0, 6.0, 0.0, 1.0, 0.0, 0)], "")).unwrap();
    let mut s = one("S");
    let mut s_cedilla = one("Ş");
    for scale in [8.0, 40.0, 150.0] {
        let (list, frame) = text_frame(&mut s_cedilla, Vec2::new(2.0, 2.0), scale);
        assert!(list.prims.iter().any(|p| p.kind == Kind::Tris), "the engine fills the fallback glyph");
        assert!(frame.prims.iter().all(|p| p.kind == Kind::Polyline && p.lw == 0.0), "{scale}");
        let segs: Vec<(Vec2, Vec2)> = frame.prims.iter().map(|p| (frame.verts[p.start as usize], frame.verts[p.start as usize + 1])).collect();
        let near = |q: Vec2| {
            segs.iter().any(|&(a, b)| {
                let t = ((q - a).dot(b - a) / (b - a).len2().max(1e-12)).clamp(0.0, 1.0);
                (a + (b - a) * t - q).len() < 0.02
            })
        };
        // Every point of the drafting font's "S" lies on the strokes of "Ş".
        let (_, plain) = text_frame(&mut s, Vec2::new(2.0, 2.0), scale);
        assert!(plain.prims.iter().all(|p| p.kind == Kind::Polyline));
        assert!(plain.prims.iter().flat_map(|p| plain.points(p)).all(|&q| near(q)), "{scale}");
        // And the cedilla hangs below the baseline from the middle of the letter.
        assert!(near(Vec2::new(2.0, -0.3)) && segs.iter().any(|(a, b)| a.y.min(b.y) < -1.2), "{scale}");
        assert!(segs.iter().all(|(a, b)| a.y.min(b.y) > -1.6 && a.x.min(b.x) > -0.1 && a.x.max(b.x) < 4.1));
    }
}

/// Text the drafting font has is drawn as before, and an ordinary fill stays filled.
#[test]
fn ascii_text_and_fills_are_left_alone() {
    let mut doc = CadDoc::open("a.dxf", &text_dxf(&[("ASCII: Istanbul Sisli 0123", 0.0, 0.0, 6.0, 0.0, 1.0, 0.0, 0)], "")).unwrap();
    let (list, frame) = text_frame(&mut doc, Vec2::new(40.0, 3.0), 4.0);
    // All of it is the drafting font's strokes (no glyph from a fallback font).
    assert!(list.prims.iter().all(|p| p.kind == Kind::Polyline));
    assert!(frame.prims.iter().all(|p| p.kind == Kind::Polyline) && frame.prims.len() > 20);
    let (o, a, thin) = (Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), Vec2::new(0.0, 0.05));
    assert!(!hairline_tris(&[o, a, Vec2::new(0.0, 10.0)], 0.1), "a solid triangle");
    assert!(!hairline_tris(&[o, Vec2::new(1.0, 0.0), thin], 0.1), "a band thick for its length");
    assert!(hairline_tris(&[o, a, thin], 0.1), "a long sub-pixel band");
    assert!(!hairline_tris(&[o, a, thin], 0.01), "the same band close up, thicker than a pixel");
}
