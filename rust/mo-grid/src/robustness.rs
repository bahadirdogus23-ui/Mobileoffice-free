//! Damaged and hostile workbooks: the package scanner, the streaming sheet tail, the .xls style
//! reader, the text decoders of CSV import and the warning parsers must return without a panic,
//! within time and memory limits (see `mo-fuzz`).

use mo_fuzz::{Entries, Rng, TEXT_TOKENS, XML_TOKENS, iterations, mutate, mutate_zip, run};

use super::*;

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

/// A workbook as GridCraft writes it, with a slicer, a drawing with ink, an extension and the
/// elements after the cells the scanner looks at.
fn xlsx_entries() -> Entries {
    let csv = "İl;Tutar\nİstanbul;1250\nİzmir;980\n";
    let mut entries = entries_of(&GridDoc::open("k.csv", csv.as_bytes(), None).unwrap().save("k.xlsx").unwrap());
    for (name, data) in entries.iter_mut() {
        if name.ends_with("worksheets/sheet1.xml") {
            let xml = String::from_utf8_lossy(data.as_slice()).replace(
                "</worksheet>",
                r#"<scenarios/><autoFilter ref="A1:B3"><filterColumn colId="0"><filters><dateGroupItem year="2024"/></filters></filterColumn></autoFilter><extLst><ext uri="x"><x14:slicerList xmlns:x14="urn:x14"/></ext></extLst></worksheet>"#,
            );
            *data = xml.into_bytes();
        }
    }
    entries.push((
        "xl/worksheets/_rels/sheet1.xml.rels".into(),
        br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r1" Type="http://schemas.microsoft.com/office/2007/relationships/slicer" Target="../slicers/slicer1.xml"/><Relationship Id="r2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="/xl/drawings/drawing1.xml"/><Relationship Id="r3" Type="x/hyperlink" Target="http://a" TargetMode="External"/></Relationships>"#.to_vec(),
    ));
    entries.push(("xl/drawings/drawing1.xml".into(), br#"<xdr:wsDr xmlns:xdr="urn:x"><xdr:twoCellAnchor><mc:AlternateContent><xdr:contentPart/></mc:AlternateContent></xdr:twoCellAnchor></xdr:wsDr>"#.to_vec()));
    entries
}

const SHEET_TOKENS: &[&[u8]] = &[
    b"sheetData",
    b"<sheetData>",
    b"</sheetData>",
    b"<x:sheetData/>",
    b"</x:sheetData >",
    b"<row r=\"1\">",
    b"<c r=\"A1\"><v>1</v></c>",
    b"</row>",
    b"<sheetData",
    b"_rels/",
    b"../",
];

#[test]
fn the_seeds_are_read() {
    let xlsx = mo_fuzz::stored_zip(&xlsx_entries());
    let losses = losses::from_package("a.xlsx", &xlsx);
    assert!(losses.contains(&Loss::Slicers) && losses.contains(&Loss::Drawings) && losses.contains(&Loss::Filters), "{losses:?}");
    assert!(GridDoc::open("a.xlsx", &xlsx, None).is_ok());
}

#[test]
fn damaged_workbook_packages_are_scanned() {
    let entries = xlsx_entries();
    let dict: Vec<&[u8]> = [XML_TOKENS, SHEET_TOKENS].concat();
    run("xlsx-package", iterations(2500), move |rng, _| mutate_zip(rng, &entries, &dict), |b| drop(losses::from_package("a.xlsm", b)));
}

#[test]
fn damaged_workbooks_are_refused_or_opened() {
    let entries = xlsx_entries();
    let dict: Vec<&[u8]> = [XML_TOKENS, SHEET_TOKENS].concat();
    run("xlsx-open", iterations(300), move |rng, _| mutate_zip(rng, &entries, &dict), |b| drop(GridDoc::open("a.xlsx", b, None)));
}

/// A reader that hands out a few bytes at a time, so markers span reads.
struct Trickle<'a>(&'a [u8], usize);

impl std::io::Read for Trickle<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.1.min(buf.len()).min(self.0.len());
        buf[..n].copy_from_slice(&self.0[..n]);
        self.0 = &self.0[n..];
        Ok(n)
    }
}

#[test]
fn damaged_sheets_have_their_tail_found() {
    let sheet = xlsx_entries().into_iter().find(|(n, _)| n.ends_with("worksheets/sheet1.xml")).unwrap().1;
    run(
        "sheet-tail",
        iterations(3000),
        move |rng, _| mutate(rng, &sheet, SHEET_TOKENS),
        |b| {
            let whole = losses::sheet_tail(b);
            // However the bytes arrive, the same tail is found.
            assert_eq!(losses::sheet_tail(Trickle(b, 1 + b.len() % 7)), whole);
            let _ = losses::sheet_losses(&whole.0, whole.1);
        },
    );
}

