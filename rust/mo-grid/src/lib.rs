//! Spreadsheet bridge: a thin, mobile-facing wrapper over the GridCraft engine.
//!
//! Everything a front end changes goes through [`GridDoc::execute`], i.e. GridCraft's own command
//! registry (`home.bold`, `cell.set`, …); this crate only adds what the engine's command surface
//! does not cover for a viewer: opening from bytes, reading a window of cell texts, and saving to
//! bytes. See docs/RAPOR-VE-PLAN.md §9.3.

use std::sync::{Arc, Once};

use gridcraft_engine::io::{self, FileKind};
use gridcraft_engine::model::Workbook;
use gridcraft_engine::{DocState, Session, Trust, display};

mod encoding;

pub use encoding::{decode_turkish_legacy, looks_like_windows_1254};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GridError {
    #[error("{0}")]
    Open(String),
    #[error("{0}")]
    Command(String),
    #[error("{0}")]
    Save(String),
    #[error("sheet {0} does not exist")]
    NoSheet(u32),
}

/// One open workbook in its own engine session.
pub struct GridDoc {
    session: Session,
    warnings: Vec<String>,
    /// Numbers and formulas typed and shown the Turkish way (see `turkish.rs`).
    turkish: bool,
}

/// Column widths and row heights in points (see [`GridDoc::sizes`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Sizes {
    pub col_widths: Vec<f32>,
    pub row_heights: Vec<f32>,
    pub default_col: f32,
    pub default_row: f32,
    /// Rows at the top and columns at the left that stay in view while scrolling.
    pub frozen_rows: u32,
    pub frozen_cols: u32,
}

/// How a cell looks in the grid. Colours are 0xAARRGGBB; 0 means the default (black text, no
/// fill). `align`: 0 general (numbers right, text left), 1 left, 2 centre, 3 right.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellLook {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: u32,
    pub fill: u32,
    pub align: u8,
}

/// The rectangle from A1 to the last used cell, as counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsedSize {
    pub rows: u32,
    pub cols: u32,
}

impl GridDoc {
    /// A new, empty workbook.
    pub fn blank() -> GridDoc {
        let mut session = Session::new();
        session.new_workbook();
        GridDoc { session, warnings: Vec::new(), turkish: false }
    }

    /// Opens an XLSX/XLSM/XLS/XLSB/ODS/CSV/TSV file from its bytes. `name` is the file name; its
    /// extension helps pick the reader, but content is sniffed first, as the engine does.
    pub fn open(name: &str, bytes: &[u8], password: Option<&str>) -> Result<GridDoc, GridError> {
        let mut warnings = Vec::new();
        let utf8;
        let bytes = if is_delimited_text(name) && looks_like_windows_1254(bytes) {
            utf8 = decode_turkish_legacy(bytes);
            warnings.push("Metin Windows-1254 (Türkçe) kodlamasından dönüştürüldü.".to_string());
            utf8.as_bytes()
        } else {
            bytes
        };
        let (wb, engine_warnings) = io::open_bytes_with_password(name, bytes, password).map_err(|e| GridError::Open(e.to_string()))?;
        warnings.extend(engine_warnings);
        let title = std::path::Path::new(name).file_name().and_then(|n| n.to_str()).unwrap_or("Book").to_string();
        let mut session = Session::new();
        session.add_document(DocState::new(wb, None, title, Trust::FromFile));
        Ok(GridDoc { session, warnings, turkish: false })
    }

