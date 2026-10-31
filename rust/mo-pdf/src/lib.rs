//! PDF page tools over the PdfCraft engine: rotate, delete, move and extract pages, combine
//! files, put a hand-drawn signature, pen and highlighter strokes or notes on a page, add or remove password protection, and make a
//! smaller copy, and fill in forms. Viewing stays with Android's PdfRenderer (docs/adr/0003-pdf-goruntuleme.md); this bridge
//! edits the document's structure and writes a whole new file, never the opened one in place.

use std::sync::Arc;

use pdfcraft_annot as annot;
use pdfcraft_cos::{CosError, Document, SaveOptions};
use pdfcraft_organize as organize;

mod create;
mod form;
mod text;
pub use create::images_to_pdf;
pub use form::{FieldKind, FormField};
pub use text::{Sensitive, fold, turkish_id_ok};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PdfError {
    #[error("{0}")]
    Open(String),
    #[error("the document is protected by a password")]
    NeedsPassword,
    #[error("the password is incorrect")]
    WrongPassword,
    #[error("{0}")]
    Edit(String),
    #[error("{0}")]
    Save(String),
}

/// One open PDF being edited.
pub struct PdfDoc {
    doc: Document,
    /// (page, index in its /Annots) of what [`PdfDoc::annotate`] added, newest last; cleared when
    /// pages move or go, which changes those positions.
    added: Vec<(usize, usize)>,
    /// The text layer of the document as it is now; dropped on every change.
    text: Option<pdfcraft_render::PageRenderer>,
}

/// One place a search or a black-out found: its page and one rectangle per line, as fractions
/// of the displayed page (x right, y down): [left, top, right, bottom].
#[derive(Clone, Debug, PartialEq)]
pub struct TextHit {
    pub page: u32,
    pub rects: Vec<[f32; 4]>,
}

fn opening(e: CosError) -> PdfError {
    match e {
        CosError::NeedsPassword => PdfError::NeedsPassword,
        CosError::WrongPassword => PdfError::WrongPassword,
        e => PdfError::Open(e.to_string()),
    }
}

fn edit<E: std::fmt::Display>(e: E) -> PdfError {
    PdfError::Edit(e.to_string())
}

impl PdfDoc {
    pub fn open(bytes: Vec<u8>, password: Option<&str>) -> Result<PdfDoc, PdfError> {
        let doc = Document::open_with_password(Arc::new(bytes), password).map_err(opening)?;
        Ok(PdfDoc { doc, added: Vec::new(), text: None })
    }

    pub fn page_count(&self) -> Result<u32, PdfError> {
        Ok(organize::page_count(&self.doc).map_err(edit)? as u32)
    }

