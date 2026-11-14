//! DXF text the engine would otherwise show wrongly, fixed before it reads the file. DWG files
//! reach the engine as DXF (CADCraft's DWG bridge converts them), so the same fixes cover both.
//!
//! - **Turkish text in a drawing that says it is Western European.** Text of pre-2007 drawings is
//!   stored in the code page the header names (`$DWGCODEPAGE`), but many Turkish drawings say
//!   `ANSI_1252` (made from an English template, or on a non-Turkish system) while their bytes
//!   are Windows-1254. Read as 1252, İ ı Ş ş Ğ ğ come out as Ý ý Þ þ Ð ð, the only six letters
//!   where the two code pages differ (Ç Ö Ü ç ö ü are the same in both). Once a drawing has been
//!   saved again as 2007 or later, those wrong letters are its Unicode text. When a drawing's
//!   text has them and does not look Icelandic ([`looks_misread`]), they are read as the Turkish
//!   letters.
//! - **`%%nnn` character codes.** AutoCAD draws `%%221` as character 221 of the drawing's code
//!   page (İ in Windows-1254); the engine takes the number as a Unicode code point (Ý). In
//!   Turkish drawings the six codes that differ become the Turkish letters.
//! - **Code pages the engine's DXF reader doesn't know** (DOS857, the Turkish DOS code page, and
//!   other DOS pages, Johab, KOI8): it reads those files as Windows-1252. The DWG reader knows
//!   them, and so a DWG in such a code page came out right from the DWG but wrong from the DXF
//!   it was converted to. Such files are decoded here with the DWG reader's tables.
//!
//! Files whose text the engine already reads right are passed on untouched.

use std::borrow::Cow;

/// Windows-1252 letters at the code points where Windows-1254 has Turkish letters, and those
/// letters.
const MISREAD: [(char, char); 6] = [('Ð', 'Ğ'), ('Ý', 'İ'), ('Þ', 'Ş'), ('ð', 'ğ'), ('ý', 'ı'), ('þ', 'ş')];

/// `%%nnn` codes whose Windows-1254 character is not the Unicode code point nnn.
const CODES: [(u32, char); 6] = [(208, 'Ğ'), (221, 'İ'), (222, 'Ş'), (240, 'ğ'), (253, 'ı'), (254, 'ş')];

/// Letters Icelandic writes often and Turkish doesn't: Icelandic text has Ý ý Þ þ Ð ð too.
const ICELANDIC: [char; 12] = ['á', 'Á', 'é', 'É', 'í', 'Í', 'ó', 'Ó', 'ú', 'Ú', 'æ', 'Æ'];

/// How the engine reads a file's legacy (not UTF-8) bytes, from the code page its header names.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Page {
    /// As Windows-1252: `ANSI_1252` and its aliases, no code page, or one nobody here knows.
    Western,
    /// Turkish (`ANSI_1254`, ISO 8859-9): read right.
    Turkish,
    /// One the engine doesn't know but the DWG reader does (`DOS857`, …): read wrong.
    DwgReaderOnly(acadrust::io::dxf::code_page::LegacyEncoding),
    /// Any other code page the engine knows: read right.
    Other,
}

