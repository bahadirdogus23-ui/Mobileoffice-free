//! UYAP's UDF documents (Turkish court and e-Devlet files) read into a WordCraft document.
//!
//! A `.udf` file is a ZIP holding `content.xml`:
//!
//! ```xml
//! <template format_id="1.8">
//!   <content><![CDATA[the whole text, paragraphs ending in \n]]></content>
//!   <properties><pageFormat mediaSizeName="1" leftMargin="70.9" … paperOrientation="1"/></properties>
//!   <elements resolver="hvl-default">
//!     <paragraph Alignment="1" LeftIndent="0" FirstLineIndent="0" size="12" family="Times New Roman">
//!       <content startOffset="0" length="9" bold="true"/> <space …/> <tab …/> <image imageData="…"/>
//!     </paragraph>
//!     <table columnCount="2" columnSpans="200,300" border="borderCell">
//!       <row><cell><paragraph>…</paragraph></cell></row>
//!     </table>
//!   </elements>
//!   <styles><style name="hvl-default" family="Times New Roman" size="12"/></styles>
//! </template>
//! ```
//!
//! Elements point into the CDATA text by `startOffset`/`length` in UTF-16 units (UYAP's editor
//! is Java). The reader turns the elements into HTML with inline styles and hands that to
//! WordCraft's HTML importer on the UDF's own page setup, so layout, search, reading aloud and
//! PDF/DOCX export all come from the engine. A `sign.sgn` entry means the document carries an
//! e-signature; it is reported, not verified.

use std::fmt::Write as _;
use std::io::Read as _;

use quick_xml::events::{BytesStart, Event};
use wordcraft_doc::props::LineSpacing;
use wordcraft_doc::{Block, Document, SectionProps};

/// What a UDF file holds beyond its text.
pub struct Udf {
    pub doc: Document,
    /// The file has an e-signature (`sign.sgn`); it is not checked.
    pub signed: bool,
}

/// One XML element with its attributes and children; text is not kept (it lives in the CDATA).
#[derive(Default)]
struct El {
    name: String,
    attrs: Vec<(String, String)>,
    kids: Vec<El>,
}

impl El {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }
    fn num(&self, key: &str) -> Option<f32> {
        self.attr(key).and_then(|v| v.trim().parse::<f32>().ok()).filter(|v| v.is_finite())
    }
    fn flag(&self, key: &str) -> bool {
        self.attr(key).is_some_and(|v| v.eq_ignore_ascii_case("true"))
    }
    fn kid(&self, name: &str) -> Option<&El> {
        self.kids.iter().find(|k| k.name == name)
    }
}

pub fn is_udf(bytes: &[u8]) -> bool {
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).is_ok_and(|z| z.index_for_name("content.xml").is_some())
}

/// Reads a UDF file.
pub fn read(bytes: &[u8]) -> Result<Udf, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("UDF: {e}"))?;
    let signed = zip.index_for_name("sign.sgn").is_some();
    let mut xml = String::new();
    zip.by_name("content.xml")
        .map_err(|_| "UDF: content.xml is missing".to_string())?
        .take(64 << 20)
        .read_to_string(&mut xml)
        .map_err(|e| format!("UDF: {e}"))?;
    let (root, text) = parse(&xml)?;
    let (html, page) = to_html(&root, &text);
    let mut doc = wordcraft_formats::import_on("html", html.as_bytes(), &page)
        .unwrap_or_else(|| Err("no HTML importer".into()))
        .map_err(|e| format!("UDF: {e}"))?;
    // UYAP's editor sets text single-spaced with no space between paragraphs; without this the
    // engine's Word defaults (space after, 1.08 lines) push documents onto an extra page.
    if let Some(normal) = doc.styles.get_mut("Normal") {
        normal.para.space_before = Some(0.0);
        normal.para.space_after = Some(0.0);
        normal.para.line_spacing = Some(LineSpacing::Multiple(1.0));
    }
    // The HTML importer splits tables evenly; give the body's tables UYAP's column proportions.
    let spans = root.kid("elements").map(|e| e.kids.iter().filter(|k| k.name == "table").map(column_spans).collect::<Vec<_>>());
    let tables = doc.body.iter_mut().filter_map(|b| match std::sync::Arc::make_mut(b) {
        Block::Table(t) => Some(t),
        Block::Para(_) => None,
    });
    for (t, spans) in tables.zip(spans.unwrap_or_default()) {
        let total: f32 = t.grid.iter().sum();
        let sum: f32 = spans.iter().sum();
        if spans.len() == t.grid.len() && total > 0.0 && sum > 0.0 {
            t.grid = spans.iter().map(|s| s / sum * total).collect();
            for row in &mut t.rows {
                for (cell, w) in row.cells.iter_mut().zip(&t.grid) {
                    cell.props.width = Some(*w);
                    cell.props.width_pct = None;
                }
            }
        }
    }
    Ok(Udf { doc, signed })
}

