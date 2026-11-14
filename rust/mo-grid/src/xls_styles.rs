//! Cell looks for Excel 97-2003 (.xls) files, which GridCraft reads without them: fonts, fills,
//! borders and alignment from the BIFF8 Workbook stream ([MS-XLS] FONT, XF, PALETTE and the cell
//! records), laid over the styles the engine read (it keeps their number formats).

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

use gridcraft_engine::model::style::{Alignment, BorderLine, BorderStyle, Borders, Color, Fill, Font, HAlign, PatternType, Style, Underline, VAlign};
use gridcraft_engine::model::{Cell, StyleId, Workbook};

const BOF: u16 = 0x0809;
const EOF: u16 = 0x000A;
const FILEPASS: u16 = 0x002F;
const FONT: u16 = 0x0031;
const XF: u16 = 0x00E0;
const PALETTE: u16 = 0x0092;
const BOUNDSHEET: u16 = 0x0085;

/// Excel 97's palette for colour indexes 8–63 (0–7 repeat the first eight).
const PALETTE_97: [u32; 56] = [
    0x000000, 0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF, 0x800000, 0x008000, 0x000080, 0x808000, 0x800080, 0x008080,
    0xC0C0C0, 0x808080, 0x9999FF, 0x993366, 0xFFFFCC, 0xCCFFFF, 0x660066, 0xFF8080, 0x0066CC, 0xCCCCFF, 0x000080, 0xFF00FF, 0xFFFF00, 0x00FFFF,
    0x800080, 0x800000, 0x008080, 0x0000FF, 0x00CCFF, 0xCCFFFF, 0xCCFFCC, 0xFFFF99, 0x99CCFF, 0xFF99CC, 0xCC99FF, 0xFFCC99, 0x3366FF, 0x33CCCC,
    0x99CC00, 0xFFCC00, 0xFF9900, 0xFF6600, 0x666699, 0x969696, 0x003366, 0x339966, 0x003300, 0x333300, 0x993300, 0x993366, 0x333399, 0x333333,
];

/// The records of a BIFF stream from `pos`: (type, body).
struct Records<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for Records<'a> {
    type Item = (u16, &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        let head = self.data.get(self.pos..self.pos + 4)?;
        let kind = u16::from_le_bytes([head[0], head[1]]);
        let len = usize::from(u16::from_le_bytes([head[2], head[3]]));
        let body = self.data.get(self.pos + 4..self.pos + 4 + len)?;
        self.pos += 4 + len;
        Some((kind, body))
    }
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(i)?, *b.get(i + 1)?]))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes([*b.get(i)?, *b.get(i + 1)?, *b.get(i + 2)?, *b.get(i + 3)?]))
}

/// A short Unicode string (count, flags, characters) at `i`.
fn short_string(b: &[u8], i: usize) -> Option<String> {
    let n = usize::from(*b.get(i)?);
    let wide = b.get(i + 1)? & 1 == 1;
    let s = i + 2;
    Some(if wide {
        let units: Vec<u16> = (0..n).filter_map(|k| u16_at(b, s + 2 * k)).collect();
        String::from_utf16_lossy(&units)
    } else {
        // Compressed: the low bytes of UTF-16, i.e. Latin-1.
        b.get(s..s + n)?.iter().map(|&c| char::from(c)).collect()
    })
}

struct Globals {
    /// Font, fill, borders and alignment per XF (the number format stays the engine's).
    xfs: Vec<(Font, Fill, Borders, Alignment)>,
    /// (sheet name, offset of its BOF) for worksheets.
    sheets: Vec<(String, usize)>,
}

fn color(palette: &[u32; 56], icv: u16) -> Color {
    match icv {
        0..=7 => Color::Rgb(palette[usize::from(icv)]),
        8..=63 => Color::Rgb(palette[usize::from(icv) - 8]),
        _ => Color::Auto,
    }
}

fn border(dg: u32) -> BorderStyle {
    match dg {
        1 => BorderStyle::Thin,
        2 => BorderStyle::Medium,
        3 => BorderStyle::Dashed,
        4 => BorderStyle::Dotted,
        5 => BorderStyle::Thick,
        6 => BorderStyle::Double,
        7 => BorderStyle::Hair,
        8 => BorderStyle::MediumDashed,
        9 => BorderStyle::DashDot,
        10 => BorderStyle::MediumDashDot,
        11 => BorderStyle::DashDotDot,
        12 => BorderStyle::MediumDashDotDot,
        13 => BorderStyle::SlantDashDot,
        _ => BorderStyle::None,
    }
}

