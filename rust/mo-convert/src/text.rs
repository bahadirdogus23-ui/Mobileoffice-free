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

/// The encoding a plain text file is in and the length of its byte order mark: the mark if
/// there is one, else UTF-16 told by its zero bytes, else UTF-8 if valid, else the legacy
/// encoding [`guess_legacy`] picks.
fn detect(bytes: &[u8]) -> (&'static Encoding, usize) {
    if let Some(found) = Encoding::for_bom(bytes) {
        return found;
    }
    // Before UTF-8: UTF-16 Latin text ("İ" is 30 01) can be valid UTF-8 too, full of zeros.
    if let Some(le) = unmarked_utf16(bytes) {
        return (if le { encoding_rs::UTF_16LE } else { encoding_rs::UTF_16BE }, 0);
    }
    if std::str::from_utf8(bytes).is_ok() {
        return (encoding_rs::UTF_8, 0);
    }
    (guess_legacy(bytes), 0)
}

/// Text in any of the forms a plain text file comes in: UTF-8 or UTF-16 (byte order mark or
/// not), else the legacy encoding [`guess_legacy`] picks. Never fails.
pub fn decode_any(bytes: &[u8]) -> String {
    let (enc, bom) = detect(bytes);
    enc.decode_without_bom_handling(&bytes[bom..]).0.into_owned()
}

/// A text file read for showing and editing, with what writing it back the same way needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    /// The encoding it was in (UTF-8, UTF-16LE/BE, windows-1254, windows-1251…).
    pub encoding: &'static Encoding,
    /// It started with a byte order mark (UTF-8 or UTF-16).
    pub bom: bool,
    /// [`encode_text`] of `text` in `encoding` gives the bytes back exactly: nothing was
    /// replaced with "�" while decoding, and the encoding writes each letter one way only.
    /// When false, saving over the file would change bytes the person never touched.
    pub exact: bool,
}

/// A text file's bytes decoded as [`decode_any`] does, with the encoding and byte order mark
/// to save it back in. `complete` is false for the first part of a file too large to read whole:
/// a letter (or UTF-16 code unit) cut in two at the end is then dropped instead of being
/// decoded as "�" or making UTF-8 look invalid.
pub fn decode_text(bytes: &[u8], complete: bool) -> Decoded {
    let bytes = if complete { bytes } else { whole_letters(bytes) };
    let (encoding, bom_len) = detect(bytes);
    let (text, had_errors) = encoding.decode_without_bom_handling(&bytes[bom_len..]);
    let text = text.into_owned();
    let bom = bom_len > 0;
    // UTF-8 and UTF-16 decoded without errors always come back the same.
    let unicode = [encoding_rs::UTF_8, encoding_rs::UTF_16LE, encoding_rs::UTF_16BE].contains(&encoding);
    let exact = !had_errors && (unicode || encode_text(&text, encoding, bom).is_ok_and(|b| b == bytes));
    Decoded { text, encoding, bom, exact }
}

/// `bytes` (the start of a longer file) up to the last whole letter: an even number of bytes
/// and no lone high surrogate at the end for UTF-16, no cut sequence at the end for UTF-8.
fn whole_letters(bytes: &[u8]) -> &[u8] {
    let even = &bytes[..bytes.len() & !1];
    let utf16 = match Encoding::for_bom(bytes) {
        Some((enc, _)) if enc == encoding_rs::UTF_8 => None,
        Some((enc, _)) => Some(enc == encoding_rs::UTF_16LE),
        None => unmarked_utf16(even),
    };
    if let Some(le) = utf16 {
        let mut end = even.len();
        if end >= 2 {
            let pair = [even[end - 2], even[end - 1]];
            let unit = if le { u16::from_le_bytes(pair) } else { u16::from_be_bytes(pair) };
            if (0xD800..0xDC00).contains(&unit) {
                end -= 2;
            }
        }
        return &bytes[..end];
    }
    match std::str::from_utf8(bytes) {
        Err(e) if e.error_len().is_none() => &bytes[..e.valid_up_to()],
        _ => bytes,
    }
}

/// The encoding a [`Decoded::encoding`] name (or any WHATWG label) stands for, if it can be
/// written: the "replacement" encoding only ever decodes.
pub fn encoding_named(name: &str) -> Option<&'static Encoding> {
    Encoding::for_label(name.trim().as_bytes()).filter(|e| *e != encoding_rs::REPLACEMENT)
}

