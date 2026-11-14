//! Turkish letters in files that store text in a code page (docs/TURKCE-METIN-DENETIMI.md), and
//! fonts for them with only the fonts the app carries.

use super::*;

/// "İıŞşĞğÇçÖöÜü" in Windows-1254 (and ISO-8859-9).
const W1254: &[u8] = b"\xdd\xfd\xde\xfe\xd0\xf0\xc7\xe7\xd6\xf6\xdc\xfc";
const TURKISH: &str = "İıŞşĞğÇçÖöÜü";

fn words(name: &str, bytes: &[u8]) -> String {
    WordDoc::open(name, bytes, None).unwrap().page_texts().join("\n")
}

/// The Windows-1254 byte of a Turkish letter, an ASCII character or a punctuation mark the tests use.
fn w1254_of(c: char) -> u8 {
    match c {
        '“' => 0x93,
        '”' => 0x94,
        '–' => 0x96,
        c if c.is_ascii() => c as u8,
        c => W1254[TURKISH.chars().position(|t| t == c).expect("a Turkish letter")],
    }
}

#[test]
fn rtf_turkish_code_pages() {
    // \ansicpg1254, a Turkish font charset (\fcharset162) with or without \f, raw 8-bit bytes,
    // and \uN with its fallback (\uc1, \uc0, \uc2).
    for rtf in [
        &br"{\rtf1\ansi\ansicpg1254\deff0{\fonttbl{\f0 Arial;}}\f0 \'dd\'fd\'de\'fe\'d0\'f0\'c7\'e7\'d6\'f6\'dc\'fc\par}"[..],
        br"{\rtf1\ansi\deff0{\fonttbl{\f0\fcharset162 Arial Tur;}}\f0 \'dd\'fd\'de\'fe\'d0\'f0\'c7\'e7\'d6\'f6\'dc\'fc\par}",
        br"{\rtf1\ansi\deff0{\fonttbl{\f0\fcharset162 Arial Tur;}}\'dd\'fd\'de\'fe\'d0\'f0\'c7\'e7\'d6\'f6\'dc\'fc\par}",
        br"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0 Arial;}{\f1\fcharset162 Times New Roman Tur;}}\f1 \'dd\'fd\'de\'fe\'d0\'f0\'c7\'e7\'d6\'f6\'dc\'fc\par}",
        b"{\\rtf1\\ansi\\ansicpg1254 \xdd\xfd\xde\xfe\xd0\xf0\xc7\xe7\xd6\xf6\xdc\xfc\\par}",
        br"{\rtf1\ansi\ansicpg1254\uc1 \u304?\u305?\u350?\u351?\u286?\u287?\u199?\u231?\u214?\u246?\u220?\u252?\par}",
        br"{\rtf1\ansi\uc0\u304 \u305 \u350 \u351 \u286 \u287 \u199 \u231 \u214 \u246 \u220 \u252 \par}",
        br"{\rtf1\ansi\ansicpg1254\uc2\u304\'dd\'dd\u305\'fd\'fd\u350\'de\'de\u351\'fe\'fe\u286\'d0\'d0\u287\'f0\'f0\u199??\u231??\u214??\u246??\u220??\u252??\par}",
        // No \ansicpg, but the document says it is Turkish: read as Windows-1254, as Word does on
        // a Turkish system.
        br"{\rtf1\ansi\deff0\deflang1055{\fonttbl{\f0 Arial;}}\f0 \'dd\'fd\'de\'fe\'d0\'f0\'c7\'e7\'d6\'f6\'dc\'fc\par}",
        br"{\rtf1\ansi{\fonttbl{\f0 Arial;}}\pard\lang1055\f0 \'dd\'fd\'de\'fe\'d0\'f0\'c7\'e7\'d6\'f6\'dc\'fc\par}",
    ] {
        assert_eq!(words("a.rtf", rtf), TURKISH, "{}", String::from_utf8_lossy(rtf));
    }
    // \uN past 32767 is written negative.
    assert_eq!(words("a.rtf", br"{\rtf1\ansi\uc1 \u-497?\par}"), "\u{FE0F}");
    // A Western RTF without \ansicpg stays Windows-1252 (Icelandic þ is not ş there).
    assert_eq!(words("a.rtf", br"{\rtf1\ansi\deflang1039 \'fe\'f0\par}"), "þð");
    assert_eq!(words("a.rtf", br"{\rtf1\ansi\deflang10550 \'fe\par}"), "þ");
}

#[test]
fn plain_text_and_markdown_in_windows_1254() {
    // Short Turkish text was read as Chinese (GBK) and longer text as Windows-1252 (Þ for Ş).
    let page = TextPage { width_pt: 595.0, height_pt: 842.0, landscape: false, margin_pt: 72.0, font: "Calibri".into(), size_pt: 11.0 };
    for text in [TURKISH, "Şişli şık", "Üniversite öğrencileri İstanbul'da yaşıyor. “Alıntı” – tire", "Çöü"] {
        let bytes: Vec<u8> = text.chars().map(w1254_of).collect();
        assert_eq!(words("not.txt", &bytes), text);
        assert_eq!(WordDoc::from_text(&bytes, &page).page_texts().join("\n"), text);
    }
    assert_eq!(words("not.md", b"# Ba\xfel\xfdk \xdd\xfe\n"), "Başlık İş");
    // UTF-16 without a byte order mark, and UTF-8, are left to the engine.
    let utf16: Vec<u8> = TURKISH.encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert_eq!(words("not.txt", &utf16), TURKISH);
    assert_eq!(words("not.txt", TURKISH.as_bytes()), TURKISH);
    // Cyrillic keeps its own code page.
    assert_eq!(words("not.txt", b"\xcf\xf0\xe8\xe2\xe5\xf2, \xea\xe0\xea \xe4\xe5\xeb\xe0? \xd5\xee\xf0\xee\xf8\xee."), "Привет, как дела? Хорошо.");
}

