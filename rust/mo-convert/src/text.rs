//! Text whose encoding is not declared (a .txt, an HTML page without `<meta charset>`, an
//! e-mail part without `charset=`, an Outlook 8-bit string), decoded the way a Turkish reader
//! expects.
//!
//! The engines guess legacy encodings with chardetng, the detector Firefox uses, without a hint
//! of where the file comes from. For short Turkish Windows-1254 text that guess goes wrong:
//! "İıŞşĞğ…" comes out as Chinese (GBK), and "Şişli" as "Þiþli" (Windows-1252). Here the same
//! detector is asked with Turkey as the origin (`.tr`), and Turkish letters win over the
//! Western reading and over a multi-byte one that only a handful of bytes suggested. Text that
//! really is in another script (Cyrillic, Greek, Chinese…) keeps the detector's answer.

use encoding_rs::Encoding;

/// The bytes Windows-1254 (and ISO-8859-9) use for the twelve Turkish letters
/// Ç ç Ö ö Ü ü Ğ ğ İ ı Ş ş.
const TURKISH_LETTERS: [u8; 12] = [0xC7, 0xE7, 0xD6, 0xF6, 0xDC, 0xFC, 0xD0, 0xF0, 0xDD, 0xFD, 0xDE, 0xFE];

/// The six of them that Windows-1252 reads as Icelandic letters (Ð ð Ý ý Þ þ).
const TURKISH_ONLY: [u8; 6] = [0xD0, 0xF0, 0xDD, 0xFD, 0xDE, 0xFE];

/// Most bytes looked at to guess.
const SNIFF: usize = 1 << 20;

/// Whether `bytes` starts with a UTF-8 or UTF-16 byte order mark.
pub fn has_bom(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xEF, 0xBB, 0xBF]) || bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF])
}

/// UTF-16 without a byte order mark, told by its zero bytes: `Some(true)` little-endian,
/// `Some(false)` big-endian (Latin text in UTF-16 has a zero in every other byte).
pub fn unmarked_utf16(bytes: &[u8]) -> Option<bool> {
    if bytes.len() < 4 || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let sample = &bytes[..bytes.len().min(4096) & !1];
    let even = sample.iter().step_by(2).filter(|b| **b == 0).count();
    let odd = sample.iter().skip(1).step_by(2).filter(|b| **b == 0).count();
    let half = sample.len() / 2;
    if odd * 10 >= half * 4 && even * 10 < half {
        Some(true)
    } else if even * 10 >= half * 4 && odd * 10 < half {
        Some(false)
    } else {
        None
    }
}

/// The encoding legacy 8-bit `bytes` are in, guessed for a Turkish reader (see the module notes).
pub fn guess_legacy(bytes: &[u8]) -> &'static Encoding {
    let sample = bytes.get(..SNIFF).unwrap_or(bytes);
    let mut det = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    det.feed(sample, sample.len() == bytes.len());
    let guess = det.guess(Some(b"tr"), chardetng::Utf8Detection::Deny);
    if guess == encoding_rs::WINDOWS_1254 {
        return guess;
    }
    let high: Vec<u8> = sample.iter().copied().filter(|b| *b >= 0x80).collect();
    let turkish = high.iter().filter(|b| TURKISH_LETTERS.contains(b)).count();
    if guess == encoding_rs::WINDOWS_1252 {
        // The same as Windows-1254 but for Turkish letters, which then decide.
        return if sample.iter().any(|b| TURKISH_ONLY.contains(b)) { encoding_rs::WINDOWS_1254 } else { guess };
    }
    // A multi-byte guess from a few bytes that are mostly Turkish letters: real Chinese, Japanese
    // or Korean text spreads its bytes over about a hundred values, of which these are twelve.
    let multibyte = [encoding_rs::GBK, encoding_rs::GB18030, encoding_rs::BIG5, encoding_rs::SHIFT_JIS, encoding_rs::EUC_JP, encoding_rs::EUC_KR];
    if multibyte.contains(&guess) && !high.is_empty() && turkish * 2 >= high.len() {
        return encoding_rs::WINDOWS_1254;
    }
    guess
}

/// The text of `bytes` when they are in a legacy 8-bit or multi-byte encoding, decoded as
/// [`guess_legacy`] guesses; `None` for UTF-8 and UTF-16 (with or without a byte order mark),
/// which the engines read right themselves.
pub fn decode_undeclared(bytes: &[u8]) -> Option<String> {
    if has_bom(bytes) || std::str::from_utf8(bytes).is_ok() || unmarked_utf16(bytes).is_some() {
        return None;
    }
    Some(guess_legacy(bytes).decode_without_bom_handling(bytes).0.into_owned())
}