    /// Turns `pages` (0-based) by `degrees` (a multiple of 90, clockwise).
    pub fn rotate(&mut self, pages: &[u32], degrees: i32) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        organize::rotate_pages(&mut self.doc, &indices(pages), i64::from(degrees)).map_err(edit)
    }

    /// The page's /Rotate, 0–270.
    pub fn rotation(&self, page: u32) -> Result<i32, PdfError> {
        Ok(organize::page_rotation(&self.doc, page as usize).map_err(edit)? as i32)
    }

    /// Removes `pages`; a document keeps at least one page.
    pub fn delete(&mut self, pages: &[u32]) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        if pages.len() as u32 >= self.page_count()? {
            return Err(PdfError::Edit("a PDF needs at least one page".into()));
        }
        self.added.clear();
        organize::delete_pages(&mut self.doc, &indices(pages)).map_err(edit)
    }

    /// Moves `pages` so the first of them lands at position `to` (0-based, in the new order).
    pub fn move_pages(&mut self, pages: &[u32], to: u32) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        self.added.clear();
        organize::move_pages(&mut self.doc, &indices(pages), to as usize).map_err(edit)
    }

    /// Appends all pages of another PDF (fields renamed where names clash).
    pub fn append(&mut self, other: Vec<u8>, password: Option<&str>) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        let src = Document::open_with_password(Arc::new(other), password).map_err(opening)?;
        let n = organize::page_count(&src).map_err(edit)?;
        let at = organize::page_count(&self.doc).map_err(edit)?;
        let all: Vec<usize> = (0..n).collect();
        organize::import_pages_with(&mut self.doc, &src, &all, at, organize::FieldConflicts::Rename).map_err(edit)?;
        Ok(())
    }

    /// A new PDF with only `pages`, in the given order; this document is unchanged.
    pub fn extract(&self, pages: &[u32]) -> Result<Vec<u8>, PdfError> {
        self.refuse_restricted()?;
        // A copy keeps the password, if any: the part stays as protected as the whole.
        let mut out = self.doc.clone();
        let n = organize::page_count(&out).map_err(edit)?;
        let keep = indices(pages);
        if keep.is_empty() || keep.iter().any(|p| *p >= n) {
            return Err(PdfError::Edit("no such page".into()));
        }
        // Copy the chosen pages to the end in order, then drop the originals.
        let copies: Vec<usize> = keep.clone();
        organize::duplicate_pages(&mut out, &copies, Some(n)).map_err(edit)?;
        organize::delete_pages(&mut out, &(0..n).collect::<Vec<_>>()).map_err(edit)?;
        pdfcraft_cos::write_full(&out, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }

    /// Puts a hand-drawn signature on `page` (0-based). Each stroke is a list of points as
    /// fractions of the page as it is displayed (x right, y down, 0–1), so the caller need not know
    /// the page's boxes or rotation.
    pub fn sign(&mut self, page: u32, strokes: &[Vec<[f64; 2]>]) -> Result<(), PdfError> {
        self.text = None;
        let strokes = self.user_strokes(page, strokes)?;
        let shape = annot::Shape::Signature { strokes };
        let style = annot::Style { width: 1.5, ..annot::Style::default_for(&shape) };
        self.annotate(page, shape, style, String::new())
    }

    /// Pen strokes on `page` (points as in [`PdfDoc::sign`]): `color` 0–1 RGB, `width` in points,
    /// `opacity` 0–1 (a highlighter is wide, yellow and see-through).
    pub fn draw(&mut self, page: u32, strokes: &[Vec<[f64; 2]>], color: [f64; 3], width: f64, opacity: f64) -> Result<(), PdfError> {
        self.text = None;
        let strokes = self.user_strokes(page, strokes)?;
        let style = annot::Style { color, opacity, width, fill: None };
        self.annotate(page, annot::Shape::Ink { strokes }, style, String::new())
    }

    /// A sticky note holding `text`, its corner at `at` (a fraction of the displayed page).
    pub fn note(&mut self, page: u32, at: [f64; 2], text: &str) -> Result<(), PdfError> {
        self.text = None;
        let [x, y] = self.user_strokes(page, &[vec![at]])?[0][0];
        let shape = annot::Shape::Note { at: [x, y], icon: annot::NoteIcon::Comment };
        let style = annot::Style::default_for(&shape);
        self.annotate(page, shape, style, text.to_string())
    }

    /// Removes the last signature, drawing or note added since the document was opened (or
    /// since pages were last moved or removed); false when there is none. What the file
    /// already had is never removed.
    pub fn undo_annotation(&mut self) -> Result<bool, PdfError> {
        self.text = None;
        let Some((page, index)) = self.added.pop() else { return Ok(false) };
        annot::delete_annotation(&mut self.doc, page, index).map_err(edit)?;
        Ok(true)
    }

    /// Strokes given as fractions of the displayed page, in `page`'s user space.
    fn user_strokes(&self, page: u32, strokes: &[Vec<[f64; 2]>]) -> Result<Vec<Vec<[f64; 2]>>, PdfError> {
        self.refuse_restricted()?;
        let page = page as usize;
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let crop = boxes.get(page).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let turn = organize::page_rotation(&self.doc, page).map_err(edit)?;
        let strokes: Vec<Vec<[f64; 2]>> =
            strokes.iter().filter(|s| !s.is_empty()).map(|s| s.iter().map(|p| user_point(crop, turn, *p)).collect()).collect();
        if strokes.is_empty() {
            return Err(PdfError::Edit("the drawing is empty".into()));
        }
        Ok(strokes)
    }

    fn annotate(&mut self, page: u32, shape: annot::Shape, style: annot::Style, contents: String) -> Result<(), PdfError> {
        let new = annot::NewAnnotation { page: page as usize, shape, style, contents, author: String::new() };
        let meta = annot::Meta { date: None, id: String::new() };
        let index = annot::add_annotation(&mut self.doc, &new, &meta).map_err(edit)?;
        self.added.push((page as usize, index));
        Ok(())
    }

    /// Whether the file written by [`PdfDoc::save`] asks for a password.
    pub fn is_protected(&self) -> bool {
        self.doc.output_handler().is_some()
    }

    /// Whether the password it was opened with keeps it from being changed: the owner set
    /// restrictions (no editing, printing…) and only the owner's password lifts them.
    pub fn is_restricted(&self) -> bool {
        self.doc.permissions().is_some_and(|p| {
            const ALL: i32 = 0b1111_0011_1100; // bits 3–6 and 9–12 (§7.6.4.2, Table 22)
            !p.owner && p.bits & ALL != ALL
        })
    }

    /// Protects the next save with `password` (AES-256), every use allowed once it is open.
    /// `seed` is 32 random bytes for the keys.
    pub fn protect(&mut self, password: &str, seed: [u8; 32]) -> Result<(), PdfError> {
        self.text = None;
        if password.is_empty() {
            return Err(PdfError::Edit("the password is empty".into()));
        }
        self.refuse_restricted()?;
        let params = pdfcraft_cos::NewEncryption {
            algorithm: pdfcraft_cos::Algorithm::Aes256,
            user_password: password,
            owner_password: password,
            permissions: -4, // every bit allowed (§7.6.4.2)
            encrypt_metadata: true,
            seed,
        };
        self.doc.set_encryption(&params).map_err(edit)
    }

    /// The next save asks for no password.
    pub fn unprotect(&mut self) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        self.doc.remove_encryption();
        Ok(())
    }

    fn refuse_restricted(&self) -> Result<(), PdfError> {
        if self.is_restricted() {
            return Err(PdfError::Edit("the owner of this PDF does not allow changing it".into()));
        }
        Ok(())
    }

    /// The document without its password, for showing on screen (never written to the person's
    /// files): Android's renderer cannot open encrypted files.
    pub fn display_copy(&self) -> Result<Vec<u8>, PdfError> {
        if !self.is_protected() && self.doc.security().is_none() {
            return self.save();
        }
        let mut copy = self.doc.clone();
        copy.remove_encryption();
        pdfcraft_cos::write_full(&copy, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }

    /// A smaller copy: pictures resampled and recompressed (to 150 ppi, or 96 ppi `for_screen`),
    /// thumbnails dropped, streams compressed. This document is unchanged.
    pub fn compress(&self, for_screen: bool) -> Result<Vec<u8>, PdfError> {
        let preset = if for_screen { pdfcraft_optimize::Preset::Mobile } else { pdfcraft_optimize::Preset::Standard };
        let mut copy = self.doc.clone();
        pdfcraft_optimize::optimize(&mut copy, &preset.settings()).map_err(edit)?;
        pdfcraft_cos::write_full(&copy, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }

    /// The form's fields, in document order (none when there is no form).
    pub fn fields(&self) -> Vec<FormField> {
        form::fields(&self.doc)
    }

    pub fn set_text(&mut self, name: &str, text: &str) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_unfillable()?;
        form::set_text(&mut self.doc, name, text)
    }

    pub fn set_check(&mut self, name: &str, on: bool) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_unfillable()?;
        form::set_check(&mut self.doc, name, on)
    }

    /// Picks `value` (an export value from [`FormField::options`]; empty for none).
    pub fn set_choice(&mut self, name: &str, value: &str) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_unfillable()?;
        form::set_choice(&mut self.doc, name, value)
    }

    fn refuse_unfillable(&self) -> Result<(), PdfError> {
        if self.is_restricted() && self.doc.permissions().is_some_and(|p| !p.fill_forms()) {
            return Err(PdfError::Edit("the owner of this PDF does not allow filling in its form".into()));
        }
        Ok(())
    }

    /// Every place `query` occurs, ignoring case, Turkish i forms and accents (at most `limit`).
    pub fn find_text(&mut self, query: &str, limit: usize) -> Result<Vec<TextHit>, PdfError> {
        let query = query.to_string();
        self.hits(limit, move |chars| text::find_folded(chars, &query))
    }

    /// The text of `page` in reading order (for reading aloud and copying).
    pub fn page_text(&mut self, page: u32) -> Result<String, PdfError> {
        let layer = self.text_layer()?;
        Ok(layer.filtered_text(page as usize, Default::default()).map_err(PdfError::Edit)?.plain_text())
    }

    /// Marks for blacking out every place of `kinds` and of `words` (whole text, matched like a
    /// search). Nothing is removed until [`PdfDoc::apply_redactions`]. Returns the places marked.
    pub fn mark_redactions(&mut self, kinds: &[Sensitive], words: &[String]) -> Result<Vec<TextHit>, PdfError> {
        self.refuse_restricted()?;
        let kinds = kinds.to_vec();
        let words: Vec<String> = words.iter().map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect();
        let hits = self.hits(usize::MAX, move |chars| {
            let mut all: Vec<std::ops::Range<usize>> = kinds.iter().flat_map(|k| text::find_sensitive(*k, chars)).collect();
            all.extend(words.iter().flat_map(|w| text::find_folded(chars, w)));
            // One mark where finds overlap ("Şule" inside "sule@ornek.com").
            all.sort_by_key(|r| (r.start, std::cmp::Reverse(r.end)));
            let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
            for r in all {
                match merged.last_mut() {
                    Some(last) if r.start < last.end => last.end = last.end.max(r.end),
                    _ => merged.push(r),
                }
            }
            merged
        })?;
        for hit in &hits {
            let quads: Vec<[f64; 8]> = hit
                .rects
                .iter()
                .map(|r| {
                    let mut q = [0.0; 8];
                    for (i, p) in [[r[0], r[1]], [r[2], r[1]], [r[0], r[3]], [r[2], r[3]]].into_iter().enumerate() {
                        let [x, y] = self.user_strokes(hit.page, &[vec![[f64::from(p[0]), f64::from(p[1])]]]).map(|s| s[0][0])?;
                        q[2 * i] = x;
                        q[2 * i + 1] = y;
                    }
                    Ok(q)
                })
                .collect::<Result<_, PdfError>>()?;
            let shape = annot::Shape::Redact { quads, overlay: String::new(), look: Default::default() };
            let style = annot::Style::default_for(&shape);
            self.annotate(hit.page, shape, style, String::new())?;
        }
        self.text = None;
        Ok(hits)
    }

    /// Drops every black-out mark not yet applied (the text under them stays).
    pub fn clear_redaction_marks(&mut self) -> Result<u32, PdfError> {
        self.text = None;
        let n = pdfcraft_redact::clear_marks(&mut self.doc, None).map_err(edit)?;
        // Undo positions no longer hold once marks are gone.
        self.added.clear();
        Ok(n as u32)
    }

    /// Black-out marks waiting to be applied.
    pub fn redaction_marks(&self) -> u32 {
        pdfcraft_redact::marks(&self.doc).len() as u32
    }

    /// Removes the text, pictures and drawings under every black-out mark for good and paints the
    /// places black. Returns how many marks were applied.
    pub fn apply_redactions(&mut self) -> Result<u32, PdfError> {
        self.refuse_restricted()?;
        self.text = None;
        let report = pdfcraft_redact::apply(&mut self.doc, None).map_err(edit)?;
        // Undo positions no longer hold: the marks are gone.
        self.added.clear();
        Ok(report.marks as u32)
    }

    /// Matches of `matcher` over each page's text, as page fractions.
    fn hits(&mut self, limit: usize, matcher: impl Fn(&[char]) -> Vec<std::ops::Range<usize>>) -> Result<Vec<TextHit>, PdfError> {
        let n = self.page_count()?;
        let sizes: Vec<(f32, f32)> = (0..n).map(|p| self.view_size(p)).collect::<Result<_, _>>()?;
        let layer = self.text_layer()?;
        let mut out = Vec::new();
        for page in 0..n {
            // A page whose text can't be read is skipped, not an error for the whole document.
            let Ok(text) = layer.filtered_text(page as usize, Default::default()) else { continue };
            let (w, h) = sizes[page as usize];
            for range in text.find_with(&matcher) {
                let rects = text.line_rects(range).into_iter().map(|r| [r[0] / w, r[1] / h, r[2] / w, r[3] / h]).collect::<Vec<_>>();
                if !rects.is_empty() {
                    out.push(TextHit { page, rects });
                    if out.len() >= limit {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }

    /// The page as displayed, in points: its crop box turned by /Rotate.
    fn view_size(&self, page: u32) -> Result<(f32, f32), PdfError> {
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let [x0, y0, x1, y1] = boxes.get(page as usize).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
        Ok(if organize::page_rotation(&self.doc, page as usize).map_err(edit)?.rem_euclid(180) == 90 { (h, w) } else { (w, h) })
    }

    fn text_layer(&mut self) -> Result<&mut pdfcraft_render::PageRenderer, PdfError> {
        if self.text.is_none() {
            let bytes = self.display_copy()?;
            self.text = Some(pdfcraft_render::PageRenderer::new(Arc::new(bytes), Default::default()));
        }
        Ok(self.text.as_mut().expect("just set"))
    }

    /// The whole document written afresh (compressed object streams, a new cross-reference).
    pub fn save(&self) -> Result<Vec<u8>, PdfError> {
        pdfcraft_cos::write_full(&self.doc, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }
}

/// A point given as a fraction of the displayed page (y down) in the page's user space: the crop
/// box turned by the page's /Rotate (clockwise).
fn user_point(crop: [f64; 4], turn: i64, [u, v]: [f64; 2]) -> [f64; 2] {
    let [x0, y0, x1, y1] = crop;
    let (w, h) = (x1 - x0, y1 - y0);
    match turn.rem_euclid(360) {
        90 => [x0 + v * w, y0 + u * h],
        180 => [x1 - u * w, y0 + v * h],
        270 => [x1 - v * w, y1 - u * h],
        _ => [x0 + u * w, y1 - v * h],
    }
}

fn indices(pages: &[u32]) -> Vec<usize> {
    pages.iter().map(|p| *p as usize).collect()
}

#[cfg(test)]
mod tests;