/// What the engine should read instead of ASCII DXF `dxf`, or `None` when it reads `dxf` right.
pub(crate) fn prepare(dxf: &[u8]) -> Option<Vec<u8>> {
    if dxf.starts_with(b"AutoCAD Binary DXF") {
        return None;
    }
    let page = page(header_code_page(dxf));
    // The text as the engine would read it; whether we decoded it differently; whether it may be
    // Turkish read as Windows-1252 (UTF-8 text of a Western or Turkish drawing may hold such
    // letters from an earlier conversion; an ASCII file is UTF-8 too).
    let (text, decoded, may_be_misread) = match std::str::from_utf8(dxf) {
        Ok(s) => (Cow::Borrowed(s), false, matches!(page, Page::Western | Page::Turkish)),
        Err(_) => match page {
            Page::Other => return None,
            Page::DwgReaderOnly(enc) => (enc.decode_without_bom_handling(dxf).0, true, false),
            Page::Turkish => (decode(dxf, "ANSI_1254")?, false, false),
            Page::Western => {
                // Most Western drawings have none of these bytes: no need to decode them.
                const SUSPECT: [u8; 6] = [0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE];
                if !dxf.iter().any(|b| SUSPECT.contains(b)) && !dxf.windows(3).any(|w| w == b"%%2") {
                    return None;
                }
                (decode(dxf, "ANSI_1252")?, false, true)
            }
        },
    };
    let misread = may_be_misread && looks_misread(&text);
    let mut changed = decoded;
    let mut out = if misread {
        changed = true;
        Cow::Owned(text.chars().map(turkish_letter).collect())
    } else {
        text
    };
    if (misread || page == Page::Turkish)
        && let Some(fixed) = fix_codes(&out)
    {
        out = Cow::Owned(fixed);
        changed = true;
    }
    changed.then(|| out.into_owned().into_bytes())
}

/// `bytes` decoded from the code page `name` as the engine decodes it.
fn decode<'a>(bytes: &[u8], name: &str) -> Option<Cow<'a, str>> {
    cadcraft_dxf::decode_code_page(bytes, name).map(Cow::Owned)
}

fn turkish_letter(c: char) -> char {
    MISREAD.iter().find(|(w, _)| *w == c).map_or(c, |(_, t)| *t)
}

/// Whether text read as Windows-1252 is Windows-1254 Turkish: it has the letters Turkish
/// becomes in 1252 (or the `%%nnn` codes of Turkish letters), and few of Icelandic's accented
/// vowels, which come with those letters in real Icelandic text.
fn looks_misread(text: &str) -> bool {
    let (mut suspect, mut icelandic) = (0usize, 0usize);
    for c in text.chars() {
        if MISREAD.iter().any(|(w, _)| *w == c) {
            suspect += 1;
        } else if ICELANDIC.contains(&c) {
            icelandic += 1;
        }
    }
    suspect += count_codes(text);
    suspect > 0 && icelandic * 4 < suspect
}

/// The `%%nnn` codes of Turkish letters in `text`, wherever they are.
fn count_codes(text: &str) -> usize {
    CODES.iter().map(|(n, _)| text.matches(&format!("%%{n}")).count()).sum()
}

/// `text` (a whole DXF file) with the `%%nnn` codes of Turkish letters in text values (group
/// codes 1 and 3: TEXT, ATTRIB and MTEXT strings) replaced by the letters, or `None` when it has
/// none. The codes are found the way the engine finds them when it draws the text.
fn fix_codes(text: &str) -> Option<String> {
    if count_codes(text) == 0 {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut code_line = true;
    let mut text_value = false;
    for line in text.split_inclusive('\n') {
        if code_line {
            text_value = matches!(line.trim(), "1" | "3");
            out.push_str(line);
        } else if text_value && let Some(fixed) = fix_codes_in(line) {
            out.push_str(&fixed);
            changed = true;
        } else {
            out.push_str(line);
        }
        code_line = !code_line;
    }
    changed.then_some(out)
}

/// One string with its `%%nnn` codes of Turkish letters replaced, or `None` when it has none.
/// Follows the engine's reading of control codes (`cadcraft_fonts::decode_controls`): `%%`
/// then up to three digits is a character code; `%%d`, `%%p`, `%%c`, `%%%`, `%%u` and `%%o`
/// take three characters; any other `%` is itself.
fn fix_codes_in(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut changed = false;
    let mut rest = s;
    while let Some(i) = rest.find("%%") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 2..];
        let digits = after.bytes().take(3).take_while(u8::is_ascii_digit).count();
        if digits > 0 {
            let n: u32 = after[..digits].parse().unwrap_or(32);
            match CODES.iter().find(|(k, _)| *k == n) {
                Some((_, letter)) => {
                    out.push(*letter);
                    changed = true;
                }
                None => out.push_str(&rest[i..i + 2 + digits]),
            }
            rest = &after[digits..];
        } else if after.bytes().next().is_some_and(|b| matches!(b.to_ascii_lowercase(), b'd' | b'p' | b'c' | b'%' | b'u' | b'o')) {
            out.push_str(&rest[i..i + 3]);
            rest = &after[1..];
        } else {
            out.push('%');
            rest = &rest[i + 1..];
        }
    }
    out.push_str(rest);
    changed.then_some(out)
}