fn pattern(fls: u32) -> PatternType {
    match fls {
        0 => PatternType::None,
        1 => PatternType::Solid,
        2 => PatternType::Gray50,
        3 => PatternType::Gray75,
        4 => PatternType::Gray25,
        5 => PatternType::DarkHorizontal,
        6 => PatternType::DarkVertical,
        7 => PatternType::DarkDown,
        8 => PatternType::DarkUp,
        9 => PatternType::DarkGrid,
        10 => PatternType::DarkTrellis,
        11 => PatternType::LightHorizontal,
        12 => PatternType::LightVertical,
        13 => PatternType::LightDown,
        14 => PatternType::LightUp,
        15 => PatternType::LightGrid,
        16 => PatternType::LightTrellis,
        17 => PatternType::Gray125,
        18 => PatternType::Gray0625,
        _ => PatternType::None,
    }
}

fn globals(stream: &[u8]) -> Option<Globals> {
    let mut palette = PALETTE_97;
    // Raw records first: the palette may come after the fonts and XFs that use it.
    let mut raw_fonts: Vec<&[u8]> = Vec::new();
    let mut raw_xfs: Vec<&[u8]> = Vec::new();
    let mut sheets = Vec::new();
    for (i, (kind, body)) in (Records { data: stream, pos: 0 }).enumerate() {
        match kind {
            BOF if i > 0 => break,
            EOF => break,
            FILEPASS => return None,
            FONT => raw_fonts.push(body),
            XF => raw_xfs.push(body),
            PALETTE => {
                let n = usize::from(u16_at(body, 0)?).min(56);
                for (k, slot) in palette.iter_mut().enumerate().take(n) {
                    let c = body.get(2 + 4 * k..2 + 4 * k + 3)?;
                    *slot = u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]);
                }
            }
            // Worksheets only (type 0); charts and macro sheets have no cells to style.
            BOUNDSHEET if body.get(5) == Some(&0) => sheets.push((short_string(body, 6)?, u32_at(body, 0)? as usize)),
            _ => {}
        }
    }
    let fonts: Vec<Font> = raw_fonts
        .iter()
        .map(|b| {
            let grbit = u16_at(b, 2).unwrap_or(0);
            Font {
                name: short_string(b, 14).unwrap_or_else(|| "Arial".into()),
                size: f32::from(u16_at(b, 0).unwrap_or(200)) / 20.0,
                bold: u16_at(b, 6).unwrap_or(400) >= 700,
                italic: grbit & 0x2 != 0,
                strike: grbit & 0x8 != 0,
                underline: match b.get(10) {
                    Some(1) => Underline::Single,
                    Some(2) => Underline::Double,
                    Some(0x21) => Underline::SingleAccounting,
                    Some(0x22) => Underline::DoubleAccounting,
                    _ => Underline::None,
                },
                color: color(&palette, u16_at(b, 4).unwrap_or(0x7FFF)),
                ..Font::default()
            }
        })
        .collect();
    let xfs = raw_xfs
        .iter()
        .map(|b| {
            let ifnt = usize::from(u16_at(b, 0).unwrap_or(0));
            // Font index 4 does not exist: indexes from 5 on are one past their place.
            let font = fonts.get(if ifnt >= 4 { ifnt - 1 } else { ifnt }).cloned().unwrap_or_default();
            let al = b.get(6).copied().unwrap_or(0);
            let align = Alignment {
                h: match al & 0x7 {
                    1 => HAlign::Left,
                    2 => HAlign::Center,
                    3 => HAlign::Right,
                    4 => HAlign::Fill,
                    5 => HAlign::Justify,
                    6 => HAlign::CenterAcross,
                    7 => HAlign::Distributed,
                    _ => HAlign::General,
                },
                v: match (al >> 4) & 0x7 {
                    0 => VAlign::Top,
                    1 => VAlign::Center,
                    3 => VAlign::Justify,
                    4 => VAlign::Distributed,
                    _ => VAlign::Bottom,
                },
                wrap: al & 0x8 != 0,
                indent: b.get(8).map_or(0, |i| i & 0xF),
                ..Alignment::default()
            };
            let b1 = u32_at(b, 10).unwrap_or(0);
            let b2 = u32_at(b, 14).unwrap_or(0);
            let line = |dg: u32, icv: u32| BorderLine { style: border(dg), color: color(&palette, icv as u16) };
            let borders = Borders {
                left: line(b1 & 0xF, (b1 >> 16) & 0x7F),
                right: line((b1 >> 4) & 0xF, (b1 >> 23) & 0x7F),
                top: line((b1 >> 8) & 0xF, b2 & 0x7F),
                bottom: line((b1 >> 12) & 0xF, (b2 >> 7) & 0x7F),
                ..Borders::default()
            };
            let colors = u16_at(b, 18).unwrap_or(0);
            let fill = Fill { pattern: pattern(b2 >> 26), fg: color(&palette, colors & 0x7F), bg: color(&palette, (colors >> 7) & 0x7F) };
            (font, fill, borders, align)
        })
        .collect();
    Some(Globals { xfs, sheets })
}

