//! Damaged and hostile UYAP UDF files, and the loss checks of the formats saved over the opened
//! file: each must return (an error is fine) without a panic, within time and memory limits
//! (see `mo-fuzz`).

use mo_fuzz::{Entries, Rng, TEXT_TOKENS, XML_TOKENS, iterations, mutate, mutate_zip, run};

use super::*;
use crate::tests::{sample_docx, sample_udf};

fn entries_of(zip: &[u8]) -> Entries {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(zip)).unwrap();
    (0..z.len())
        .map(|i| {
            let mut f = z.by_index(i).unwrap();
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut f, &mut data).unwrap();
            (f.name().to_string(), data)
        })
        .collect()
}

const UDF_TOKENS: &[&[u8]] = &[
    b"<paragraph>",
    b"</paragraph>",
    b"<table columnSpans=\"1,0,-5\">",
    b"<row><cell colspan=\"1e30\">",
    b"</cell></row></table>",
    b"<content startOffset=\"0\" length=\"99999999\"/>",
    b"<content startOffset=\"-1\" length=\"NaN\"/>",
    b"<image imageData=\"iVBORw0KGgo=\" width=\"1e30\" height=\"-1\"/>",
    b"<page-break/>",
    b" size=\"1e38\"",
    b" LineSpacing=\"inf\"",
    b" foreground=\"-99999999999999999999\"",
    b"<![CDATA[",
    b"]]>",
    b"encoding=\"windows-1254\"",
    b"encoding=\"ibm857\"",
    b"\xdd\xfe",
];

fn word_tokens() -> Vec<&'static [u8]> {
    [XML_TOKENS, UDF_TOKENS].concat()
}

#[test]
fn the_seeds_are_read() {
    assert!(WordDoc::open("a.udf", &mo_fuzz::stored_zip(&entries_of(&sample_udf(true))), None).is_ok_and(|d| d.is_signed()));
    let docx = mo_fuzz::stored_zip(&entries_of(&sample_docx()));
    assert!(WordDoc::open("a.docx", &docx, None).is_ok_and(|d| d.losses().is_empty()));
}

#[test]
fn damaged_udf_files_are_refused_or_read() {
    let entries = entries_of(&sample_udf(true));
    let dict = word_tokens();
    run("udf", iterations(1500), move |rng, _| mutate_zip(rng, &entries, &dict), |b| drop(WordDoc::open("a.udf", b, None)));
}

