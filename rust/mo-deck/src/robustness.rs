//! Damaged and hostile presentations: our own ODP and PPT importers, the font name-table reader
//! and opening PPTX must return (an error is fine) without a panic, within time and memory limits
//! (see `mo-fuzz`).

use mo_fuzz::{Entries, XML_TOKENS, iterations, mutate, mutate_zip, run};

use super::*;
use crate::tests::{ppt_record, ppt_text, sample_pptx};

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

const ODP_TOKENS: &[&[u8]] = &[
    b"<text:p>",
    b"</text:p>",
    b"<text:s text:c=\"99999999999\"/>",
    b"<text:line-break/>",
    b"<draw:frame svg:width=\"1e308cm\" svg:height=\"-5mm\" svg:x=\"NaNin\">",
    b"</draw:frame>",
    b"<draw:page>",
    b"</draw:page>",
    b"<draw:text-box>",
    b"<draw:image xlink:href=\"Pictures/a.png\"/>",
    b"<draw:custom-shape svg:width=\"1cm\" svg:height=\"1cm\"><draw:enhanced-geometry draw:type=\"star5\"/></draw:custom-shape>",
    b"<presentation:notes>",
    b"presentation:class=\"notes\"",
    b"fo:page-width=\"0cm\"",
    b"%",
];

/// The mimetype entry first, uncompressed, as `odp::sniff` wants it.
fn odp_entries() -> Entries {
    let mut e = entries_of(include_bytes!("../testdata/sample.odp"));
    e.sort_by_key(|(n, _)| n != "mimetype");
    e
}

/// A .ppt's "PowerPoint Document" record stream: a document atom, two slides, notes.
fn ppt_stream() -> Vec<u8> {
    let mut doc_atom = 5760i32.to_le_bytes().to_vec();
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
    ppt_record(0x0F, 0, 0x03E8, &container)
}

/// `stream` as the "PowerPoint Document" stream of a compound file.
fn ppt_file(stream: &[u8]) -> Vec<u8> {
    let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    std::io::Write::write_all(&mut file.create_stream("/PowerPoint Document").unwrap(), stream).unwrap();
    file.into_inner().into_inner()
}

/// What the app reads of an opened deck.
fn look_at(d: &DeckDoc) {
    for slide in 0..d.slide_count().min(3) {
        let _ = d.text_shapes(slide);
        let _ = d.notes(slide);
    }
}

#[test]
fn the_seeds_are_read() {
    let odp = mo_fuzz::stored_zip(&odp_entries());
    assert!(odp::sniff(&odp));
    assert!(DeckDoc::open(&odp).is_ok_and(|d| d.slide_count() > 0));
    assert!(DeckDoc::open(&ppt_file(&ppt_stream())).is_ok_and(|d| d.slide_count() == 2));
}

#[test]
fn damaged_opendocument_presentations_are_refused_or_read() {
    let entries = odp_entries();
    let dict: Vec<&[u8]> = [XML_TOKENS, ODP_TOKENS].concat();
    run(
        "odp",
        iterations(2000),
        move |rng, _| {
            // Keep the mimetype entry first and whole, so the bytes still sniff as ODP.
            let mut rest = mo_fuzz::mutate_entries(rng, &entries[1..], &dict);
            rest.insert(0, entries[0].clone());
            mo_fuzz::stored_zip(&rest)
        },
        |b| {
            if let Ok((pres, _)) = odp::import(b) {
                let _ = DeckDoc::from_presentation(pres);
            }
        },
    );
}

#[test]
fn damaged_old_powerpoint_files_are_refused_or_read() {
    let stream = ppt_stream();
    let tokens: &[&[u8]] = &[b"\x0f\x00\xf0\x0f", b"\x00\x00\xf3\x03\x14\x00\x00\x00", b"\x00\x00\xa0\x0f", b"\x00\x00\xa8\x0f", b"\xff\xff\xff\xff"];
    run("ppt", iterations(1500), move |rng, _| ppt_file(&mutate(rng, &stream, tokens)), |b| drop(ppt::import(b)));
    // The compound file itself damaged.
    let file = ppt_file(&ppt_stream());
    run("ppt-cfb", iterations(1000), move |rng, _| mutate(rng, &file, &[]), |b| drop(DeckDoc::open(b).map(|d| look_at(&d))));
}

#[test]
fn damaged_font_name_tables_are_read() {
    let fonts: Vec<&'static [u8]> = wordcraft_fonts::bundled().iter().map(|f| &f[..]).take(3).collect();
    // Only the tables at the front are read: mutate a copy cut to its first 64 KB, and the whole.
    run(
        "font-names",
        iterations(3000),
        move |rng, case| {
            let f = fonts[case % fonts.len()];
            if rng.one_in(4) { mutate(rng, f, &[]) } else { mutate(rng, &f[..f.len().min(64 << 10)], &[b"name", b"\xff\xff", b"\x00\x03\x00\x01"]) }
        },
        |b| drop(fonts::family(b)),
    );
}

#[test]
fn damaged_pptx_files_are_refused_or_opened() {
    let seeds = [entries_of(&sample_pptx()), entries_of(include_bytes!("../testdata/grouped-text.pptx"))];
    run(
        "pptx",
        iterations(250),
        move |rng, _| {
            let i = rng.below(seeds.len());
            mutate_zip(rng, &seeds[i], XML_TOKENS)
        },
        |b| drop(DeckDoc::open(b).map(|d| look_at(&d))),
    );
}

/// ODP built to repeat its work: deep nesting, one picture in many frames.
#[test]
fn pathological_presentations_stay_bounded() {
    let content = |body: &str| {
        format!(
            r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"><office:body><office:presentation>{body}</office:presentation></office:body></office:document-content>"#
        )
    };
    let odp = |content: String, extra: Vec<(String, Vec<u8>)>| {
        let mut e: Entries =
            vec![("mimetype".into(), b"application/vnd.oasis.opendocument.presentation".to_vec()), ("content.xml".into(), content.into_bytes())];
        e.extend(extra);
        mo_fuzz::stored_zip(&e)
    };
    // Paragraphs nested in paragraphs, each holding the text below it.
    let deep = odp(
        content(&format!(
            "<draw:page><draw:frame svg:width=\"1cm\" svg:height=\"1cm\"><draw:text-box>{}{}{}</draw:text-box></draw:frame></draw:page>",
            "<text:p>".repeat(200),
            "ş".repeat(200_000),
            "</text:p>".repeat(200)
        )),
        Vec::new(),
    );
    let deeper = odp(content(&format!("<draw:page>{}</draw:page>", "<text:span>".repeat(100_000))), Vec::new());
    // One 4 MB picture in 2000 frames.
    let picture = [&[0x89u8, b'P', b'N', b'G'][..], &vec![1u8; 4 << 20]].concat();
    let frames = "<draw:frame svg:width=\"1cm\" svg:height=\"1cm\"><draw:image xlink:href=\"Pictures/a.png\"/></draw:frame>".repeat(2000);
    let pictures = odp(content(&format!("<draw:page>{frames}</draw:page>")), vec![("Pictures/a.png".into(), picture)]);
    let cases = [deep, deeper, pictures];
    mo_fuzz::run_with(
        "odp-bounded",
        cases.len(),
        mo_fuzz::Limits { memory: 128 << 20, per_input_byte: 8, ..Default::default() },
        move |_, i| cases[i].clone(),
        |b| drop(odp::import(b)),
    );
}
