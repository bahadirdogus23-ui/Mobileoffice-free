//! What saving an opened XLSX workbook back over its file would delete.
//!
//! GridCraft opens an XLSX leniently and keeps what it does not model only where its pass-through
//! knows how (crates/xlsx/src/passthrough.rs: custom XML, links to other workbooks, data
//! connections, query tables, form controls, OLE objects, printer settings, chart sheets…).
//! Everything else it leaves out is either reported as a warning ("… were dropped", "… is not
//! supported") or, for parts and elements it never looks at, not reported at all: slicers,
//! timelines, a Power Pivot data model, what-if scenarios, custom views, sheet extensions it
//! does not read. Each still opens, but saving it over the original file would delete what was
//! left out (AGENTS.md invariant 4); the app then offers only "Save as".
//!
//! Two checks, both mirroring the engine code they name (pinned commit c0ee120):
//! - [`from_warnings`]: the engine's own warnings, by their wording (pinned by unit tests).
//!   Wording not known to be harmless that says something was dropped, skipped or not kept
//!   counts as a loss, so new engine warnings err on the side of "Save as".
//! - [`from_package`]: the package itself: relationship kinds, workbook and worksheet elements
//!   and extensions the reader skips without a word.

use std::io::{Cursor, Read};

/// Something an opened XLSX file holds that saving it back would not write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Loss {
    /// Hyperlinks past the engine's limits, links to other workbooks or data connections.
    Links,
    /// Conditional formats of a type or extension the engine does not read, or past its limit.
    ConditionalFormats,
    /// PivotTables the engine could not read or turned into plain cells.
    PivotTables,
    /// Slicers and timelines.
    Slicers,
    /// Charts, embedded objects, grouped shapes, ink, linked pictures, pictures in cells,
    /// controls the engine could not keep.
    Drawings,
    /// A VBA project (macros) that the file's type cannot hold or the engine could not read.
    Macros,
    /// Formulas kept only as their values (what-if data tables, Python cells, shared formulas
    /// past the engine's limits).
    Formulas,
    /// AutoFilter criteria the engine does not read.
    Filters,
    /// Cell values the engine could not read.
    CellValues,
    /// Anything else (what-if scenarios, custom views, a data model, workbook sharing settings,
    /// sensitivity labels, unknown parts…).
    Other,
}

/// The losses GridCraft's warnings report (see the module documentation), sorted, no repeats.
pub fn from_warnings(warnings: &[String]) -> Vec<Loss> {
    let mut out: Vec<Loss> = warnings.iter().filter_map(|w| warning_loss(w)).collect();
    out.sort();
    out.dedup();
    out
}

