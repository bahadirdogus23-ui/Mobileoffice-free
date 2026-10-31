//! Word-processing bridge: opens a document with the WordCraft engine, lays it out into pages and
//! renders pages (or parts of pages) to premultiplied RGBA, the layout Android's ARGB_8888
//! `Bitmap` uses. Editing goes through WordCraft's editing session and its command registry
//! (`text.insert`, `format.bold`, …), as for spreadsheets: nothing here edits the document itself.

mod find;
pub mod udf;

pub use find::{Hit, MAX_MATCHES, fold};
use std::sync::Arc;

use wordcraft_doc::{Document, SectionProps};
use wordcraft_engine::Session;
use wordcraft_layout::DocLayout;
use wordcraft_render::{RenderOptions, render_page, render_region};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WordError {
    #[error("{0}")]
    Open(String),
    #[error("{0}")]
    Save(String),
    #[error("page {0} does not exist")]
    NoPage(u32),
    #[error("{0}")]
    Command(String),
}

/// Where the caret is drawn: its page, and its left edge, top and height in points.
#[derive(Debug, Clone, PartialEq)]
pub struct CaretBox {
    pub page: u32,
    pub x: f32,
    pub top: f32,
    pub height: f32,
}

/// One rectangle of the selection on a page, in points.
#[derive(Debug, Clone, PartialEq)]
pub struct PageBox {
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Page setup for [`WordDoc::from_text`], in points (1/72 inch).
#[derive(Debug, Clone, PartialEq)]
pub struct TextPage {
    pub width_pt: f32,
    pub height_pt: f32,
    pub landscape: bool,
    pub margin_pt: f32,
    pub font: String,
    pub size_pt: f32,
}

/// A page picture: `width`×`height` premultiplied RGBA8, row by row.
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// One open document and its current layout.
pub struct WordDoc {
    /// The engine's editing session; it owns the document, the selection and the undo history.
    session: Session,
    layout: Arc<DocLayout>,
    signed: bool,
}

impl WordDoc {
    /// Opens DOCX/DOCM/DOTX, DOC, ODT, RTF, HTML, Markdown or text bytes; `name` is the file name.
    pub fn open(name: &str, bytes: &[u8], password: Option<&str>) -> Result<WordDoc, WordError> {
        // UYAP UDF: WordCraft has no reader for it; udf.rs turns it into a document.
        if name.to_ascii_lowercase().ends_with(".udf") {
            let udf = udf::read(bytes).map_err(WordError::Open)?;
            let mut doc = WordDoc::from_document(udf.doc);
            doc.signed = udf.signed;
            return Ok(doc);
        }
        let (doc, _encrypted) = wordcraft_engine::io::open_bytes_with(name, bytes, password).map_err(WordError::Open)?;
        Ok(WordDoc::from_document(doc))
    }

    /// Plain text laid out on `page`: one paragraph per line, in `page.font` at `page.size_pt`.
    /// The bytes may be UTF-8, UTF-16 with a BOM or a Windows code page (WordCraft decides).
    pub fn from_text(bytes: &[u8], page: &TextPage) -> WordDoc {
        let (long, short) = (page.width_pt.max(page.height_pt), page.width_pt.min(page.height_pt));
        let sp = SectionProps {
            page_w: if page.landscape { long } else { short },
            page_h: if page.landscape { short } else { long },
            landscape: page.landscape,
            margin_top: page.margin_pt,
            margin_bottom: page.margin_pt,
            margin_left: page.margin_pt,
            margin_right: page.margin_pt,
            ..SectionProps::default()
        };
        let mut doc = wordcraft_formats::txt::import_on(bytes, &sp);
        if let Some(normal) = doc.styles.get_mut("Normal") {
            normal.chr.font = Some(page.font.clone());
            normal.chr.size = Some(page.size_pt);
        }
        WordDoc::from_document(doc)
    }

    fn from_document(doc: Document) -> WordDoc {
        let mut session = Session::new(doc);
        let layout = session.layout();
        WordDoc { session, layout, signed: false }
    }

    fn doc(&self) -> &Document {
        &self.session.doc
    }

    /// Runs the engine command `id` with JSON `params` (`{}` for none) and lays the document out
    /// again. Returns the command's JSON result.
    pub fn execute(&mut self, id: &str, params: &str) -> Result<String, WordError> {
        let params: serde_json::Value = if params.trim().is_empty() {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::from_str(params).map_err(|e| WordError::Command(e.to_string()))?
        };
        let out = self.session.run(id, &params).map_err(|e| WordError::Command(e.to_string()));
        // Even a refused command may have moved the caret; the layout follows the session.
        self.layout = self.session.layout();
        Ok(out?.to_string())
    }

