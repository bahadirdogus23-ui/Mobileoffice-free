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
mod losses;

pub use encoding::{decode_turkish_legacy, looks_like_windows_1254};
pub use losses::Loss;

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
/// Where a chart sits on its sheet: its anchor cell, the offset from that cell's top-left corner
/// and its size, in the grid's unit.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartBox {
    pub index: u32,
    pub row: u32,
    pub col: u32,
    pub dx: f32,
    pub dy: f32,
    pub width: f32,
    pub height: f32,
}

/// A chart drawn: straight RGBA pixels (`width`×`height`, opaque on white) and the words to
/// write over them.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartPicture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub texts: Vec<ChartText>,
}

/// A word on a chart: anchored at (`x`, `y`) pixels by `align` (0 left, 1 centre, 2 right) and
/// `valign` (0 top, 1 middle, 2 bottom), turned `rotation` radians; `color` is ARGB.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartText {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub size: f32,
    pub color: u32,
    pub bold: bool,
    pub align: u8,
    pub valign: u8,
    pub rotation: f32,
}

/// A note or comment on a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellNote {
    pub row: u32,
    pub col: u32,
    pub author: String,
    pub text: String,
}

/// A cell found by [`GridDoc::find_cells`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellHit {
    pub sheet: u32,
    pub row: u32,
    pub col: u32,
}

/// Text lowered the Turkish way one character at a time (I → ı, İ → i), so positions in the
/// result are positions in the original.
fn fold_chars(text: &str) -> Vec<char> {
    text.chars()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            _ => c.to_lowercase().next().unwrap_or(c),
        })
        .collect()
}

/// Text in its search form, as the document, PDF and file searches fold it: lower case, the
/// dotted and dotless i merged and accents dropped, so "istanbul" finds "İSTANBUL" and
/// "Istanbul", and "cagri" finds "Çağrı" (people type without the Turkish keyboard as often as
/// with it). Only finding uses it; replacing keeps the exact Turkish case rules of [`fold_chars`].
fn search_fold(text: &str) -> Vec<char> {
    text.chars()
        .map(|c| match c {
            'ı' | 'I' | 'İ' | 'î' | 'Î' | 'í' | 'Í' | 'ì' | 'Ì' | 'ï' | 'Ï' => 'i',
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ö' | 'Ö' | 'ó' | 'Ó' | 'ò' | 'Ò' | 'ô' | 'Ô' | 'õ' | 'Õ' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' | 'û' | 'Û' | 'ú' | 'Ú' | 'ù' | 'Ù' => 'u',
            'â' | 'Â' | 'á' | 'Á' | 'à' | 'À' | 'ä' | 'Ä' | 'ã' | 'Ã' => 'a',
            'é' | 'É' | 'è' | 'È' | 'ê' | 'Ê' | 'ë' | 'Ë' => 'e',
            'ñ' | 'Ñ' => 'n',
            _ => c.to_lowercase().next().unwrap_or(c),
        })
        .collect()
}

