//! Turkish legacy text encoding (Windows-1254 / ISO-8859-9).
//!
//! Files exported by Turkish-locale Excel and older Windows programs are often Windows-1254.
//! Read as Windows-1252, their ğ ı ş Ğ İ Ş come out as ð ý þ Ð Ý Þ. Windows-1254 differs from
//! Windows-1252 only in those six positions (plus a few unused C1 slots), so choosing it whenever
//! one of them appears in text that is not UTF-8 never damages Western European text, except
//! Icelandic, whose ð/þ/ý are the same bytes.

/// The bytes that are Turkish letters in Windows-1254 but ð ý þ Ð Ý Þ in Windows-1252.
const TURKISH_ONLY: [u8; 6] = [0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE];

/// True when `bytes` is not UTF-8 and contains a Windows-1254 Turkish letter.
pub fn looks_like_windows_1254(bytes: &[u8]) -> bool {
    let has_bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]) || bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
    !has_bom && std::str::from_utf8(bytes).is_err() && bytes.iter().any(|b| TURKISH_ONLY.contains(b))
}

/// Decodes Windows-1254 text to a Rust string.
pub fn decode_turkish_legacy(bytes: &[u8]) -> String {
    encoding_rs::WINDOWS_1254.decode_without_bom_handling(bytes).0.into_owned()
}
