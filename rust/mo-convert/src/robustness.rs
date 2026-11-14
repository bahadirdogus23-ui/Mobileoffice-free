//! Damaged and hostile e-books, e-mails, contacts, calendars and text files: every reader must
//! return (an error is fine) without a panic, within time and memory limits (see `mo-fuzz`).

use mo_fuzz::{Entries, Rng, TEXT_TOKENS, XML_TOKENS, iterations, mutate, mutate_zip, run};

const HTML_TOKENS: &[&[u8]] = &[
    b"<body>",
    b"</body>",
    b"<BODY ",
    b"<script>",
    b"</script>",
    b"<style",
    b"<head>",
    b"src=\"",
    b"src='",
    b"SRC=",
    b"src=\"../img/k.png\"",
    b"src=\"data:",
    b"src=\"cid:",
    b"%",
    b"%2",
    b"%C4%B0",
    b"../",
    b"/",
    b"#",
    b"<img src=\"../img/k.png\"/>",
    b"<meta charset=windows-1254>",
    b"<meta charset=\"",
    b"\xdd\xfe",
];

fn epub_entries() -> Entries {
    let png = [0x89u8, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 0];
    vec![
        ("mimetype".into(), b"application/epub+zip".to_vec()),
        ("META-INF/container.xml".into(), br#"<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#.to_vec()),
        ("OEBPS/content.opf".into(), r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Çalıkuşu</dc:title><dc:creator>Reşat Nuri</dc:creator></metadata><manifest><item id="c2" href="text/b.xhtml" media-type="application/xhtml+xml"/><item id="c1" href="text/a%20b.xhtml" media-type="application/xhtml+xml"/><item id="i" href="img/k.png" media-type="image/png"/></manifest><spine><itemref idref="c1"/><itemref idref="c2"/><itemref idref="c1"/></spine></package>"#.as_bytes().to_vec()),
        ("OEBPS/text/a b.xhtml".into(), r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><style>p{}</style></head><body><h2>Birinci bölüm</h2><p>Feride okula gitti.</p><img src="../img/k.png"/><img src='../img/k.png#x'/><script>x()</script></body></html>"#.as_bytes().to_vec()),
        ("OEBPS/text/b.xhtml".into(), r#"<html xmlns="http://www.w3.org/1999/xhtml"><body><h2>İkinci bölüm</h2><img src="data:image/png;base64,AAAA"/></body></html>"#.as_bytes().to_vec()),
        ("OEBPS/img/k.png".into(), png.to_vec()),
    ]
}

const MAIL_TOKENS: &[&[u8]] = &[
    b"\r\n",
    b"\r\n\r\n",
    b"\n\n",
    b"--b1\r\n",
    b"--b1--\r\n",
    b"Content-Type: multipart/mixed; boundary=\"b1\"\r\n",
    b"Content-Type: multipart/related; boundary=b1\r\n",
    b"Content-Type: text/html; charset=ibm857\r\n",
    b"Content-Type: text/plain; charset=windows-1254\r\n",
    b"Content-Type: text/plain\r\n",
    b"Content-Type: message/rfc822\r\n",
    b"Content-Transfer-Encoding: base64\r\n",
    b"Content-Transfer-Encoding: quoted-printable\r\n",
    b"Content-Disposition: attachment; filename=\"a.txt\"\r\n",
    b"Content-ID: <p1>\r\n",
    b"=?UTF-8?B?",
    b"=?ISO-8859-9?Q?",
    b"?=",
    b"=C5=9E",
    b"=\r\n",
    b"<img src=\"cid:p1\">",
    b"<body>",
    b"<script>",
    b"\xdd\xfe\xfd",
    b"From: ",
    b"Subject: ",
    b"Date: Sun, 8 Nov 2026 09:30:00 +0300\r\n",
];

fn eml_seeds() -> Vec<Vec<u8>> {
    let png = "iVBORw0KGgoAAAAAAAA=";
    vec![
        b"From: =?UTF-8?Q?Ay=C5=9Fe_Kaya?= <ayse@example.com>\r\nTo: Ali <ali@example.com>\r\nSubject: =?ISO-8859-9?Q?Toplant=FD_notlar=FD?=\r\nDate: Sun, 8 Nov 2026 09:30:00 +0300\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"b1\"\r\n\r\n--b1\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><head><style>p{}</style></head><body><p>Merhaba, <b>ekte</b> notlar var.</p><img src=\"cid:p1\"><script>x()</script></body></html>\r\n--b1\r\nContent-Type: image/png\r\nContent-ID: <p1>\r\nContent-Transfer-Encoding: base64\r\n\r\nPNG\r\n--b1\r\nContent-Type: text/plain; name=\"not.txt\"\r\nContent-Disposition: attachment; filename=\"not.txt\"\r\nContent-Transfer-Encoding: base64\r\n\r\nxZ5pcmtldA==\r\n--b1--\r\n".replace(b"PNG", png.as_bytes()),
        b"From: Ay\xfee <a@b.c>\r\nSubject: \xdd\xfe toplant\xfds\xfd\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=ibm857\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\n=98=8D=9E=9F =80=87\r\n".to_vec(),
        b"From: a@b.c\r\nSubject: x\r\nMIME-Version: 1.0\r\nContent-Type: text/html\r\nContent-Transfer-Encoding: 8bit\r\n\r\n<html><head><meta charset=windows-1254></head><body><p>\xdd\xfd\xde\xfe</p></body></html>".to_vec(),
    ]
}

trait Replace {
    fn replace(&self, from: &[u8], to: &[u8]) -> Vec<u8>;
}

impl Replace for [u8] {
    fn replace(&self, from: &[u8], to: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.len() {
            if self[i..].starts_with(from) {
                out.extend_from_slice(to);
                i += from.len();
            } else {
                out.push(self[i]);
                i += 1;
            }
        }
        out
    }
}

const VCF: &[u8] = b"BEGIN:VCARD\r\nVERSION:2.1\r\nN;CHARSET=UTF-8;ENCODING=QUOTED-PRINTABLE:=C3=96zt=C3=BCrk;=C5=9Eule;;;\r\nNOTE;ENCODING=QUOTED-PRINTABLE:a=\r\nb=\r\nc\r\nTEL;CELL:+905321112233\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nFN:Ahmet Y\xc4\xb1lmaz\r\nitem1.ORG:\xc3\x96rnek A.\xc5\x9e.;Muhasebe\r\nTEL;TYPE=WORK,VOICE:0212 555 00 00\r\nEMAIL;TYPE=\"INTERNET\":ahmet@ornek.com.tr\r\nADR;TYPE=HOME:;;Atat\xc3\xbcrk Cd. 5;\xc4\xb0zmir;;35000;T\xc3\xbcrkiye\r\nNOTE:Sat\xc4\xb1r bir\\nSat\xc4\xb1r iki\r\n  devam\r\nEND:VCARD\r\n";

const ICS: &[u8] = b"BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nSUMMARY:\xc4\xb0kinci toplant\xc4\xb1\nDTSTART:20261110T130000Z\nDTEND:20261110T140000Z\nLOCATION:Ankara\nEND:VEVENT\nBEGIN:VTODO\nSUMMARY:Haftal\xc4\xb1k rapor\nDTSTART;TZID=Europe/Istanbul:20261108T090000\nDUE:20261109\nRRULE:FREQ=WEEKLY;COUNT=4;UNTIL=20261231;INTERVAL=2\nORGANIZER;CN=\"Ay\xc5\x9fe: Kaya\":mailto:ayse@example.com\nATTENDEE;CN=Ali:MAILTO:ali@example.com\nSTATUS:CONFIRMED\nDESCRIPTION:G\xc3\xbcndem:\\n- sat\xc4\xb1\xc5\x9flar\nBEGIN:VALARM\nSUMMARY:Gizli alarm\nEND:VALARM\nEND:VTODO\nEND:VCALENDAR\n";

const CARD_TOKENS: &[&[u8]] = &[
    b"\r\n",
    b"\n ",
    b"\r\n\t",
    b"=\r\n",
    b"=",
    b";ENCODING=QUOTED-PRINTABLE",
    b";CHARSET=ibm857",
    b";CHARSET=\"",
    b":",
    b";",
    b"\"",
    b"\\",
    b"BEGIN:VCARD\r\n",
    b"END:VCARD\r\n",
    b"BEGIN:VEVENT\n",
    b"END:VEVENT\n",
    b"BEGIN:VALARM\n",
    b"END:VALARM\n",
    b"DTSTART:2026",
    b"Z",
    b"\xdd\xfe",
    b"\xc4\xb0",
    b"\xff\xfe",
];

#[test]
fn damaged_e_books_are_refused_or_read() {
    let entries = epub_entries();
    let mut dict: Vec<&[u8]> = XML_TOKENS.to_vec();
    dict.extend_from_slice(HTML_TOKENS);
    run("epub", iterations(1500), move |rng, _| mutate_zip(rng, &entries, &dict), |b| drop(crate::to_html("k.epub", b)));
}

#[test]
fn damaged_e_mails_are_refused_or_read() {
    let seeds = eml_seeds();
    let mut dict: Vec<&[u8]> = MAIL_TOKENS.to_vec();
    dict.extend_from_slice(HTML_TOKENS);
    run("eml", iterations(2500), move |rng, _| pick_mutate(rng, &seeds, &dict), |b| drop(crate::to_html("a.eml", b)));
}

#[test]
fn damaged_outlook_messages_are_refused_or_read() {
    let html = b"<html><head><meta charset=windows-1254></head><body><p>\xdd\xfd<img src=\"cid:x\"></p></body></html>";
    let seeds = vec![
        crate::tests::msg_file(Some(1254), Some(1254), html),
        crate::tests::msg_file(None, Some(65001), "<p>İş</p>".as_bytes()),
        crate::tests::msg_file(Some(1252), None, b""),
    ];
    run("msg", iterations(2000), move |rng, _| pick_mutate(rng, &seeds, &[]), |b| drop(crate::to_html("a.msg", b)));
}

#[test]
fn damaged_contacts_and_calendars_are_read() {
    run("vcf", iterations(3000), |rng, _| mutate(rng, VCF, CARD_TOKENS), |b| drop(crate::to_html("a.vcf", b)));
    run("ics", iterations(3000), |rng, _| mutate(rng, ICS, CARD_TOKENS), |b| drop(crate::to_html("a.ics", b)));
}

/// One of `seeds`, mutated.
fn pick_mutate(rng: &mut Rng, seeds: &[Vec<u8>], dict: &[&[u8]]) -> Vec<u8> {
    let i = rng.below(seeds.len());
    mutate(rng, &seeds[i], dict)
}

#[test]
fn any_bytes_decode_and_exact_text_saves_back_the_same() {
    use crate::text::*;
    let le: Vec<u8> = "İstanbul 😀 şık".encode_utf16().flat_map(u16::to_le_bytes).collect();
    let seeds: Vec<Vec<u8>> = vec![
        b"\xdd\xfd\xde\xfe\xd0\xf0\xc7\xe7 \x93al\xfdnt\xfd\x94 \x96 tire\r\n".to_vec(),
        "Şişli'de ığdır çağrı İĞÜŞÖÇ\r\n“alıntı”\r\n".as_bytes().to_vec(),
        le.clone(),
        [&[0xFF, 0xFE][..], &le].concat(),
        [&[0xEF, 0xBB, 0xBF][..], "İ".as_bytes()].concat(),
        encoding_rs::WINDOWS_1251.encode("Привет, как дела?").0.into_owned(),
        encoding_rs::GBK.encode("中文测试，你好世界").0.into_owned(),
        b"<html><head><meta charset=\"windows-1254\"><META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=iso-8859-9\">".to_vec(),
        vec![0x98, 0x8D, 0x9E, 0x9F, 0xA6, 0xA7, 0x80, 0x87, 0xD5, 0xFF],
    ];
    let labels = ["windows-1254", "ibm857", "\"cp857\"", "utf-16le", "UTF-16BE", "replacement", "x-user-defined", "", "iso-8859-9", "gbk", "nope"];
    run(
        "text",
        iterations(2000),
        move |rng, _| pick_mutate(rng, &seeds, TEXT_TOKENS),
        move |b| {
            let _ = decode_any(b);
            let _ = decode_undeclared(b);
            let _ = html_meta_charset(b);
            let _ = decode_cp857(b);
            for label in labels {
                let _ = decode_label(b, label);
                if let Some(enc) = encoding_named(label) {
                    let _ = decode_declared(b, enc);
                }
            }
            for complete in [true, false] {
                let d = decode_text(b, complete);
                if d.exact && complete {
                    // What reads back exactly must be written back byte for byte.
                    assert_eq!(encode_text(&d.text, d.encoding, d.bom).as_deref(), Ok(b), "{}", d.encoding.name());
                }
                if let Ok(again) = encode_text(&d.text, d.encoding, d.bom) {
                    // A text written in its encoding reads back as the same text (UTF-16 without
                    // a mark can be read as other UTF-16 or 8-bit text; only marked text is
                    // certain to come back in the same encoding).
                    if d.bom {
                        assert_eq!(decode_text(&again, true).text, d.text);
                    }
                }
            }
        },
    );
}

#[test]
fn damaged_packages_are_checked() {
    use crate::package::{XmlLimits, damaged_entries, xml_reads_whole};
    let limits = XmlLimits { max_elements: 10_000, max_depth: 64, max_attrs: Some(16), max_bytes: 1 << 20, allow_unmatched_ends: false };
    let entries: Entries = vec![
        ("[Content_Types].xml".into(), b"<?xml version=\"1.0\"?><Types xmlns=\"urn:x\"><Default Extension=\"xml\"/></Types>".to_vec()),
        (
            "word/document.xml".into(),
            "<w:document xmlns:w=\"urn:w\"><w:body><w:p><w:r><w:t>Şişli &amp; İzmir</w:t></w:r></w:p></w:body></w:document>".as_bytes().to_vec(),
        ),
        ("word/media/a.bin".into(), vec![0, 1, 2, 3, 4]),
        ("utf16.xml".into(), [0xFF, 0xFE].into_iter().chain("<a>ş</a>".encode_utf16().flat_map(u16::to_le_bytes)).collect()),
    ];
    run(
        "package",
        iterations(3000),
        move |rng, _| mutate_zip(rng, &entries, XML_TOKENS),
        move |b| {
            let _ = damaged_entries(b, |n| n.ends_with(".xml"), &limits);
            let _ = xml_reads_whole(b, &limits);
        },
    );
}

/// Inputs built to make work grow faster than the file: each must finish quickly.
#[test]
fn pathological_inputs_finish_in_time() {
    use std::time::{Duration, Instant};
    let quick = |what: &str, f: &dyn Fn()| {
        let t = Instant::now();
        f();
        assert!(t.elapsed() < Duration::from_secs(10), "{what} took {:?}", t.elapsed());
    };
    // Many scripts, styles and pictures in one page.
    let scripts = "<script>x</script><style>y</style>".repeat(40_000);
    quick("scripts", &|| drop(crate::body_of(&format!("<body>{scripts}</body>"))));
    let srcs = "<img src=\"a\">".repeat(80_000);
    quick("pictures", &|| drop(crate::rewrite_src(&srcs, |_| Some("b".into()))));
    let unquoted = "src=".repeat(200_000);
    quick("unquoted", &|| drop(crate::rewrite_src(&unquoted, |_| None)));
    // A quoted-printable value folded over many lines.
    let mut vcf = b"BEGIN:VCARD\r\nNOTE;ENCODING=QUOTED-PRINTABLE:".to_vec();
    vcf.extend(b"=C5=9E=\r\n".repeat(100_000));
    vcf.extend_from_slice(b"x\r\nEND:VCARD\r\n");
    quick("folded quoted-printable", &|| drop(crate::to_html("a.vcf", &vcf)));
    // A chapter that shows the same picture many times, read many times over.
    let mut entries = epub_entries();
    let big = [&[0x89u8, b'P', b'N', b'G'][..], &vec![7u8; 1 << 20]].concat();
    entries[5].1 = big;
    entries[3].1 = format!("<html><body>{}</body></html>", "<img src=\"../img/k.png\"/>".repeat(2000)).into_bytes();
    entries[2].1 =
        String::from_utf8(entries[2].1.clone()).unwrap().replace("<itemref idref=\"c2\"/>", &"<itemref idref=\"c1\"/>".repeat(200)).into_bytes();
    let zip = mo_fuzz::stored_zip(&entries);
    mo_fuzz::run_with(
        "epub-pictures",
        1,
        mo_fuzz::Limits { memory: 512 << 20, ..Default::default() },
        move |_, _| zip.clone(),
        |b| {
            let c = crate::to_html("k.epub", b).unwrap();
            assert!(c.notes.iter().any(|n| n.contains("resim")), "{:?}", c.notes);
        },
    );
}

/// The inputs the mutations start from are read whole (else the cases test only the first check).
#[test]
fn the_seeds_are_read() {
    let epub = crate::to_html("k.epub", &mo_fuzz::stored_zip(&epub_entries()));
    assert!(epub.as_ref().is_ok_and(|c| c.html.contains("İkinci bölüm") && c.html.contains("data:image/png")), "{epub:?}");
    for eml in eml_seeds() {
        assert!(crate::to_html("a.eml", &eml).is_ok_and(|c| c.html.contains("<h1>")));
    }
}