    /// What the import could not keep (shown to the person before they edit).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    fn wb(&self) -> &Arc<Workbook> {
        // A GridDoc always holds exactly one document (see `blank` and `open`).
        &self.session.documents()[0].wb
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.wb().sheets.iter().map(|s| s.name.clone()).collect()
    }

    pub fn active_sheet(&self) -> u32 {
        self.wb().active_sheet() as u32
    }

    /// Makes `sheet` the one commands act on.
    pub fn set_active_sheet(&self, sheet: u32) -> Result<(), GridError> {
        if (sheet as usize) >= self.wb().sheets.len() {
            return Err(GridError::NoSheet(sheet));
        }
        self.wb().set_active_sheet(sheet as usize);
        Ok(())
    }

    /// Size of the used area of `sheet`, counted from A1 (0×0 for an empty sheet).
    pub fn used_size(&self, sheet: u32) -> Result<UsedSize, GridError> {
        let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        Ok(match sh.used_range() {
            Some(r) => UsedSize { rows: r.end.row + 1, cols: r.end.col + 1 },
            None => UsedSize { rows: 0, cols: 0 },
        })
    }

    /// The displayed text (number formats applied) of a `rows`×`cols` window starting at
    /// `(row, col)`, row by row. This is what a virtualised grid asks for while scrolling.
    pub fn cell_texts(&self, sheet: u32, row: u32, col: u32, rows: u32, cols: u32) -> Result<Vec<String>, GridError> {
        let wb = self.wb();
        let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        let mut out = Vec::with_capacity((rows as usize) * (cols as usize));
        for r in row..row.saturating_add(rows) {
            for c in col..col.saturating_add(cols) {
                let cell = gridcraft_engine::cell(r, c);
                let text = display::cell_text(wb, sh, cell);
                out.push(if self.turkish && shows_number(wb, sh, cell) { number_display(&text) } else { text });
            }
        }
        Ok(out)
    }

    /// Column widths and row heights in points for the first `cols` columns and `rows` rows of
    /// `sheet` (hidden ones are 0), and the sizes of the ones after them.
    pub fn sizes(&self, sheet: u32, cols: u32, rows: u32) -> Result<Sizes, GridError> {
        let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        Ok(Sizes {
            col_widths: (0..cols).map(|c| sh.col_width(c)).collect(),
            row_heights: (0..rows).map(|r| sh.row_height(r)).collect(),
            default_col: sh.default_col_width,
            default_row: sh.default_row_height,
            frozen_rows: sh.freeze.map_or(0, |(r, _)| r),
            frozen_cols: sh.freeze.map_or(0, |(_, c)| c),
        })
    }

    /// How each cell of a `rows`×`cols` window looks, row by row, for drawing the grid: the
    /// cell's own style (conditional formats are left to the PDF and print paths).
    pub fn cell_looks(&self, sheet: u32, row: u32, col: u32, rows: u32, cols: u32) -> Result<Vec<CellLook>, GridError> {
        use gridcraft_engine::model::style::{HAlign, PatternType, Underline};
        let wb = self.wb();
        let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        let argb = |rgb: Option<[u8; 3]>| rgb.map_or(0, |[r, g, b]| 0xFF00_0000 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b));
        let mut out = Vec::with_capacity((rows as usize) * (cols as usize));
        for r in row..row.saturating_add(rows) {
            for c in col..col.saturating_add(cols) {
                let st = wb.styles.get(sh.style_id(gridcraft_engine::cell(r, c)));
                out.push(CellLook {
                    bold: st.font.bold,
                    italic: st.font.italic,
                    underline: st.font.underline != Underline::None,
                    color: argb(st.font.color.resolve(&wb.theme)),
                    fill: if st.fill.pattern == PatternType::Solid { argb(st.fill.fg.resolve(&wb.theme)) } else { 0 },
                    align: match st.align.h {
                        HAlign::Left => 1,
                        HAlign::Center | HAlign::CenterAcross => 2,
                        HAlign::Right => 3,
                        _ => 0,
                    },
                });
            }
        }
        Ok(out)
    }

    /// Type and show numbers and formulas the Turkish way ("1.250,5", `=TOPLA(A1;A2)`).
    pub fn set_turkish(&mut self, on: bool) {
        self.turkish = on;
    }

    /// Enters what the person typed in cell (`row`, `col`) of the active sheet, read the
    /// Turkish way when that is on.
    pub fn set_input(&mut self, row: u32, col: u32, input: &str) -> Result<(), GridError> {
        let input = if self.turkish { to_engine(input) } else { input.to_string() };
        let params = serde_json::json!({ "cell": gridcraft_engine::cell(row, col).a1(), "input": input });
        self.execute("cell.set", &params.to_string()).map(|_| ())
    }

    /// Function names to suggest while a formula is typed (Turkish ones when Turkish is on).
    pub fn function_names(&self) -> Vec<String> {
        function_names(self.turkish).into_iter().map(str::to_string).collect()
    }

    /// Sorts the active sheet's table (A1 to the last used cell; a header row stays on top) by
    /// column `col`. Turkish text follows the Turkish alphabet when Turkish is on.
    pub fn sort_column(&mut self, col: u32, ascending: bool) -> Result<(), GridError> {
        let sheet = self.active_sheet();
        let used = self.used_size(sheet)?;
        if used.rows < 2 || used.cols == 0 {
            return Ok(());
        }
        let last = gridcraft_engine::cell(used.rows - 1, used.cols - 1).a1();
        let column = gridcraft_engine::cell(0, col).a1().trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
        let order = if ascending { "asc" } else { "desc" };
        let mut texts: Vec<String> = {
            let wb = self.wb();
            let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
            (0..used.rows)
                .filter_map(|r| match sh.value(gridcraft_engine::cell(r, col)) {
                    gridcraft_engine::core::Value::Text(t) => Some(t.to_string()),
                    _ => None,
                })
                .collect()
        };
        let params = if self.turkish && !texts.is_empty() {
            // The engine orders text by code point (Ç and Ş after Z); a list in Turkish order
            // sorts the text, and a plain key after it sorts numbers among themselves.
            texts.sort_by_key(|t| collation_key(t));
            texts.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
            serde_json::json!({ "range": format!("A1:{last}"), "keys": [
                { "column": column, "order": order, "customList": texts },
                { "column": column, "order": order },
            ] })
        } else {
            serde_json::json!({ "range": format!("A1:{last}"), "keys": [{ "column": column, "order": order }] })
        };
        self.execute("data.sort", &params.to_string()).map(|_| ())
    }

    /// What the formula bar shows for a cell: the formula, or the value as typed.
    pub fn edit_text(&self, sheet: u32, row: u32, col: u32) -> Result<String, GridError> {
        let wb = self.wb();
        let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        let text = display::edit_text(wb, sh, gridcraft_engine::cell(row, col));
        Ok(if self.turkish { from_engine(&text) } else { text })
    }

    /// Runs one GridCraft command by id with JSON parameters and returns its JSON result.
    /// Commands never open dialogs here; requests for one are dropped.
    pub fn execute(&mut self, id: &str, params_json: &str) -> Result<String, GridError> {
        let params: serde_json::Value = if params_json.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(params_json).map_err(|e| GridError::Command(format!("{id}: invalid parameters: {e}")))?
        };
        let result = self.session.execute(id, params).map_err(|e| GridError::Command(e.to_string()));
        self.session.take_ui_requests();
        result.map(|v| v.to_string())
    }

    pub fn is_dirty(&self) -> bool {
        self.session.documents()[0].is_dirty()
    }

    /// Encodes the workbook in the format `file_name`'s extension names (.xlsx, .xlsm, .csv, .tsv…).
    /// The workbook as a PDF, following each sheet's Page Setup: every sheet when `all_sheets`,
    /// else the active one, only `range` of it if given ("A1:F40"); `fit_to_page` shrinks each
    /// sheet onto one page.
    pub fn export_pdf(&mut self, all_sheets: bool, range: Option<&str>, fit_to_page: bool) -> Result<Vec<u8>, GridError> {
        register_pdf_fonts();
        let mut params = serde_json::json!({"sheets": if all_sheets { "all" } else { "active" }, "fitToPage": fit_to_page});
        if let Some(r) = range.map(str::trim).filter(|r| !r.is_empty()) {
            params["range"] = serde_json::json!(r);
        }
        let out = self.session.execute("file.exportPdf", params).map_err(|e| GridError::Save(e.to_string()))?;
        out["base64"].as_str().and_then(io::base64_decode).ok_or_else(|| GridError::Save("no PDF produced".into()))
    }

    pub fn save(&self, file_name: &str) -> Result<Vec<u8>, GridError> {
        let kind = FileKind::from_path(file_name).unwrap_or(FileKind::Xlsx);
        io::save_bytes_as(self.wb(), file_name, kind).map_err(|e| GridError::Save(e.to_string()))
    }
}

