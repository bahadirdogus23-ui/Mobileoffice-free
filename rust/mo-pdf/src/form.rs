//! Filling PDF forms (AcroForm) through PdfCraft, with Turkish letters drawn.
//!
//! A field's text is drawn in the font its `/DA` names, usually the standard Helvetica in
//! WinAnsiEncoding, which has no Ğ ğ İ ı Ş ş: PdfCraft would mark the field for the viewer to
//! redraw, and Android's renderer does not. Text with such letters is drawn instead in an embedded
//! TrueType font (the app's bundled Source Sans 3) in WinAnsiEncoding with the Turkish letters at
//! their Windows-1254 codes, the encoding the spreadsheet PDFs use (vendor/gridcraft-pdf).

use pdfcraft_cos::{Dict, Document, Object, PdfString, Stream};
use pdfcraft_forms as forms;

use crate::PdfError;

/// The `/DR` font name of the embedded Turkish font.
pub(crate) const TURKISH_FONT: &str = "MoTr";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    CheckBox,
    Radio,
    /// A combo box or list box.
    Choice,
    /// Buttons and signature fields: shown, not filled here.
    Other,
}

/// One form field as the person fills it.
#[derive(Clone, Debug, PartialEq)]
pub struct FormField {
    /// Fully qualified name; the key for filling.
    pub name: String,
    /// What a person reads: the tooltip (`/TU`) when there is one, else the name.
    pub label: String,
    pub kind: FieldKind,
    /// Text: the text. Radio / choice: the chosen export value. Check box: "" or its on state.
    pub value: String,
    /// Radio and choice: (export value, text shown).
    pub options: Vec<(String, String)>,
    pub read_only: bool,
    pub multiline: bool,
    /// 0-based page of its first widget.
    pub page: Option<u32>,
}

pub(crate) fn fields(doc: &Document) -> Vec<FormField> {
    forms::fields(doc)
        .into_iter()
        .map(|f| {
            let kind = match f.kind {
                forms::FieldKind::Text => FieldKind::Text,
                forms::FieldKind::CheckBox => FieldKind::CheckBox,
                forms::FieldKind::Radio => FieldKind::Radio,
                forms::FieldKind::Combo | forms::FieldKind::List => FieldKind::Choice,
                forms::FieldKind::PushButton | forms::FieldKind::Signature => FieldKind::Other,
            };
            let options = match kind {
                FieldKind::Radio => {
                    let mut seen = Vec::<(String, String)>::new();
                    for on in f.widgets.iter().filter_map(|w| w.on_state.clone()) {
                        if !seen.iter().any(|(v, _)| *v == on) {
                            seen.push((on.clone(), on));
                        }
                    }
                    seen
                }
                FieldKind::Choice => f.options.clone(),
                _ => Vec::new(),
            };
            FormField {
                label: f.tooltip.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| f.name.clone()),
                value: f.value.first().cloned().unwrap_or_default(),
                read_only: f.read_only(),
                multiline: kind == FieldKind::Text && f.has(forms::flags::MULTILINE),
                page: f.widgets.iter().find_map(|w| w.page).map(|p| p as u32),
                name: f.name,
                kind,
                options,
            }
        })
        .collect()
}

fn form_error(e: forms::FormError) -> PdfError {
    PdfError::Edit(e.to_string())
}

pub(crate) fn set_text(doc: &mut Document, name: &str, text: &str) -> Result<(), PdfError> {
    if !text.chars().all(|c| matches!(c, '\n' | '\r') || pdfcraft_fonts_drawable(c)) {
        use_turkish_font(doc, name)?;
    }
    forms::set_value(doc, name, &forms::FieldValue::Text(text.to_string())).map_err(form_error)
}

pub(crate) fn set_check(doc: &mut Document, name: &str, on: bool) -> Result<(), PdfError> {
    forms::set_value(doc, name, &forms::FieldValue::Check(on)).map_err(form_error)
}

/// Picks `value` (an export value; empty picks none) in a radio group or a choice field.
pub(crate) fn set_choice(doc: &mut Document, name: &str, value: &str) -> Result<(), PdfError> {
    let field = forms::fields(doc).into_iter().find(|f| f.name == name).ok_or_else(|| PdfError::Edit(format!("no field {name:?}")))?;
    let v = match field.kind {
        forms::FieldKind::Radio => forms::FieldValue::Radio((!value.is_empty()).then(|| value.to_string())),
        _ => forms::FieldValue::Choice(if value.is_empty() { Vec::new() } else { vec![value.to_string()] }),
    };
    forms::set_value(doc, name, &v).map_err(form_error)
}

