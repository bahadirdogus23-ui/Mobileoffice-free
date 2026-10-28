//! The native library the Android app loads (`libmo_ffi.so`). It exposes the format bridges
//! (`mo-grid`, later `mo-word`, `mo-deck`, `mo-pdf`, `mo-cad`) to Kotlin through UniFFI.
//!
//! Engines run behind a `Mutex` per document: each document is used from one engine thread on
//! the Kotlin side, so the lock is uncontended; it only makes the object `Send + Sync` as UniFFI
//! requires. A panic that escapes an engine reaches Kotlin as an exception, not a crash.

use std::sync::{Mutex, MutexGuard};

use mo_cad::{CadDoc, CadError};
use mo_deck::{DeckDoc, DeckError};
use mo_grid::{GridDoc, GridError};
use mo_word::{WordDoc, WordError};

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum MoError {
    #[error("{0}")]
    Open(String),
    #[error("{0}")]
    Command(String),
    #[error("{0}")]
    Save(String),
    #[error("{0}")]
    NotFound(String),
}

impl From<GridError> for MoError {
    fn from(e: GridError) -> MoError {
        match e {
            GridError::Open(m) => MoError::Open(m),
            GridError::Command(m) => MoError::Command(m),
            GridError::Save(m) => MoError::Save(m),
            e @ GridError::NoSheet(_) => MoError::NotFound(e.to_string()),
        }
    }
}

impl From<WordError> for MoError {
    fn from(e: WordError) -> MoError {
        match e {
            WordError::Open(m) => MoError::Open(m),
            e @ WordError::NoPage(_) => MoError::NotFound(e.to_string()),
        }
    }
}

impl From<DeckError> for MoError {
    fn from(e: DeckError) -> MoError {
        match e {
            DeckError::Open(m) => MoError::Open(m),
            e @ DeckError::NoSlide(_) => MoError::NotFound(e.to_string()),
        }
    }
}

impl From<CadError> for MoError {
    fn from(e: CadError) -> MoError {
        match e {
            CadError::Open(m) => MoError::Open(m),
            e => MoError::NotFound(e.to_string()),
        }
    }
}

/// The used area of a sheet, from A1.
#[derive(uniffi::Record)]
pub struct SheetSize {
    pub rows: u32,
    pub cols: u32,
}

/// Version of this native library, for the About screen and bug reports.
#[uniffi::export]
pub fn native_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// A spreadsheet open in the GridCraft engine.
#[derive(uniffi::Object)]
pub struct SheetDocument {
    doc: Mutex<GridDoc>,
}

impl SheetDocument {
    fn doc(&self) -> MutexGuard<'_, GridDoc> {
        // A poisoned lock means a panic escaped mid-command; the engine restores its own state,
        // so keep using the document.
        self.doc.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[uniffi::export]
impl SheetDocument {
    /// Opens XLSX, XLSM, XLS, XLSB, ODS, CSV or TSV bytes. `name` is the file name.
    #[uniffi::constructor]
    pub fn open(name: String, bytes: Vec<u8>, password: Option<String>) -> Result<Self, MoError> {
        let doc = GridDoc::open(&name, &bytes, password.as_deref())?;
        Ok(SheetDocument { doc: Mutex::new(doc) })
    }

    /// A new, empty workbook.
    #[uniffi::constructor]
    pub fn blank() -> Self {
        SheetDocument { doc: Mutex::new(GridDoc::blank()) }
    }

    pub fn warnings(&self) -> Vec<String> {
        self.doc().warnings().to_vec()
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.doc().sheet_names()
    }

    pub fn active_sheet(&self) -> u32 {
        self.doc().active_sheet()
    }

    pub fn set_active_sheet(&self, sheet: u32) -> Result<(), MoError> {
        Ok(self.doc().set_active_sheet(sheet)?)
    }

    pub fn used_size(&self, sheet: u32) -> Result<SheetSize, MoError> {
        let s = self.doc().used_size(sheet)?;
        Ok(SheetSize { rows: s.rows, cols: s.cols })
    }

    /// Displayed texts of a window of cells, row by row.
    pub fn cell_texts(&self, sheet: u32, row: u32, col: u32, rows: u32, cols: u32) -> Result<Vec<String>, MoError> {
        Ok(self.doc().cell_texts(sheet, row, col, rows, cols)?)
    }

    /// The formula or typed value of one cell, as the formula bar shows it.
    pub fn edit_text(&self, sheet: u32, row: u32, col: u32) -> Result<String, MoError> {
        Ok(self.doc().edit_text(sheet, row, col)?)
    }

    /// Runs a GridCraft command (e.g. `cell.set`, `home.bold`) with JSON parameters.
    pub fn execute(&self, id: String, params_json: String) -> Result<String, MoError> {
        Ok(self.doc().execute(&id, &params_json)?)
    }

    pub fn is_dirty(&self) -> bool {
        self.doc().is_dirty()
    }

    /// Encodes the workbook as `file_name`'s extension says (.xlsx by default).
    pub fn save(&self, file_name: String) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().save(&file_name)?)
    }
}

/// Page size in points (1/72 inch).
#[derive(uniffi::Record)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

/// Premultiplied RGBA8 pixels, row by row: what an ARGB_8888 `Bitmap` holds in memory.
#[derive(uniffi::Record)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl From<mo_word::Raster> for Raster {
    fn from(r: mo_word::Raster) -> Raster {
        Raster { width: r.width, height: r.height, pixels: r.pixels }
    }
}

impl From<mo_deck::Raster> for Raster {
    fn from(r: mo_deck::Raster) -> Raster {
        Raster { width: r.width, height: r.height, pixels: r.pixels }
    }
}

