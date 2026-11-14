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

/// "İıŞşĞğÇçÖöÜü “alıntı” – tire" in Windows-1254 (ISO-8859-9 has the same letter bytes).
const W1254: &[u8] = b"\xdd\xfd\xde\xfe\xd0\xf0\xc7\xe7\xd6\xf6\xdc\xfc \x93al\xfdnt\xfd\x94 \x96 tire";
const TURKISH: &str = "İıŞşĞğÇçÖöÜü “alıntı” – tire";

fn eml_body(headers: &str, body: &[u8]) -> String {
    let mut m = format!("From: a@b.c\r\nSubject: x\r\nMIME-Version: 1.0\r\n{headers}\r\n\r\n").into_bytes();
    m.extend_from_slice(body);
    to_html("a.eml", &m).unwrap().html
}

#[test]
fn e_mails_in_turkish_code_pages_keep_their_letters() {
    for headers in [
        // Labelled Windows-1254 and ISO-8859-9 (Turkish Outlook's label, with Windows-1254's quotes).
        "Content-Type: text/plain; charset=windows-1254",
        "Content-Type: text/plain; charset=iso-8859-9\r\nContent-Transfer-Encoding: 8bit",
        // No charset, or no Content-Type at all: Turkish first, not "�".
        "Content-Type: text/plain",
        "X-Mailer: eski",
    ] {
        let html = eml_body(headers, W1254);
        assert!(html.contains(TURKISH), "{headers}: {html}");
    }
    // Quoted-printable ISO-8859-9.
    let html = eml_body("Content-Type: text/plain; charset=ISO-8859-9\r\nContent-Transfer-Encoding: quoted-printable", b"=DD=FD=DE=FE =93x=94");
    assert!(html.contains("İıŞş “x”"), "{html}");
    // An HTML part that names its charset only in a <meta>.
    let mut page = b"<html><head><meta charset=windows-1254></head><body><p>".to_vec();
    page.extend_from_slice(W1254);
    page.extend_from_slice(b"</p></body></html>");
    let html = eml_body("Content-Type: text/html\r\nContent-Transfer-Encoding: 8bit", &page);
    assert!(html.contains(TURKISH), "{html}");
    // A UTF-8 part without a charset is still UTF-8.
    assert!(eml_body("Content-Type: text/plain", TURKISH.as_bytes()).contains(TURKISH));
    // A raw 8-bit subject (no RFC 2047 encoded word), as old mail programs wrote it.
    let raw = b"From: Ay\xfee <a@b.c>\r\nSubject: \xdd\xfe toplant\xfds\xfd\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=windows-1254\r\n\r\nx";
    let html = to_html("a.eml", raw).unwrap().html;
    assert!(html.contains("<h1>İş toplantısı</h1>") && html.contains("Ayşe"), "{html}");
}

#[test]
fn e_mails_in_the_turkish_dos_code_page_keep_their_letters() {
    // "İıŞşĞğÇçÖöÜü" in CP857, as 8 bits and quoted-printable, under the names it goes by.
    let dos = b"\x98\x8d\x9e\x9f\xa6\xa7\x80\x87\x99\x94\x9a\x81";
    for charset in ["ibm857", "IBM857", "cp857", "\"cp857\""] {
        let html = eml_body(&format!("Content-Type: text/plain; charset={charset}\r\nContent-Transfer-Encoding: 8bit"), dos);
        assert!(html.contains("İıŞşĞğÇçÖöÜü"), "{charset}: {html}");
    }
    let html = eml_body("Content-Type: text/plain; charset=ibm857\r\nContent-Transfer-Encoding: quoted-printable", b"=98=8D=9E=9F =80=87");
    assert!(html.contains("İıŞş Çç"), "{html}");
}

#[test]
fn contacts_and_calendars_in_windows_1254_keep_quotes_and_dashes() {
    let vcf = b"BEGIN:VCARD\r\nVERSION:2.1\r\nN;CHARSET=ISO-8859-9;ENCODING=QUOTED-PRINTABLE:=DDnce;=DE=FCkr=FC\r\nNOTE:\x93Al\xfdnt\xfd\x94 \x96 \xfeirket\r\nEND:VCARD\r\n";
    let html = to_html("rehber.vcf", vcf).unwrap().html;
    assert!(html.contains("Şükrü İnce") && html.contains("“Alıntı” – şirket"), "{html}");
    let ics = b"BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:\xddzmir toplant\xfds\xfd \x96 \x93Ar\xfeiv\x94\r\nDTSTART:20261110T130000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let html = to_html("davet.ics", ics).unwrap().html;
    assert!(html.contains("İzmir toplantısı – “Arşiv”"), "{html}");
}

/// An Outlook .msg with 8-bit (001E) strings in code page `cp`, an HTML body in internet code
/// page `internet_cp` and one attachment with an 8-bit name.
fn msg_file(cp: Option<u32>, internet_cp: Option<u32>, html: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    file.create_storage("/__attach_version1.0_#00000000").unwrap();
    let mut put = |path: &str, data: &[u8]| file.create_stream(path).unwrap().write_all(data).unwrap();
    let mut props = vec![0u8; 32];
    for (id, value) in [(0x3FFDu32, cp), (0x3FDE, internet_cp)] {
        if let Some(v) = value {
            props.extend(((id << 16) | 3).to_le_bytes());
            props.extend(6u32.to_le_bytes());
            props.extend(v.to_le_bytes());
            props.extend(0u32.to_le_bytes());
        }
    }
    put("/__properties_version1.0", &props);
    put("/__substg1.0_0037001E", b"\xdd\xfe toplant\xfds\xfd\0");
    put("/__substg1.0_0C1A001E", b"Ay\xfee Kaya");
    put("/__substg1.0_10130102", html);
    put("/__attach_version1.0_#00000000/__substg1.0_37010102", b"veri");
    put("/__attach_version1.0_#00000000/__substg1.0_3707001E", b"\xdezmir \xfeube.pdf");
    file.into_inner().into_inner()
}

#[test]
fn outlook_messages_read_their_code_pages() {
    let mut page = b"<html><body><p>".to_vec();
    page.extend_from_slice(W1254);
    page.extend_from_slice(b"</p></body></html>");
    for (cp, icp) in [(Some(1254), Some(1254)), (Some(1254), Some(28599)), (None, None), (Some(1252), None)] {
        let c = to_html("ileti.msg", &msg_file(cp, icp, &page)).unwrap();
        assert!(c.html.contains("<h1>İş toplantısı</h1>") && c.html.contains("Ayşe Kaya"), "{cp:?}: {}", c.html);
        assert!(c.html.contains(TURKISH), "{cp:?}/{icp:?}: {}", c.html);
        assert_eq!(c.attachments[0].name, "Şzmir şube.pdf");
    }
    // A UTF-8 HTML body (internet code page 65001) stays UTF-8.
    let c = to_html("ileti.msg", &msg_file(Some(1254), Some(65001), format!("<p>{TURKISH}</p>").as_bytes())).unwrap();
    assert!(c.html.contains(TURKISH), "{}", c.html);
}

#[test]
fn unknown_files_are_not_taken() {
    assert_eq!(kind_of("a.txt", b"hello"), None);
    assert_eq!(kind_of("x", b"BEGIN:VCARD\n"), Some(Kind::VCard));
    assert!(to_html("a.txt", b"hello").is_err());
}