/// The `$DWGCODEPAGE` value of an ASCII DXF file, found as the engine finds it: the line two
/// lines after a `$DWGCODEPAGE` line (group code 3 between them), in the HEADER section only.
fn header_code_page(bytes: &[u8]) -> Option<String> {
    let mut lines = bytes.split(|&b| b == b'\n').map(<[u8]>::trim_ascii);
    while let Some(line) = lines.next() {
        if line == b"ENDSEC" {
            return None;
        }
        if line.eq_ignore_ascii_case(b"$DWGCODEPAGE") {
            return match (lines.next(), lines.next()) {
                (Some(b"3"), Some(value)) => Some(String::from_utf8_lossy(value).into_owned()),
                _ => None,
            };
        }
    }
    None
}

fn page(code_page: Option<String>) -> Page {
    let Some(raw) = code_page else { return Page::Western };
    let name = raw.trim().to_ascii_uppercase().replace(['-', ' '], "_");
    match name.as_str() {
        "" | "ANSI_1252" | "ISO8859_1" | "ISO_8859_1" | "ASCII" => Page::Western,
        "ANSI_1254" | "ISO8859_9" | "ISO_8859_9" => Page::Turkish,
        _ if cadcraft_dxf::decode_code_page(b"", &name).is_some() => Page::Other,
        _ => match acadrust::io::dxf::code_page::try_encoding_from_code_page(raw.trim()) {
            Some(enc) => Page::DwgReaderOnly(enc),
            // The engine reads a code page it doesn't know as Windows-1252.
            None => Page::Western,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_read_as_the_engine_reads_them() {
        assert_eq!(fix_codes_in("%%221stanbul %%253%%254"), Some("İstanbul ış".into()));
        // Up to three digits; a fourth is text.
        assert_eq!(fix_codes_in("%%2210"), Some("İ0".into()));
        // `%%%` is a percent sign, so the digits after it are text.
        assert_eq!(fix_codes_in("%%%221"), None);
        assert_eq!(fix_codes_in("%%d %%c25 %%199 %%u%%221%%u"), Some("%%d %%c25 %%199 %%uİ%%u".into()));
        assert_eq!(fix_codes_in("50% %%221"), Some("50% İ".into()));
        assert_eq!(fix_codes_in("%%%%221"), None);
        assert_eq!(fix_codes_in("% %%253"), Some("% ı".into()));
        assert_eq!(fix_codes_in("plain"), None);
    }

    #[test]
    fn icelandic_text_is_left_alone() {
        assert!(looks_misread("Ýstanbul ýþýk"));
        assert!(looks_misread("KAPI %%221"));
        assert!(!looks_misread("Þetta er það sem ég vil á Íslandi, ég fór í bíó"));
        assert!(!looks_misread("Çizim ölçü üst"));
    }

    #[test]
    fn code_pages_are_classified_as_the_engine_reads_them() {
        assert_eq!(page(None), Page::Western);
        assert_eq!(page(Some("ansi_1252".into())), Page::Western);
        assert_eq!(page(Some("ANSI_1361".into())), Page::Western);
        assert_eq!(page(Some("ANSI_1254".into())), Page::Turkish);
        assert_eq!(page(Some("ISO8859-9".into())), Page::Turkish);
        assert_eq!(page(Some("ANSI_1251".into())), Page::Other);
        assert_eq!(page(Some("DOS850".into())), Page::Other);
        assert!(matches!(page(Some("DOS857".into())), Page::DwgReaderOnly(_)));
    }
}