/// Whether the standard fonts in WinAnsiEncoding can draw `c`.
fn pdfcraft_fonts_drawable(c: char) -> bool {
    c == '?' || (c as u32) < 0x80 || (gridcraft_pdf::win_ansi(c) != b'?' && !TURKISH.contains(&c))
}

const TURKISH: [char; 6] = ['Ğ', 'ğ', 'İ', 'ı', 'Ş', 'ş'];

/// Points field `name` (and its widgets) at the embedded Turkish font, adding the font to the
/// form's resources the first time.
fn use_turkish_font(doc: &mut Document, name: &str) -> Result<(), PdfError> {
    add_turkish_font(doc)?;
    let field = forms::fields(doc).into_iter().find(|f| f.name == name).ok_or_else(|| PdfError::Edit(format!("no field {name:?}")))?;
    let da = with_font(&field.da, TURKISH_FONT);
    let da_obj = Object::String(PdfString::literal(da.into_bytes()));
    doc.update_dict(field.obj, |d| d.set(b"DA".to_vec(), da_obj.clone())).map_err(|e| PdfError::Edit(e.to_string()))?;
    for w in field.widgets.iter().filter(|w| w.obj != field.obj) {
        let has_own = doc.get(w.obj).as_dict().is_some_and(|d| d.get(b"DA").is_some());
        if has_own {
            doc.update_dict(w.obj, |d| d.set(b"DA".to_vec(), da_obj.clone())).map_err(|e| PdfError::Edit(e.to_string()))?;
        }
    }
    Ok(())
}

/// `da` (a default appearance like `/Helv 0 Tf 0 g`) with its font replaced by `font`.
fn with_font(da: &str, font: &str) -> String {
    let tokens: Vec<&str> = da.split_whitespace().collect();
    match tokens.iter().rposition(|t| *t == "Tf") {
        Some(i) if i >= 2 && tokens[i - 2].starts_with('/') => {
            let mut out: Vec<String> = tokens.iter().map(|t| t.to_string()).collect();
            out[i - 2] = format!("/{font}");
            out.join(" ")
        }
        _ => format!("/{font} 0 Tf 0 g {da}").trim().to_string(),
    }
}

/// Adds the embedded Turkish font to the form's `/DR /Font`, once.
fn add_turkish_font(doc: &mut Document) -> Result<(), PdfError> {
    let root = doc.root().ok_or_else(|| PdfError::Edit("no catalog".into()))?;
    let entry = doc.get(root).as_dict().and_then(|c| c.get(b"AcroForm").cloned()).ok_or_else(|| PdfError::Edit("no form".into()))?;
    let (af_ref, mut af) = match entry {
        Object::Ref(r) => (Some(r), doc.get(r).as_dict().cloned().unwrap_or_default()),
        Object::Dict(d) => (None, d),
        _ => return Err(PdfError::Edit("no form".into())),
    };
    let mut dr = af.get(b"DR").map(|o| doc.resolve(o)).and_then(|o| o.as_dict().cloned()).unwrap_or_default();
    let mut fonts = dr.get(b"Font").map(|o| doc.resolve(o)).and_then(|o| o.as_dict().cloned()).unwrap_or_default();
    if fonts.get(TURKISH_FONT.as_bytes()).is_some() {
        return Ok(());
    }
    let font = turkish_font(doc).ok_or_else(|| PdfError::Edit("the Turkish font could not be read".into()))?;
    fonts.set(TURKISH_FONT.as_bytes().to_vec(), Object::Ref(font));
    match dr.get(b"Font").and_then(Object::as_ref) {
        Some(r) => doc.set(r, Object::Dict(fonts)),
        None => dr.set(b"Font".to_vec(), Object::Dict(fonts)),
    }
    match af.get(b"DR").and_then(Object::as_ref) {
        Some(r) => doc.set(r, Object::Dict(dr)),
        None => af.set(b"DR".to_vec(), Object::Dict(dr)),
    }
    match af_ref {
        Some(r) => doc.set(r, Object::Dict(af)),
        None => {
            let mut catalog = doc.get(root).as_dict().cloned().unwrap_or_default();
            catalog.set(b"AcroForm".to_vec(), Object::Dict(af));
            doc.set(root, Object::Dict(catalog));
        }
    }
    Ok(())
}

