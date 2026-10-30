//! Spreadsheet bridge: a thin, mobile-facing wrapper over the GridCraft engine.
//!
//! Everything a front end changes goes through [`GridDoc::execute`], i.e. GridCraft's own command
//! registry (`home.bold`, `cell.set`, …); this crate only adds what the engine's command surface
//! does not cover for a viewer: opening from bytes, reading a window of cell texts, and saving to
//! bytes. See docs/RAPOR-VE-PLAN.md §9.3.

use std::sync::Arc;

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
        GridDoc { session, warnings: Vec::new() }
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
        Ok(GridDoc { session, warnings })
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
                out.push(display::cell_text(wb, sh, gridcraft_engine::cell(r, c)));
            }
        }
        Ok(out)
    }

    /// What the formula bar shows for a cell: the formula, or the value as typed.
    pub fn edit_text(&self, sheet: u32, row: u32, col: u32) -> Result<String, GridError> {
        let wb = self.wb();
        let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        Ok(display::edit_text(wb, sh, gridcraft_engine::cell(row, col)))
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