/// Whether `w` (a GridCraft XLSX read warning) means the workbook lost something the file holds.
///
/// Not losses: references to parts that do not exist (`missing part`, `missing picture`, a chart
/// "missing from the package" or "without its chart part", a sheet that "has no part", macros
/// that "reference a missing part"), content the file format cannot hold (cells outside the grid,
/// phonetic runs with bad offsets) and content that was kept ("kept but not executed", "kept for
/// saving", links "kept; they are not followed"). Sheets left out and parts that did not parse
/// are reported by [`crate::GridDoc::unread_sheets`] and [`crate::GridDoc::skipped_parts`].
pub(crate) fn warning_loss(w: &str) -> Option<Loss> {
    // Handled elsewhere (unread sheets, skipped parts, cut sheets and strings).
    if crate::parse_unread_sheet(w).is_some()
        || crate::parse_skipped_part(w).is_some()
        || crate::parse_truncation(w).is_some()
        || crate::parse_strings_cut(w).is_some()
    {
        return None;
    }
    // Sheet, PivotTable and field names come from the file: leave them out of the matching.
    let w = &without_quoted(w);
    const HARMLESS: &[&str] = &[
        // read.rs:97, :142, python_part.rs:77, drawing.rs:166: a relationship to a part that is not there.
        "missing part ",
        "missing picture ",
        // drawing.rs:178, :193, :196: a chart frame whose chart part is not there.
        "without its chart part was skipped",
        "is missing from the package and was skipped",
        // read.rs:327: a <sheet> without a part.
        "\" has no part; skipped",
        // read.rs:461: a VBA relationship to a part that is not there.
        "macros (VBA project) reference a missing part",
        // read.rs:459 (the VBA project is written back to .xlsm; see `from_package` for .xlsx).
        "macros (VBA project) were kept but not executed",
        // sheet_read.rs:206: cells Excel itself cannot hold.
        "outside the 1,048,576 × 16,384 grid were dropped",
        // read.rs:137: phonetic runs with offsets outside their text or no reading.
        "phonetic runs were dropped (bad sb/eb or no reading)",
        // passthrough/read.rs:254, :355, :399: kept on save.
        "links to external workbooks were kept",
        "they are kept for saving",
        // pivot/read.rs:88: the PivotTable is read; its source names another workbook.
        "its source reference may not resolve",
        // sheet_read.rs:689: a shared formula without its master cell (Excel cannot read it either).
        "refers to a missing master cell",
    ];
    if HARMLESS.iter().any(|h| w.contains(h)) {
        return None;
    }
    // Specific wording first, then the generic fallback.
    let loss = if w.starts_with("unmodelled content of a Strict Open XML workbook") {
        // passthrough/read.rs:190.
        Loss::Other
    } else if w.contains("hyperlink") || w.starts_with("links to external workbooks were dropped") || w.starts_with("connections part ") {
        // sheet_read/parts.rs:16, :37, :41; passthrough/read.rs:212, :256.
        Loss::Links
    } else if w.contains("conditional format") {
        // sheet_read/rules.rs:114, :120; sheet_read.rs:865.
        Loss::ConditionalFormats
    } else if w.contains("PivotTable") || w.contains("pivot cache") {
        // read.rs:356; pivot/read.rs.
        Loss::PivotTables
    } else if w.contains("macros (VBA project)") {
        // read.rs:463.
        Loss::Macros
    } else if w.contains("Python")
        || w.contains("what-if data tables")
        || w.contains("shared formula")
        || w.contains("their formulas were not retained")
    {
        // python_part.rs:62-109; sheet_read.rs:606, :698, :706, :736.
        Loss::Formulas
    } else if w.contains("AutoFilter") {
        // sheet_read/parts.rs:87.
        Loss::Filters
    } else if w.starts_with("some date cells could not be read") || w.starts_with("some numeric cells could not be read") {
        // sheet_read.rs:578, :587: the cell opens empty.
        Loss::CellValues
    } else if w.contains("cell picture")
        || w.contains("cell-picture")
        || w.contains("rich value")
        || w.contains("XLRICHVALUE")
        || w.contains("picture")
        || w.contains("chart")
        || w.contains("embedded object")
        || w.contains("oleObjects")
        || w.contains("grouped shapes")
        || w.contains("controls")
    {
        // richdata.rs; read.rs:279; sheet_read.rs:600; drawing.rs:161, :190, :198, :207;
        // passthrough/read.rs:333, :357, :375, :395.
        Loss::Drawings
    } else if ["dropped", "skipped", "not kept", "not supported", "unsupported", "not loaded", "not retained", "were not read"]
        .iter()
        .any(|k| w.contains(k))
    {
        // read.rs:79 (phonetic readings), passthrough/read.rs:190, :205, :212, :296, :304, :342…
        Loss::Other
    } else {
        return None;
    };
    Some(loss)
}

/// `w` with the text between its first and last double quote taken out.
fn without_quoted(w: &str) -> String {
    match (w.find('"'), w.rfind('"')) {
        (Some(a), Some(b)) if a < b => format!("{}{}", &w[..=a], &w[b..]),
        _ => w.to_string(),
    }
}