/// Source Sans 3 Regular as a simple TrueType font in the Turkish text encoding.
fn turkish_font(doc: &mut Document) -> Option<pdfcraft_cos::ObjRef> {
    let data: &[u8] = wordcraft_fonts::bundled().first()?;
    let face = ttf_parser::Face::parse(data, 0).ok()?;
    let upem = f64::from(face.units_per_em().max(1));
    let em = |v: f64| (v * 1000.0 / upem).round() as i64;
    let widths: Vec<Object> = (32u8..=255)
        .map(|code| {
            let w = gridcraft_pdf::code_char(code)
                .and_then(|c| face.glyph_index(c))
                .and_then(|g| face.glyph_hor_advance(g))
                .map_or(0, |a| em(f64::from(a)));
            Object::Int(w)
        })
        .collect();
    let name: String = face
        .names()
        .into_iter()
        .find(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
        .and_then(|n| n.to_string())
        .unwrap_or_else(|| "SourceSans3-Regular".into())
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let b = face.global_bounding_box();
    let num = |v: i64| Object::Int(v);

    let mut file = Dict::new();
    file.set(b"Length1".to_vec(), num(data.len() as i64));
    let file = doc.add(Object::Stream(Stream::from_raw(
        {
            file.set(b"Filter".to_vec(), Object::name("FlateDecode"));
            file
        },
        miniz_oxide::deflate::compress_to_vec_zlib(data, 6),
    )));

    let mut desc = Dict::new();
    desc.set(b"Type".to_vec(), Object::name("FontDescriptor"));
    desc.set(b"FontName".to_vec(), Object::name(&name));
    desc.set(b"Flags".to_vec(), num(32));
    desc.set(
        b"FontBBox".to_vec(),
        Object::Array(vec![num(em(f64::from(b.x_min))), num(em(f64::from(b.y_min))), num(em(f64::from(b.x_max))), num(em(f64::from(b.y_max)))]),
    );
    desc.set(b"ItalicAngle".to_vec(), num(0));
    desc.set(b"Ascent".to_vec(), num(em(f64::from(face.ascender()))));
    desc.set(b"Descent".to_vec(), num(em(f64::from(face.descender()))));
    desc.set(b"CapHeight".to_vec(), num(em(f64::from(face.capital_height().unwrap_or(face.ascender())))));
    desc.set(b"StemV".to_vec(), num(80));
    desc.set(b"FontFile2".to_vec(), Object::Ref(file));
    let desc = doc.add(Object::Dict(desc));

    let mut enc = Dict::new();
    enc.set(b"Type".to_vec(), Object::name("Encoding"));
    enc.set(b"BaseEncoding".to_vec(), Object::name("WinAnsiEncoding"));
    let mut diffs = Vec::new();
    for (code, glyph) in [(208, "Gbreve"), (221, "Idotaccent"), (222, "Scedilla"), (240, "gbreve"), (253, "dotlessi"), (254, "scedilla")] {
        diffs.push(num(code));
        diffs.push(Object::name(glyph));
    }
    enc.set(b"Differences".to_vec(), Object::Array(diffs));

    let mut font = Dict::new();
    font.set(b"Type".to_vec(), Object::name("Font"));
    font.set(b"Subtype".to_vec(), Object::name("TrueType"));
    font.set(b"BaseFont".to_vec(), Object::name(&name));
    font.set(b"FirstChar".to_vec(), num(32));
    font.set(b"LastChar".to_vec(), num(255));
    font.set(b"Widths".to_vec(), Object::Array(widths));
    font.set(b"Encoding".to_vec(), Object::Dict(enc));
    font.set(b"FontDescriptor".to_vec(), Object::Ref(desc));
    // Readers that know only some glyph names (PdfCraft lacks Gbreve, Idotaccent, Scedilla…)
    // learn the Turkish codes from this map.
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /MoTr-UCS def /CMapType 2 def 1 begincodespacerange <00> <FF> endcodespacerange 6 beginbfchar\n",
    );
    for (code, c) in [(0xD0, 'Ğ'), (0xDD, 'İ'), (0xDE, 'Ş'), (0xF0, 'ğ'), (0xFD, 'ı'), (0xFE, 'ş')] {
        cmap.push_str(&format!("<{code:02X}> <{:04X}>\n", c as u32));
    }
    cmap.push_str("endbfchar endcmap CMapName currentdict /CMap defineresource pop end end\n");
    let to_unicode = doc.add(Object::Stream(Stream::flate(Dict::new(), cmap.as_bytes())));
    font.set(b"ToUnicode".to_vec(), Object::Ref(to_unicode));
    Some(doc.add(Object::Dict(font)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_font_in_a_default_appearance() {
        assert_eq!(with_font("/Helv 0 Tf 0 g", "MoTr"), "/MoTr 0 Tf 0 g");
        assert_eq!(with_font("0 0 1 rg /TiRo 12 Tf", "MoTr"), "0 0 1 rg /MoTr 12 Tf");
        assert_eq!(with_font("", "MoTr"), "/MoTr 0 Tf 0 g");
    }

    #[test]
    fn knows_which_letters_need_the_embedded_font() {
        assert!("Çiçek ÖÜü é€".chars().all(pdfcraft_fonts_drawable));
        assert!(TURKISH.iter().all(|c| !pdfcraft_fonts_drawable(*c)));
    }
}