/// (row, column, XF index) of every cell record in the worksheet whose BOF is at `start`.
fn cell_xfs(stream: &[u8], start: usize) -> Vec<(u32, u32, u16)> {
    let mut out = Vec::new();
    for (i, (kind, b)) in (Records { data: stream, pos: start }).enumerate() {
        match kind {
            BOF if i > 0 => break,
            EOF => break,
            // NUMBER, RK, LABELSST, LABEL, BLANK, BOOLERR, FORMULA: row, column, XF.
            0x0203 | 0x027E | 0x00FD | 0x0204 | 0x0201 | 0x0205 | 0x0006 => {
                if let (Some(r), Some(c), Some(x)) = (u16_at(b, 0), u16_at(b, 2), u16_at(b, 4)) {
                    out.push((u32::from(r), u32::from(c), x));
                }
            }
            // MULRK (XF + RK value per cell) and MULBLANK (XF per cell).
            0x00BD | 0x00BE => {
                let step = if kind == 0x00BD { 6 } else { 2 };
                if let (Some(r), Some(first)) = (u16_at(b, 0), u16_at(b, 2)) {
                    let n = b.len().saturating_sub(6) / step;
                    for k in 0..n {
                        if let Some(x) = u16_at(b, 4 + k * step) {
                            out.push((u32::from(r), u32::from(first) + k as u32, x));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Lays the .xls file's fonts, fills, borders and alignment over the cells of `wb`, read from
/// the same `bytes`. Returns how many cells got them; nothing changes when the file can't be read.
pub(crate) fn apply(wb: &mut Workbook, bytes: &[u8]) -> usize {
    let Ok(mut file) = cfb::CompoundFile::open(std::io::Cursor::new(bytes)) else { return 0 };
    let mut stream = Vec::new();
    let opened = file.open_stream("/Workbook").or_else(|_| file.open_stream("/Book"));
    let Ok(mut s) = opened else { return 0 };
    if s.read_to_end(&mut stream).is_err() {
        return 0;
    }
    let Some(g) = globals(&stream) else { return 0 };
    let mut made: HashMap<(StyleId, u16), StyleId> = HashMap::new();
    let mut count = 0;
    for (name, start) in &g.sheets {
        let Some(index) = wb.sheets.iter().position(|s| &s.name == name) else { continue };
        let cells = cell_xfs(&stream, *start);
        if cells.is_empty() {
            continue;
        }
        let mut sheet = Arc::clone(&wb.sheets[index]);
        let sh = Arc::make_mut(&mut sheet);
        for (r, c, x) in cells {
            let Some((font, fill, borders, align)) = g.xfs.get(usize::from(x)) else { continue };
            let at = gridcraft_engine::cell(r, c);
            let old = sh.style_id(at);
            let id = *made.entry((old, x)).or_insert_with(|| {
                let base = wb.styles.get(old).clone();
                wb.styles.intern(Style { font: font.clone(), fill: *fill, borders: *borders, align: *align, ..base })
            });
            if id == old {
                continue;
            }
            match sh.cells.get_mut(at) {
                Some(cell) => cell.style = id,
                None => sh.cells.set(at, Cell { value: gridcraft_engine::core::Value::Empty, formula: None, style: id, py: None }),
            }
            count += 1;
        }
        wb.sheets[index] = sheet;
    }
    count
}
