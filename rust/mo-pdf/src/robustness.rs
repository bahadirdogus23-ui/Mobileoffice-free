//! Damaged and hostile PDFs, and hostile input to our code around the engine (open notes, typed
//! texts, recognised words, personal-data finders): each must return (an error is fine) without
//! a panic, within time and memory limits (see `mo-fuzz`).

use mo_fuzz::{iterations, mutate, run};

use super::*;

const PDF_TOKENS: &[&[u8]] = &[
    b"obj",
    b"endobj",
    b"stream\n",
    b"endstream",
    b"xref\n",
    b"trailer",
    b"startxref\n",
    b"/Parent 3 0 R",
    b"/Kids [3 0 R 3 0 R]",
    b"/Count 999999999",
    b"/Annots [5 0 R]",
    b"/Rotate 45",
    b"/MediaBox [0 0 1e308 -1]",
    b"/Length 99999999",
    b"/Filter /FlateDecode",
    b"<<",
    b">>",
    b"[",
    b"]",
    b"(",
    b")",
    b"\\",
    b"%%EOF",
    b" 0 R",
];

fn seeds() -> Vec<Vec<u8>> {
    let mut typed = PdfDoc::open(tests::pdf(2, "Sayfa"), None).unwrap();
    typed.add_text(0, [0.1, 0.1], "İş yeri\nŞişli", 12.0, TextFont::Sans, [0.0, 0.0, 0.0]).unwrap();
    typed.note(1, [0.5, 0.5], "Not").unwrap();
    vec![
        tests::pdf(2, "Şule 10000000146 TR330006100519786457841326"),
        typed.save().unwrap(),
        include_bytes!("../testdata/pdflib-form.pdf").to_vec(),
        include_bytes!("../testdata/jspdf-form.pdf").to_vec(),
    ]
}

/// Recognised words decoded from fuzz bytes: odd text, NaN, infinite and inverted boxes.
fn words_from(b: &[u8]) -> Vec<(String, [f64; 4])> {
    b.chunks(12)
        .take(64)
        .map(|c| {
            let f = |i: usize| match c.get(i).copied().unwrap_or(0) % 8 {
                0 => f64::NAN,
                1 => f64::INFINITY,
                2 => -1e300,
                3 => 0.0,
                4 => 1.0,
                5 => 0.5,
                6 => -0.25,
                _ => f64::from(c.get(i + 1).copied().unwrap_or(0)) / 255.0,
            };
            (String::from_utf8_lossy(&c[c.len().min(5)..]).into_owned(), [f(0), f(1), f(2), f(3)])
        })
        .collect()
}

#[test]
fn damaged_pdfs_are_refused_or_read() {
    let seeds = seeds();
    run(
        "pdf",
        iterations(200),
        move |rng, _| {
            let i = rng.below(seeds.len());
            mutate(rng, &seeds[i], PDF_TOKENS)
        },
        |b| {
            let Ok(mut d) = PdfDoc::open(b.to_vec(), None) else { return };
            let _ = d.open_notes().len();
            let _ = d.typed_texts(0);
            let _ = d.fields();
            let _ = d.page_text(0);
            let _ = d.find_text("şule", 10);
            let _ = d.mark_redactions(&[Sensitive::TurkishId, Sensitive::Iban, Sensitive::Phone, Sensitive::Email], &["İş".into()]);
            let _ = d.add_ocr_words(0, &words_from(&b[..b.len().min(256)]));
            let _ = d.save();
        },
    );
}

#[test]
fn hostile_words_and_text_are_handled() {
    let base = tests::pdf(1, "Sayfa");
    run(
        "ocr-words",
        iterations(150),
        |rng, _| mutate(rng, b"Istanbul\x00\x01\x02\x03\x04\x05\x06\x07(\\)\xc4\xb0\xff\xfe", &[]),
        move |b| {
            let mut d = PdfDoc::open(base.clone(), None).unwrap();
            let _ = d.add_ocr_words(0, &words_from(b));
            let _ = d.add_text(0, [f64::NAN, 2.0], &String::from_utf8_lossy(b), f64::INFINITY, TextFont::Serif, [9.0, -1.0, f64::NAN]);
            let _ = d.typed_texts(0);
            let _ = d.save();
        },
    );
    run(
        "pdf-text",
        iterations(4000),
        |rng, _| mutate(rng, "Tel: 0532 123 45 67, +90 (212) 555-12-34 TR33 0006 1005 1978 6457 8413 26 10000000146 a@b.co İSTANBUL".as_bytes(), &[]),
        |b| {
            let chars: Vec<char> = String::from_utf8_lossy(b).chars().collect();
            for kind in [Sensitive::TurkishId, Sensitive::Iban, Sensitive::Phone, Sensitive::Email] {
                for r in text::find_sensitive(kind, &chars) {
                    assert!(r.start <= r.end && r.end <= chars.len());
                }
            }
            let q: String = chars.iter().take(4).collect();
            let _ = text::find_folded(&chars, &q);
        },
    );
}

/// A page whose /Parent leads back to itself and that has no resources: adding recognised words
/// walks up the tree for inherited resources (bounded since; PdfCraft's stamp gives the page
/// resources first, so this file does not reach the walk's limit, but the walk no longer relies
/// on that).
#[test]
fn a_page_tree_loop_ends() {
    let body = [
        "1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj",
        "2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj",
        "3 0 obj << /Type /Page /Parent 3 0 R /MediaBox [0 0 200 200] >> endobj",
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for o in body {
        offsets.push(pdf.len());
        pdf.extend_from_slice(o.as_bytes());
        pdf.push(b'\n');
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
    for o in offsets {
        pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer << /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    run(
        "pdf-parent-loop",
        1,
        move |_, _| pdf.clone(),
        |b| {
            let mut d = PdfDoc::open(b.to_vec(), None).unwrap();
            assert_eq!(d.add_ocr_words(0, &[("Merhaba".into(), [0.1, 0.1, 0.5, 0.2])]).unwrap(), 1);
        },
    );
}
