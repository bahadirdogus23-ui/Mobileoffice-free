//! Stand-in fonts for letters Android's renderer cannot draw.
//!
//! A PDF may name Helvetica, Times or Courier (or Arial, or any other font) without embedding
//! it; the reader then draws the text in a font of its own. Android's `PdfRenderer` (PDFium)
//! uses its built-in Foxit fonts, which hold only the 229 glyphs of Adobe's standard Latin set:
//! WinAnsi's letters and a few more (ı Ł ł). Old Turkish PDFs name the other letters in the
//! font's `/Differences` (`/gbreve /scedilla /Idotaccent /Gbreve /Scedilla`), so on a phone
//! "Toplantı Tutanağı" read "Toplantı Tutana ı": every ğ, ş, İ (and the Central European letters
//! such as ł's neighbours ą ę ć ź ő) left a gap. The file is valid; every desktop reader draws it.
//!
//! The copy shown on screen ([`crate::PdfDoc::display_copy`]) gets, for each such font, a
//! TrueType font that has the letters, embedded with the same codes and the same advance widths,
//! so the text keeps its place: the metric-compatible Liberation Sans / Serif the app ships
//! (handed over with [`register_fonts`]) or, without them, the fonts compiled into the library.
//! The person's file is never changed.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use pdfcraft_cos::{Dict, Document, ObjRef, Object, Stream};
use pdfcraft_fonts::Std14;

/// Letters beyond WinAnsi's that the built-in fonts of Android's renderer draw (found by drawing
/// each glyph of the standard fonts' 315-glyph metrics with PDFium).
const BUILT_IN_EXTRA: &[char] = &['ı', 'Ł', 'ł', 'ˇ', '˘', '˙', '˚', '˛', '˝', '⁄', '−', 'ﬁ', 'ﬂ'];

/// WinAnsiEncoding, codes 0x80–0x9F (ISO 32000-2 Annex D); the other codes are Latin-1.
const WIN_ANSI_80: [u16; 32] = [
    0x20AC, 0, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0, 0x017D, 0, 0, 0x2018, 0x2019, 0x201C,
    0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0, 0x017E, 0x0178,
];

/// StandardEncoding, codes 0xA1–0xFF (ISO 32000-2 Annex D); below, ASCII except ’ and ‘.
const STANDARD_A1: [u16; 95] = [
    0x00A1, 0x00A2, 0x00A3, 0x2044, 0x00A5, 0x0192, 0x00A7, 0x00A4, 0x0027, 0x201C, 0x00AB, 0x2039, 0x203A, 0xFB01, 0xFB02, 0, 0x2013, 0x2020,
    0x2021, 0x00B7, 0, 0x00B6, 0x2022, 0x201A, 0x201E, 0x201D, 0x00BB, 0x2026, 0x2030, 0, 0x00BF, 0, 0x0060, 0x00B4, 0x02C6, 0x02DC, 0x00AF, 0x02D8,
    0x02D9, 0x00A8, 0, 0x02DA, 0x00B8, 0, 0x02DD, 0x02DB, 0x02C7, 0x2014, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00C6, 0, 0x00AA, 0, 0, 0,
    0, 0x0141, 0x00D8, 0x0152, 0x00BA, 0, 0, 0, 0, 0, 0x00E6, 0, 0, 0, 0x0131, 0, 0, 0x0142, 0x00F8, 0x0153, 0x00DF, 0, 0, 0, 0,
];