fn find_folded(hay: &[char], needle: &[char]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

/// `text` with every match of `needle` (already folded) replaced by `with`.
fn replace_folded(text: &str, needle: &[char], with: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let folded = fold_chars(text);
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if folded.len() - i >= needle.len() && folded[i..i + needle.len()] == *needle {
            out.push_str(with);
            i += needle.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// One value a column filter can show.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterValue {
    /// As the filter matches it (the cell's text); empty for blank cells.
    pub value: String,
    /// As shown in the list (Turkish number style when that is on).
    pub label: String,
    pub count: u32,
    pub selected: bool,
}

pub struct GridDoc {
    session: Session,
    warnings: Vec<String>,
    /// Sheets the engine stopped reading part-way (see [`Truncation`]).
    truncated: Vec<Truncation>,
    /// The shared strings the engine kept when it stopped reading that list part-way.
    strings_kept: Option<u64>,
    /// Sheets the engine did not read at all, and parts it skipped (see [`GridDoc::unread_sheets`]).
    unread_sheets: Vec<String>,
    skipped_parts: Vec<String>,
    /// Everything else an XLSX file holds that saving it back would not write (see [`Loss`]).
    losses: Vec<Loss>,
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
/// fill). `align`: 0 default text, 1 left, 2 centre, 3 right. General alignment is resolved
/// from the cell value (numbers right, booleans/errors centred), not the displayed characters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CellLook {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: u32,
    pub fill: u32,
    pub align: u8,
    /// Border weights, a byte per side (left, right, top, bottom from the low byte): 0 none,
    /// 1 thin, 2 medium or double, 3 thick.
    pub borders: u32,
    /// The borders' colour (0 = automatic, black).
    pub border_color: u32,
    /// Text wraps onto more lines within the cell.
    pub wrap: bool,
    /// 0 bottom (Excel's default), 1 centre, 2 top.
    pub valign: u8,
    /// A conditional format's data bar: the fraction (0–1) of the cell it fills (0 for none) and its colour.
    pub bar: f32,
    pub bar_color: u32,
    /// A conditional format's icon: 0 none, 1 arrow, 2 rating (filled share), 3 dot; its index
    /// (0 = lowest) among `icon_count` icons.
    pub icon_kind: u8,
    pub icon_index: u8,
    pub icon_count: u8,
}

/// A sheet the engine stopped reading part-way: what follows its last kept row is not in the
/// workbook, so saving it over the original file would drop those rows for good.
///
/// GridCraft reports this only as a warning (`sheet "X" is damaged (...); kept N rows`), e.g.
/// when a worksheet holds invalid XML or a single row passes its XML element limit. (Before
/// c0ee120 any sheet past ~1,000,000 cell elements was cut this way:
/// docs/upstream-notlar/gridcraft-satir-siniri.md.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Truncation {
    pub sheet: String,
    /// Index of the sheet in the workbook.
    pub index: u32,
    /// `<row>` elements the engine read before it stopped.
    pub rows_kept: u64,
    /// The last row (1-based, as shown in the grid) the sheet now holds; 0 when it is empty.
    pub last_row: u32,
}

/// The sheet name and kept-row count of GridCraft's "sheet is damaged" warning, if `w` is one.
fn parse_truncation(w: &str) -> Option<(&str, u64)> {
    let rest = w.strip_prefix("sheet \"")?;
    let (name, _) = rest.rsplit_once("\" is damaged (")?;
    let (_, tail) = rest.rsplit_once("; kept ")?;
    let rows = tail.strip_suffix(" rows")?.parse().ok()?;
    Some((name, rows))
}

/// The kept-string count of GridCraft's "shared strings are damaged" warning, if `w` is one.
/// Cells that refer to a string after the cut are read as empty.
fn parse_strings_cut(w: &str) -> Option<u64> {
    let rest = w.strip_prefix("shared strings are damaged (")?;
    let (_, tail) = rest.rsplit_once("; kept ")?;
    tail.strip_suffix(" strings")?.parse().ok()
}

/// The sheet name of GridCraft's warnings for a sheet it did not read at all, if `w` is one:
/// `sheet "X" could not be read: …` (an empty sheet stands in for it), `sheet "X" is a K and was
/// skipped`, `chart sheet "X" was skipped and is not kept on save` and `chart sheet "X" was
/// dropped: …` (crates/xlsx/src/read.rs:337-347, passthrough/read.rs:187, :232). Saving over the
/// original would delete such a sheet.
fn parse_unread_sheet(w: &str) -> Option<&str> {
    if let Some(rest) = w.strip_prefix("chart sheet \"") {
        return rest.strip_suffix("\" was skipped and is not kept on save").or_else(|| rest.rsplit_once("\" was dropped: ").map(|(name, _)| name));
    }
    let rest = w.strip_prefix("sheet \"")?;
    if let Some((name, _)) = rest.split_once("\" could not be read: ") {
        return Some(name);
    }
    let (name, kind) = rest.rsplit_once("\" is a ")?;
    kind.strip_suffix(" and was skipped").filter(|k| !k.contains('"')).map(|_| name)
}

/// The part named by GridCraft's warning for an optional part it could not parse (styles, theme,
/// comments, drawings, tables…: crates/xlsx/src/read.rs:88), if `w` is one.
fn parse_skipped_part(w: &str) -> Option<&str> {
    let rest = w.strip_prefix("skipped ")?;
    rest.split_once(": ").map(|(part, _)| part).filter(|p| !p.is_empty() && !p.contains(' '))
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
        GridDoc {
            session,
            warnings: Vec::new(),
            truncated: Vec::new(),
            strings_kept: None,
            unread_sheets: Vec::new(),
            skipped_parts: Vec::new(),
            losses: Vec::new(),
            turkish: false,
        }
    }

    /// Opens an XLSX/XLSM/XLS/XLSB/ODS/CSV/TSV file from its bytes. `name` is the file name; its
    /// extension helps pick the reader, but content is sniffed first, as the engine does.
    pub fn open(name: &str, bytes: &[u8], password: Option<&str>) -> Result<GridDoc, GridError> {
        let mut warnings = Vec::new();
        let utf8;
        // UTF-16 first: its bytes are not UTF-8 and may hold a Windows-1254 letter's value.
        let bytes = if is_delimited_text(name)
            && let Some(text) = encoding::decode_unmarked_utf16(bytes)
        {
            utf8 = text;
            utf8.as_bytes()
        } else if is_delimited_text(name) && looks_like_windows_1254(bytes) {
            utf8 = decode_turkish_legacy(bytes);
            warnings.push("Metin Windows-1254 (Türkçe) kodlamasından dönüştürüldü.".to_string());
            utf8.as_bytes()
        } else {
            bytes
        };
        let (mut wb, engine_warnings) = io::open_bytes_with_password(name, bytes, password).map_err(|e| GridError::Open(e.to_string()))?;
        // Excel 97-2003 files: the engine reads no fonts, fills or borders; they are read here.
        let xls = bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]);
        let styled = if xls { xls_styles::apply(&mut wb, bytes) } else { 0 };
        warnings.extend(engine_warnings.into_iter().map(|w| if styled > 0 { w.replace("Fonts, fills, borders, charts", "Charts") } else { w }));
        let title = std::path::Path::new(name).file_name().and_then(|n| n.to_str()).unwrap_or("Book").to_string();
        let mut session = Session::new();
        session.add_document(DocState::new(wb, None, title, Trust::FromFile));
        let strings_kept = warnings.iter().find_map(|w| parse_strings_cut(w));
        let unread_sheets = warnings.iter().filter_map(|w| parse_unread_sheet(w)).map(str::to_string).collect();
        let skipped_parts = warnings.iter().filter_map(|w| parse_skipped_part(w)).map(str::to_string).collect();
        // The warnings below are the XLSX reader's; the app saves only XLSX over the opened file.
        let mut losses = if xlsx_family(name) && bytes.starts_with(b"PK") { losses::from_warnings(&warnings) } else { Vec::new() };
        losses.extend(losses::from_package(name, bytes));
        losses.sort();
        losses.dedup();
        let mut doc = GridDoc { session, warnings, truncated: Vec::new(), strings_kept, unread_sheets, skipped_parts, losses, turkish: false };
        doc.truncated = doc.find_truncations();
        Ok(doc)
    }

    fn find_truncations(&self) -> Vec<Truncation> {
        let names = self.sheet_names();
        let mut out: Vec<Truncation> = Vec::new();
        for (name, rows_kept) in self.warnings.iter().filter_map(|w| parse_truncation(w)) {
            let Some(index) = names.iter().position(|n| n == name) else { continue };
            if out.iter().any(|t| t.index as usize == index) {
                continue;
            }
            let last_row = self.used_size(index as u32).map(|u| u.rows).unwrap_or(0);
            out.push(Truncation { sheet: name.to_string(), index: index as u32, rows_kept, last_row });
        }
        out
    }

    /// What the import could not keep (shown to the person before they edit).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Sheets that were cut short on opening. When not empty, the workbook must not be saved over
    /// the file it came from: the rows after each cut would be lost (AGENTS.md invariant 4).
    pub fn truncated(&self) -> &[Truncation] {
        &self.truncated
    }

    /// When the engine stopped reading the shared-string list part-way: how many strings it kept.
    /// Cells holding a later string open empty, so, as with [`GridDoc::truncated`], the workbook
    /// must not be saved over the file it came from.
    pub fn strings_kept(&self) -> Option<u64> {
        self.strings_kept
    }

    /// Sheets the engine did not read at all (a damaged worksheet opens empty; a dialog, macro or
    /// chart sheet is left out). As with [`GridDoc::truncated`], the workbook must not be saved
    /// over the file it came from: these sheets would be lost.
    pub fn unread_sheets(&self) -> &[String] {
        &self.unread_sheets
    }

    /// Parts the engine could not parse and left out (styles, theme, comments, drawings, tables…):
    /// saving over the original would drop what they hold.
    pub fn skipped_parts(&self) -> &[String] {
        &self.skipped_parts
    }

    /// What else the opened XLSX file holds that saving it back would not write: hyperlinks or
    /// conditional formats past the engine's limits, slicers, a VBA project in an `.xlsx`, what-if
    /// scenarios… (see [`Loss`]). As with [`GridDoc::truncated`], the workbook must not be saved
    /// over the file it came from.
    pub fn losses(&self) -> &[Loss] {
        &self.losses
    }

    /// Whether opening lost data the file holds (a cut sheet or string list, a sheet or part not
    /// read, anything in [`GridDoc::losses`]): saving over the original would make that loss
    /// permanent.
    pub fn opened_incomplete(&self) -> bool {
        !self.truncated.is_empty()
            || self.strings_kept.is_some()
            || !self.unread_sheets.is_empty()
            || !self.skipped_parts.is_empty()
            || !self.losses.is_empty()
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

    /// The merged ranges of `sheet` as (first row, first column, last row, last column).
    pub fn merges(&self, sheet: u32) -> Result<Vec<[u32; 4]>, GridError> {
        let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        Ok(sh.merges.iter().map(|m| [m.start.row, m.start.col, m.end.row, m.end.col]).collect())
    }

    /// How each cell of a `rows`×`cols` window looks, row by row, for drawing the grid: the
    /// cell's own style (conditional formats are left to the PDF and print paths).
    pub fn cell_looks(&self, sheet: u32, row: u32, col: u32, rows: u32, cols: u32) -> Result<Vec<CellLook>, GridError> {
        use gridcraft_engine::model::style::{BorderLine, BorderStyle, HAlign, PatternType, Underline, VAlign};
        // 0 none, 1 thin (hair, dashed, dotted too), 2 medium or double, 3 thick.
        let weight = |b: &BorderLine| -> u32 {
            match b.style {
                BorderStyle::None => 0,
                BorderStyle::Thick => 3,
                BorderStyle::Medium
                | BorderStyle::Double
                | BorderStyle::MediumDashed
                | BorderStyle::MediumDashDot
                | BorderStyle::MediumDashDotDot
                | BorderStyle::SlantDashDot => 2,
                _ => 1,
            }
        };
        let wb = self.wb();
        let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        let argb = |rgb: Option<[u8; 3]>| rgb.map_or(0, |[r, g, b]| 0xFF00_0000 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b));
        // Conditional formats (colour scales, data bars, highlight rules), as Excel shows them.
        let mut cf = (!sh.cond_formats.is_empty()).then(gridcraft_engine::cf::CfCache::new);
        let mut out = Vec::with_capacity((rows as usize) * (cols as usize));
        for r in row..row.saturating_add(rows) {
            for c in col..col.saturating_add(cols) {
                let at = gridcraft_engine::cell(r, c);
                // Inside a table (Format as Table, or one in the file): its banding and header.
                let table_style =
                    sh.tables.iter().find(|t| t.range.contains(at)).map(|t| gridcraft_engine::tables::table_cell_style(wb, sheet as usize, t, at));
                let base = table_style.as_ref().unwrap_or_else(|| wb.styles.get(sh.style_id(at)));
                let cfl = cf.as_mut().and_then(|c| c.look(wb, sheet as usize, at));
                let merged = cfl.as_ref().and_then(|l| l.style.as_ref()).map(|d| {
                    use gridcraft_engine::model::Color;
                    // A differential style: only what it sets applies.
                    let mut st = base.clone();
                    if d.fill.pattern != PatternType::None {
                        st.fill = d.fill;
                    }
                    if d.font.color != Color::Auto {
                        st.font.color = d.font.color;
                    }
                    st.font.bold |= d.font.bold;
                    st.font.italic |= d.font.italic;
                    if d.font.underline != Underline::None {
                        st.font.underline = d.font.underline;
                    }
                    for (dst, src) in [
                        (&mut st.borders.left, d.borders.left),
                        (&mut st.borders.right, d.borders.right),
                        (&mut st.borders.top, d.borders.top),
                        (&mut st.borders.bottom, d.borders.bottom),
                    ] {
                        if !src.is_none() {
                            *dst = src;
                        }
                    }
                    st
                });
                let st = merged.as_ref().unwrap_or(base);
                let scale_fill = cfl.as_ref().and_then(|l| l.scale_fill);
                let bar = cfl.as_ref().and_then(|l| l.bar);
                let icon = cfl.as_ref().and_then(|l| l.icon.clone());
                let b = &st.borders;
                let sides = [&b.left, &b.right, &b.top, &b.bottom];
                let borders = sides.iter().enumerate().fold(0u32, |acc, (i, l)| acc | weight(l) << (8 * i));
                let border_color = sides.iter().find(|l| l.style != BorderStyle::None).map_or(0, |l| argb(l.color.resolve(&wb.theme)));
                out.push(CellLook {
                    borders,
                    border_color,
                    bold: st.font.bold,
                    italic: st.font.italic,
                    underline: st.font.underline != Underline::None,
                    color: argb(st.font.color.resolve(&wb.theme)),
                    fill: match scale_fill {
                        Some(rgb) => argb(Some(rgb)),
                        None if st.fill.pattern == PatternType::Solid => argb(st.fill.fg.resolve(&wb.theme)),
                        None => 0,
                    },
                    bar: bar.map_or(0.0, |(f, _, _)| f.clamp(0.0, 1.0)),
                    bar_color: bar.map_or(0, |(_, rgb, _)| argb(Some(rgb))),
                    icon_kind: icon.as_ref().map_or(0, |(set, _)| {
                        let set = set.to_ascii_lowercase();
                        if set.contains("arrow") {
                            1
                        } else if set.contains("rating") || set.contains("quarter") {
                            2
                        } else {
                            3
                        }
                    }),
                    icon_index: icon.as_ref().map_or(0, |(_, i)| (*i).min(255) as u8),
                    icon_count: icon.as_ref().map_or(0, |(set, _)| gridcraft_engine::cf::icon_count(set).min(255) as u8),
                    wrap: st.align.wrap,
                    valign: match st.align.v {
                        VAlign::Center | VAlign::Justify | VAlign::Distributed => 1,
                        VAlign::Top => 2,
                        _ => 0,
                    },
                    align: match st.align.h {
                        HAlign::Left => 1,
                        HAlign::Center | HAlign::CenterAcross => 2,
                        HAlign::Right => 3,
                        HAlign::General if sh.show_formulas && sh.cell(at).is_some_and(|cell| cell.formula.is_some()) => 0,
                        HAlign::General => match sh.value(at) {
                            gridcraft_engine::core::Value::Number(_) => 3,
                            gridcraft_engine::core::Value::Bool(_) | gridcraft_engine::core::Value::Error(_) => 2,
                            _ => 0,
                        },
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
    /// Every function with its arguments and what it does, for hints and the formula help page.
    pub fn function_docs(&self) -> Vec<FunctionDoc> {
        function_docs(self.turkish)
    }

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

    /// The different values in column `col` of the active sheet's filter range (or of the used
    /// area under row 1 when it has none), sorted, each with how many rows hold it and whether
    /// the column's filter shows it now. An empty `value` stands for blank cells.
    pub fn filter_values(&self, col: u32) -> Result<Vec<FilterValue>, GridError> {
        use gridcraft_engine::model::FilterCriterion;
        let sheet = self.active_sheet();
        let wb = self.wb();
        let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        let (first, last, shown): (u32, u32, Option<(Vec<String>, bool)>) = match &sh.autofilter {
            Some(af) if col >= af.range.start.col && col <= af.range.end.col => {
                let crit = af.criteria.iter().find(|(o, _)| *o == col - af.range.start.col).and_then(|(_, c)| match c {
                    FilterCriterion::Values { values, blanks } => Some((values.clone(), *blanks)),
                    _ => None,
                });
                (af.range.start.row + 1, af.range.end.row, crit)
            }
            _ => (1, self.used_size(sheet)?.rows.saturating_sub(1), None),
        };
        // Each value with its number, if it is one: numbers sort by size, before text.
        let mut found: Vec<(FilterValue, Option<f64>)> = Vec::new();
        for r in first..=last {
            let cell = gridcraft_engine::cell(r, col);
            let v = sh.value(cell);
            let blank = v.is_empty() || matches!(&v, gridcraft_engine::core::Value::Text(t) if t.is_empty());
            let value = if blank { String::new() } else { display::cell_text(wb, sh, cell) };
            if let Some((f, _)) = found.iter_mut().find(|(f, _)| f.value.eq_ignore_ascii_case(&value)) {
                f.count += 1;
                continue;
            }
            let label = if self.turkish && !blank && shows_number(wb, sh, cell) { number_display(&value) } else { value.clone() };
            let selected = match &shown {
                None => true,
                Some((_, blanks)) if blank => *blanks,
                Some((values, _)) => values.iter().any(|x| x.eq_ignore_ascii_case(&value)),
            };
            found.push((FilterValue { value, label, count: 1, selected }, if blank { None } else { v.as_f64() }));
        }
        found.sort_by(|(a, na), (b, nb)| {
            let text_key = |f: &FilterValue| -> Vec<u32> {
                if self.turkish { collation_key(&f.label) } else { f.label.to_lowercase().chars().map(u32::from).collect() }
            };
            (a.value.is_empty(), na.is_none())
                .cmp(&(b.value.is_empty(), nb.is_none()))
                .then_with(|| na.unwrap_or(0.0).total_cmp(&nb.unwrap_or(0.0)))
                .then_with(|| text_key(a).cmp(&text_key(b)))
        });
        Ok(found.into_iter().map(|(f, _)| f).collect())
    }

    /// Shows only the rows whose column `col` holds one of `values` (empty for blank cells),
    /// putting a filter on the used area first when the sheet has none.
    pub fn filter_column(&mut self, col: u32, values: &[String]) -> Result<(), GridError> {
        let sheet = self.active_sheet();
        let has_filter = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?.autofilter.is_some();
        if !has_filter {
            let used = self.used_size(sheet)?;
            if used.rows < 2 || used.cols == 0 {
                return Ok(());
            }
            let last = gridcraft_engine::cell(used.rows - 1, used.cols - 1).a1();
            self.execute("data.filter", &serde_json::json!({ "range": format!("A1:{last}") }).to_string())?;
        }
        let letter = gridcraft_engine::cell(0, col).a1().trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
        let blanks = values.iter().any(String::is_empty);
        let shown: Vec<&String> = values.iter().filter(|v| !v.is_empty()).collect();
        self.execute("data.filterBy", &serde_json::json!({ "column": letter, "values": shown, "blanks": blanks }).to_string()).map(|_| ())
    }

    /// Shows every row again and takes the filter off the sheet.
    pub fn remove_filter(&mut self) -> Result<(), GridError> {
        let sheet = self.active_sheet();
        if self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?.autofilter.is_some() {
            self.execute("data.clearFilter", "{}")?;
            self.execute("data.filter", "{}")?;
        }
        Ok(())
    }

    /// The notes and comments of `sheet`, row by row: where they are, who wrote them and their
    /// text (a thread's replies after it, each on its own line).
    pub fn notes(&self, sheet: u32) -> Result<Vec<CellNote>, GridError> {
        let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        Ok(sh
            .comments
            .iter()
            .map(|(at, c)| {
                let mut text = c.text.clone();
                for (who, reply) in &c.replies {
                    text.push('\n');
                    if !who.is_empty() {
                        text.push_str(who);
                        text.push_str(": ");
                    }
                    text.push_str(reply);
                }
                CellNote { row: at.row, col: at.col, author: c.author.clone(), text }
            })
            .collect())
    }

    /// The choices of cell (`row`, `col`)'s drop-down list (Data Validation › List) on the active
    /// sheet: written out in the rule or read from a range. Empty when it has none.
    pub fn list_choices(&self, row: u32, col: u32) -> Vec<String> {
        use gridcraft_engine::model::ValidationKind;
        let sheet = self.active_sheet() as usize;
        let wb = self.wb();
        let Some(sh) = wb.sheets.get(sheet) else { return Vec::new() };
        let at = gridcraft_engine::cell(row, col);
        sh.validations
            .iter()
            .find(|v| v.kind == ValidationKind::List && v.ranges.iter().any(|r| r.contains(at)))
            .map(|v| gridcraft_engine::cmd::data::list_items(wb, sheet, at, v))
            .unwrap_or_default()
    }

    /// The charts on `sheet`: where each is (its anchor cell, the offset from that cell's corner
    /// and its size, in the grid's unit).
    pub fn charts(&self, sheet: u32) -> Result<Vec<ChartBox>, GridError> {
        let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        Ok(sh
            .charts
            .iter()
            .enumerate()
            .map(|(i, c)| ChartBox {
                index: i as u32,
                row: c.anchor.cell.row,
                col: c.anchor.cell.col,
                dx: c.anchor.dx,
                dy: c.anchor.dy,
                width: c.anchor.width,
                height: c.anchor.height,
            })
            .collect())
    }

    /// Changes chart `index` of `sheet` with GridCraft's `chart.set` (`params`: type, title, at,
    /// dx, dy, width, height…), or deletes it when `params` is `{"delete": true}`.
    pub fn change_chart(&mut self, sheet: u32, index: u32, params: &str) -> Result<(), GridError> {
        let id = {
            let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
            sh.charts.get(index as usize).map(|c| c.id).ok_or_else(|| GridError::Command("no such chart".into()))?
        };
        // The engine's chart commands work on the active sheet.
        self.set_active_sheet(sheet)?;
        let mut p: serde_json::Value = serde_json::from_str(params).map_err(|e| GridError::Command(e.to_string()))?;
        let delete = p.get("delete").and_then(serde_json::Value::as_bool) == Some(true);
        p["chart"] = id.into();
        if delete {
            self.execute("chart.delete", &serde_json::json!({ "chart": id }).to_string())?;
        } else {
            self.execute("chart.set", &p.to_string())?;
        }
        Ok(())
    }

    /// The pictures on `sheet`: where each is, like [`GridDoc::charts`] (`index` counts pictures).
    pub fn pictures(&self, sheet: u32) -> Result<Vec<ChartBox>, GridError> {
        let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        Ok(sh
            .images
            .iter()
            .enumerate()
            .map(|(i, im)| ChartBox {
                index: i as u32,
                row: im.anchor.cell.row,
                col: im.anchor.cell.col,
                dx: im.anchor.dx,
                dy: im.anchor.dy,
                width: im.anchor.width,
                height: im.anchor.height,
            })
            .collect())
    }

    /// Picture `index` of `sheet` as its file's bytes (PNG or JPEG).
    pub fn picture_bytes(&self, sheet: u32, index: u32) -> Result<Vec<u8>, GridError> {
        let sh = self.wb().sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        sh.images.get(index as usize).map(|im| im.data.as_ref().clone()).ok_or_else(|| GridError::Command("no such picture".into()))
    }

    /// Chart `index` of `sheet` drawn `scale` times its size on white: its shapes as straight
    /// RGBA pixels, and its words (titles, labels, legend) to be written over them.
    pub fn render_chart(&self, sheet: u32, index: u32, scale: f32) -> Result<ChartPicture, GridError> {
        use gridcraft_chart::Prim;
        let wb = self.wb();
        let sh = wb.sheets.get(sheet as usize).ok_or(GridError::NoSheet(sheet))?;
        let chart = sh.charts.get(index as usize).ok_or_else(|| GridError::Command("no such chart".into()))?;
        let (w, h) = (chart.anchor.width.max(1.0), chart.anchor.height.max(1.0));
        let scale = scale.clamp(0.1, 4.0);
        let data = gridcraft_chart::resolve(wb, sheet as usize, chart);
        let prims = gridcraft_chart::render(chart, &data, w, h, &gridcraft_chart::ApproxMeasure);
        let k = |v: f32| v * scale;
        let kp = |pts: &[[f32; 2]]| pts.iter().map(|p| [p[0] * scale, p[1] * scale]).collect::<Vec<_>>();
        let ks = |s: Option<([u8; 4], f32)>| s.map(|(c, wd)| (c, wd * scale));
        let mut texts = Vec::new();
        let scaled: Vec<Prim> = prims
            .into_iter()
            .filter_map(|p| match p {
                Prim::Rect { x, y, w, h, fill, stroke, radius } => {
                    Some(Prim::Rect { x: k(x), y: k(y), w: k(w), h: k(h), fill, stroke: ks(stroke), radius: k(radius) })
                }
                Prim::Line { pts, color, width, dash } => Some(Prim::Line { pts: kp(&pts), color, width: k(width), dash }),
                Prim::Polygon { pts, fill, stroke } => Some(Prim::Polygon { pts: kp(&pts), fill, stroke: ks(stroke) }),
                Prim::Wedge { cx, cy, r_outer, r_inner, a0, a1, fill, stroke } => {
                    Some(Prim::Wedge { cx: k(cx), cy: k(cy), r_outer: k(r_outer), r_inner: k(r_inner), a0, a1, fill, stroke: ks(stroke) })
                }
                Prim::Circle { cx, cy, r, fill, stroke } => Some(Prim::Circle { cx: k(cx), cy: k(cy), r: k(r), fill, stroke: ks(stroke) }),
                Prim::Text { x, y, text, size, color, bold, align, valign, rotation } => {
                    texts.push(ChartText {
                        x: k(x),
                        y: k(y),
                        text,
                        size: k(size),
                        color: u32::from(color[3]) << 24 | u32::from(color[0]) << 16 | u32::from(color[1]) << 8 | u32::from(color[2]),
                        bold,
                        align: match align {
                            gridcraft_chart::HAlign::Left => 0,
                            gridcraft_chart::HAlign::Center => 1,
                            gridcraft_chart::HAlign::Right => 2,
                        },
                        valign: match valign {
                            gridcraft_chart::VAlign::Top => 0,
                            gridcraft_chart::VAlign::Middle => 1,
                            gridcraft_chart::VAlign::Bottom => 2,
                        },
                        rotation,
                    });
                    None
                }
            })
            .collect();
        let (pw, ph) = ((w * scale).round().max(1.0) as u32, (h * scale).round().max(1.0) as u32);
        let pixels = gridcraft_chart::rasterize(&scaled, pw, ph, [255, 255, 255, 255]);
        Ok(ChartPicture { width: pw, height: ph, pixels, texts })
    }

    /// Whether the active sheet has a filter on.
    pub fn has_filter(&self) -> bool {
        self.wb().sheets.get(self.active_sheet() as usize).is_some_and(|sh| sh.autofilter.is_some())
    }

    /// The cells of every sheet whose shown text contains `query`, ignoring case, accents and
    /// the dotted/dotless i ([`search_fold`]), sheet by sheet and row by row.
    pub fn find_cells(&self, query: &str) -> Result<Vec<CellHit>, GridError> {
        let needle = search_fold(query.trim());
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let wb = self.wb();
        let mut hits = Vec::new();
        for (si, sh) in wb.sheets.iter().enumerate() {
            let used = self.used_size(si as u32)?;
            for r in 0..used.rows {
                for c in 0..used.cols {
                    let cell = gridcraft_engine::cell(r, c);
                    let text = display::cell_text(wb, sh, cell);
                    let shown = if self.turkish && shows_number(wb, sh, cell) { number_display(&text) } else { text };
                    if !shown.is_empty() && find_folded(&search_fold(&shown), &needle).is_some() {
                        hits.push(CellHit { sheet: si as u32, row: r, col: c });
                    }
                }
            }
        }
        Ok(hits)
    }

    /// Replaces `find` with `with` (ignoring case the Turkish way) in every cell of every sheet
    /// that holds typed text or a number; formulas are left alone. Returns how many cells changed.
    pub fn replace_all(&mut self, find: &str, with: &str) -> Result<u32, GridError> {
        let needle = fold_chars(find);
        if needle.is_empty() {
            return Ok(0);
        }
        let active = self.active_sheet();
        let mut changes: Vec<(u32, u32, u32, String)> = Vec::new();
        for si in 0..self.wb().sheets.len() as u32 {
            let used = self.used_size(si)?;
            for r in 0..used.rows {
                for c in 0..used.cols {
                    let typed = self.edit_text(si, r, c)?;
                    if typed.is_empty() || typed.starts_with('=') {
                        continue;
                    }
                    let replaced = replace_folded(&typed, &needle, with);
                    if replaced != typed {
                        changes.push((si, r, c, replaced));
                    }
                }
            }
        }
        for (si, r, c, text) in &changes {
            self.set_active_sheet(*si)?;
            self.set_input(*r, *c, text)?;
        }
        self.set_active_sheet(active)?;
        Ok(changes.len() as u32)
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

fn xlsx_family(name: &str) -> bool {
    matches!(FileKind::from_path(name), Some(FileKind::Xlsx))
}

fn is_delimited_text(name: &str) -> bool {
    matches!(FileKind::from_path(name), Some(FileKind::Csv | FileKind::Tsv))
}

#[cfg(test)]
mod tests;
mod turkish;
mod turkish_docs;
mod xls_styles;
pub use turkish::{FunctionDoc, collation_key, from_engine, function_docs, function_names, number_display, to_engine};

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
#[cfg(test)]
mod turkish_text_tests;
