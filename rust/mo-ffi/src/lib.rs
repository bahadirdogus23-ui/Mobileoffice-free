//! The native library the Android app loads (`libmo_ffi.so`). It exposes the format bridges
//! (`mo-grid`, later `mo-word`, `mo-deck`, `mo-pdf`, `mo-cad`) to Kotlin through UniFFI.
//!
//! Engines run behind a `Mutex` per document: each document is used from one engine thread on
//! the Kotlin side, so the lock is uncontended; it only makes the object `Send + Sync` as UniFFI
//! requires. A panic that escapes an engine reaches Kotlin as an exception, not a crash.

use std::sync::{Mutex, MutexGuard};

use mo_grid::{GridDoc, GridError};

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