/// MacRomanEncoding, codes 0x80–0xFF.
const MAC_ROMAN_80: [u16; 128] = [
    0x00C4, 0x00C5, 0x00C7, 0x00C9, 0x00D1, 0x00D6, 0x00DC, 0x00E1, 0x00E0, 0x00E2, 0x00E4, 0x00E3, 0x00E5, 0x00E7, 0x00E9, 0x00E8, 0x00EA, 0x00EB,
    0x00ED, 0x00EC, 0x00EE, 0x00EF, 0x00F1, 0x00F3, 0x00F2, 0x00F4, 0x00F6, 0x00F5, 0x00FA, 0x00F9, 0x00FB, 0x00FC, 0x2020, 0x00B0, 0x00A2, 0x00A3,
    0x00A7, 0x2022, 0x00B6, 0x00DF, 0x00AE, 0x00A9, 0x2122, 0x00B4, 0x00A8, 0x2260, 0x00C6, 0x00D8, 0x221E, 0x00B1, 0x2264, 0x2265, 0x00A5, 0x00B5,
    0x2202, 0x2211, 0x220F, 0x03C0, 0x222B, 0x00AA, 0x00BA, 0x03A9, 0x00E6, 0x00F8, 0x00BF, 0x00A1, 0x00AC, 0x221A, 0x0192, 0x2248, 0x2206, 0x00AB,
    0x00BB, 0x2026, 0x00A0, 0x00C0, 0x00C3, 0x00D5, 0x0152, 0x0153, 0x2013, 0x2014, 0x201C, 0x201D, 0x2018, 0x2019, 0x00F7, 0x25CA, 0x00FF, 0x0178,
    0x2044, 0x00A4, 0x2039, 0x203A, 0xFB01, 0xFB02, 0x2021, 0x00B7, 0x201A, 0x201E, 0x2030, 0x00C2, 0x00CA, 0x00C1, 0x00CB, 0x00C8, 0x00CD, 0x00CE,
    0x00CF, 0x00CC, 0x00D3, 0x00D4, 0, 0x00D2, 0x00DA, 0x00DB, 0x00D9, 0x0131, 0x02C6, 0x02DC, 0x00AF, 0x02D8, 0x02D9, 0x02DA, 0x00B8, 0x02DD,
    0x02DB, 0x02C7,
];

/// Glyph names of the standard fonts' metrics that PdfCraft's glyph list (Annex D) lacks, and
/// `Idot`, which Turkish tools write for İ.
const EXTENDED_NAMES: &[(&str, char)] = &[
    ("Abreve", 'Ă'),
    ("Amacron", 'Ā'),
    ("Aogonek", 'Ą'),
    ("Cacute", 'Ć'),
    ("Ccaron", 'Č'),
    ("Dcaron", 'Ď'),
    ("Dcroat", 'Đ'),
    ("Delta", '∆'),
    ("Ecaron", 'Ě'),
    ("Edotaccent", 'Ė'),
    ("Emacron", 'Ē'),
    ("Eogonek", 'Ę'),
    ("Gbreve", 'Ğ'),
    ("Gcommaaccent", 'Ģ'),
    ("Idot", 'İ'),
    ("Idotaccent", 'İ'),
    ("Imacron", 'Ī'),
    ("Iogonek", 'Į'),
    ("Kcommaaccent", 'Ķ'),
    ("Lacute", 'Ĺ'),
    ("Lcaron", 'Ľ'),
    ("Lcommaaccent", 'Ļ'),
    ("Nacute", 'Ń'),
    ("Ncaron", 'Ň'),
    ("Ncommaaccent", 'Ņ'),
    ("Ohungarumlaut", 'Ő'),
    ("Omacron", 'Ō'),
    ("Racute", 'Ŕ'),
    ("Rcaron", 'Ř'),
    ("Rcommaaccent", 'Ŗ'),
    ("Sacute", 'Ś'),
    ("Scedilla", 'Ş'),
    ("Scommaaccent", 'Ș'),
    ("Tcaron", 'Ť'),
    ("Tcommaaccent", 'Ţ'),
    ("Uhungarumlaut", 'Ű'),
    ("Umacron", 'Ū'),
    ("Uogonek", 'Ų'),
    ("Uring", 'Ů'),
    ("Zacute", 'Ź'),
    ("Zdotaccent", 'Ż'),
    ("abreve", 'ă'),
    ("amacron", 'ā'),
    ("aogonek", 'ą'),
    ("cacute", 'ć'),
    ("ccaron", 'č'),
    ("dcaron", 'ď'),
    ("dcroat", 'đ'),
    ("ecaron", 'ě'),
    ("edotaccent", 'ė'),
    ("emacron", 'ē'),
    ("eogonek", 'ę'),
    ("gbreve", 'ğ'),
    ("gcommaaccent", 'ģ'),
    ("greaterequal", '≥'),
    ("imacron", 'ī'),
    ("iogonek", 'į'),
    ("kcommaaccent", 'ķ'),
    ("lacute", 'ĺ'),
    ("lcaron", 'ľ'),
    ("lcommaaccent", 'ļ'),
    ("lessequal", '≤'),
    ("lozenge", '◊'),
    ("nacute", 'ń'),
    ("ncaron", 'ň'),
    ("ncommaaccent", 'ņ'),
    ("notequal", '≠'),
    ("ohungarumlaut", 'ő'),
    ("omacron", 'ō'),
    ("partialdiff", '∂'),
    ("racute", 'ŕ'),
    ("radical", '√'),
    ("rcaron", 'ř'),
    ("rcommaaccent", 'ŗ'),
    ("sacute", 'ś'),
    ("scedilla", 'ş'),
    ("scommaaccent", 'ș'),
    ("summation", '∑'),
    ("tcaron", 'ť'),
    ("tcommaaccent", 'ţ'),
    ("uhungarumlaut", 'ű'),
    ("umacron", 'ū'),
    ("uogonek", 'ų'),
    ("uring", 'ů'),
    ("zacute", 'ź'),
    ("zdotaccent", 'ż'),
];