#[test]
fn damaged_xls_styles_are_skipped() {
    let xls: &[u8] = include_bytes!("../testdata/styled.xls");
    // The engine's workbook, read once; the style reader lays the damaged file's looks over a copy.
    let (wb, _) = io::open_bytes_with_password("styled.xls", xls, None).unwrap();
    run(
        "xls-styles",
        iterations(1500),
        |rng, _| mutate(rng, include_bytes!("../testdata/styled.xls"), &[]),
        move |b| {
            let mut wb = wb.clone();
            let _ = xls_styles::apply(&mut wb, b);
        },
    );
    run("xls-open", iterations(200), |rng, _| mutate(rng, include_bytes!("../testdata/styled.xls"), &[]), |b| drop(GridDoc::open("a.xls", b, None)));
}

#[test]
fn damaged_csv_files_are_opened() {
    let seeds: Vec<Vec<u8>> = vec![
        encoding_rs::WINDOWS_1254.encode("İl;İlçe;Tutar\nİstanbul;Şişli;1250\n\"İz;mir\";Karşıyaka;=1+2\n").0.into_owned(),
        "a,b\n\"x\"\"y\",2\n".encode_utf16().flat_map(u16::to_le_bytes).collect(),
        "a\tb\n1\t2\n".as_bytes().to_vec(),
    ];
    const CSV_TOKENS: &[&[u8]] = &[b"\"", b";", b",", b"\t", b"=", b"=SUM(A1:A9)", b"\xdd", b"\x00"];
    let dict: Vec<&[u8]> = [TEXT_TOKENS, CSV_TOKENS].concat();
    run(
        "csv",
        iterations(400),
        move |rng, case| pick(rng, &seeds, &dict, case),
        |b| {
            let _ = encoding::decode_unmarked_utf16(b);
            let _ = looks_like_windows_1254(b);
            let _ = decode_turkish_legacy(b);
            let _ = GridDoc::open(if b.len() % 2 == 0 { "a.csv" } else { "a.tsv" }, b, None);
        },
    );
}

fn pick(rng: &mut Rng, seeds: &[Vec<u8>], dict: &[&[u8]], _case: usize) -> Vec<u8> {
    let i = rng.below(seeds.len());
    mutate(rng, &seeds[i], dict)
}

#[test]
fn damaged_warnings_are_parsed() {
    let seeds: Vec<Vec<u8>> = [
        "sheet \"Ş\"ube\" is damaged (bad xml); kept 12 rows",
        "shared strings are damaged (x); kept 5 strings",
        "sheet \"A\" could not be read: x",
        "chart sheet \"C\" was dropped: y",
        "skipped xl/styles.xml: bad",
        "3 hyperlinks were dropped",
        "\"\"",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    run(
        "warnings",
        iterations(3000),
        move |rng, case| pick(rng, &seeds, TEXT_TOKENS, case),
        |b| {
            let w = String::from_utf8_lossy(b);
            let _ = losses::from_warnings(&[w.to_string()]);
            let _ = (parse_truncation(&w), parse_strings_cut(&w), parse_unread_sheet(&w), parse_skipped_part(&w));
        },
    );
}

/// Inputs built to make the scanners' work grow faster than the file.
#[test]
fn pathological_packages_finish_in_time() {
    // "<sheetData" over and over, without a `>`.
    let sheet = [b"<worksheet>".as_slice(), &b"<sheetData".repeat(300_000)].concat();
    let t = std::time::Instant::now();
    let _ = losses::sheet_tail(&sheet[..]);
    assert!(t.elapsed() < std::time::Duration::from_secs(10), "{:?}", t.elapsed());
    // Many relationships to worksheets, and many entries.
    let rels = format!(
        "<Relationships>{}</Relationships>",
        (0..40_000).map(|i| format!("<Relationship Type=\"x/worksheet\" Target=\"s{i}.xml\"/>")).collect::<String>()
    );
    let mut entries: Entries = vec![("xl/_rels/workbook.xml.rels".into(), rels.into_bytes())];
    entries.extend((0..20_000).map(|i| (format!("xl/x{i}.bin"), Vec::new())));
    let zip = mo_fuzz::stored_zip(&entries);
    mo_fuzz::run("xlsx-many-parts", 1, move |_, _| zip.clone(), |b| drop(losses::from_package("a.xlsx", b)));
}