/// Relationship kinds (the last segment of the type, lower case) whose parts GridCraft reads and
/// writes back, keeps through pass-through, or regenerates without loss to the person.
const KEPT_KINDS: &[&str] = &[
    // Package and workbook parts the reader models (read.rs).
    "officedocument",
    "core-properties",
    "extended-properties",
    "worksheet",
    "theme",
    "styles",
    "sharedstrings",
    "person",
    "sheetmetadata",
    "featurepropertybag",
    "python",
    // Rich values (pictures in cells): read by richdata.rs, which warns about what it cannot keep.
    "rdrichvalue",
    "rdrichvaluestructure",
    "rdrichvaluetypes",
    "rdrichvaluewebimage",
    "rdarray",
    "rdsupportingpropertybag",
    "rdsupportingpropertybagstructure",
    "richvaluerel",
    // Sheet content the reader models (sheet_read.rs, drawing.rs, pivot/read.rs, controls.rs).
    "drawing",
    "chart",
    "chartex",
    "image",
    "hyperlink",
    "comments",
    "vmldrawing",
    "threadedcomment",
    "table",
    "pivottable",
    "pivotcachedefinition",
    "pivotcacherecords",
    // Kept by pass-through (passthrough.rs: PACKAGE_ROOTS, WORKBOOK_ROOTS, SHEET_ROOTS,
    // SHEET_ELEMENT_KINDS, chart sheets, external links), which warns when it cannot keep them.
    "custom-properties",
    "extensibility",
    "webextensiontaskpanes",
    "webextension",
    "customxml",
    "customxmlprops",
    "connections",
    "xmlmaps",
    "querytable",
    "tablesinglecells",
    "oleobject",
    "ctrlprop",
    "control",
    "activexcontrolbinary",
    "package",
    "printersettings",
    "chartsheet",
    "externallink",
    "externallinkpath",
    "xlpathmissing",
    "xlstartup",
    "xlalternatestartup",
    "xllibrary",
    // Read as a sheet the engine leaves out: reported by `unread_sheets` (read.rs:337).
    "dialogsheet",
    "xlmacrosheet",
    "xlintlmacrosheet",
    // Regenerated or not the person's content: the calculation chain and volatile-function cache
    // (rebuilt by Excel), the thumbnail, chart style gallery parts and chart theme overrides
    // (chart formatting the engine does not model is a known writer limitation), digital
    // signatures (Excel drops them too when an edited workbook is saved).
    "calcchain",
    "volatiledependencies",
    "thumbnail",
    "chartstyle",
    "chartcolorstyle",
    "themeoverride",
    "origin",
    "signature",
];

/// What the relationship kind `kind` (lower case) from the relationships part `rels` loses, if
/// anything. `macro_free`: the file's type cannot hold macros (`.xlsx`, `.xltx`).
fn rel_loss(kind: &str, rels: &str, macro_free: bool) -> Option<Loss> {
    match kind {
        // Ink and other content parts in a drawing (drawing.rs:204 reads only pictures,
        // charts and shapes); the workbook's own custom XML is passed through.
        "customxml" if rels.starts_with("xl/drawings/") => Some(Loss::Drawings),
        // write.rs:422: a macro-free package can't hold a VBA project, so it is not written.
        "vbaproject" => macro_free.then_some(Loss::Macros),
        "vbaprojectsignature" | "vbaprojectsignatureagile" | "vbaprojectsignaturev3" => macro_free.then_some(Loss::Macros),
        // passthrough.rs:146-150: never kept, and no reader models them.
        "slicer" | "slicercache" | "timeline" | "timelinecache" => Some(Loss::Slicers),
        // passthrough.rs:124: shapes drawn over a chart.
        "chartusershapes" | "hdphoto" => Some(Loss::Drawings),
        k if KEPT_KINDS.contains(&k) => None,
        // Power Pivot data model, sheet custom properties, shared-workbook revisions, named sheet
        // views, sensitivity labels and anything else unknown.
        _ => Some(Loss::Other),
    }
}