/// The character a glyph name stands for: Annex D names, `uniXXXX`/`uXXXX`, and [`EXTENDED_NAMES`].
fn name_char(name: &[u8]) -> Option<char> {
    let name = std::str::from_utf8(name).ok()?;
    EXTENDED_NAMES.iter().find(|(n, _)| *n == name).map(|(_, c)| *c).or_else(|| pdfcraft_fonts::pdf::glyph_unicode(name))
}

fn win_ansi(code: u8) -> Option<char> {
    match code {
        0x20..=0x7E | 0xA1..=0xFF => Some(char::from(code)),
        0xA0 => Some(' '),
        0x80..=0x9F => char::from_u32(u32::from(WIN_ANSI_80[usize::from(code - 0x80)])).filter(|c| *c != '\0'),
        _ => None,
    }
}

fn standard(code: u8) -> Option<char> {
    match code {
        0x27 => Some('’'),
        0x60 => Some('‘'),
        0x20..=0x7E => Some(char::from(code)),
        0xA1..=0xFF => char::from_u32(u32::from(STANDARD_A1[usize::from(code - 0xA1)])).filter(|c| *c != '\0'),
        _ => None,
    }
}

fn mac_roman(code: u8) -> Option<char> {
    match code {
        0x20..=0x7E => Some(char::from(code)),
        0x80..=0xFF => char::from_u32(u32::from(MAC_ROMAN_80[usize::from(code - 0x80)])).filter(|c| *c != '\0'),
        _ => None,
    }
}

/// Whether the built-in fonts of Android's renderer have a glyph for `c`.
fn built_in_draws(c: char) -> bool {
    BUILT_IN_EXTRA.contains(&c) || (0x20..=0xFFu8).any(|code| win_ansi(code) == Some(c))
}

// --------------------------------------------------------------------------------------------
// The fonts that stand in

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Sans,
    Serif,
    Mono,
}

struct Candidate {
    data: Data,
    class: Class,
    bold: bool,
    italic: bool,
    /// Handed over by the app (metric-compatible), preferred over the compiled-in fonts.
    registered: bool,
}

