//! Presentation bridge: reads PPTX/PPSX/POTX with DeckCraft and renders slides to premultiplied
//! RGBA, the layout Android's ARGB_8888 `Bitmap` uses. Editing (text, slides) works on the model
//! (see `edit.rs`), so the app carries neither the editing engine nor its media decoders.

mod edit;
mod odp;
mod ppt;
pub use edit::{ObjectKind, ObjectStyle, SlideObject, TextShape};

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
    /// The presentation before each change (newest last), and changes taken back (for redo).
    undo: Vec<Presentation>,
    redo: Vec<Presentation>,
}

/// How many changes can be taken back.
const UNDO_STEPS: usize = 30;

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
        if ppt::sniff(bytes) {
            let (pres, warnings) = ppt::import(bytes)?;
            return Ok(DeckDoc { pres, warnings, dirty: false, undo: Vec::new(), redo: Vec::new() });
        }
        if odp::sniff(bytes) {
            let (pres, warnings) = odp::import(bytes)?;
            return Ok(DeckDoc { pres, warnings, dirty: false, undo: Vec::new(), redo: Vec::new() });
        }
        if !deckcraft_pptx::sniff(bytes) {
            return Err(DeckError::Open("Not a PowerPoint (.pptx) presentation.".into()));
        }
        let (pres, warnings) = deckcraft_pptx::import_checked(bytes).map_err(|e| DeckError::Open(e.to_string()))?;
        Ok(DeckDoc { pres, warnings, dirty: false, undo: Vec::new(), redo: Vec::new() })
    }

    /// A new presentation: one title slide on the default 16:9 theme.
    pub fn blank() -> Result<DeckDoc, DeckError> {
        let mut pres = Presentation::default();
        let layout = pres.masters.first().and_then(|m| m.layouts.first()).map(|l| l.id).ok_or_else(|| DeckError::Open("no slide layout".into()))?;
        if pres.slides.is_empty() {
            let slide = deckcraft_model::defaults::new_slide(&mut pres, layout).ok_or_else(|| DeckError::Open("no room for a slide".into()))?;
            pres.slides.push(std::sync::Arc::new(slide));
        }
        Ok(DeckDoc::from_presentation(pres))
    }

    fn from_presentation(pres: Presentation) -> DeckDoc {
        DeckDoc { pres, warnings: Vec::new(), dirty: false, undo: Vec::new(), redo: Vec::new() }
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

    /// Makes `slide`'s speaker notes read `text` (`\n` between paragraphs).
    pub fn set_notes(&mut self, slide: u32, text: &str) -> Result<(), DeckError> {
        self.change(|p| edit::set_notes(p, slide as usize, text))
    }

    /// The whole slide at `scale` pixels per point.
    pub fn render_slide(&self, slide: u32, scale: f32) -> Result<Raster, DeckError> {
        self.check(slide)?;
        let img = deckcraft_render::render_slide(&self.pres, slide as usize, &RenderOpts { scale: scale as f64, ..Default::default() });
        Ok(Raster { width: img.width, height: img.height, pixels: img.pixels })
    }

    /// The whole slide at `scale` with object `id` left out: what is behind it, shown while the
    /// object is dragged.
    pub fn render_slide_without(&self, slide: u32, id: u32, scale: f32) -> Result<Raster, DeckError> {
        self.check(slide)?;
        let mut pres = self.pres.clone();
        let s = std::sync::Arc::make_mut(&mut pres.slides[slide as usize]);
        if let Some(shape) = s.shapes.iter_mut().find(|sh| sh.id.0 == id) {
            shape.hidden = true;
        }
        let img = deckcraft_render::render_slide(&pres, slide as usize, &RenderOpts { scale: scale as f64, ..Default::default() });
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
        self.change(|p| edit::set_text(p, slide as usize, id, text))?;
        Ok(())
    }

    /// The objects on `slide` that can be moved, front-most last.
    pub fn objects(&self, slide: u32) -> Result<Vec<edit::SlideObject>, DeckError> {
        edit::objects(&self.pres, slide as usize)
    }

    /// Moves object `id` by (`dx`, `dy`) points, scales it by `scale` about its centre and turns
    /// it `rotate` degrees clockwise.
    pub fn place_object(&mut self, slide: u32, id: u32, dx: f64, dy: f64, scale: f64, rotate: f64) -> Result<(), DeckError> {
        self.change(|p| edit::place_object(p, slide as usize, id, dx, dy, scale, rotate))?;
        Ok(())
    }

    /// Object `id` given the box (`x`, `y`, `w`, `h`) in points, keeping its turn (a text box's
    /// text flows anew; its letters keep their size).
    pub fn set_object_box(&mut self, slide: u32, id: u32, x: f64, y: f64, w: f64, h: f64) -> Result<(), DeckError> {
        self.change(|p| edit::set_object_box(p, slide as usize, id, x, y, w, h))
    }

    /// A new text box reading `text` in the middle of `slide`; returns its id.
    pub fn add_text_box(&mut self, slide: u32, text: &str) -> Result<u32, DeckError> {
        let id = self.change(|p| edit::add_text_box(p, slide as usize, text))?;
        Ok(id)
    }

    /// Picture `data` (PNG or JPEG, `width`×`height` pixels) in the middle of `slide`; returns its id.
    pub fn add_picture(&mut self, slide: u32, data: Vec<u8>, width: u32, height: u32) -> Result<u32, DeckError> {
        let id = self.change(|p| edit::add_picture(p, slide as usize, data, width, height))?;
        Ok(id)
    }

    /// A new shape (PowerPoint's name for it, e.g. "ellipse") filled with 0xRRGGBB in the
    /// middle of `slide`; returns its id.
    pub fn add_shape(&mut self, slide: u32, preset: &str, fill: u32) -> Result<u32, DeckError> {
        let id = self.change(|p| edit::add_shape(p, slide as usize, preset, fill))?;
        Ok(id)
    }

    /// A copy of object `id` a little below and right of it; returns its id.
    pub fn duplicate_object(&mut self, slide: u32, id: u32) -> Result<u32, DeckError> {
        let new = self.change(|p| edit::duplicate_object(p, slide as usize, id))?;
        Ok(new)
    }

    /// Object `id` brought in front of the others, or sent behind them.
    pub fn arrange_object(&mut self, slide: u32, id: u32, to_front: bool) -> Result<(), DeckError> {
        self.change(|p| edit::arrange_object(p, slide as usize, id, to_front))?;
        Ok(())
    }

    /// Changes the look of object `id`'s text or box.
    pub fn style_object(&mut self, slide: u32, id: u32, style: ObjectStyle) -> Result<(), DeckError> {
        self.change(|p| edit::style_object(p, slide as usize, id, style))?;
        Ok(())
    }

    /// Whether object `id`'s text starts bold.
    pub fn text_is_bold(&self, slide: u32, id: u32) -> bool {
        edit::text_is_bold(&self.pres, slide as usize, id)
    }

    /// Removes object `id` from `slide`.
    pub fn delete_object(&mut self, slide: u32, id: u32) -> Result<(), DeckError> {
        self.change(|p| edit::delete_object(p, slide as usize, id))?;
        Ok(())
    }

    /// Makes cell (`row`, `col`) of table `id` on `slide` read `text`.
    pub fn set_cell_text(&mut self, slide: u32, id: u32, row: u32, col: u32, text: &str) -> Result<(), DeckError> {
        self.change(|p| edit::set_cell_text(p, slide as usize, id, row, col, text))?;
        Ok(())
    }

    /// A new slide after `slide` with the same layout; returns its index.
    pub fn new_slide(&mut self, slide: u32) -> Result<u32, DeckError> {
        let at = self.change(|p| edit::new_slide(p, slide as usize))?;
        Ok(at as u32)
    }

    /// A copy of `slide` right after it; returns its index.
    pub fn duplicate_slide(&mut self, slide: u32) -> Result<u32, DeckError> {
        let at = self.change(|p| edit::duplicate_slide(p, slide as usize))?;
        Ok(at as u32)
    }

    pub fn delete_slide(&mut self, slide: u32) -> Result<(), DeckError> {
        self.change(|p| edit::delete_slide(p, slide as usize))?;
        Ok(())
    }

    pub fn move_slide(&mut self, from: u32, to: u32) -> Result<(), DeckError> {
        self.change(|p| edit::move_slide(p, from as usize, to as usize))?;
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

    /// Runs `edit` on the presentation, remembering it as it was so the change can be taken
    /// back; a failed edit leaves the presentation as it was.
    fn change<T>(&mut self, edit: impl FnOnce(&mut Presentation) -> Result<T, DeckError>) -> Result<T, DeckError> {
        // Slides and media are shared, so a copy costs little.
        let before = self.pres.clone();
        match edit(&mut self.pres) {
            Ok(v) => {
                self.undo.push(before);
                if self.undo.len() > UNDO_STEPS {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.dirty = true;
                Ok(v)
            }
            Err(e) => {
                self.pres = before;
                Err(e)
            }
        }
    }

    /// Takes back the last change; false when there is none.
    pub fn undo(&mut self) -> bool {
        let Some(before) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(&mut self.pres, before));
        self.dirty = true;
        true
    }

    /// Makes again the last change taken back; false when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(after) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(&mut self.pres, after));
        self.dirty = true;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn check(&self, slide: u32) -> Result<(), DeckError> {
        if (slide as usize) < self.pres.slides.len() { Ok(()) } else { Err(DeckError::NoSlide(slide)) }
    }
}

#[cfg(test)]
mod tests;