/// Worksheet elements (local names, children of `<worksheet>`) the reader models or that
/// pass-through keeps (sheet_read.rs:216-341, passthrough.rs `SHEET_SLOTS`), or that hold no
/// content of the person's (the last sort, ignored error marks, calculation flags, phonetic
/// display settings).
const SHEET_KEPT: &[&str] = &[
    "sheetPr",
    "dimension",
    "sheetViews",
    "sheetFormatPr",
    "cols",
    "sheetData",
    "sheetCalcPr",
    "sheetProtection",
    "protectedRanges",
    "autoFilter",
    "sortState",
    "mergeCells",
    "phoneticPr",
    "conditionalFormatting",
    "dataValidations",
    "hyperlinks",
    "printOptions",
    "pageMargins",
    "pageSetup",
    "headerFooter",
    "rowBreaks",
    "colBreaks",
    "ignoredErrors",
    "drawing",
    "legacyDrawing",
    "legacyDrawingHF",
    "picture",
    "oleObjects",
    "controls",
    "tableParts",
    "extLst",
    "AlternateContent",
];

/// Worksheet extension elements (children of `<ext>`) the reader models (sheet_read.rs:823-870)
/// or that hold no content (ignored error marks).
const SHEET_EXT_KEPT: &[&str] = &["conditionalFormattings", "dataValidations", "sparklineGroups", "ignoredErrors"];

/// Workbook elements (children of `<workbook>`) the reader models or pass-through keeps
/// (read.rs:204-240, :283-434, passthrough/read.rs:236-283), or that hold no content of the
/// person's (function groups of old macro sheets, the embedded-object size, smart tag and web
/// publishing settings, recovery settings, the absolute path and revision pointer Excel writes).
const WORKBOOK_KEPT: &[&str] = &[
    "fileVersion",
    "workbookPr",
    "workbookProtection",
    "bookViews",
    "sheets",
    "functionGroups",
    "externalReferences",
    "definedNames",
    "calcPr",
    "oleSize",
    "pivotCaches",
    "smartTagPr",
    "smartTagTypes",
    "webPublishing",
    "fileRecoveryPr",
    "extLst",
    "AlternateContent",
    "revisionPtr",
];

/// Workbook extension elements kept (passthrough/read.rs:271: calculation features) or that hold
/// no content (the Excel 2010/2013 workbook properties).
const WORKBOOK_EXT_KEPT: &[&str] = &["calcFeatures", "workbookPr"];