/// Text in any of the forms a plain text file comes in: UTF-8 or UTF-16 (byte order mark or
/// not), else the legacy encoding [`guess_legacy`] picks. Never fails.
pub fn decode_any(bytes: &[u8]) -> String {
    if let Some((enc, bom)) = Encoding::for_bom(bytes) {
        return enc.decode_without_bom_handling(&bytes[bom..]).0.into_owned();
    }
    // Before UTF-8: UTF-16 Latin text ("İ" is 30 01) can be valid UTF-8 too, full of zeros.
    if let Some(le) = unmarked_utf16(bytes) {
        let enc = if le { encoding_rs::UTF_16LE } else { encoding_rs::UTF_16BE };
        return enc.decode_without_bom_handling(bytes).0.into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    guess_legacy(bytes).decode_without_bom_handling(bytes).0.into_owned()
}

/// `bytes` in the charset called `label` (a MIME or HTML name: "iso-8859-9", "windows-1254",
/// "latin5", "utf-8"…); `None` when the name is unknown. ISO-8859-9 is read as Windows-1254,
/// as browsers do: Turkish Windows programs label their text so and use its quotes and dashes.
pub fn decode_label(bytes: &[u8], label: &str) -> Option<String> {
    let enc = Encoding::for_label(label.trim().trim_matches(['"', '\'']).as_bytes())?;
    Some(enc.decode_without_bom_handling(bytes).0.into_owned())
}

/// `bytes` in the encoding a file declares, `enc`, except text declared Western (Windows-1252,
/// ISO-8859-1) that holds Turkish letters: many Turkish files carry the Western label of an
/// English template or system, and Windows-1252 reads İ ı Ş ş Ğ ğ as Ý ý Þ þ Ð ð. Valid UTF-8
/// stays UTF-8 whatever the label says.
pub fn decode_declared(bytes: &[u8], enc: &'static Encoding) -> String {
    if enc != encoding_rs::UTF_16LE
        && enc != encoding_rs::UTF_16BE
        && let Ok(s) = std::str::from_utf8(bytes)
    {
        return s.to_string();
    }
    let enc = if enc == encoding_rs::WINDOWS_1252 && guess_legacy(bytes) == encoding_rs::WINDOWS_1254 { encoding_rs::WINDOWS_1254 } else { enc };
    enc.decode_without_bom_handling(bytes).0.into_owned()
}

/// The charset an HTML page declares in its first bytes (`<meta charset="…">` or
/// `<meta http-equiv="Content-Type" content="…; charset=…">`).
pub fn html_meta_charset(bytes: &[u8]) -> Option<&'static Encoding> {
    let head = &bytes[..bytes.len().min(4096)];
    let lower: Vec<u8> = head.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = find(&lower[from..], b"<meta") {
        let at = from + i;
        let end = lower[at..].iter().position(|b| *b == b'>').map_or(lower.len(), |j| at + j);
        let tag = &lower[at..end];
        if let Some(c) = find(tag, b"charset=") {
            let value = &tag[c + 8..];
            let value: Vec<u8> = value
                .iter()
                .copied()
                .skip_while(|b| matches!(b, b'"' | b'\'' | b' '))
                .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':' | b'.'))
                .collect();
            if let Some(enc) = Encoding::for_label(&value) {
                return Some(enc);
            }
        }
        from = end;
    }
    None
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w1254(s: &str) -> Vec<u8> {
        encoding_rs::WINDOWS_1254.encode(s).0.into_owned()
    }

    #[test]
    fn short_and_long_turkish_text_reads_as_windows_1254() {
        for s in ["İıŞşĞğÇçÖöÜü", "Şişli şık", "İstanbul", "Çöü", "Üniversite öğrencileri İstanbul'da yaşıyor; “alıntı” – tire"]
        {
            assert_eq!(decode_any(&w1254(s)), s, "{s}");
        }
        assert_eq!(decode_undeclared(&w1254("İıŞşĞğÇçÖöÜü")).as_deref(), Some("İıŞşĞğÇçÖöÜü"));
    }

    #[test]
    fn other_scripts_and_unicode_are_left_alone() {
        // UTF-8, UTF-16 with and without a byte order mark: nothing to do here.
        assert_eq!(decode_undeclared("İıŞş".as_bytes()), None);
        let utf16: Vec<u8> = "İıŞş merhaba".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode_undeclared(&utf16), None);
        assert_eq!(decode_any(&utf16), "İıŞş merhaba");
        let be: Vec<u8> = [0xFE, 0xFF].into_iter().chain("Ğğ".encode_utf16().flat_map(u16::to_be_bytes)).collect();
        assert_eq!(decode_any(&be), "Ğğ");
        // Western text keeps its letters (Windows-1252 and -1254 agree on them).
        assert_eq!(decode_any(b"Le caf\xe9 est tr\xe8s bon, gar\xe7on"), "Le café est très bon, garçon");
        // Real Cyrillic and Chinese text keeps the detector's answer.
        let ru = encoding_rs::WINDOWS_1251.encode("Привет, как дела? Хорошо, спасибо.").0.into_owned();
        assert_eq!(decode_any(&ru), "Привет, как дела? Хорошо, спасибо.");
        let zh = encoding_rs::GBK.encode("中文测试，你好世界，这是一个比较长的句子。").0.into_owned();
        assert_eq!(decode_undeclared(&zh).as_deref(), Some("中文测试，你好世界，这是一个比较长的句子。"));
    }

    #[test]
    fn labels_and_meta_charsets() {
        // ISO-8859-9 as browsers read it: Windows-1254, quotes and dashes included.
        assert_eq!(decode_label(b"\x93\xdd\xfe\x94 \x96", "ISO-8859-9").as_deref(), Some("“İş” –"));
        assert_eq!(decode_label(b"\xdd", "latin5").as_deref(), Some("İ"));
        assert_eq!(decode_label(b"x", "no-such-charset"), None);
        assert_eq!(html_meta_charset(b"<html><head><meta charset=\"windows-1254\">"), Some(encoding_rs::WINDOWS_1254));
        assert_eq!(
            html_meta_charset(b"<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=iso-8859-9\">"),
            Some(encoding_rs::WINDOWS_1254)
        );
        assert_eq!(html_meta_charset(b"<meta name=\"x\" content=\"y\"><p>charset=utf-8</p>"), None);
    }
}
