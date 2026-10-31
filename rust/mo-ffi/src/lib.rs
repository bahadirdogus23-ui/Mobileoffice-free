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
use mo_pdf::{FieldKind, FormField, PdfDoc, PdfError, Sensitive};
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
    /// The file asks for a password (none was given).
    #[error("{0}")]
    NeedsPassword(String),
    #[error("{0}")]
    WrongPassword(String),
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
            WordError::Save(m) => MoError::Save(m),
            e @ WordError::NoPage(_) => MoError::NotFound(e.to_string()),
        }
    }
}

impl From<DeckError> for MoError {
    fn from(e: DeckError) -> MoError {
        match e {
            DeckError::Open(m) => MoError::Open(m),
            DeckError::Export(m) => MoError::Save(m),
            e @ DeckError::NoSlide(_) => MoError::NotFound(e.to_string()),
        }
    }
}

impl From<PdfError> for MoError {
    fn from(e: PdfError) -> MoError {
        match e {
            PdfError::Open(m) => MoError::Open(m),
            e @ PdfError::NeedsPassword => MoError::NeedsPassword(e.to_string()),
            e @ PdfError::WrongPassword => MoError::WrongPassword(e.to_string()),
            PdfError::Edit(m) => MoError::Command(m),
            PdfError::Save(m) => MoError::Save(m),
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

/// A file picked by the person: its name and bytes.
#[derive(uniffi::Record)]
pub struct NamedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// A new PDF of pictures, one per page, in order: on A4 pages (turned to fit each picture)
/// when `fit_a4`, else each page the picture's own size.
#[uniffi::export]
pub fn images_to_pdf(images: Vec<NamedFile>, fit_a4: bool) -> Result<Vec<u8>, MoError> {
    let images: Vec<(String, Vec<u8>)> = images.into_iter().map(|f| (f.name, f.bytes)).collect();
    Ok(mo_pdf::images_to_pdf(&images, fit_a4)?)
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
    /// The workbook as a PDF following each sheet's Page Setup: every sheet when `all_sheets`,
    /// else the active one (only `range` of it, if given); `fit_to_page` puts each sheet on one page.
    pub fn export_pdf(&self, all_sheets: bool, range: Option<String>, fit_to_page: bool) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().export_pdf(all_sheets, range.as_deref(), fit_to_page)?)
    }

    pub fn save(&self, file_name: String) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().save(&file_name)?)
    }
}

/// Page setup for turning plain text into a document, in points (1/72 inch).
#[derive(uniffi::Record)]
pub struct TextPage {
    pub width_pt: f32,
    pub height_pt: f32,
    pub landscape: bool,
    pub margin_pt: f32,
    pub font: String,
    pub size_pt: f32,
}

/// One rectangle of search match number `index` on page `page`, in page points.
#[derive(uniffi::Record)]
pub struct SearchHit {
    pub index: u32,
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
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

    /// Plain text laid out on `page`, one paragraph per line, for converting to PDF, DOCX…
    #[uniffi::constructor]
    pub fn from_text(bytes: Vec<u8>, page: TextPage) -> Self {
        let page = mo_word::TextPage {
            width_pt: page.width_pt,
            height_pt: page.height_pt,
            landscape: page.landscape,
            margin_pt: page.margin_pt,
            font: page.font,
            size_pt: page.size_pt,
        };
        WordDocument { doc: Mutex::new(WordDoc::from_text(&bytes, &page)) }
    }

    pub fn page_count(&self) -> u32 {
        self.doc().page_count()
    }

    pub fn page_size(&self, page: u32) -> Result<PageSize, MoError> {
        let (width, height) = self.doc().page_size(page)?;
        Ok(PageSize { width, height })
    }

    /// The whole page at `scale` pixels per point; `dark` draws it for night reading.
    pub fn render_page(&self, page: u32, scale: f32, dark: bool) -> Result<Raster, MoError> {
        Ok(self.doc().render_page(page, scale, dark)?.into())
    }

