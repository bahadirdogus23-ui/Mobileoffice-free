use super::*;

#[test]
fn contacts_from_old_and_new_phones() {
    let vcf = "BEGIN:VCARD\r\nVERSION:2.1\r\nN;CHARSET=UTF-8;ENCODING=QUOTED-PRINTABLE:=C3=96zt=C3=BCrk;=C5=9Eule;;;\r\nTEL;CELL:+905321112233\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nFN:Ahmet Yılmaz\r\nORG:Örnek A.Ş.;Muhasebe\r\nTEL;TYPE=WORK,VOICE:0212 555 00 00\r\nEMAIL;TYPE=INTERNET:ahmet@ornek.com.tr\r\nADR;TYPE=HOME:;;Atatürk Cd. 5;İzmir;;35000;Türkiye\r\nNOTE:Satır bir\\nSatır iki\r\nEND:VCARD\r\n";
    let c = to_html("rehber.vcf", vcf.as_bytes()).unwrap();
    assert!(c.html.contains("2 kişi"));
    assert!(c.html.contains("Şule Öztürk"), "quoted-printable name: {}", c.html);
    assert!(c.html.contains("Telefon (cep)") && c.html.contains("+905321112233"));
    assert!(c.html.contains("Örnek A.Ş. · Muhasebe") && c.html.contains("Telefon (iş)"));
    assert!(c.html.contains("Atatürk Cd. 5, İzmir, 35000, Türkiye"));
    assert!(c.html.contains("Satır bir<br/>Satır iki"));
}

#[test]
fn calendar_events_in_date_order() {
    let ics = "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nSUMMARY:İkinci toplantı\nDTSTART:20261110T130000Z\nDTEND:20261110T140000Z\nLOCATION:Ankara\nEND:VEVENT\nBEGIN:VEVENT\nSUMMARY:Haftalık rapor\nDTSTART;TZID=Europe/Istanbul:20261108T090000\nRRULE:FREQ=WEEKLY;COUNT=4\nORGANIZER;CN=Ayşe Kaya:mailto:ayse@example.com\nDESCRIPTION:Gündem:\\n- satışlar\nBEGIN:VALARM\nSUMMARY:Gizli alarm\nEND:VALARM\nEND:VEVENT\nEND:VCALENDAR\n";
    let c = to_html("davet.ics", ics.as_bytes()).unwrap();
    let a = c.html.find("Haftalık rapor").unwrap();
    let b = c.html.find("İkinci toplantı").unwrap();
    assert!(a < b, "sorted by start");
    assert!(c.html.contains("08.11.2026 09:00 (Europe/Istanbul)"));
    assert!(c.html.contains("10.11.2026 13:00 (UTC)"));
    assert!(c.html.contains("her hafta, 4 kez"));
    assert!(c.html.contains("Ayşe Kaya &lt;ayse@example.com&gt;"));
    assert!(!c.html.contains("Gizli alarm"));
}

#[test]
fn an_email_with_a_turkish_subject_html_body_and_attachment() {
    let eml = "From: =?UTF-8?Q?Ay=C5=9Fe_Kaya?= <ayse@example.com>\r\nTo: Ali <ali@example.com>\r\nSubject: =?ISO-8859-9?Q?Toplant=FD_notlar=FD?=\r\nDate: Sun, 8 Nov 2026 09:30:00 +0300\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"b1\"\r\n\r\n--b1\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><head><style>p{}</style></head><body><p>Merhaba, <b>ekte</b> notlar var.</p><script>x()</script></body></html>\r\n--b1\r\nContent-Type: text/plain; name=\"not.txt\"\r\nContent-Disposition: attachment; filename=\"not.txt\"\r\nContent-Transfer-Encoding: base64\r\n\r\nxZ5pcmtldA==\r\n--b1--\r\n";
    let c = to_html("ileti.eml", eml.as_bytes()).unwrap();
    assert!(c.html.contains("Toplantı notları"), "{}", c.html);
    assert!(c.html.contains("Ayşe Kaya &lt;ayse@example.com&gt;"));
    assert!(c.html.contains("08.11.2026 09:30 (GMT+03:00)"));
    assert!(c.html.contains("<b>ekte</b>"));
    assert!(!c.html.contains("x()") && !c.html.contains("p{}"), "scripts and styles left out");
    assert_eq!(c.attachments.len(), 1);
    assert_eq!(c.attachments[0].name, "not.txt");
    assert_eq!(String::from_utf8(c.attachments[0].data.clone()).unwrap(), "Şirket");
}

#[test]
fn an_epub_in_spine_order_with_its_pictures() {
    use std::io::Write;
    let mut out = std::io::Cursor::new(Vec::new());
    let mut z = zip::ZipWriter::new(&mut out);
    let o = zip::write::SimpleFileOptions::default();
    let png = [0x89u8, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 0];
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("mimetype", b"application/epub+zip".to_vec()),
        ("META-INF/container.xml", br#"<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#.to_vec()),
        ("OEBPS/content.opf", r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Çalıkuşu</dc:title><dc:creator>Reşat Nuri</dc:creator></metadata><manifest><item id="c2" href="text/b.xhtml" media-type="application/xhtml+xml"/><item id="c1" href="text/a.xhtml" media-type="application/xhtml+xml"/><item id="i" href="img/k.png" media-type="image/png"/></manifest><spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#.as_bytes().to_vec()),
        ("OEBPS/text/a.xhtml", r#"<html xmlns="http://www.w3.org/1999/xhtml"><body><h2>Birinci bölüm</h2><p>Feride okula gitti.</p><img src="../img/k.png"/></body></html>"#.as_bytes().to_vec()),
        ("OEBPS/text/b.xhtml", r#"<html xmlns="http://www.w3.org/1999/xhtml"><body><h2>İkinci bölüm</h2></body></html>"#.as_bytes().to_vec()),
        ("OEBPS/img/k.png", png.to_vec()),
    ];
    for (n, d) in files {
        z.start_file(n, o).unwrap();
        z.write_all(&d).unwrap();
    }
    z.finish().unwrap();
    let c = to_html("kitap.epub", &out.into_inner()).unwrap();
    assert!(c.html.contains("<h1>Çalıkuşu</h1>") && c.html.contains("Reşat Nuri"));
    assert!(c.html.find("Birinci bölüm").unwrap() < c.html.find("İkinci bölüm").unwrap(), "spine order");
    assert!(c.html.contains("src=\"data:image/png;base64,"), "pictures inside the page");
    assert!(c.notes[0].contains("2 bölüm"));
}

#[test]
fn unknown_files_are_not_taken() {
    assert_eq!(kind_of("a.txt", b"hello"), None);
    assert_eq!(kind_of("x", b"BEGIN:VCARD\n"), Some(Kind::VCard));
    assert!(to_html("a.txt", b"hello").is_err());
}
