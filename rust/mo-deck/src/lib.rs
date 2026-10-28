//! Presentation bridge: reads PPTX/PPSX/POTX with DeckCraft and renders slides to premultiplied
//! RGBA, the layout Android's ARGB_8888 `Bitmap` uses. Editing will go through DeckCraft's
//! command registry later; viewing needs only the reader and the renderer.

use deckcraft_model::Presentation;
use deckcraft_render::RenderOpts;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeckError {
    #[error("{0}")]
    Open(String),
    #[error("slide {0} does not exist")]
    NoSlide(u32),
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
}

impl DeckDoc {
    /// Opens PPTX, PPSX or POTX bytes.
    pub fn open(bytes: &[u8]) -> Result<DeckDoc, DeckError> {
        if deckcraft_pptx::is_encrypted(bytes) {
            return Err(DeckError::Open("This presentation is password-protected.".into()));
        }
        if !deckcraft_pptx::sniff(bytes) {
            return Err(DeckError::Open("Not a PowerPoint (.pptx) presentation.".into()));
        }
        let (pres, warnings) = deckcraft_pptx::import_checked(bytes).map_err(|e| DeckError::Open(e.to_string()))?;
        Ok(DeckDoc { pres, warnings })
    }

    #[cfg(test)]
    fn from_presentation(pres: Presentation) -> DeckDoc {
        DeckDoc { pres, warnings: Vec::new() }
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

    fn check(&self, slide: u32) -> Result<(), DeckError> {
        if (slide as usize) < self.pres.slides.len() { Ok(()) } else { Err(DeckError::NoSlide(slide)) }
    }
}

#[cfg(test)]
mod tests;
