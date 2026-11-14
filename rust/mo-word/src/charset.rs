//! Turkish text in files that do not say (or say wrongly) which code page they are in, put right
//! before WordCraft reads them. WordCraft reads these formats well when they declare their
//! encoding; without a declaration it guesses with chardetng and no hint of the file's origin,
//! and short Turkish Windows-1254 text comes out as Chinese or with Ý ý Þ þ Ð ð for İ ı Ş ş Ğ ğ
//! (see `mo_convert::text`).
//!
//! - Plain text and Markdown that are neither UTF-8 nor UTF-16 are decoded here, Turkish first,
//!   and handed over as UTF-8 with a byte order mark.
//! - HTML without a byte order mark or `<meta charset>` the same; Turkish named character
//!   references the engine does not know (`&scedil;`, `&Idot;`, `&imath;`…) become numeric ones.
//! - RTF in Turkish (`\deflang1055`, `\lang1055`) without `\ansicpg` gets `\ansicpg1254`: RTF's
//!   default code page is 1252, which turns `\'dd` into Ý instead of İ.

/// The bytes WordCraft should read instead of `bytes` (a file called `name`), or `None` to read
/// them as they are.
pub(crate) fn prepare(name: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "txt" | "text" | "log" | "md" | "markdown" => mo_convert::text::decode_undeclared(bytes).map(with_bom),
        "htm" | "html" | "xhtml" | "xht" => {
            let decoded = if mo_convert::text::has_bom(bytes) || mo_convert::text::html_meta_charset(bytes).is_some() {
                None
            } else {
                mo_convert::text::decode_undeclared(bytes).map(with_bom)
            };
            // The references are ASCII: replaced in any ASCII-based encoding, never in UTF-16.
            let source = decoded.as_deref().unwrap_or(bytes);
            let utf16 = source.starts_with(&[0xFF, 0xFE]) || source.starts_with(&[0xFE, 0xFF]) || mo_convert::text::unmarked_utf16(source).is_some();
            let entities = if utf16 { None } else { turkish_entities(source) };
            entities.or(decoded)
        }
        _ if bytes.starts_with(b"{\\rtf") => rtf_code_page(bytes),
        _ => None,
    }
}

fn with_bom(text: String) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() + 3);
    out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    out.extend_from_slice(text.as_bytes());
    out
}

/// HTML5's named references for the Turkish letters outside Latin-1, as numbers.
const ENTITIES: [(&[u8], &[u8]); 7] = [
    (b"&Idot;", b"&#304;"),
    (b"&imath;", b"&#305;"),
    (b"&inodot;", b"&#305;"),
    (b"&Scedil;", b"&#350;"),
    (b"&scedil;", b"&#351;"),
    (b"&Gbreve;", b"&#286;"),
    (b"&gbreve;", b"&#287;"),
];

/// `html` with [`ENTITIES`] replaced, or `None` when it has none.
fn turkish_entities(html: &[u8]) -> Option<Vec<u8>> {
    if !ENTITIES.iter().any(|(name, _)| find(html, name).is_some()) {
        return None;
    }
    let mut out = Vec::with_capacity(html.len());
    let mut i = 0;
    'scan: while i < html.len() {
        if html[i] == b'&' {
            for (name, number) in ENTITIES {
                if html[i..].starts_with(name) {
                    out.extend_from_slice(number);
                    i += name.len();
                    continue 'scan;
                }
            }
        }
        out.push(html[i]);
        i += 1;
    }
    Some(out)
}

/// An RTF document in Turkish that names no code page, with `\ansicpg1254` added after
/// `{\rtf1`; `None` for any other.
fn rtf_code_page(rtf: &[u8]) -> Option<Vec<u8>> {
    if find(rtf, b"\\ansicpg").is_some() {
        return None;
    }
    let turkish = [&b"\\deflang1055"[..], b"\\lang1055", b"\\langfe1055", b"\\deflangfe1055"].iter().any(|w| {
        let mut from = 0;
        while let Some(i) = find(&rtf[from..], w) {
            let end = from + i + w.len();
            if !rtf.get(end).is_some_and(u8::is_ascii_digit) {
                return true;
            }
            from = end;
        }
        false
    });
    if !turkish {
        return None;
    }
    // After "{\rtf" and its version number.
    let at = 5 + rtf[5..].iter().take_while(|b| b.is_ascii_digit()).count();
    let mut out = Vec::with_capacity(rtf.len() + 12);
    out.extend_from_slice(&rtf[..at]);
    out.extend_from_slice(b"\\ansicpg1254");
    out.extend_from_slice(&rtf[at..]);
    Some(out)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}
