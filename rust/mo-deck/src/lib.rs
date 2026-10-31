//! Presentation bridge: reads PPTX/PPSX/POTX with DeckCraft and renders slides to premultiplied
//! RGBA, the layout Android's ARGB_8888 `Bitmap` uses. Editing (text, slides) works on the model
//! (see `edit.rs`), so the app carries neither the editing engine nor its media decoders.

mod edit;
pub use edit::TextShape;

use deckcraft_model::Presentation;
use deckcraft_render::RenderOpts;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeckError {
    #[error("{0}")]
    Open(String),
    #[error("slide {0} does not exist")]
    NoSlide(u32),
    #[error("{0}")]
    Export(String),
    #[error("{0}")]
    Edit(String),
}

/// A slide picture: `width`×`height` premultiplied RGBA8, row by row.
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct DeckDoc {
    pres: Presentation,
    warnings: Vec<String>,
    dirty: bool,
}

impl DeckDoc {
    /// The presentation as a PDF, one slide per page, with real text and shapes.
    pub fn export_pdf(&self) -> Result<Vec<u8>, DeckError> {
        deckcraft_pdf::export(&self.pres, &Default::default()).map_err(|e| DeckError::Export(e.to_string()))
    }

    /// Opens PPTX, PPSX or POTX bytes.
    pub fn open(bytes: &[u8]) -> Result<DeckDoc, DeckError> {
        if deckcraft_pptx::is_encrypted(bytes) {
            return Err(DeckError::Open("This presentation is password-protected.".into()));
        }
        if !deckcraft_pptx::sniff(bytes) {
            return Err(DeckError::Open("Not a PowerPoint (.pptx) presentation.".into()));
        }
        let (pres, warnings) = deckcraft_pptx::import_checked(bytes).map_err(|e| DeckError::Open(e.to_string()))?;
        Ok(DeckDoc { pres, warnings, dirty: false })
    }

    #[cfg(test)]
    fn from_presentation(pres: Presentation) -> DeckDoc {
        DeckDoc { pres, warnings: Vec::new(), dirty: false }
    }

    /// What the import could not keep.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn slide_count(&self) -> u32 {
        self.pres.slides.len() as u32
    }

    /// Slide size in points; every slide of a presentation has the same size.
    pub fn slide_size(&self) -> (f32, f32) {
        (self.pres.slide_size.width as f32, self.pres.slide_size.height as f32)
    }

    /// Speaker notes of a slide as plain text (empty when it has none).
    pub fn notes(&self, slide: u32) -> Result<String, DeckError> {
        let s = self.pres.slides.get(slide as usize).ok_or(DeckError::NoSlide(slide))?;
        Ok(s.notes.text())
    }

    /// The whole slide at `scale` pixels per point.
    pub fn render_slide(&self, slide: u32, scale: f32) -> Result<Raster, DeckError> {
        self.check(slide)?;
        let img = deckcraft_render::render_slide(&self.pres, slide as usize, &RenderOpts { scale: scale as f64, ..Default::default() });
        Ok(Raster { width: img.width, height: img.height, pixels: img.pixels })
    }

    /// A `width`×`height` tile of the slide at `scale`, whose corner is pixel `(x, y)` of the whole
    /// slide at that scale.
    pub fn render_tile(&self, slide: u32, scale: f32, x: f32, y: f32, width: u32, height: u32) -> Result<Raster, DeckError> {
        self.check(slide)?;
        let opts = RenderOpts { scale: scale as f64, offset: (-(x as f64), -(y as f64)), size: Some((width, height)), ..Default::default() };
        let img = deckcraft_render::render_slide(&self.pres, slide as usize, &opts);
        Ok(Raster { width: img.width, height: img.height, pixels: img.pixels })
    }

    /// The shapes with text on `slide`, with their boxes in points (for tapping on them).
    pub fn text_shapes(&self, slide: u32) -> Result<Vec<TextShape>, DeckError> {
        edit::text_shapes(&self.pres, slide as usize)
    }

    /// Makes shape `id`'s text on `slide` read `text`, keeping the formatting of what stays.
    pub fn set_text(&mut self, slide: u32, id: u32, text: &str) -> Result<(), DeckError> {
        edit::set_text(&mut self.pres, slide as usize, id, text)?;
        self.dirty = true;
        Ok(())
    }

    /// A new slide after `slide` with the same layout; returns its index.
    pub fn new_slide(&mut self, slide: u32) -> Result<u32, DeckError> {
        let at = edit::new_slide(&mut self.pres, slide as usize)?;
        self.dirty = true;
        Ok(at as u32)
    }

    /// A copy of `slide` right after it; returns its index.
    pub fn duplicate_slide(&mut self, slide: u32) -> Result<u32, DeckError> {
        let at = edit::duplicate_slide(&mut self.pres, slide as usize)?;
        self.dirty = true;
        Ok(at as u32)
    }

    pub fn delete_slide(&mut self, slide: u32) -> Result<(), DeckError> {
        edit::delete_slide(&mut self.pres, slide as usize)?;
        self.dirty = true;
        Ok(())
    }

    pub fn move_slide(&mut self, from: u32, to: u32) -> Result<(), DeckError> {
        edit::move_slide(&mut self.pres, from as usize, to as usize)?;
        self.dirty = true;
        Ok(())
    }

    /// Changes not yet saved.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// The presentation as PPTX bytes (`file_name`'s extension may also be ppsx or potx).
    pub fn save(&mut self, file_name: &str) -> Result<Vec<u8>, DeckError> {
        let ext = file_name.rsplit('.').next().unwrap_or("pptx").to_ascii_lowercase();
        let bytes = deckcraft_pptx::export_as(&self.pres, &ext).map_err(|e| DeckError::Export(e.to_string()))?;
        self.dirty = false;
        Ok(bytes)
    }

    fn check(&self, slide: u32) -> Result<(), DeckError> {
        if (slide as usize) < self.pres.slides.len() { Ok(()) } else { Err(DeckError::NoSlide(slide)) }
    }
}

#[cfg(test)]
mod tests;