/// The seeds of the formats `losses::find` checks: (file name, bytes or zip entries).
enum Seed {
    Zip(&'static str, Entries),
    Bytes(&'static str, Vec<u8>),
}

fn loss_seeds() -> Vec<Seed> {
    let docx = sample_docx();
    let odt = WordDoc::open("a.docx", &docx, None).unwrap().export("a.odt").unwrap();
    vec![
        Seed::Zip("a.docx", entries_of(&docx)),
        Seed::Zip("a.odt", entries_of(&odt)),
        Seed::Bytes(
            "a.rtf",
            br"{\rtf1\ansi{\fonttbl{\f0 Arial;}}{\*\generator Word;}Merhaba \b d\'fcnya\b0{\footnote\pard Dipnot}{\*\shppict{\pict\pngblip\bin4 \x}}{\object{\*\objdata 01}}\par}".to_vec(),
        ),
        Seed::Bytes("a.html", "<html><head><style>p{}</style></head><body><h1>Başlık</h1><p>Metin <img src=\"data:image/png;base64,AA\"><img src='a.png'></p><script>x</script></body></html>".as_bytes().to_vec()),
        Seed::Bytes("a.md", "---\ntitle: x\n---\n# Başlık\n\n![logo](logo.png)\n```\n<script>\n```\n<div>x</div>\n    kod\n".as_bytes().to_vec()),
    ]
}

/// One of the loss seeds, mutated: its file name and bytes.
fn mutated(rng: &mut Rng, seeds: &[Seed], dict: &[&[u8]]) -> Vec<u8> {
    // The name goes first, before a NUL, so the check knows which format the bytes are.
    let (name, bytes) = match &seeds[rng.below(seeds.len())] {
        Seed::Zip(name, entries) => (*name, mutate_zip(rng, entries, dict)),
        Seed::Bytes(name, bytes) => (*name, mutate(rng, bytes, dict)),
    };
    [name.as_bytes(), b"\0", &bytes].concat()
}

fn split(input: &[u8]) -> (&str, &[u8]) {
    let at = input.iter().position(|b| *b == 0).unwrap_or(0);
    (std::str::from_utf8(&input[..at]).unwrap_or("a.txt"), &input[(at + 1).min(input.len())..])
}

#[test]
fn damaged_files_are_checked_for_losses() {
    let seeds = loss_seeds();
    let dict: Vec<&[u8]> =
        [XML_TOKENS, TEXT_TOKENS, &[b"{", b"}", br"\*", br"\bin", br"\bin99999999 ", br"\'", b"![", b"](", b"<img src=", b"<script"]].concat();
    run(
        "word-losses",
        iterations(4000),
        move |rng, _| mutated(rng, &seeds, &dict),
        |input| {
            let (name, bytes) = split(input);
            let _ = losses::find(name, bytes);
        },
    );
}

#[test]
fn damaged_files_are_refused_or_opened() {
    let seeds = loss_seeds();
    let dict: Vec<&[u8]> = [XML_TOKENS, TEXT_TOKENS].concat();
    run(
        "word-open",
        iterations(300),
        move |rng, _| mutated(rng, &seeds, &dict),
        |input| {
            let (name, bytes) = split(input);
            let _ = WordDoc::open(name, bytes, None);
        },
    );
}

/// Inputs built to make work grow faster than the file, or to nest deeper than a stack holds.
#[test]
fn pathological_inputs_finish_in_time() {
    use std::time::{Duration, Instant};
    let quick = |what: &str, f: &dyn Fn()| {
        let t = Instant::now();
        f();
        assert!(t.elapsed() < Duration::from_secs(10), "{what} took {:?}", t.elapsed());
    };
    let lt = "<".repeat(300_000);
    quick("unclosed tags", &|| drop(losses::find("a.html", lt.as_bytes())));
    let imgs = "<img".repeat(100_000);
    quick("unclosed pictures", &|| drop(losses::find("a.html", imgs.as_bytes())));
    let bangs = "![".repeat(200_000);
    quick("picture marks", &|| drop(losses::find("a.md", bangs.as_bytes())));
    let groups = [b"{\\rtf1 ".as_slice(), &b"{".repeat(300_000)].concat();
    quick("rtf groups", &|| drop(losses::find("a.rtf", &groups)));

    let udf = |content: String| mo_fuzz::stored_zip(&[("content.xml".into(), content.into_bytes())]);
    // Nested far deeper than any document: refused, not a stack overflow.
    let deep = udf(format!("<template><content><![CDATA[x]]></content><elements>{}</elements></template>", "<a>".repeat(200_000)));
    assert!(udf::read(&deep).is_err());
    // Many runs that each name the whole text: the HTML stays bounded.
    let text = "ş".repeat(100_000);
    let runs = "<content startOffset=\"0\" length=\"100000\"/>".repeat(20_000);
    let many = udf(format!("<template><content><![CDATA[{text}]]></content><elements><paragraph>{runs}</paragraph></elements></template>"));
    mo_fuzz::run_with("udf-runs", 1, mo_fuzz::Limits { memory: 256 << 20, ..Default::default() }, move |_, _| many.clone(), |b| drop(udf::read(b)));
    // A cell that spans 1e30 columns.
    let wide = udf("<template><content><![CDATA[a\n]]></content><elements><table><row><cell colspan=\"1e30\"><paragraph><content startOffset=\"0\" length=\"1\"/></paragraph></cell></row></table></elements></template>".into());
    mo_fuzz::run("udf-colspan", 1, move |_, _| wide.clone(), |b| drop(WordDoc::open("a.udf", b, None)));
}