/// The element tree under `<template>` and the CDATA text as UTF-16 units.
fn parse(xml: &str) -> Result<(El, Vec<u16>), String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut stack: Vec<El> = vec![El::default()];
    let mut text = String::new();
    let mut in_content_text = false;
    loop {
        match reader.read_event().map_err(|e| format!("UDF: {e}"))? {
            Event::Start(e) => {
                let el = element(&e);
                // The first <content> directly under <template> holds the text; the ones inside
                // paragraphs are empty runs.
                in_content_text = el.name == "content" && stack.len() == 2 && stack.last().is_some_and(|p| p.name == "template");
                stack.push(el);
            }
            Event::Empty(e) => {
                let el = element(&e);
                if let Some(parent) = stack.last_mut() {
                    parent.kids.push(el);
                }
            }
            Event::End(_) => {
                in_content_text = false;
                if stack.len() > 1 {
                    let done = stack.pop().unwrap_or_default();
                    if let Some(parent) = stack.last_mut() {
                        parent.kids.push(done);
                    }
                }
            }
            Event::CData(c) if in_content_text => text.push_str(&String::from_utf8_lossy(&c.into_inner())),
            Event::Text(t) if in_content_text => text.push_str(&t.decode().map_err(|e| format!("UDF: {e}"))?),
            Event::Eof => break,
            _ => {}
        }
    }
    let root = stack.into_iter().next().unwrap_or_default();
    let template = root.kids.into_iter().find(|k| k.name == "template").ok_or("UDF: no <template>")?;
    Ok((template, text.encode_utf16().collect()))
}

fn element(e: &BytesStart<'_>) -> El {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let attrs = e
        .attributes()
        .flatten()
        .map(|a| {
            (
                String::from_utf8_lossy(a.key.as_ref()).into_owned(),
                a.normalized_value(quick_xml::XmlVersion::Implicit1_0).map(|v| v.into_owned()).unwrap_or_default(),
            )
        })
        .collect();
    El { name, attrs, kids: Vec::new() }
}

/// The page from `<pageFormat>` (points; A4 unless it says otherwise) and the body as HTML.
fn to_html(t: &El, text: &[u16]) -> (String, SectionProps) {
    let mut page = SectionProps { page_w: 595.3, page_h: 841.9, ..SectionProps::default() };
    if let Some(pf) = t.kid("properties").and_then(|p| p.kid("pageFormat")) {
        for (key, slot) in [
            ("leftMargin", &mut page.margin_left),
            ("rightMargin", &mut page.margin_right),
            ("topMargin", &mut page.margin_top),
            ("bottomMargin", &mut page.margin_bottom),
        ] {
            if let Some(v) = pf.num(key).filter(|v| (0.0..300.0).contains(v)) {
                *slot = v;
            }
        }
        // paperOrientation 1 is portrait, 2 landscape (UYAP editor's own numbering).
        if pf.attr("paperOrientation") == Some("2") {
            page.landscape = true;
            std::mem::swap(&mut page.page_w, &mut page.page_h);
        }
    }
    // The document's base font, from the style its elements resolve to.
    let resolver = t.kid("elements").and_then(|e| e.attr("resolver")).unwrap_or("hvl-default");
    let base = t.kid("styles").and_then(|s| s.kids.iter().find(|st| st.attr("name") == Some(resolver)));
    let family = base.and_then(|b| b.attr("family")).unwrap_or("Times New Roman");
    let size = base.and_then(|b| b.num("size")).unwrap_or(12.0);
    let mut html = format!("<html><body style=\"font-family:'{}';font-size:{size}pt\">", esc(family));
    if let Some(elements) = t.kid("elements") {
        blocks(&elements.kids, text, &mut html);
    }
    html.push_str("</body></html>");
    (html, page)
}

fn blocks(els: &[El], text: &[u16], out: &mut String) {
    for el in els {
        match el.name.as_str() {
            "paragraph" => paragraph(el, text, out),
            "table" => table(el, text, out),
            "page-break" | "pageBreak" => out.push_str("<p style=\"page-break-before:always\"></p>"),
            // Headers and footers of the page are not part of the body text.
            "header" | "footer" => {}
            _ => blocks(&el.kids, text, out),
        }
    }
}