#[derive(Clone)]
enum Data {
    Static(&'static [u8]),
    Shared(Arc<Vec<u8>>),
}

impl Data {
    fn bytes(&self) -> &[u8] {
        match self {
            Data::Static(b) => b,
            Data::Shared(b) => b,
        }
    }
}

/// The families taken from the app ([`register_fonts`]) and from the library's own fonts.
const REGISTERED_FAMILIES: &[&str] = &["Liberation Sans", "Liberation Serif", "Liberation Mono"];
const BUNDLED_FAMILIES: &[&str] = &["Source Sans 3", "Source Serif 4", "JetBrains Mono"];

fn candidate(data: Data, families: &[&str], registered: bool) -> Option<Candidate> {
    let face = ttf_parser::Face::parse(data.bytes(), 0).ok()?;
    let family = face
        .names()
        .into_iter()
        .filter(|n| n.name_id == ttf_parser::name_id::TYPOGRAPHIC_FAMILY || n.name_id == ttf_parser::name_id::FAMILY)
        .filter_map(|n| n.to_string())
        .find(|f| families.contains(&f.as_str()))?;
    let class = if family.contains("Mono") {
        Class::Mono
    } else if family.contains("Serif") {
        Class::Serif
    } else {
        Class::Sans
    };
    let (bold, italic) = (face.is_bold() || face.weight().to_number() >= 600, face.is_italic());
    Some(Candidate { class, bold, italic, registered, data })
}

fn registered() -> &'static Mutex<Vec<Candidate>> {
    static REGISTERED: OnceLock<Mutex<Vec<Candidate>>> = OnceLock::new();
    REGISTERED.get_or_init(|| Mutex::new(Vec::new()))
}

fn bundled() -> &'static [Candidate] {
    static BUNDLED: OnceLock<Vec<Candidate>> = OnceLock::new();
    BUNDLED.get_or_init(|| wordcraft_fonts::bundled().iter().filter_map(|d| candidate(Data::Static(d), BUNDLED_FAMILIES, false)).collect())
}

/// Keeps the Liberation Sans, Serif and Mono files among `files` (unmodified) as stand-ins for
/// fonts a PDF names without embedding; other files are ignored, and a face kept before is not
/// added again. Returns the number of faces added.
pub fn register_fonts(files: &[Vec<u8>]) -> usize {
    let Ok(mut list) = registered().lock() else { return 0 };
    let mut added = 0;
    for file in files {
        let Some(c) = candidate(Data::Shared(Arc::new(file.clone())), REGISTERED_FAMILIES, true) else { continue };
        if list.iter().any(|o| o.class == c.class && o.bold == c.bold && o.italic == c.italic) {
            continue;
        }
        list.push(c);
        added += 1;
    }
    added
}

/// The best stand-in for `want`: of its class, registered first, then the same weight and slant.
fn choose(want: Want) -> Option<(Data, bool)> {
    let score = |c: &Candidate| u8::from(c.registered) * 4 + u8::from(c.bold == want.bold) * 2 + u8::from(c.italic == want.italic);
    let list = registered().lock().ok();
    let reg = list.iter().flat_map(|l| l.iter());
    reg.chain(bundled().iter()).filter(|c| c.class == want.class).max_by_key(|c| score(c)).map(|c| (c.data.clone(), c.registered))
}

// --------------------------------------------------------------------------------------------
// Finding the fonts

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Want {
    class: Class,
    bold: bool,
    italic: bool,
    /// The standard font it names, whose widths are exact for WinAnsi's letters.
    std14: Option<Std14>,
}

/// A font a page uses that Android's renderer would draw in its built-in fonts with letters
/// missing: a simple font without an embedded program whose `/Differences` name a letter beyond
/// the built-in fonts' set.
struct Missing {
    want: Want,
    /// The character each code shows, by the font's encoding.
    chars: [Option<char>; 256],
    /// The letters the built-in fonts lack.
    lacking: Vec<char>,
}

fn dict_of(doc: &Document, o: Option<&Object>) -> Option<Dict> {
    o.and_then(|o| doc.resolve(o).as_dict().cloned())
}

