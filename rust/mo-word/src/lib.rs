//! Word-processing bridge: opens a document with the WordCraft engine, lays it out into pages and
//! renders pages (or parts of pages) to premultiplied RGBA, the layout Android's ARGB_8888
//! `Bitmap` uses. Editing will go through WordCraft's command registry, as for spreadsheets.

mod find;

pub use find::{Hit, MAX_MATCHES, fold};
use wordcraft_doc::{Document, SectionProps};
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions};
use wordcraft_render::{RenderOptions, render_page, render_region};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WordError {
    #[error("{0}")]
    Open(String),
    #[error("{0}")]
    Save(String),
    #[error("page {0} does not exist")]
    NoPage(u32),
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
    doc: Document,
    cache: LayoutCache,
    layout: DocLayout,
}

impl WordDoc {
    /// Opens DOCX/DOCM/DOTX, DOC, ODT, RTF, HTML, Markdown or text bytes; `name` is the file name.
    pub fn open(name: &str, bytes: &[u8], password: Option<&str>) -> Result<WordDoc, WordError> {
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
        let mut cache = LayoutCache::new();
        let layout = wordcraft_layout::layout(&doc, &mut cache, &LayoutOptions::default());
        WordDoc { doc, cache, layout }
    }

    /// The document written in the format `file_name`'s extension names: pdf, docx, odt, rtf,
    /// txt, md, html… (anything WordCraft writes).
    pub fn export(&self, file_name: &str) -> Result<Vec<u8>, WordError> {
        wordcraft_engine::io::save_bytes(file_name, &self.doc).map_err(WordError::Save)
    }

    /// Lays the document out again (after an edit).
    pub fn relayout(&mut self) {
        self.layout = wordcraft_layout::layout(&self.doc, &mut self.cache, &LayoutOptions::default());
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
        let r = render_page(&self.doc, p, scale, &options(dark));
        Ok(Raster { width: r.width, height: r.height, pixels: r.pixels })
    }

    /// A `width`×`height` tile of the page at `scale`, whose top-left corner is pixel `(x, y)` of
    /// the whole page at that scale. Zoomed-in views draw only the tiles on screen.
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile(&self, page: u32, scale: f32, x: f32, y: f32, width: u32, height: u32, dark: bool) -> Result<Raster, WordError> {
        let p = self.layout.pages.get(page as usize).ok_or(WordError::NoPage(page))?;
        let view = kurbo_affine(scale, x, y);
        let r = render_region(&self.doc, p, width, height, view, &options(dark));
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