/// The losses the package `bytes` holds that GridCraft's reader skips without a warning, for an
/// XLSX-family file named `name`; empty for anything else.
pub fn from_package(name: &str, bytes: &[u8]) -> Vec<Loss> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "xlsx" | "xlsm" | "xltx" | "xltm") || !bytes.starts_with(b"PK") {
        return Vec::new();
    }
    let macro_free = matches!(ext.as_str(), "xlsx" | "xltx");
    let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(bytes)) else { return Vec::new() };
    // Part names compare without case (OPC); the zip index of each.
    let entries: Vec<(String, usize)> =
        (0..zip.len()).filter_map(|i| zip.by_index_raw(i).ok().map(|f| (f.name().trim_start_matches('/').to_ascii_lowercase(), i))).collect();
    let index_of = |part: &str| entries.iter().find(|(n, _)| *n == part.to_ascii_lowercase()).map(|&(_, i)| i);
    let mut out = Vec::new();
    let mut workbook = String::from("xl/workbook.xml");
    let mut sheets: Vec<String> = Vec::new();
    for (rels_name, i) in entries.iter().filter(|(n, _)| n.ends_with(".rels")) {
        let xml = read_entry(&mut zip, *i, MAX_META);
        // `xl/worksheets/_rels/sheet1.xml.rels` is the relationships of `xl/worksheets/sheet1.xml`.
        let source = rels_name.strip_suffix(".rels").map(|s| s.replacen("_rels/", "", 1)).unwrap_or_default();
        let dir = source.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for (kind, target, external) in relationships(&xml) {
            out.extend(rel_loss(&kind, rels_name, macro_free));
            if external {
                continue;
            }
            let part = resolve(dir, &target);
            match kind.as_str() {
                "officedocument" if rels_name == "_rels/.rels" => workbook = part,
                "worksheet" => sheets.push(part),
                _ => {}
            }
        }
    }
    if let Some(i) = index_of(&workbook) {
        out.extend(workbook_losses(&read_entry(&mut zip, i, MAX_META)));
    }
    sheets.sort();
    sheets.dedup();
    for sheet in &sheets {
        if let Some(i) = index_of(sheet)
            && let Ok(f) = zip.by_index(i)
        {
            let (xml, tail) = sheet_tail(f);
            out.extend(sheet_losses(&xml, tail));
        }
    }
    // Drawings: ink and other content parts (drawing.rs:204 skips them without a warning).
    let drawings: Vec<usize> =
        entries.iter().filter(|(n, _)| n.starts_with("xl/drawings/") && n.ends_with(".xml") && !n.contains("/_rels/")).map(|&(_, i)| i).collect();
    for i in drawings {
        let mut ink = false;
        visit(&read_entry(&mut zip, i, MAX_META), &mut Vec::new(), &mut |_, name| ink |= name == "contentPart");
        if ink {
            out.push(Loss::Drawings);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Largest relationships, workbook or drawing part read here (passthrough.rs `MAX_META`).
const MAX_META: u64 = 16 * 1024 * 1024;

fn read_entry(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, i: usize, limit: u64) -> Vec<u8> {
    let mut out = Vec::new();
    if let Ok(f) = zip.by_index(i) {
        let _ = f.take(limit).read_to_end(&mut out);
    }
    out
}

/// `target` (a relationship target) resolved against the folder `dir` of its source part, lower
/// case, without a leading slash.
fn resolve(dir: &str, target: &str) -> String {
    let target = target.to_ascii_lowercase();
    let (mut parts, rest): (Vec<&str>, &str) = match target.strip_prefix('/') {
        Some(abs) => (Vec::new(), abs),
        None => (dir.split('/').filter(|s| !s.is_empty()).collect(), target.as_str()),
    };
    for seg in rest.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

/// The relationships of a `.rels` part: kind (last segment of the type, lower case), target and
/// whether it is external.
fn relationships(xml: &[u8]) -> Vec<(String, String, bool)> {
    use quick_xml::events::Event;
    let mut r = quick_xml::Reader::from_reader(xml);
    let mut out = Vec::new();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match r.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) if local(e.name().as_ref()) == b"Relationship" => {
                let (mut kind, mut target, mut external) = (String::new(), String::new(), false);
                for a in e.attributes().flatten() {
                    let v = String::from_utf8_lossy(&a.value).into_owned();
                    match a.key.as_ref() {
                        b"Type" => kind = v.rsplit('/').next().unwrap_or("").to_ascii_lowercase(),
                        b"Target" => target = v,
                        b"TargetMode" => external = v.eq_ignore_ascii_case("External"),
                        _ => {}
                    }
                }
                out.push((kind, target, external));
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out
}

/// The local part of an XML name (`x14:slicerList` → `slicerList`).
fn local(name: &[u8]) -> &[u8] {
    name.iter().rposition(|&b| b == b':').map_or(name, |i| &name[i + 1..])
}

/// Calls `f(ancestors, local name)` for every element of `xml`, `ancestors` being the local names
/// of the open elements around it (starting with `stack`). Stops at the first XML error.
fn visit(xml: &[u8], stack: &mut Vec<String>, f: &mut dyn FnMut(&[String], &str)) {
    use quick_xml::events::Event;
    let mut r = quick_xml::Reader::from_reader(xml);
    {
        let c = r.config_mut();
        c.check_end_names = false;
        c.allow_unmatched_ends = true;
    }
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match r.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(local(e.name().as_ref())).into_owned();
                f(stack, &name);
                stack.push(name);
            }
            Ok(Event::Empty(e)) => f(stack, &String::from_utf8_lossy(local(e.name().as_ref()))),
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

fn workbook_losses(xml: &[u8]) -> Vec<Loss> {
    let mut out = Vec::new();
    visit(xml, &mut Vec::new(), &mut |up, name| {
        let loss = match up {
            [_workbook] if !WORKBOOK_KEPT.contains(&name) => Some(Loss::Other),
            [_, ext_list, ext] if ext_list == "extLst" && ext == "ext" => match name {
                // x14/x15 slicer and timeline caches.
                "slicerCaches" | "timelineCacheRefs" => Some(Loss::Slicers),
                // The PivotTable caches only slicers (x14) or data-model PivotTables (x15) use.
                "pivotCaches" | "pivotTableReferences" => Some(Loss::PivotTables),
                n if WORKBOOK_EXT_KEPT.contains(&n) => None,
                // The data model (Power Pivot) and anything else.
                _ => Some(Loss::Other),
            },
            _ => None,
        };
        out.extend(loss);
    });
    out
}

/// `xml`: a worksheet's elements after its cells (`tail`), or the whole worksheet.
fn sheet_losses(xml: &[u8], tail: bool) -> Vec<Loss> {
    let mut out = Vec::new();
    let mut stack = if tail { vec!["worksheet".to_string()] } else { Vec::new() };
    visit(xml, &mut stack, &mut |up, name| {
        let loss = match up {
            [_worksheet] => match name {
                n if SHEET_KEPT.contains(&n) => None,
                // A header or footer DrawingML drawing (the reader keeps only the VML one).
                "drawingHF" => Some(Loss::Drawings),
                // Scenarios, consolidation, custom views, custom properties, watched cells, smart
                // tags, web publishing items, anything else.
                _ => Some(Loss::Other),
            },
            [_, ext_list, ext] if ext_list == "extLst" && ext == "ext" => match name {
                n if SHEET_EXT_KEPT.contains(&n) => None,
                "slicerList" | "timelineRefs" => Some(Loss::Slicers),
                _ => Some(Loss::Other),
            },
            // sheet_read/parts.rs:67-71 reads only the `<filter>` values of a `<filters>` list: a
            // date grouping ("2024", "March") is dropped without a warning.
            [.., filters] if filters == "filters" && name == "dateGroupItem" => Some(Loss::Filters),
            _ => None,
        };
        out.extend(loss);
    });
    out
}

/// The part of a worksheet after its cells (everything after `</sheetData>`), read as a stream so
/// the cells are never held. Every element the reader may skip comes after the cells in the
/// schema order. A sheet without `<sheetData>` yields its first 16 MB, and `false`.
fn sheet_tail(mut f: impl Read) -> (Vec<u8>, bool) {
    const CHUNK: usize = 1 << 20;
    const NEEDLE: &[u8] = b"sheetData";
    let mut buf: Vec<u8> = Vec::new();
    let mut head: Vec<u8> = Vec::new();
    let mut found = false;
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let n = match f.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        if !found && head.len() < MAX_META as usize {
            head.extend_from_slice(&chunk[..n]);
        }
        buf.extend_from_slice(&chunk[..n]);
        // The last end of a `sheetData` tag (`</sheetData>`, `</x:sheetData>`, `<sheetData/>`)
        // whose closing `>` has arrived. `<` is always escaped in text and attribute values, so a
        // name preceded by `<` or `</` (and maybe a prefix) is markup.
        let mut cut = None;
        let mut from = 0;
        while let Some(p) = find(&buf[from..], NEEDLE).map(|p| p + from) {
            from = p + NEEDLE.len();
            if let Some(open) = markup(&buf[..p])
                && let Some(gt) = buf[from..].iter().position(|&b| b == b'>')
            {
                // An end tag, or an empty element; not the start tag of the cells.
                let closing = !open || buf[from..from + gt].ends_with(b"/");
                if closing {
                    cut = Some(from + gt + 1);
                }
            }
        }
        match cut {
            Some(c) => {
                found = true;
                head = Vec::new();
                buf.drain(..c);
            }
            // Nothing before the last few bytes can still start a marker.
            None if !found && buf.len() > 4096 => {
                let keep = buf.len() - 4096;
                buf.drain(..keep);
            }
            None => {}
        }
        if found && buf.len() > MAX_META as usize {
            break;
        }
    }
    if found { (buf, true) } else { (head, false) }
}

/// Whether the bytes before a name end with `<` (`Some(true)`, a start tag) or `</`
/// (`Some(false)`, an end tag), maybe followed by a prefix and `:`.
fn markup(before: &[u8]) -> Option<bool> {
    let mut end = before.len();
    if end > 0 && before[end - 1] == b':' {
        end -= 1;
        while end > 0 && (before[end - 1].is_ascii_alphanumeric() || matches!(before[end - 1], b'_' | b'-' | b'.')) {
            end -= 1;
        }
    }
    let rest = &before[..end];
    if rest.ends_with(b"</") {
        Some(false)
    } else if rest.ends_with(b"<") {
        Some(true)
    } else {
        None
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that hands out one byte at a time, so every marker spans reads.
    struct Trickle<'a>(&'a [u8]);

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let Some((&b, rest)) = self.0.split_first() else { return Ok(0) };
            buf[0] = b;
            self.0 = rest;
            Ok(1)
        }
    }

    #[test]
    fn finds_what_follows_the_cells() {
        for (xml, tail) in [
            (
                "<worksheet><sheetData><row r=\"1\"><c><is><t>sheetData&gt;</t></is></c></row></sheetData><scenarios/></worksheet>",
                "<scenarios/></worksheet>",
            ),
            ("<x:worksheet><x:sheetData><x:row/></x:sheetData ><x:hyperlinks/></x:worksheet>", "<x:hyperlinks/></x:worksheet>"),
            ("<worksheet><sheetData/><cellWatches/></worksheet>", "<cellWatches/></worksheet>"),
        ] {
            assert_eq!(sheet_tail(Trickle(xml.as_bytes())), (tail.as_bytes().to_vec(), true), "{xml}");
            assert_eq!(sheet_tail(xml.as_bytes()), (tail.as_bytes().to_vec(), true), "{xml}");
        }
        assert_eq!(sheet_tail(&b"<worksheet><cols/></worksheet>"[..]), (b"<worksheet><cols/></worksheet>".to_vec(), false));
        assert_eq!(sheet_losses(b"<scenarios/></worksheet>", true), [Loss::Other]);
        assert_eq!(sheet_losses(b"<worksheet><cols/><scenarios/></worksheet>", false), [Loss::Other]);
        assert!(sheet_losses(b"<worksheet><cols/><pageMargins/></worksheet>", false).is_empty());
    }

    #[test]
    fn resolves_relationship_targets() {
        assert_eq!(resolve("xl", "worksheets/Sheet1.xml"), "xl/worksheets/sheet1.xml");
        assert_eq!(resolve("xl/worksheets", "../drawings/drawing1.xml"), "xl/drawings/drawing1.xml");
        assert_eq!(resolve("xl/worksheets", "/xl/slicers/slicer1.xml"), "xl/slicers/slicer1.xml");
        assert_eq!(resolve("", "xl/workbook.xml"), "xl/workbook.xml");
    }
}