fn paragraph(p: &El, text: &[u16], out: &mut String) {
    let mut style = String::new();
    match p.attr("Alignment") {
        Some("1") => style.push_str("text-align:center;"),
        Some("2") => style.push_str("text-align:right;"),
        Some("3") => style.push_str("text-align:justify;"),
        _ => {}
    }
    for (attr, css) in [
        ("LeftIndent", "margin-left"),
        ("RightIndent", "margin-right"),
        ("FirstLineIndent", "text-indent"),
        ("SpaceAbove", "margin-top"),
        ("SpaceBelow", "margin-bottom"),
    ] {
        if let Some(v) = p.num(attr).filter(|v| *v != 0.0) {
            let _ = write!(style, "{css}:{v}pt;");
        }
    }
    if let Some(ls) = p.num("LineSpacing").filter(|v| *v > 0.0) {
        // UYAP stores extra spacing as a fraction (0.5 = one and a half lines).
        let _ = write!(style, "line-height:{};", 1.0 + ls);
    }
    if let Some(f) = p.attr("family") {
        let _ = write!(style, "font-family:'{}';", esc(f));
    }
    if let Some(s) = p.num("size") {
        let _ = write!(style, "font-size:{s}pt;");
    }
    // pre-wrap keeps tabs and runs of spaces, which UYAP documents align their labels with.
    let _ = write!(out, "<p style=\"margin-top:0;margin-bottom:0;white-space:pre-wrap;{style}\">");
    let mut any = false;
    for run in &p.kids {
        any |= self::run(run, text, out);
    }
    if !any {
        out.push_str("&nbsp;");
    }
    out.push_str("</p>");
}

/// One piece of a paragraph; returns whether it put anything visible in.
fn run(r: &El, text: &[u16], out: &mut String) -> bool {
    if r.name == "image" {
        if let Some(data) = r.attr("imageData").filter(|d| !d.is_empty()) {
            let size = match (r.num("width"), r.num("height")) {
                (Some(w), Some(h)) => format!(" width=\"{w}\" height=\"{h}\""),
                _ => String::new(),
            };
            let _ = write!(out, "<img src=\"data:image/png;base64,{}\"{size}>", esc(data.trim()));
            return true;
        }
        return false;
    }
    let (Some(start), Some(len)) = (r.num("startOffset"), r.num("length")) else { return false };
    let (start, len) = (start.max(0.0) as usize, len.max(0.0) as usize);
    let end = start.saturating_add(len).min(text.len());
    if start >= end {
        return false;
    }
    let piece = String::from_utf16_lossy(&text[start..end]);
    // The paragraph's own line end belongs to the paragraph, not to its text.
    let piece = piece.trim_end_matches(['\n', '\r']);
    if piece.is_empty() {
        return false;
    }
    let mut style = String::new();
    if r.flag("bold") {
        style.push_str("font-weight:bold;");
    }
    if r.flag("italic") {
        style.push_str("font-style:italic;");
    }
    if r.flag("underline") {
        style.push_str("text-decoration:underline;");
    }
    if r.flag("strikethrough") {
        style.push_str("text-decoration:line-through;");
    }
    if let Some(f) = r.attr("family") {
        let _ = write!(style, "font-family:'{}';", esc(f));
    }
    if let Some(s) = r.num("size") {
        let _ = write!(style, "font-size:{s}pt;");
    }
    if let Some(c) = r.attr("foreground").and_then(|c| c.trim().parse::<i64>().ok()) {
        // A Java ARGB int: keep the RGB part; black is the default anyway.
        let rgb = (c as u32) & 0x00FF_FFFF;
        if rgb != 0 {
            let _ = write!(style, "color:#{rgb:06x};");
        }
    }
    let body = esc(piece).replace('\n', "<br>");
    if style.is_empty() {
        out.push_str(&body);
    } else {
        let _ = write!(out, "<span style=\"{style}\">{body}</span>");
    }
    true
}

/// A table's `columnSpans` ("100,300"): relative column widths.
fn column_spans(t: &El) -> Vec<f32> {
    t.attr("columnSpans").map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect()).unwrap_or_default()
}

fn table(t: &El, text: &[u16], out: &mut String) {
    let bordered = t.attr("border").is_none_or(|b| !b.eq_ignore_ascii_case("borderNone"));
    let spans = column_spans(t);
    let total: f32 = spans.iter().sum();
    let _ = write!(out, "<table{} style=\"border-collapse:collapse;width:100%\">", if bordered { " border=\"1\"" } else { "" });
    for row in t.kids.iter().filter(|k| k.name == "row") {
        out.push_str("<tr>");
        for (i, cell) in row.kids.iter().filter(|k| k.name == "cell").enumerate() {
            let width = spans.get(i).filter(|_| total > 0.0).map(|w| format!(" width=\"{:.0}%\"", w * 100.0 / total)).unwrap_or_default();
            let span = cell.num("colspan").filter(|c| *c > 1.0).map(|c| format!(" colspan=\"{c:.0}\"")).unwrap_or_default();
            let _ = write!(out, "<td{width}{span}>");
            blocks(&cell.kids, text, out);
            out.push_str("</td>");
        }
        out.push_str("</tr>");
    }
    out.push_str("</table>");
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}
