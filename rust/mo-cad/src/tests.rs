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