#[test]
fn web_pages_in_turkish_code_pages() {
    let page = |head: &[u8]| -> Vec<u8> { [b"<html><head>".as_slice(), head, b"</head><body><p>", W1254, b"</p></body></html>"].concat() };
    assert_eq!(words("a.html", &page(b"<meta charset=\"windows-1254\">")), TURKISH);
    assert_eq!(words("a.htm", &page(b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=iso-8859-9\">")), TURKISH);
    // No charset at all: Turkish first, not Chinese.
    assert_eq!(words("a.html", &page(b"")), TURKISH);
    // HTML5's names for the Turkish letters outside Latin-1.
    assert_eq!(words("a.html", b"<p>&Idot;&imath;&inodot;&Scedil;&scedil;&Gbreve;&gbreve;&Ccedil;&ouml;</p>"), "İııŞşĞğÇö");
    let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain("<p>İıŞş</p>".encode_utf16().flat_map(u16::to_le_bytes)).collect();
    assert_eq!(words("a.html", &utf16), "İıŞş");
}

/// A Word 97 document whose text is `pieces`: (compressed, bytes); compressed pieces are 8-bit,
/// the others UTF-16LE ([MS-DOC] 2.4.1; the same minimal FIB as WordCraft's own tests build).
fn word97(pieces: &[(bool, Vec<u8>)]) -> Vec<u8> {
    use std::io::Write;
    const TEXT_FC: u32 = 0x600;
    const CLX_AT: u32 = 0x40;
    let chars = |p: &(bool, Vec<u8>)| if p.0 { p.1.len() } else { p.1.len() / 2 } as u32;
    let mut word = vec![0u8; TEXT_FC as usize];
    let mut plc = 0u32.to_le_bytes().to_vec();
    let mut cp = 0;
    for p in pieces {
        cp += chars(p);
        plc.extend(cp.to_le_bytes());
    }
    let mut off = 0;
    for p in pieces {
        let fc = if p.0 { (2 * (TEXT_FC + off)) | 0x4000_0000 } else { TEXT_FC + off };
        plc.extend([0, 0]);
        plc.extend(fc.to_le_bytes());
        plc.extend([0, 0]);
        off += p.1.len() as u32;
        word.extend_from_slice(&p.1);
    }
    let mut table = vec![0u8; CLX_AT as usize];
    table.push(0x02);
    table.extend((plc.len() as u32).to_le_bytes());
    table.extend(&plc);
    let mut fib = vec![0u8; 0x9A + 0x5D * 8 + 2];
    let mut put = |at: usize, v: &[u8]| fib[at..at + v.len()].copy_from_slice(v);
    put(0x00, &0xA5ECu16.to_le_bytes());
    put(0x02, &0x00C1u16.to_le_bytes());
    put(0x0A, &0x1000u16.to_le_bytes());
    put(0x0C, &0x00BFu16.to_le_bytes());
    put(0x20, &0x000Eu16.to_le_bytes());
    put(0x3E, &0x0016u16.to_le_bytes());
    put(0x40, &0x1000u32.to_le_bytes());
    put(0x4C, &cp.to_le_bytes());
    put(0x98, &0x005Du16.to_le_bytes());
    put(0x9A + 33 * 8, &CLX_AT.to_le_bytes());
    put(0x9A + 33 * 8 + 4, &(plc.len() as u32 + 5).to_le_bytes());
    word[..fib.len()].copy_from_slice(&fib);
    let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    file.create_stream("/WordDocument").unwrap().write_all(&word).unwrap();
    file.create_stream("/0Table").unwrap().write_all(&table).unwrap();
    file.into_inner().into_inner()
}

#[test]
fn word_97_documents_keep_turkish_letters() {
    // Word keeps a run 8-bit (Windows-1252) only when every letter fits; ğ ı ş İ Ş Ğ never do,
    // so they come in UTF-16 pieces, while Ç ö ü and curly quotes may come 8-bit.
    let unicode: Vec<u8> = "İıŞşĞğ ".encode_utf16().flat_map(u16::to_le_bytes).collect();
    let doc = word97(&[(false, unicode), (true, b"\xc7\xe7\xd6\xf6\xdc\xfc \x93x\x94 \x96\r".to_vec())]);
    assert_eq!(words("eski.doc", &doc), "İıŞşĞğ ÇçÖöÜü “x” –");
}

/// Families Turkish documents ask for that the phone has no file for. Only the fonts compiled
/// into the app are available (no font folders, as on Android, where WordCraft finds none).
const MISSING_FAMILIES: [&str; 6] = ["Calibri", "Times New Roman", "Arial", "Cambria", "Tahoma", "Comic Sans MS"];

#[test]
fn missing_fonts_fall_back_to_a_face_with_turkish_letters() {
    let db = wordcraft_fonts::FontDb::with_font_dirs(Vec::new());
    for family in MISSING_FAMILIES {
        for style in ["Regular", "Bold", "Italic"] {
            let face = db.face(family, style);
            for c in TURKISH.chars() {
                assert_ne!(face.glyph_for(c), 0, "{family} {style} → {} has no {c}", face.family);
            }
        }
    }
}