    /// Puts the caret at (`x`, `y`) points on `page`, or extends the selection there.
    pub fn tap(&mut self, page: u32, x: f32, y: f32, extend: bool) -> Result<(), WordError> {
        let params = serde_json::json!({ "page": page, "x": x, "y": y, "extend": extend });
        self.execute("caret.set", &params.to_string()).map(|_| ())
    }

    /// Where the caret is drawn, if it is on a page.
    pub fn caret(&self) -> Option<CaretBox> {
        let c = self.layout.caret_on(&self.session.sel.focus, self.session.page_hint)?;
        Some(CaretBox { page: c.page as u32, x: c.x, top: c.top, height: c.height })
    }

    /// The selected text's rectangles (none when the selection is just a caret).
    pub fn selection(&self) -> Vec<PageBox> {
        let (a, b) = self.session.sel.ordered();
        if a == b {
            return Vec::new();
        }
        self.layout
            .selection_rects(self.doc(), &a, &b, self.session.page_hint)
            .into_iter()
            .map(|(page, r)| PageBox { page: page as u32, x: r.x, y: r.y, w: r.w, h: r.h })
            .collect()
    }

    /// The selected text.
    pub fn selected_text(&self) -> String {
        self.session.selected_text()
    }

    /// Changes not yet saved.
    pub fn is_dirty(&self) -> bool {
        self.session.dirty
    }

    /// The saved state is now the current one (after the person saved).
    pub fn mark_saved(&mut self) {
        self.session.dirty = false;
    }

    /// The file carries an e-signature (a UDF's `sign.sgn`); it is not verified.
    pub fn is_signed(&self) -> bool {
        self.signed
    }

    /// The document written in the format `file_name`'s extension names: pdf, docx, odt, rtf,
    /// txt, md, html… (anything WordCraft writes).
    pub fn export(&self, file_name: &str) -> Result<Vec<u8>, WordError> {
        wordcraft_engine::io::save_bytes(file_name, self.doc()).map_err(WordError::Save)
    }

    /// Lays the document out again (after an edit).
    pub fn relayout(&mut self) {
        self.session.relayout();
        self.layout = self.session.layout();
    }

    pub fn page_count(&self) -> u32 {
        self.layout.pages.len() as u32
    }

    /// Page size in points.
    pub fn page_size(&self, page: u32) -> Result<(f32, f32), WordError> {
        let p = self.layout.pages.get(page as usize).ok_or(WordError::NoPage(page))?;
        Ok((p.w, p.h))
    }

    /// The whole page at `scale` pixels per point. A `dark` page has its colours' lightness
    /// inverted (white paper turns dark grey, black text light) while pictures stay as they are.
    pub fn render_page(&self, page: u32, scale: f32, dark: bool) -> Result<Raster, WordError> {
        let p = self.layout.pages.get(page as usize).ok_or(WordError::NoPage(page))?;
        let r = render_page(self.doc(), p, scale, &options(dark));
        Ok(Raster { width: r.width, height: r.height, pixels: r.pixels })
    }

    /// A `width`×`height` tile of the page at `scale`, whose top-left corner is pixel `(x, y)` of
    /// the whole page at that scale. Zoomed-in views draw only the tiles on screen.
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile(&self, page: u32, scale: f32, x: f32, y: f32, width: u32, height: u32, dark: bool) -> Result<Raster, WordError> {
        let p = self.layout.pages.get(page as usize).ok_or(WordError::NoPage(page))?;
        let view = kurbo_affine(scale, x, y);
        let r = render_region(self.doc(), p, width, height, view, &options(dark));
        Ok(Raster { width: r.width, height: r.height, pixels: r.pixels })
    }
}

fn options(dark: bool) -> RenderOptions {
    RenderOptions { dark, ..RenderOptions::default() }
}

/// Page points → tile pixels: scale, then move the tile's corner to the origin.
fn kurbo_affine(scale: f32, x: f32, y: f32) -> kurbo::Affine {
    kurbo::Affine::translate((-(x as f64), -(y as f64))) * kurbo::Affine::scale(scale as f64)
}

#[cfg(test)]
mod tests;