    /// The document in the format `file_name`'s extension names (pdf, docx, odt, rtf, txt, md, html).
    pub fn export(&self, file_name: String) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().export(&file_name)?)
    }

    /// The file carries an e-signature (UYAP UDF); it is not verified.
    pub fn is_signed(&self) -> bool {
        self.doc().is_signed()
    }

    /// The body text starting on each page, for reading aloud.
    pub fn page_texts(&self) -> Vec<String> {
        self.doc().page_texts()
    }

    /// Where `query` appears in the body, ignoring case, accents and dotted/dotless i.
    pub fn find(&self, query: String) -> Vec<SearchHit> {
        self.doc().find(&query).into_iter().map(|h| SearchHit { index: h.index, page: h.page, x: h.x, y: h.y, width: h.w, height: h.h }).collect()
    }

    /// A `width`×`height` tile whose corner is pixel `(x, y)` of the page drawn at `scale`.
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile(&self, page: u32, scale: f32, x: f32, y: f32, width: u32, height: u32, dark: bool) -> Result<Raster, MoError> {
        Ok(self.doc().render_tile(page, scale, x, y, width, height, dark)?.into())
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
    /// The presentation as a PDF, one slide per page.
    pub fn export_pdf(&self) -> Result<Vec<u8>, MoError> {
        Ok(self.doc.export_pdf()?)
    }

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

/// How strokes are drawn: colour 0–1 RGB, width in points, opacity 0–1.
#[derive(uniffi::Record)]
pub struct Pen {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub width: f64,
    pub opacity: f64,
}

/// Personal data that can be found to black out.
#[derive(uniffi::Enum)]
pub enum SensitiveKind {
    /// T.C. kimlik numarası (check digits verified).
    TurkishId,
    Iban,
    Phone,
    Email,
}

/// A rectangle as fractions of the displayed page (y down).
#[derive(uniffi::Record)]
pub struct PageRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// One place found on a page: a rectangle per line it spans.
#[derive(uniffi::Record)]
pub struct PdfTextHit {
    pub page: u32,
    pub rects: Vec<PageRect>,
}

fn text_hits(hits: Vec<mo_pdf::TextHit>) -> Vec<PdfTextHit> {
    hits.into_iter()
        .map(|h| PdfTextHit {
            page: h.page,
            rects: h.rects.into_iter().map(|[left, top, right, bottom]| PageRect { left, top, right, bottom }).collect(),
        })
        .collect()
}

/// Flat `x, y` pairs as points.
fn points(strokes: &[Vec<f64>]) -> Vec<Vec<[f64; 2]>> {
    strokes.iter().map(|s| s.as_chunks::<2>().0.to_vec()).collect()
}

/// What a form field holds.
#[derive(uniffi::Enum)]
pub enum FormFieldKind {
    Text,
    CheckBox,
    Radio,
    Choice,
    Other,
}

/// One choice of a radio group or a combo / list box.
#[derive(uniffi::Record)]
pub struct FieldOption {
    /// What filling stores (the export value).
    pub value: String,
    pub text: String,
}

/// One form field as the person fills it.
#[derive(uniffi::Record)]
pub struct PdfFormField {
    /// The key for filling.
    pub name: String,
    /// What to show the person (the field's tooltip, else its name).
    pub label: String,
    pub kind: FormFieldKind,
    /// Text: the text. Radio / choice: the chosen value. Check box: empty when not ticked.
    pub value: String,
    pub options: Vec<FieldOption>,
    pub read_only: bool,
    pub multiline: bool,
    /// 0-based page of the field.
    pub page: Option<u32>,
}

impl From<FormField> for PdfFormField {
    fn from(f: FormField) -> Self {
        PdfFormField {
            name: f.name,
            label: f.label,
            kind: match f.kind {
                FieldKind::Text => FormFieldKind::Text,
                FieldKind::CheckBox => FormFieldKind::CheckBox,
                FieldKind::Radio => FormFieldKind::Radio,
                FieldKind::Choice => FormFieldKind::Choice,
                FieldKind::Other => FormFieldKind::Other,
            },
            value: f.value,
            options: f.options.into_iter().map(|(value, text)| FieldOption { value, text }).collect(),
            read_only: f.read_only,
            multiline: f.multiline,
            page: f.page,
        }
    }
}

/// A PDF whose pages are being reorganised (rotate, delete, move, combine, extract). Viewing
/// stays with Android's PdfRenderer; [`PdfDocument::save`] gives the file to show and to keep.
#[derive(uniffi::Object)]
pub struct PdfDocument {
    doc: Mutex<PdfDoc>,
}

impl PdfDocument {
    fn doc(&self) -> MutexGuard<'_, PdfDoc> {
        self.doc.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[uniffi::export]
impl PdfDocument {
    #[uniffi::constructor]
    pub fn open(bytes: Vec<u8>, password: Option<String>) -> Result<Self, MoError> {
        Ok(PdfDocument { doc: Mutex::new(PdfDoc::open(bytes, password.as_deref())?) })
    }

    pub fn page_count(&self) -> Result<u32, MoError> {
        Ok(self.doc().page_count()?)
    }

    /// Turns `pages` (0-based) by `degrees` clockwise (a multiple of 90).
    pub fn rotate(&self, pages: Vec<u32>, degrees: i32) -> Result<(), MoError> {
        Ok(self.doc().rotate(&pages, degrees)?)
    }

    /// Removes `pages`; at least one page always stays.
    pub fn delete(&self, pages: Vec<u32>) -> Result<(), MoError> {
        Ok(self.doc().delete(&pages)?)
    }

    /// Moves `pages` so the first of them lands at `to`.
    pub fn move_pages(&self, pages: Vec<u32>, to: u32) -> Result<(), MoError> {
        Ok(self.doc().move_pages(&pages, to)?)
    }

    /// Appends every page of another PDF.
    pub fn append(&self, bytes: Vec<u8>, password: Option<String>) -> Result<(), MoError> {
        Ok(self.doc().append(bytes, password.as_deref())?)
    }

    /// A new PDF of just `pages`, in that order.
    pub fn extract(&self, pages: Vec<u32>) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().extract(&pages)?)
    }

    /// Puts a hand-drawn signature on `page`. Each stroke is flat `x, y` pairs, fractions of the
    /// page as displayed (y down).
    pub fn sign(&self, page: u32, strokes: Vec<Vec<f64>>) -> Result<(), MoError> {
        Ok(self.doc().sign(page, &points(&strokes))?)
    }

    /// Pen or highlighter strokes (points as for [`PdfDocument::sign`]).
    pub fn draw(&self, page: u32, strokes: Vec<Vec<f64>>, pen: Pen) -> Result<(), MoError> {
        Ok(self.doc().draw(page, &points(&strokes), [pen.red, pen.green, pen.blue], pen.width, pen.opacity)?)
    }

    /// A sticky note holding `text` at (`x`, `y`), fractions of the displayed page.
    pub fn note(&self, page: u32, x: f64, y: f64, text: String) -> Result<(), MoError> {
        Ok(self.doc().note(page, [x, y], &text)?)
    }

    /// Removes the last signature, drawing or note added; false when there is none.
    pub fn undo_annotation(&self) -> Result<bool, MoError> {
        Ok(self.doc().undo_annotation()?)
    }

    /// Whether the saved file asks for a password.
    pub fn is_protected(&self) -> bool {
        self.doc().is_protected()
    }

    /// The owner restricted changes and only their password lifts that.
    pub fn is_restricted(&self) -> bool {
        self.doc().is_restricted()
    }

    /// Protects the next save with `password`; `seed` is 32 random bytes.
    pub fn protect(&self, password: String, seed: Vec<u8>) -> Result<(), MoError> {
        let seed: [u8; 32] = seed.try_into().map_err(|_| MoError::Command("the seed must be 32 bytes".into()))?;
        Ok(self.doc().protect(&password, seed)?)
    }

    /// The next save asks for no password.
    pub fn unprotect(&self) -> Result<(), MoError> {
        Ok(self.doc().unprotect()?)
    }

    /// The document without its password, to show on screen only.
    pub fn display_copy(&self) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().display_copy()?)
    }

    /// A smaller copy (pictures resampled to 150 ppi, or 96 `for_screen`); this one is unchanged.
    pub fn compress(&self, for_screen: bool) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().compress(for_screen)?)
    }

    /// The form's fields (none when there is no form).
    pub fn fields(&self) -> Vec<PdfFormField> {
        self.doc().fields().into_iter().map(PdfFormField::from).collect()
    }

    pub fn set_text(&self, name: String, text: String) -> Result<(), MoError> {
        Ok(self.doc().set_text(&name, &text)?)
    }

    pub fn set_check(&self, name: String, on: bool) -> Result<(), MoError> {
        Ok(self.doc().set_check(&name, on)?)
    }

    /// Picks `value` (an option's value; empty for none) in a radio group or choice field.
    pub fn set_choice(&self, name: String, value: String) -> Result<(), MoError> {
        Ok(self.doc().set_choice(&name, &value)?)
    }

    /// Every place `query` occurs (ignoring case, Turkish i forms and accents), at most `limit`.
    pub fn find_text(&self, query: String, limit: u32) -> Result<Vec<PdfTextHit>, MoError> {
        Ok(text_hits(self.doc().find_text(&query, limit as usize)?))
    }

    /// The text of `page` in reading order.
    pub fn page_text(&self, page: u32) -> Result<String, MoError> {
        Ok(self.doc().page_text(page)?)
    }

    /// Marks every place of `kinds` and `words` for blacking out; returns the places.
    pub fn mark_redactions(&self, kinds: Vec<SensitiveKind>, words: Vec<String>) -> Result<Vec<PdfTextHit>, MoError> {
        let kinds: Vec<Sensitive> = kinds
            .into_iter()
            .map(|k| match k {
                SensitiveKind::TurkishId => Sensitive::TurkishId,
                SensitiveKind::Iban => Sensitive::Iban,
                SensitiveKind::Phone => Sensitive::Phone,
                SensitiveKind::Email => Sensitive::Email,
            })
            .collect();
        Ok(text_hits(self.doc().mark_redactions(&kinds, &words)?))
    }

    /// Drops the black-out marks not yet applied; returns how many.
    pub fn clear_redaction_marks(&self) -> Result<u32, MoError> {
        Ok(self.doc().clear_redaction_marks()?)
    }

    /// Black-out marks waiting to be applied.
    pub fn redaction_marks(&self) -> u32 {
        self.doc().redaction_marks()
    }

    /// Removes what is under every black-out mark for good; returns how many were applied.
    pub fn apply_redactions(&self) -> Result<u32, MoError> {
        Ok(self.doc().apply_redactions()?)
    }

    /// The whole document as a new file.
    pub fn save(&self) -> Result<Vec<u8>, MoError> {
        Ok(self.doc().save()?)
    }
}