fn flags(doc: &Document, font: &Dict) -> i64 {
    dict_of(doc, font.get(b"FontDescriptor")).and_then(|d| d.get(b"Flags").and_then(|f| doc.resolve(f).as_int())).unwrap_or(0)
}

fn missing(doc: &Document, font: &Dict) -> Option<Missing> {
    if !matches!(font.name(b"Subtype"), Some(b"Type1" | b"MMType1" | b"TrueType")) {
        return None;
    }
    if let Some(desc) = dict_of(doc, font.get(b"FontDescriptor"))
        && [&b"FontFile"[..], b"FontFile2", b"FontFile3"].iter().any(|k| desc.get(k).is_some())
    {
        return None;
    }
    let enc = dict_of(doc, font.get(b"Encoding"))?;
    let diffs = doc.resolve(enc.get(b"Differences")?);
    let base = String::from_utf8_lossy(font.name(b"BaseFont").unwrap_or_default()).into_owned();
    let want = match Std14::recognise(&base) {
        Some((Std14::Symbol | Std14::ZapfDingbats, _)) => return None,
        Some((f, _)) => {
            use Std14::*;
            let class = match f {
                TimesRoman | TimesBold | TimesItalic | TimesBoldItalic => Class::Serif,
                Courier | CourierBold | CourierOblique | CourierBoldOblique => Class::Mono,
                _ => Class::Sans,
            };
            let bold = matches!(f, HelveticaBold | HelveticaBoldOblique | TimesBold | TimesBoldItalic | CourierBold | CourierBoldOblique);
            let italic = matches!(f, HelveticaOblique | HelveticaBoldOblique | TimesItalic | TimesBoldItalic | CourierOblique | CourierBoldOblique);
            Want { class, bold, italic, std14: Some(f) }
        }
        None => {
            let f = flags(doc, font);
            let lower = base.to_ascii_lowercase();
            let class = if f & 1 != 0 {
                Class::Mono
            } else if f & 2 != 0 {
                Class::Serif
            } else {
                Class::Sans
            };
            let bold = ["bold", "black", "heavy", "demi"].iter().any(|w| lower.contains(w)) || f & (1 << 18) != 0;
            let italic = f & 64 != 0 || lower.contains("italic") || lower.contains("oblique");
            Want { class, bold, italic, std14: None }
        }
    };
    let table: fn(u8) -> Option<char> = match enc.name(b"BaseEncoding") {
        Some(b"WinAnsiEncoding") => win_ansi,
        Some(b"MacRomanEncoding") => mac_roman,
        _ => standard,
    };
    let mut chars = [None; 256];
    for (code, c) in chars.iter_mut().enumerate() {
        *c = table(code as u8);
    }
    let mut code: Option<usize> = None;
    let mut lacking = Vec::new();
    for item in diffs.as_array()? {
        match doc.resolve(item).as_ref() {
            Object::Int(n) => code = usize::try_from(*n).ok(),
            Object::Name(name) => {
                if let Some(slot) = code.and_then(|c| chars.get_mut(c)) {
                    let c = name_char(name.as_slice());
                    *slot = c;
                    if let Some(c) = c.filter(|c| !built_in_draws(*c) && !lacking.contains(c)) {
                        lacking.push(c);
                    }
                }
                code = code.map(|c| c + 1);
            }
            _ => {}
        }
    }
    (!lacking.is_empty()).then_some(Missing { want, chars, lacking })
}

