//! Turkish letters in spreadsheets that store text in a code page or in UTF-16
//! (docs/TURKCE-METIN-DENETIMI.md).

use super::*;

const TURKISH: &str = "İıŞşĞğÇçÖöÜü";

fn cells(name: &str, bytes: &[u8]) -> Vec<String> {
    let doc = GridDoc::open(name, bytes, None).unwrap();
    let size = doc.used_size(0).unwrap();
    doc.cell_texts(0, 0, 0, size.rows, size.cols).unwrap()
}

#[test]
fn delimited_text_in_turkish_encodings() {
    let w1254 = |s: &str| encoding_rs::WINDOWS_1254.encode(s).0.into_owned();
    // Windows-1254 (ISO-8859-9 has the same letter bytes), quotes and dashes included.
    assert_eq!(cells("a.csv", &w1254("İl;“Şişli” – ğüşıöç\n")), ["İl", "“Şişli” – ğüşıöç"]);
    // Only the letters Windows-1252 shares (Ç ç Ö ö Ü ü): the engine's own reading is right.
    assert_eq!(cells("a.csv", b"\xc7i\xe7ek;\xd6rnek;\xdcz\xfcm\n"), ["Çiçek", "Örnek", "Üzüm"]);
    // UTF-16 with a byte order mark (Excel's "Unicode text") and without one.
    let text = format!("İl\t{TURKISH}\n");
    let le: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let be: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert_eq!(cells("a.txt", &[&[0xFF, 0xFE][..], &le].concat()), ["İl", TURKISH]);
    assert_eq!(cells("a.txt", &le), ["İl", TURKISH]);
    assert_eq!(cells("a.tsv", &be), ["İl", TURKISH]);
    // UTF-8 with and without a byte order mark.
    assert_eq!(cells("a.csv", format!("\u{FEFF}{TURKISH};x\n").as_bytes()), [TURKISH, "x"]);
}

/// A record of an Excel BIFF8 stream.
fn record(kind: u16, body: &[u8]) -> Vec<u8> {
    [&kind.to_le_bytes()[..], &(body.len() as u16).to_le_bytes(), body].concat()
}

/// Characters as BIFF8 stores them: compressed (one byte, U+0000–U+00FF) when every one fits,
/// else UTF-16 ([MS-XLS] 2.5.293); returns the flags byte and the bytes.
fn biff_chars(s: &str) -> (u8, Vec<u8>) {
    if s.chars().all(|c| (c as u32) < 0x100) {
        (0, s.chars().map(|c| c as u8).collect())
    } else {
        (1, s.encode_utf16().flat_map(u16::to_le_bytes).collect())
    }
}

/// A minimal Excel 97–2003 workbook: CODEPAGE 1254 (Turkish Excel writes it), one sheet called
/// `sheet` with `texts` in A1, B1, … through the shared string table.
fn biff8(sheet: &str, texts: &[&str]) -> Vec<u8> {
    use std::io::Write;
    let bof =
        |dt: u16| record(0x0809, &[&0x0600u16.to_le_bytes()[..], &dt.to_le_bytes(), &[0xBB, 0x0D, 0xCC, 0x07], &[0; 4], &[6, 0, 0, 0]].concat());
    let (flags, name) = biff_chars(sheet);
    let mut sst = [(texts.len() as u32).to_le_bytes(), (texts.len() as u32).to_le_bytes()].concat();
    for t in texts {
        let (f, b) = biff_chars(t);
        sst.extend((t.encode_utf16().count() as u16).to_le_bytes());
        sst.push(f);
        sst.extend(b);
    }
    let boundsheet_len = 4 + 2 + 2 + name.len() + 4;
    let globals_len = bof(5).len() + record(0x0042, &[0; 2]).len() + boundsheet_len + record(0x00FC, &sst).len() + 4;
    let mut stream = bof(5);
    stream.extend(record(0x0042, &1254u16.to_le_bytes()));
    let bs = [&(globals_len as u32).to_le_bytes()[..], &[0, 0], &[sheet.encode_utf16().count() as u8, flags], &name].concat();
    stream.extend(record(0x0085, &bs));
    stream.extend(record(0x00FC, &sst));
    stream.extend(record(0x000A, &[]));
    assert_eq!(stream.len(), globals_len);
    stream.extend(bof(0x10));
    let dims = [&0u32.to_le_bytes()[..], &1u32.to_le_bytes(), &0u16.to_le_bytes(), &(texts.len() as u16).to_le_bytes(), &[0, 0]].concat();
    stream.extend(record(0x0200, &dims));
    for i in 0..texts.len() {
        let cell = [&0u16.to_le_bytes()[..], &(i as u16).to_le_bytes(), &15u16.to_le_bytes(), &(i as u32).to_le_bytes()].concat();
        stream.extend(record(0x00FD, &cell));
    }
    stream.extend(record(0x000A, &[]));
    let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    file.create_stream("/Workbook").unwrap().write_all(&stream).unwrap();
    file.into_inner().into_inner()
}

#[test]
fn excel_97_workbooks_with_turkish_code_page() {
    // Excel stores "Çöü" one byte each (Latin-1) and anything with ğ ı ş İ Ş Ğ in UTF-16, whatever
    // the workbook's CODEPAGE; both read back as written.
    let xls = biff8("Şube Özeti", &[TURKISH, "Çiçek Ölçü", "İstanbul"]);
    let doc = GridDoc::open("rapor.xls", &xls, None).unwrap();
    assert_eq!(doc.sheet_names(), ["Şube Özeti"]);
    assert_eq!(cells("rapor.xls", &xls), [TURKISH, "Çiçek Ölçü", "İstanbul"]);
}

#[test]
fn finding_merges_i_forms_and_accents_but_replacing_keeps_turkish_case() {
    let mut doc = GridDoc::blank();
    doc.set_turkish(true);
    for (r, t) in ["İSTANBUL", "Istanbul", "istanbul", "Çağrı Merkezi", "ılık su", "ilik"].iter().enumerate() {
        doc.set_input(r as u32, 0, t).unwrap();
    }
    let rows = |doc: &GridDoc, q: &str| doc.find_cells(q).unwrap().iter().map(|h| h.row).collect::<Vec<_>>();
    // As the document, PDF and file searches do: typed with or without the Turkish keyboard.
    for q in ["istanbul", "ISTANBUL", "İstanbul", "ıstanbul"] {
        assert_eq!(rows(&doc, q), [0, 1, 2], "{q}");
    }
    assert_eq!(rows(&doc, "cagri"), [3]);
    assert_eq!(rows(&doc, "ÇAĞRI"), [3]);
    // Replacing changes data: "ılık" (lukewarm) is not "ilik" (marrow), and İ/i, I/ı pair up.
    assert_eq!(doc.replace_all("ılık", "sıcak").unwrap(), 1);
    assert_eq!(doc.edit_text(0, 4, 0).unwrap(), "sıcak su");
    assert_eq!(doc.edit_text(0, 5, 0).unwrap(), "ilik");
    assert_eq!(doc.replace_all("istanbul", "Ankara").unwrap(), 2, "İSTANBUL and istanbul, not Istanbul");
}