/// `text` in `encoding`, after a byte order mark when `bom` and the encoding has one (UTF-8,
/// UTF-16). A letter the encoding has no code for (an emoji or "€" in ISO-8859-9… "ğ" in
/// Windows-1252) is `Err` with that letter, never a silent "?" or "&#287;".
pub fn encode_text(text: &str, encoding: &'static Encoding, bom: bool) -> Result<Vec<u8>, char> {
    let utf16 = |be: bool| -> Vec<u8> {
        let mark: &[u8] = match (bom, be) {
            (false, _) => &[],
            (true, false) => &[0xFF, 0xFE],
            (true, true) => &[0xFE, 0xFF],
        };
        let units = text.encode_utf16().flat_map(|u| if be { u.to_be_bytes() } else { u.to_le_bytes() });
        mark.iter().copied().chain(units).collect()
    };
    // encoding_rs writes UTF-16 as UTF-8 (the WHATWG rule for forms), so it is done here.
    if encoding == encoding_rs::UTF_16LE {
        return Ok(utf16(false));
    }
    if encoding == encoding_rs::UTF_16BE {
        return Ok(utf16(true));
    }
    if encoding == encoding_rs::UTF_8 {
        let mark: &[u8] = if bom { &[0xEF, 0xBB, 0xBF] } else { &[] };
        return Ok([mark, text.as_bytes()].concat());
    }
    let mut encoder = encoding.new_encoder();
    let mut out = Vec::with_capacity(text.len() + 16);
    let mut rest = text;
    loop {
        let (result, read) = encoder.encode_from_utf8_to_vec_without_replacement(rest, &mut out, true);
        rest = &rest[read..];
        match result {
            encoding_rs::EncoderResult::InputEmpty => return Ok(out),
            encoding_rs::EncoderResult::OutputFull => out.reserve(rest.len() + 16),
            encoding_rs::EncoderResult::Unmappable(c) => return Err(c),
        }
    }
}

/// `bytes` in the charset called `label` (a MIME or HTML name: "iso-8859-9", "windows-1254",
/// "latin5", "utf-8", "ibm857"…); `None` when the name is unknown. ISO-8859-9 is read as Windows-1254,
/// as browsers do: Turkish Windows programs label their text so and use its quotes and dashes.
pub fn decode_label(bytes: &[u8], label: &str) -> Option<String> {
    let label = label.trim().trim_matches(['"', '\'']);
    if is_cp857(label) {
        return Some(decode_cp857(bytes));
    }
    let enc = Encoding::for_label(label.as_bytes())?;
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

/// What CP857 (IBM857, the Turkish DOS code page) has at 0x80..=0xFF. 0xD5, 0xE7 and 0xF2 are
/// unassigned ("�"), as in IBM's table; the same table the app uses for ZIP names.
const CP857_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ı', 'Ä', 'Å', //
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'İ', 'Ö', 'Ü', 'ø', '£', 'Ø', 'Ş', 'ş', //
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'Ğ', 'ğ', '¿', '®', '¬', '½', '¼', '¡', '«', '»', //
    '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©', '╣', '║', '╗', '╝', '¢', '¥', '┐', //
    '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦', '╠', '═', '╬', '¤', //
    'º', 'ª', 'Ê', 'Ë', 'È', '\u{FFFD}', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì', '▀', //
    'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', '\u{FFFD}', '×', 'Ú', 'Û', 'Ù', 'ì', 'ÿ', '¯', '´', //
    '\u{AD}', '±', '\u{FFFD}', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', '\u{A0}', //
];

/// Whether `label` names CP857, which encoding_rs (WHATWG) does not know: "ibm857", "cp857",
/// "857", "csIBM857", "ibm-857"…
pub fn is_cp857(label: &str) -> bool {
    let l = label.trim().trim_matches(['"', '\'']).to_ascii_lowercase();
    matches!(l.as_str(), "ibm857" | "ibm-857" | "cp857" | "cp-857" | "857" | "csibm857" | "x-ibm857")
}

