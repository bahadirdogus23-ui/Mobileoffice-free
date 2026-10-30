//! PDF page tools over the PdfCraft engine: rotate, delete, move and extract pages, and combine
//! files. Viewing stays with Android's PdfRenderer (docs/adr/0003-pdf-goruntuleme.md); this bridge
//! edits the document's structure and writes a whole new file, never the opened one in place.

use std::sync::Arc;

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

    /// The whole document written afresh (compressed object streams, a new cross-reference).
    pub fn save(&self) -> Result<Vec<u8>, PdfError> {
        pdfcraft_cos::write_full(&self.doc, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }
}

fn indices(pages: &[u32]) -> Vec<usize> {
    pages.iter().map(|p| *p as usize).collect()
}

#[cfg(test)]
mod tests;
