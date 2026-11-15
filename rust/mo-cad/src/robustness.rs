//! Damaged and hostile drawings: the code page and `%%nnn` fixes, opening a drawing and the
//! fallback font reader must return (an error is fine) without a panic, within time and memory
//! limits (see `mo-fuzz`).

use mo_fuzz::{iterations, mutate, run};

use super::*;

const DXF: &[u8] = include_bytes!("../tests/data/apartment.dxf");

const DXF_TOKENS: &[&[u8]] = &[
    b"  9\n$DWGCODEPAGE\n  3\nANSI_1252\n",
    b"  9\n$DWGCODEPAGE\n  3\nDOS857\n",
    b"  9\n$DWGCODEPAGE\n  3\nANSI_1254\n",
    b"  9\n$DWGCODEPAGE\n  3\n\xff\xfe\n",
    b"  1\n%%221stanbul %%253%%254\n",
    b"  3\n%%%%2\n",
    b"%%",
    b"%%2",
    b"%%u",
    b"\xdd\xfd\xde\xfe\xd0\xf0",
    b"\xc4\xb0",
    b"\r\n",
    b"\n",
    b"ENDSEC\n",
    b"  0\nTEXT\n",
];

#[test]
fn damaged_drawing_text_is_prepared() {
    run(
        "dxf-text",
        iterations(1500),
        |rng, _| {
            // Seed text fixes on a header that names a code page, so each path is taken.
            let page: &[u8] = rng.pick(&[b"ANSI_1252".as_slice(), b"DOS857", b"ANSI_1254", b"ANSI_1251", b""]);
            let head = [b"  0\nSECTION\n  2\nHEADER\n  9\n$DWGCODEPAGE\n  3\n".as_slice(), page, b"\n  0\nENDSEC\n"].concat();
            let body = &DXF[..DXF.len().min(8 << 10)];
            mutate(rng, &[&head[..], b"  0\nTEXT\n  1\n%%221 \xdd\xfe\n", body].concat(), DXF_TOKENS)
        },
        |b| drop(text::prepare(b)),
    );
}

#[test]
fn damaged_drawings_are_refused_or_opened() {
    run(
        "dxf-open",
        iterations(600),
        |rng, _| mutate(rng, DXF, DXF_TOKENS),
        |b| {
            if let Ok(mut d) = CadDoc::open("a.dxf", b) {
                let _ = d.layers();
                let _ = d.extents(0);
                let _ = d.render(0, 0.0, 0.0, 1.0, 32, 32, false);
            }
        },
    );
}

#[test]
fn damaged_fonts_are_refused_or_read() {
    let host = ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/dejavu/DejaVuSans.ttf"];
    let Some(font) = host.iter().find_map(|p| std::fs::read(p).ok()) else {
        eprintln!("no DejaVu Sans on this system; skipped");
        return;
    };
    run(
        "ttf",
        iterations(1500),
        move |rng, _| mutate(rng, &font, &[b"glyf", b"cmap", b"\xff\xff\xff\xff", b"\x00\x00"]),
        |b| {
            for face in cadcraft_fonts::ttf::faces(&std::sync::Arc::new(b.to_vec())) {
                let _ = "İıŞşĞğ".chars().filter(|c| face.has_glyph(*c)).count();
            }
        },
    );
}

#[test]
fn pathological_codes_finish_in_time() {
    let codes = [b"  9\n$DWGCODEPAGE\n  3\nANSI_1254\n  0\nENDSEC\n  1\n".as_slice(), &b"%%221%%%%2%".repeat(200_000), b"\n"].concat();
    let t = std::time::Instant::now();
    assert!(text::prepare(&codes).is_some());
    assert!(t.elapsed() < std::time::Duration::from_secs(10), "{:?}", t.elapsed());
}