/// The fonts the pages use, by reference: in page resources (inherited too) and in the form
/// XObjects they draw, without repeats.
fn page_fonts(doc: &Document) -> Vec<ObjRef> {
    let Ok(pages) = pdfcraft_annot::page_refs(doc) else { return Vec::new() };
    let mut fonts = Vec::new();
    let mut seen_fonts = HashSet::new();
    let mut seen_resources = HashSet::new();
    let mut stack: Vec<(Object, u8)> = Vec::new();
    for page in pages {
        // Resources are inherited from the page tree.
        let mut node = Some(page);
        for _ in 0..32 {
            let Some(r) = node else { break };
            let d = doc.get(r);
            let Some(d) = d.as_dict() else { break };
            if let Some(res) = d.get(b"Resources") {
                stack.push((res.clone(), 0));
                break;
            }
            node = d.reference(b"Parent");
        }
        while let Some((res, depth)) = stack.pop() {
            if let Some(r) = res.as_ref()
                && !seen_resources.insert(r)
            {
                continue;
            }
            let Some(res) = dict_of(doc, Some(&res)) else { continue };
            if let Some(list) = dict_of(doc, res.get(b"Font")) {
                for (_, f) in list.iter() {
                    if let Some(r) = f.as_ref()
                        && seen_fonts.insert(r)
                    {
                        fonts.push(r);
                    }
                }
            }
            if depth >= 8 {
                continue;
            }
            if let Some(list) = dict_of(doc, res.get(b"XObject")) {
                for (_, x) in list.iter() {
                    let Some(r) = x.as_ref() else { continue };
                    if !seen_resources.insert(r) {
                        continue;
                    }
                    if let Object::Stream(s) = doc.get(r).as_ref()
                        && s.dict.name(b"Subtype") == Some(b"Form")
                        && let Some(inner) = s.dict.get(b"Resources")
                    {
                        stack.push((inner.clone(), depth + 1));
                    }
                }
            }
        }
    }
    fonts
}

/// Whether a page uses a font whose letters Android's renderer would leave out.
pub(crate) fn needed(doc: &Document) -> bool {
    page_fonts(doc).into_iter().any(|r| doc.get(r).as_dict().is_some_and(|f| missing(doc, f).is_some()))
}

// --------------------------------------------------------------------------------------------
// Embedding

/// Gives each font of `doc` whose letters Android's renderer would leave out an embedded
/// stand-in with the same codes and widths. Returns how many fonts were given one.
pub(crate) fn embed(doc: &mut Document) -> usize {
    let mut files: HashMap<*const u8, ObjRef> = HashMap::new();
    let mut count = 0;
    for r in page_fonts(doc) {
        let Some(font) = doc.get(r).as_dict().cloned() else { continue };
        let Some(m) = missing(doc, &font) else { continue };
        if let Some(new) = stand_in(doc, &font, &m, &mut files) {
            doc.set(r, Object::Dict(new));
            count += 1;
        }
    }
    count
}

fn compressed(data: &[u8]) -> Arc<Vec<u8>> {
    static CACHE: OnceLock<Mutex<HashMap<usize, Arc<Vec<u8>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = data.as_ptr() as usize;
    if let Some(c) = cache.lock().ok().and_then(|m| m.get(&key).cloned()) {
        return c;
    }
    let c = Arc::new(miniz_oxide::deflate::compress_to_vec_zlib(data, 6));
    if let Ok(mut m) = cache.lock() {
        m.insert(key, c.clone());
    }
    c
}