/// A word-processing document (DOCX, DOC, ODT, RTF, …) open in the WordCraft engine.
#[derive(uniffi::Object)]
pub struct WordDocument {
    doc: Mutex<WordDoc>,
}

impl WordDocument {
    fn doc(&self) -> MutexGuard<'_, WordDoc> {
        self.doc.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[uniffi::export]
impl WordDocument {
    /// Opens DOCX/DOCM/DOTX, DOC, ODT, RTF, HTML, Markdown or text bytes.
    #[uniffi::constructor]
    pub fn open(name: String, bytes: Vec<u8>, password: Option<String>) -> Result<Self, MoError> {
        Ok(WordDocument { doc: Mutex::new(WordDoc::open(&name, &bytes, password.as_deref())?) })
    }

    pub fn page_count(&self) -> u32 {
        self.doc().page_count()
    }

    pub fn page_size(&self, page: u32) -> Result<PageSize, MoError> {
        let (width, height) = self.doc().page_size(page)?;
        Ok(PageSize { width, height })
    }

    /// The whole page at `scale` pixels per point.
    pub fn render_page(&self, page: u32, scale: f32) -> Result<Raster, MoError> {
        Ok(self.doc().render_page(page, scale)?.into())
    }

    /// A `width`×`height` tile whose corner is pixel `(x, y)` of the page drawn at `scale`.
    pub fn render_tile(&self, page: u32, scale: f32, x: f32, y: f32, width: u32, height: u32) -> Result<Raster, MoError> {
        Ok(self.doc().render_tile(page, scale, x, y, width, height)?.into())
    }
}

/// A presentation (PPTX, PPSX, POTX) open in the DeckCraft renderer.
#[derive(uniffi::Object)]
pub struct SlidesDocument {
    doc: DeckDoc,
}

#[uniffi::export]
impl SlidesDocument {
    #[uniffi::constructor]
    pub fn open(bytes: Vec<u8>) -> Result<Self, MoError> {
        Ok(SlidesDocument { doc: DeckDoc::open(&bytes)? })
    }

    pub fn warnings(&self) -> Vec<String> {
        self.doc.warnings().to_vec()
    }

    pub fn slide_count(&self) -> u32 {
        self.doc.slide_count()
    }

    /// Slide size in points (the same for every slide).
    pub fn slide_size(&self) -> PageSize {
        let (width, height) = self.doc.slide_size();
        PageSize { width, height }
    }

    pub fn notes(&self, slide: u32) -> Result<String, MoError> {
        Ok(self.doc.notes(slide)?)
    }

    /// The whole slide at `scale` pixels per point.
    pub fn render_slide(&self, slide: u32, scale: f32) -> Result<Raster, MoError> {
        Ok(self.doc.render_slide(slide, scale)?.into())
    }

    /// A `width`×`height` tile whose corner is pixel `(x, y)` of the slide drawn at `scale`.
    pub fn render_tile(&self, slide: u32, scale: f32, x: f32, y: f32, width: u32, height: u32) -> Result<Raster, MoError> {
        Ok(self.doc.render_tile(slide, scale, x, y, width, height)?.into())
    }
}

/// A CAD layer for the layer panel; `color` is 0xRRGGBB.
#[derive(uniffi::Record)]
pub struct CadLayer {
    pub name: String,
    pub visible: bool,
    pub color: u32,
}

/// A rectangle in drawing units.
#[derive(uniffi::Record)]
pub struct CadExtents {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// A DXF or DWG drawing open in the CADCraft viewer (view only).
#[derive(uniffi::Object)]
pub struct CadDocument {
    doc: Mutex<CadDoc>,
}

impl CadDocument {
    fn doc(&self) -> MutexGuard<'_, CadDoc> {
        self.doc.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[uniffi::export]
impl CadDocument {
    /// Opens DXF (R12–2018) or DWG (R13–2018) bytes; `name` is the file name.
    #[uniffi::constructor]
    pub fn open(name: String, bytes: Vec<u8>) -> Result<Self, MoError> {
        Ok(CadDocument { doc: Mutex::new(CadDoc::open(&name, &bytes)?) })
    }

    /// What reading the file could not keep.
    pub fn notes(&self) -> Vec<String> {
        self.doc().notes()
    }

    /// "Model", then the paper-space layouts in tab order.
    pub fn space_names(&self) -> Vec<String> {
        self.doc().space_names()
    }

    pub fn layers(&self) -> Vec<CadLayer> {
        self.doc()
            .layers()
            .into_iter()
            .map(|l| CadLayer {
                name: l.name,
                visible: l.visible,
                color: (u32::from(l.color.0) << 16) | (u32::from(l.color.1) << 8) | u32::from(l.color.2),
            })
            .collect()
    }

    pub fn set_layer_visible(&self, name: String, visible: bool) -> Result<(), MoError> {
        Ok(self.doc().set_layer_visible(&name, visible)?)
    }

    pub fn extents(&self, space: u32) -> Result<CadExtents, MoError> {
        let e = self.doc().extents(space)?;
        Ok(CadExtents { min_x: e.min_x, min_y: e.min_y, max_x: e.max_x, max_y: e.max_y })
    }

    /// Draws a `width`×`height` view centred on (`center_x`, `center_y`) at `scale` pixels per unit.
    #[allow(clippy::too_many_arguments)]
    pub fn render(&self, space: u32, center_x: f64, center_y: f64, scale: f64, width: u32, height: u32, dark: bool) -> Result<Raster, MoError> {
        let r = self.doc().render(space, center_x, center_y, scale, width, height, dark)?;
        Ok(Raster { width: r.width, height: r.height, pixels: r.pixels })
    }
}
