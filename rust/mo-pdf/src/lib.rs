//! PDF page tools over the PdfCraft engine: rotate, delete, move and extract pages, combine
//! files, and put a hand-drawn signature on a page. Viewing stays with Android's PdfRenderer (docs/adr/0003-pdf-goruntuleme.md); this bridge
//! edits the document's structure and writes a whole new file, never the opened one in place.

use std::sync::Arc;

use pdfcraft_annot as annot;
use pdfcraft_cos::{Document, SaveOptions};
use pdfcraft_organize as organize;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PdfError {
    #[error("{0}")]
    Open(String),
    #[error("{0}")]
    Edit(String),
    #[error("{0}")]
    Save(String),
}

/// One open PDF being edited.
pub struct PdfDoc {
    doc: Document,
}

fn edit<E: std::fmt::Display>(e: E) -> PdfError {
    PdfError::Edit(e.to_string())
}

impl PdfDoc {
    pub fn open(bytes: Vec<u8>, password: Option<&str>) -> Result<PdfDoc, PdfError> {
        let doc = Document::open_with_password(Arc::new(bytes), password).map_err(|e| PdfError::Open(e.to_string()))?;
        Ok(PdfDoc { doc })
    }

    pub fn page_count(&self) -> Result<u32, PdfError> {
        Ok(organize::page_count(&self.doc).map_err(edit)? as u32)
    }

    /// Turns `pages` (0-based) by `degrees` (a multiple of 90, clockwise).
    pub fn rotate(&mut self, pages: &[u32], degrees: i32) -> Result<(), PdfError> {
        organize::rotate_pages(&mut self.doc, &indices(pages), i64::from(degrees)).map_err(edit)
    }

    /// The page's /Rotate, 0–270.
    pub fn rotation(&self, page: u32) -> Result<i32, PdfError> {
        Ok(organize::page_rotation(&self.doc, page as usize).map_err(edit)? as i32)
    }

    /// Removes `pages`; a document keeps at least one page.
    pub fn delete(&mut self, pages: &[u32]) -> Result<(), PdfError> {
        if pages.len() as u32 >= self.page_count()? {
            return Err(PdfError::Edit("a PDF needs at least one page".into()));
        }
        organize::delete_pages(&mut self.doc, &indices(pages)).map_err(edit)
    }

    /// Moves `pages` so the first of them lands at position `to` (0-based, in the new order).
    pub fn move_pages(&mut self, pages: &[u32], to: u32) -> Result<(), PdfError> {
        organize::move_pages(&mut self.doc, &indices(pages), to as usize).map_err(edit)
    }

    /// Appends all pages of another PDF (fields renamed where names clash).
    pub fn append(&mut self, other: Vec<u8>, password: Option<&str>) -> Result<(), PdfError> {
        let src = Document::open_with_password(Arc::new(other), password).map_err(|e| PdfError::Open(e.to_string()))?;
        let n = organize::page_count(&src).map_err(edit)?;
        let at = organize::page_count(&self.doc).map_err(edit)?;
        let all: Vec<usize> = (0..n).collect();
        organize::import_pages_with(&mut self.doc, &src, &all, at, organize::FieldConflicts::Rename).map_err(edit)?;
        Ok(())
    }

    /// A new PDF with only `pages`, in the given order; this document is unchanged.
    pub fn extract(&self, pages: &[u32]) -> Result<Vec<u8>, PdfError> {
        let mut out = Document::open(Arc::new(self.save()?)).map_err(|e| PdfError::Save(e.to_string()))?;
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
        let page = page as usize;
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let crop = boxes.get(page).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let turn = organize::page_rotation(&self.doc, page).map_err(edit)?;
        let strokes: Vec<Vec<[f64; 2]>> =
            strokes.iter().filter(|s| !s.is_empty()).map(|s| s.iter().map(|p| user_point(crop, turn, *p)).collect()).collect();
        if strokes.is_empty() {
            return Err(PdfError::Edit("the signature is empty".into()));
        }
        let shape = annot::Shape::Signature { strokes };
        let style = annot::Style { width: 1.5, ..annot::Style::default_for(&shape) };
        let new = annot::NewAnnotation { page, shape, style, contents: String::new(), author: String::new() };
        let meta = annot::Meta { date: None, id: String::new() };
        annot::add_annotation(&mut self.doc, &new, &meta).map_err(edit)?;
        Ok(())
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