fn stand_in(doc: &mut Document, font: &Dict, m: &Missing, files: &mut HashMap<*const u8, ObjRef>) -> Option<Dict> {
    let (data, _) = choose(m.want)?;
    let bytes = data.bytes();
    let face = ttf_parser::Face::parse(bytes, 0).ok()?;
    if !m.lacking.iter().any(|c| face.glyph_index(*c).is_some()) {
        return None;
    }
    let upem = f64::from(face.units_per_em().max(1));
    let em = |v: f64| (v * 1000.0 / upem).round() as i64;
    let name: String = face
        .names()
        .into_iter()
        .filter(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
        .find_map(|n| n.to_string())
        .unwrap_or_else(|| "MoStandIn".into())
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();

    let file = match files.get(&bytes.as_ptr()) {
        Some(r) => *r,
        None => {
            let mut d = Dict::new();
            d.set(b"Length1".to_vec(), Object::Int(bytes.len() as i64));
            d.set(b"Filter".to_vec(), Object::name("FlateDecode"));
            let r = doc.add(Object::Stream(Stream::from_raw(d, compressed(bytes).as_ref().clone())));
            files.insert(bytes.as_ptr(), r);
            r
        }
    };

    // Same widths: the font's own, or those the reader would have used.
    let mut out = font.clone();
    if font.get(b"Widths").is_none() {
        let widths = (0..=255u8)
            .map(|code| {
                let Some(c) = m.chars[usize::from(code)] else { return Object::Int(0) };
                if m.want.class == Class::Mono && m.want.std14.is_some() {
                    return Object::Int(600);
                }
                let exact = m.want.std14.and_then(|f| pdfcraft_fonts::win_ansi_byte(c).map(|b| f.width(b))).filter(|w| *w > 0.0);
                let w =
                    exact.map(|w| w.round() as i64).or_else(|| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map(|a| em(f64::from(a))));
                Object::Int(w.unwrap_or(0))
            })
            .collect();
        out.set(b"FirstChar".to_vec(), Object::Int(0));
        out.set(b"LastChar".to_vec(), Object::Int(255));
        out.set(b"Widths".to_vec(), Object::Array(widths));
    }

    // The same codes, written over WinAnsi (TrueType fonts take no StandardEncoding).
    let mut diffs = Vec::new();
    let mut next = None;
    for code in 0..=255u8 {
        let c = m.chars[usize::from(code)];
        if c == win_ansi(code) {
            continue;
        }
        let glyph = match c {
            Some(c) => {
                let n = EXTENDED_NAMES.iter().find(|(n, ch)| *ch == c && *n != "Idot").map(|(n, _)| (*n).to_string());
                n.or_else(|| standard_name(c)).unwrap_or_else(|| format!("uni{:04X}", c as u32))
            }
            None => ".notdef".into(),
        };
        if next != Some(code) {
            diffs.push(Object::Int(i64::from(code)));
        }
        diffs.push(Object::name(&glyph));
        next = code.checked_add(1);
    }
    let mut enc = Dict::new();
    enc.set(b"Type".to_vec(), Object::name("Encoding"));
    enc.set(b"BaseEncoding".to_vec(), Object::name("WinAnsiEncoding"));
    if !diffs.is_empty() {
        enc.set(b"Differences".to_vec(), Object::Array(diffs));
    }

    let class_flags = match m.want.class {
        Class::Mono => 1,
        Class::Serif => 2,
        Class::Sans => 0,
    };
    let mut desc = dict_of(doc, font.get(b"FontDescriptor")).unwrap_or_default();
    let old_flags = desc.get(b"Flags").and_then(|f| doc.resolve(f).as_int()).unwrap_or(class_flags | if m.want.italic { 64 } else { 0 });
    let b = face.global_bounding_box();
    desc.set(b"Type".to_vec(), Object::name("FontDescriptor"));
    desc.set(b"FontName".to_vec(), Object::name(&name));
    // Nonsymbolic, so the codes are read through the encoding's glyph names.
    desc.set(b"Flags".to_vec(), Object::Int((old_flags & !4) | 32));
    desc.set(b"FontBBox".to_vec(), Object::Array([b.x_min, b.y_min, b.x_max, b.y_max].iter().map(|v| Object::Int(em(f64::from(*v)))).collect()));
    desc.set(b"ItalicAngle".to_vec(), Object::Int(face.italic_angle().round() as i64));
    desc.set(b"Ascent".to_vec(), Object::Int(em(f64::from(face.ascender()))));
    desc.set(b"Descent".to_vec(), Object::Int(em(f64::from(face.descender()))));
    desc.set(b"CapHeight".to_vec(), Object::Int(em(f64::from(face.capital_height().unwrap_or(face.ascender())))));
    if desc.get(b"StemV").is_none() {
        desc.set(b"StemV".to_vec(), Object::Int(if m.want.bold { 140 } else { 80 }));
    }
    desc.set(b"FontFile2".to_vec(), Object::Ref(file));
    let desc = doc.add(Object::Dict(desc));

    out.set(b"Subtype".to_vec(), Object::name("TrueType"));
    out.set(b"BaseFont".to_vec(), Object::name(&name));
    out.set(b"Encoding".to_vec(), Object::Dict(enc));
    out.set(b"FontDescriptor".to_vec(), Object::Ref(desc));
    if font.get(b"ToUnicode").is_none() {
        out.set(b"ToUnicode".to_vec(), Object::Ref(to_unicode(doc, &m.chars)));
    }
    Some(out)
}

/// The Annex D name of `c`, when PdfCraft's glyph list has one (reverse of `glyph_unicode`,
/// for the letters a font's codes commonly show).
fn standard_name(c: char) -> Option<String> {
    const NAMES: &[(char, &str)] = &[
        ('’', "quoteright"),
        ('‘', "quoteleft"),
        ('ı', "dotlessi"),
        ('Ł', "Lslash"),
        ('ł', "lslash"),
        ('ﬁ', "fi"),
        ('ﬂ', "fl"),
        ('⁄', "fraction"),
        ('ˇ', "caron"),
        ('˘', "breve"),
        ('˙', "dotaccent"),
        ('˚', "ring"),
        ('˛', "ogonek"),
        ('˝', "hungarumlaut"),
        ('−', "minus"),
    ];
    NAMES.iter().find(|(ch, _)| *ch == c).map(|(_, n)| (*n).to_string())
}

/// A ToUnicode CMap for a simple font whose codes show `chars`.
fn to_unicode(doc: &mut Document, chars: &[Option<char>; 256]) -> ObjRef {
    let entries: Vec<(usize, char)> = chars.iter().enumerate().filter_map(|(code, c)| c.map(|c| (code, c))).collect();
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /MoStandIn-UCS def /CMapType 2 def 1 begincodespacerange <00> <FF> endcodespacerange\n",
    );
    for block in entries.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", block.len()));
        for (code, c) in block {
            let mut units = [0u16; 2];
            let hex: String = c.encode_utf16(&mut units).iter().map(|u| format!("{u:04X}")).collect();
            cmap.push_str(&format!("<{code:02X}> <{hex}>\n"));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap CMapName currentdict /CMap defineresource pop end end\n");
    doc.add(Object::Stream(Stream::flate(Dict::new(), cmap.as_bytes())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_what_the_built_in_fonts_draw() {
        assert!("AzÇçÖöÜüıé€“”–ﬁŁł".chars().all(built_in_draws));
        assert!("ĞğİŞşąęćźő".chars().all(|c| !built_in_draws(c)));
    }

    #[test]
    fn reads_glyph_names() {
        assert_eq!(name_char(b"gbreve"), Some('ğ'));
        assert_eq!(name_char(b"Idot"), Some('İ'));
        assert_eq!(name_char(b"Idotaccent"), Some('İ'));
        assert_eq!(name_char(b"dotlessi"), Some('ı'));
        assert_eq!(name_char(b"uni015F"), Some('ş'));
        assert_eq!(name_char(b"ccedilla"), Some('ç'));
    }

    #[test]
    fn decodes_the_base_encodings() {
        assert_eq!(standard(0x27), Some('’'));
        assert_eq!(standard(0xF5), Some('ı'));
        assert_eq!(standard(0xE8), Some('Ł'));
        assert_eq!(standard(0xFB), Some('ß'));
        assert_eq!(win_ansi(0x80), Some('€'));
        assert_eq!(win_ansi(0x81), None);
        assert_eq!(win_ansi(0xFD), Some('ý'));
        assert_eq!(mac_roman(0x8D), Some('ç'));
        assert_eq!(mac_roman(0xF5), Some('ı'));
    }

    #[test]
    fn bundled_stand_ins_cover_every_class() {
        for class in [Class::Sans, Class::Serif, Class::Mono] {
            assert!(bundled().iter().any(|c| c.class == class), "{class:?}");
        }
    }
}