/// `bytes` in CP857 (ASCII below 0x80).
pub fn decode_cp857(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| if b < 0x80 { char::from(b) } else { CP857_HIGH[usize::from(b - 0x80)] }).collect()
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

    #[test]
    fn decoded_files_save_back_byte_for_byte() {
        let turkish = "Şişli'de ığdır çağrı İĞÜŞÖÇ\r\n“alıntı” – tire\r\nson satır\r\n";
        let le: Vec<u8> = turkish.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let be: Vec<u8> = turkish.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let cases: Vec<(Vec<u8>, &Encoding, bool)> = vec![
            (w1254(turkish), encoding_rs::WINDOWS_1254, false),
            (turkish.as_bytes().to_vec(), encoding_rs::UTF_8, false),
            ([&[0xEF, 0xBB, 0xBF][..], turkish.as_bytes()].concat(), encoding_rs::UTF_8, true),
            (le.clone(), encoding_rs::UTF_16LE, false),
            ([&[0xFF, 0xFE][..], &le].concat(), encoding_rs::UTF_16LE, true),
            (be.clone(), encoding_rs::UTF_16BE, false),
            ([&[0xFE, 0xFF][..], &be].concat(), encoding_rs::UTF_16BE, true),
        ];
        for (bytes, enc, bom) in cases {
            let d = decode_text(&bytes, true);
            assert_eq!((d.text.as_str(), d.encoding, d.bom, d.exact), (turkish, enc, bom, true), "{}", enc.name());
            // Windows line ends stay as they were.
            assert_eq!(d.text.matches("\r\n").count(), 3);
            assert_eq!(encode_text(&d.text, d.encoding, d.bom).unwrap(), bytes, "{} bom={bom}", enc.name());
            // An edit is written in the same encoding, with the same mark.
            let edited = d.text.replace("son", "İlk");
            let saved = encode_text(&edited, d.encoding, d.bom).unwrap();
            let back = decode_text(&saved, true);
            assert_eq!((back.text.as_str(), back.encoding, back.bom), (edited.as_str(), enc, bom));
        }
        // Cyrillic Windows-1251 is saved back as such, not as Turkish or UTF-8.
        let ru = encoding_rs::WINDOWS_1251.encode("Привет, как дела? Хорошо, спасибо.\r\n").0.into_owned();
        let d = decode_text(&ru, true);
        assert_eq!((d.encoding, d.exact), (encoding_rs::WINDOWS_1251, true));
        assert_eq!(encode_text(&d.text, encoding_named(d.encoding.name()).unwrap(), false).unwrap(), ru);
    }

    #[test]
    fn letters_an_encoding_lacks_are_refused_not_replaced() {
        assert_eq!(encode_text("Merhaba 😀", encoding_rs::WINDOWS_1254, false), Err('😀'));
        assert_eq!(encode_text("dağ", encoding_rs::WINDOWS_1252, false), Err('ğ'));
        assert_eq!(encode_text("Şişli €5", encoding_rs::WINDOWS_1254, false).unwrap(), b"\xdei\xfeli \x805");
        // UTF-8 and UTF-16 hold everything; a byte order mark only where there is one.
        assert_eq!(encode_text("😀", encoding_rs::UTF_16LE, true).unwrap(), [0xFF, 0xFE, 0x3D, 0xD8, 0x00, 0xDE]);
        assert_eq!(encode_text("İ", encoding_rs::WINDOWS_1254, true).unwrap(), [0xDD]);
        assert_eq!(encoding_named("windows-1254"), Some(encoding_rs::WINDOWS_1254));
        assert_eq!(encoding_named("UTF-16LE"), Some(encoding_rs::UTF_16LE));
        assert_eq!(encoding_named("replacement"), None);
        assert_eq!(encoding_named("no-such"), None);
    }

    #[test]
    fn files_that_would_not_come_back_the_same_say_so() {
        // A lone surrogate in UTF-16 decodes to "�", which saving would write instead.
        let mut lone = vec![0xFF, 0xFE];
        lone.extend_from_slice(&[0x61, 0x00, 0x00, 0xD8, 0x62, 0x00]);
        let d = decode_text(&lone, true);
        assert_eq!((d.encoding, d.exact), (encoding_rs::UTF_16LE, false));
        assert!(d.text.contains('\u{FFFD}'));
        // Undeclared Windows-1254 C1 bytes round-trip (WHATWG maps them to control characters).
        let c1 = [w1254("Şişli ığdır çağrı "), vec![0x81]].concat();
        let d = decode_text(&c1, true);
        assert_eq!((d.encoding, d.exact), (encoding_rs::WINDOWS_1254, true));
    }

    #[test]
    fn the_start_of_a_large_file_drops_a_cut_letter() {
        let s = "aaaş".as_bytes(); // 'ş' is two bytes
        let d = decode_text(&s[..s.len() - 1], false);
        assert_eq!((d.text.as_str(), d.encoding), ("aaa", encoding_rs::UTF_8));
        // An odd cut of BOM-less UTF-16 is still UTF-16, and half a surrogate pair is dropped.
        let le: Vec<u8> = "İstanbul 😀".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let d = decode_text(&le[..le.len() - 3], false);
        assert_eq!((d.text.as_str(), d.encoding), ("İstanbul ", encoding_rs::UTF_16LE));
        let d = decode_text(&le[..le.len() - 1], false);
        assert_eq!(d.text, "İstanbul ");
        // Complete Windows-1254 text is left alone.
        assert_eq!(decode_text(&w1254("Şişli"), false).text, "Şişli");
    }

    #[test]
    fn dos_code_page_857() {
        // "İıŞşĞğÇçÖöÜü" as Turkish DOS programs and Windows' own ZIP write it.
        let dos = [0x98, 0x8D, 0x9E, 0x9F, 0xA6, 0xA7, 0x80, 0x87, 0x99, 0x94, 0x9A, 0x81];
        assert_eq!(decode_cp857(&dos), "İıŞşĞğÇçÖöÜü");
        assert_eq!(decode_cp857(b"\xb3 \xc4 \xd5 a"), "│ ─ \u{FFFD} a");
        for label in ["ibm857", "IBM857", "cp857", "\"cp857\"", "857", "csIBM857", "ibm-857"] {
            assert_eq!(decode_label(&dos, label).as_deref(), Some("İıŞşĞğÇçÖöÜü"), "{label}");
        }
        assert!(!is_cp857("ibm850"));
    }
}
