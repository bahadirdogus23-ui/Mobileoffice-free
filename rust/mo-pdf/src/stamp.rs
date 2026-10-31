//! Text typed onto a page: a FreeText annotation (typewriter, no border) whose appearance is
//! drawn in an embedded font that has the Turkish letters, so it shows and prints the same in
//! every reader, PdfCraft's standard Helvetica appearance having no Ğ ğ İ ı Ş ş.

use pdfcraft_annot as annot;
use pdfcraft_cos::{Dict, Document, Object, PdfString, Stream};

use crate::PdfError;
use crate::form::embedded_font;

/// The typefaces text can be typed in (all bundled, all with the Turkish letters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFont {
    Sans,
    SansBold,
    Serif,
    SerifBold,
    Mono,
}

impl TextFont {
    fn data(self) -> Option<&'static [u8]> {
        let i = match self {
            TextFont::Sans => 0,
            TextFont::SansBold => 2,
            TextFont::Serif => 4,
            TextFont::SerifBold => 7,
            TextFont::Mono => 12,
        };
        wordcraft_fonts::bundled().get(i).copied()
    }
}

/// Line spacing, in font sizes.
const LEADING: f64 = 1.25;

/// Adds `text` (lines split at `\n`) on page `page`, its top-left corner at `top_left` in user
/// space; returns the annotation's place in the page's `/Annots`.
pub(crate) fn add_text(
    doc: &mut Document,
    page: usize,
    top_left: [f64; 2],
    text: &str,
    size: f64,
    font: TextFont,
    color: [f64; 3],
) -> Result<usize, PdfError> {
    let edit = |e: &dyn std::fmt::Display| PdfError::Edit(e.to_string());
    let text = text.replace("\r\n", "\n");
    if text.trim().is_empty() {
        return Err(PdfError::Edit("the text is empty".into()));
    }
    let size = size.clamp(4.0, 144.0);
    let data = font.data().ok_or_else(|| PdfError::Edit("the font could not be read".into()))?;
    let face = ttf_parser::Face::parse(data, 0).map_err(|e| edit(&e))?;
    let upem = f64::from(face.units_per_em().max(1));
    let advance = |c: char| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map_or(upem / 2.0, f64::from) / upem * size;
    let lines: Vec<&str> = text.split('\n').collect();
    let width = lines.iter().map(|l| l.chars().map(advance).sum::<f64>()).fold(0.0, f64::max).max(size);
    let height = lines.len() as f64 * size * LEADING;
    let ascent = f64::from(face.ascender()) / upem * size;
    let [x, top] = top_left;
    let rect = [x, top - height, x + width, top];

    // The annotation itself, with ASCII contents (PdfCraft refuses text its fonts can't draw);
    // its contents, look and appearance are then replaced.
    let shape = annot::Shape::Typewriter { rect, font_size: size };
    let style = annot::Style { color, ..annot::Style::default_for(&shape) };
    let new = annot::NewAnnotation { page, shape, style, contents: "-".into(), author: String::new() };
    let index = annot::add_annotation(doc, &new, &annot::Meta { date: None, id: String::new() }).map_err(|e| edit(&e))?;
    let page_ref = *annot::page_refs(doc).map_err(|e| edit(&e))?.get(page).ok_or_else(|| PdfError::Edit("no such page".into()))?;
    let list = doc.get(page_ref).as_dict().and_then(|d| d.get(b"Annots").cloned()).map(|o| doc.resolve(&o));
    let annot_ref = list
        .as_ref()
        .and_then(|o| o.as_array())
        .and_then(|a| a.get(index))
        .and_then(Object::as_ref)
        .ok_or_else(|| PdfError::Edit("the text box was not added".into()))?;

    let font_ref = embedded_font(doc, data).ok_or_else(|| PdfError::Edit("the font could not be read".into()))?;
    let n = |v: f64| format!("{:.3}", v).trim_end_matches('0').trim_end_matches('.').to_string();
    let mut content =
        format!("q {} {} {} rg BT /F1 {} Tf {} TL 0 {} Td\n", n(color[0]), n(color[1]), n(color[2]), n(size), n(size * LEADING), n(height - ascent))
            .into_bytes();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            content.extend_from_slice(b"T* ");
        }
        content.push(b'(');
        for c in line.chars() {
            let b = gridcraft_pdf::win_ansi(c);
            if matches!(b, b'(' | b')' | b'\\') {
                content.push(b'\\');
            }
            content.push(b);
        }
        content.extend_from_slice(b") Tj\n");
    }
    content.extend_from_slice(b"ET Q\n");

    let mut fonts = Dict::new();
    fonts.set(b"F1".to_vec(), Object::Ref(font_ref));
    let mut resources = Dict::new();
    resources.set(b"Font".to_vec(), Object::Dict(fonts));
    let mut form = Dict::new();
    form.set(b"Type".to_vec(), Object::name("XObject"));
    form.set(b"Subtype".to_vec(), Object::name("Form"));
    form.set(b"BBox".to_vec(), Object::Array([0.0, 0.0, width, height].iter().map(|v| Object::Real(*v)).collect()));
    form.set(b"Resources".to_vec(), Object::Dict(resources));
    let ap_stream = doc.add(Object::Stream(Stream::flate(form, &content)));
    let mut ap = Dict::new();
    ap.set(b"N".to_vec(), Object::Ref(ap_stream));

    let da = format!("/F1 {} Tf {} {} {} rg", n(size), n(color[0]), n(color[1]), n(color[2]));
    doc.update_dict(annot_ref, |d| {
        d.set(b"AP".to_vec(), Object::Dict(ap.clone()));
        d.set(b"Contents".to_vec(), Object::String(PdfString::text(&text)));
        d.set(b"DA".to_vec(), Object::String(PdfString::literal(da.clone().into_bytes())));
        d.remove(pdfcraft_fonts::UNDRAWN_MARK);
    })
    .map_err(|e| edit(&e))?;
    Ok(index)
}