fn is_delimited_text(name: &str) -> bool {
    matches!(FileKind::from_path(name), Some(FileKind::Csv | FileKind::Tsv))
}

#[cfg(test)]
mod tests;
mod turkish;
pub use turkish::{collation_key, from_engine, function_names, number_display, to_engine};

/// Hands WordCraft's bundled fonts (OFL: Source Sans 3, Source Serif 4, JetBrains Mono) to the
/// PDF writer once, so spreadsheet PDFs embed real fonts with every Turkish letter instead of
/// relying on the viewer's standard fonts (some lack İ).
fn register_pdf_fonts() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // wordcraft-fonts' bundle order: Sans Regular, Semibold, Bold, Italic; Serif Regular,
        // Italic, Semibold, Bold, BoldItalic; Inter ×3; JetBrains Mono.
        let b = wordcraft_fonts::bundled();
        if b.len() < 13 {
            return;
        }
        let (sans, sans_bold, sans_it) = (b[0], b[2], b[3]);
        let (serif, serif_it, serif_bold, serif_bold_it) = (b[4], b[5], b[7], b[8]);
        let mono = b[12];
        gridcraft_pdf::set_text_fonts([sans, sans_bold, sans_it, sans_bold, serif, serif_bold, serif_it, serif_bold_it, mono, mono, mono, mono]);
    });
}

/// Whether a cell shows a number (not a date, time or text), whose separators follow the locale.
fn shows_number(wb: &Workbook, sh: &gridcraft_engine::model::Sheet, c: gridcraft_engine::core::CellRef) -> bool {
    if !matches!(sh.value(c), gridcraft_engine::core::Value::Number(_)) || sh.cell(c).is_some_and(|x| x.formula.is_some() && sh.show_formulas) {
        return false;
    }
    let fmt = gridcraft_engine::numfmt::NumberFormat::parse_cached(wb.styles.get(sh.style_id(c)).num_fmt.as_str());
    !fmt.is_date() && !fmt.is_text()
}
