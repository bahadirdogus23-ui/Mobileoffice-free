//! What opening a document left out, for the formats the app saves over the opened file (DOCX,
//! DOCM, ODT, RTF, HTML, Markdown).
//!
//! WordCraft opens these leniently: a damaged part of a DOCX package is skipped (styles, notes,
//! comments, a header…), an ODT's notes, comments, text boxes, shapes and form controls are
//! not read at all, an RTF's notes, comments, objects and shapes are skipped, and a web page's
//! scripts, style sheets and pictures from other files are dropped. Each still opens, but saving
//! it over the original file would delete what was left out (AGENTS.md invariant 4); the app then
//! offers only "Save as". Each check mirrors the engine code it names (pinned commit c4fafc2).

use mo_convert::package::{XmlLimits, damaged_entries};
use std::io::{Cursor, Read};

/// Something the file holds that the opened document does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Loss {
    /// Part of the file is damaged or past the engine's limits, and was skipped or read in part.
    Damaged,
    /// The file holds another format than its name says: saving would replace it with that one.
    OtherFormat,
    /// Footnotes or endnotes.
    Notes,
    /// Comments.
    Comments,
    /// Text boxes and drawing shapes.
    Shapes,
    /// Embedded objects (charts, formulas, OLE objects).
    Objects,
    /// Pictures the engine cannot keep (Windows metafiles in RTF, pictures linked from other files).
    Pictures,
    /// Form fields and controls.
    Forms,
    /// A web page's scripts, style sheets, frames, media and pictures from other files.
    WebContent,
    /// Anything else left out (index entries, field declarations, nesting past the limits…).
    Other,
}

/// What opening `bytes` as `name` leaves out, for the formats saved over the opened file; empty
/// for other formats and for files read whole.
pub fn find(name: &str, bytes: &[u8]) -> Vec<Loss> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    let mut out = match ext.as_str() {
        // io.rs `signature_contradicts`: a Word name on anything but a zip is opened by content.
        "docx" | "docm" | "dotx" | "dotm" if !bytes.starts_with(b"PK") => vec![Loss::OtherFormat],
        "docx" | "docm" | "dotx" | "dotm" => docx(bytes),
        "odt" if !bytes.starts_with(b"PK") => vec![Loss::OtherFormat],
        "odt" => odt(bytes),
        "rtf" if !bytes.starts_with(b"{\\rtf") => vec![Loss::OtherFormat],
        "rtf" => rtf(bytes),
        "html" | "htm" => html(&wordcraft_formats::txt::decode(bytes)),
        "md" | "markdown" => markdown(&wordcraft_formats::txt::decode(bytes)),
        _ => Vec::new(),
    };
    out.sort();
    out.dedup();
    out
}

/// crates/docx/src/xml.rs:15-19 (`MAX_DEPTH`, `MAX_ELEMENTS`, `MAX_ATTRS`) and package.rs:12
/// (`MAX_PART`); the lenient reader does not check end names.
const DOCX_XML: XmlLimits =
    XmlLimits { max_elements: 8_000_000, max_depth: 256, max_attrs: Some(1024), max_bytes: 256 * 1024 * 1024, allow_unmatched_ends: false };

/// A DOCX package: crates/docx/src/package.rs:221-246 skips entries that do not inflate, and
/// read.rs:198-203 skips a styles, numbering, settings, notes or comments part that does not
/// parse (`lenient`); headers, footers and relationships are read the same way.
fn docx(bytes: &[u8]) -> Vec<Loss> {
    let parsed = |n: &str| n.ends_with(".rels") || (n.ends_with(".xml") && (n.starts_with("word/") || n.starts_with("docprops/")));
    if damaged_entries(bytes, parsed, &DOCX_XML).is_empty() { Vec::new() } else { vec![Loss::Damaged] }
}

/// crates/formats/src/odt/xml.rs:53-86: malformed XML ends the stream (what follows is not read).
const ODT_XML: XmlLimits = XmlLimits { max_elements: usize::MAX, max_depth: 512, max_attrs: None, max_bytes: 256 << 20, allow_unmatched_ends: true };

/// Most bytes of pictures an ODT's frames load (crates/formats/src/odt/limits.rs:2, import.rs:135).
const ODT_PICTURES: u64 = 256 << 20;

/// An ODT package. The importer (crates/formats/src/odt.rs:10-11, body.rs:516-531) reads only
/// paragraphs, lists, tables, pictures and tracked changes: notes, annotations, text boxes,
/// drawing shapes and forms are skipped, and nesting past its limits is dropped.
fn odt(bytes: &[u8]) -> Vec<Loss> {
    let parsed = |n: &str| matches!(n, "content.xml" | "styles.xml" | "meta.xml");
    let mut out = Vec::new();
    if !damaged_entries(bytes, parsed, &ODT_XML).is_empty() {
        out.push(Loss::Damaged);
    }
    let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(bytes)) else { return vec![Loss::Damaged] };
    let pictures: u64 = (0..zip.len()).filter_map(|i| zip.by_index_raw(i).ok().filter(|f| f.name().starts_with("Pictures/")).map(|f| f.size())).sum();
    if pictures > ODT_PICTURES {
        out.push(Loss::Pictures);
    }
    let mut content = Vec::new();
    if let Ok(f) = zip.by_name("content.xml") {
        let _ = f.take(ODT_XML.max_bytes).read_to_end(&mut content);
    }
    out.extend(odt_content(&content));
    out
}

/// What the ODT importer skips in `content.xml`. Prefixes are matched as written, as the
/// importer matches them (body.rs).
fn odt_content(xml: &[u8]) -> Vec<Loss> {
    use quick_xml::events::Event;
    let mut r = quick_xml::Reader::from_reader(xml);
    {
        let c = r.config_mut();
        c.check_end_names = false;
        c.allow_unmatched_ends = true;
    }
    let mut out = Vec::new();
    // Open tables and lists (body.rs:556 `model::MAX_DEPTH` = 8 tables, :534 64 lists).
    let (mut tables, mut lists, mut forms) = (0usize, 0usize, 0usize);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let (name, open) = match r.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => (String::from_utf8_lossy(e.name().as_ref()).into_owned(), true),
            Ok(Event::Empty(e)) => (String::from_utf8_lossy(e.name().as_ref()).into_owned(), false),
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"table:table" => tables = tables.saturating_sub(1),
                    b"text:list" => lists = lists.saturating_sub(1),
                    b"office:forms" => forms = forms.saturating_sub(1),
                    _ => {}
                }
                continue;
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => continue,
        };
        let loss = match name.as_str() {
            "text:note" => Some(Loss::Notes),
            "office:annotation" => Some(Loss::Comments),
            "draw:text-box"
            | "draw:rect"
            | "draw:line"
            | "draw:polyline"
            | "draw:polygon"
            | "draw:regular-polygon"
            | "draw:path"
            | "draw:circle"
            | "draw:ellipse"
            | "draw:connector"
            | "draw:caption"
            | "draw:measure"
            | "draw:custom-shape" => Some(Loss::Shapes),
            "draw:object" | "draw:object-ole" => Some(Loss::Objects),
            "draw:control" => Some(Loss::Forms),
            _ if forms > 0 => Some(Loss::Forms),
            "text:variable-decl" | "text:user-field-decl" | "text:format-change" => Some(Loss::Other),
            "table:table" if tables >= 8 => Some(Loss::Other),
            "text:list" if lists >= 64 => Some(Loss::Other),
            _ => None,
        };
        out.extend(loss);
        if open {
            match name.as_str() {
                "table:table" => tables += 1,
                "text:list" => lists += 1,
                "office:forms" => forms += 1,
                _ => {}
            }
        }
    }
    out
}

/// An RTF file. crates/formats/src/rtf/word.rs:286-292 skips these destinations, and a picture
/// in a format other than PNG or JPEG (text.rs:151) is dropped.
fn rtf(bytes: &[u8]) -> Vec<Loss> {
    let mut out = Vec::new();
    // Per open group: whether the engine skips it (a skipped destination or `\*` one).
    let mut skipped: Vec<bool> = vec![false];
    let mut group_start = false;
    let mut starred = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                let inherited = skipped.last().copied().unwrap_or(false);
                skipped.push(inherited);
                // word.rs/state.rs `MAX_GROUPS`: deeper groups are not read.
                if skipped.len() > 512 {
                    out.push(Loss::Other);
                }
                group_start = true;
                starred = false;
                i += 1;
            }
            b'}' => {
                if skipped.len() > 1 {
                    skipped.pop();
                }
                group_start = false;
                i += 1;
            }
            b'\\' => {
                i += 1;
                let Some(&c) = bytes.get(i) else { break };
                if !c.is_ascii_alphabetic() {
                    if c == b'*' {
                        starred = true;
                    } else if c == b'\'' {
                        i += 2;
                    }
                    i += 1;
                    continue;
                }
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = &bytes[start..i];
                let num_start = i;
                if i < bytes.len() && bytes[i] == b'-' {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let param: Option<usize> = std::str::from_utf8(&bytes[num_start..i]).ok().and_then(|s| s.parse().ok());
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
                let first = group_start;
                group_start = false;
                if word == b"bin" {
                    // Binary data follows: skip it unread.
                    i = i.saturating_add(param.unwrap_or(0));
                    continue;
                }
                let in_skipped = skipped.last().copied().unwrap_or(false);
                let loss = match word {
                    b"footnote" => Some(Loss::Notes),
                    b"annotation" | b"atnid" => Some(Loss::Comments),
                    b"object" => Some(Loss::Objects),
                    b"shpinst" | b"do" => Some(Loss::Shapes),
                    b"xe" | b"tc" | b"txe" | b"docvar" => Some(Loss::Other),
                    b"wmetafile" | b"emfblip" | b"pmmetafile" | b"macpict" | b"dibitmap" | b"wbitmap" if !in_skipped => Some(Loss::Pictures),
                    _ => None,
                };
                out.extend(loss);
                let skips = RTF_SKIPPED.contains(&word)
                    || (starred && first && !matches!(word, b"shppict" | b"fldinst" | b"bkmkstart" | b"pict") && !RTF_READ.contains(&word));
                if skips && let Some(s) = skipped.last_mut() {
                    *s = true;
                }
                starred = false;
            }
            // Line ends are not text in RTF.
            b'\r' | b'\n' => i += 1,
            _ => {
                group_start = false;
                i += 1;
            }
        }
    }
    out
}

/// Destinations the RTF reader skips whole (word.rs:286-292), pictures inside them included.
const RTF_SKIPPED: &[&[u8]] = &[
    b"footnote",
    b"annotation",
    b"object",
    b"nonshppict",
    b"shpinst",
    b"do",
    b"xe",
    b"tc",
    b"txe",
    b"comment",
    b"themedata",
    b"datastore",
    b"objdata",
    b"listpicture",
    b"background",
    b"info",
];

/// Starred destinations the RTF reader does read (word.rs:267-286).
const RTF_READ: &[&[u8]] = &[b"listtable", b"listoverridetable", b"revtbl", b"oldcprops", b"oldpprops", b"bkmkstart"];

/// A web page (crates/formats/src/html/builder.rs:511 drops scripts, style sheets and the head;
/// :596-619 keeps a picture only from a `data:` address, opened from bytes there is no folder to
/// load others from; export.rs writes back only the text, tables and inline pictures).
fn html(text: &str) -> Vec<Loss> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut rest = lower.as_str();
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        let dropped = match name.as_str() {
            "script" | "style" | "link" | "iframe" | "frame" | "object" | "embed" | "video" | "audio" | "canvas" | "svg" | "form" | "input"
            | "select" | "textarea" | "button" => true,
            "img" => !attr_value(tag, "src").is_some_and(|src| src.trim_start().starts_with("data:")),
            _ => false,
        };
        if dropped {
            out.push(Loss::WebContent);
            break;
        }
    }
    out
}

/// The value of attribute `key` in a lower-case tag's text (`img src="a.png" alt=x`).
fn attr_value<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = tag;
    loop {
        let at = rest.find(key)?;
        let before = rest[..at].chars().next_back();
        rest = &rest[at + key.len()..];
        if !before.is_some_and(|c| c.is_ascii_whitespace()) {
            continue;
        }
        let Some(value) = rest.trim_start().strip_prefix('=') else { continue };
        let value = value.trim_start();
        return Some(match value.chars().next() {
            Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or(""),
            _ => value.split(|c: char| c.is_ascii_whitespace()).next().unwrap_or(""),
        });
    }
}

/// A Markdown file (crates/formats/src/markdown.rs:1-10): a picture from another file is read
/// as its alt text and any picture is written back as a placeholder; front matter and HTML other
/// than comments, `<br>` and `<a id>` are not kept as written.
fn markdown(text: &str) -> Vec<Loss> {
    let mut out = Vec::new();
    let body = text.trim_start_matches('\u{FEFF}');
    if body.starts_with("---\n") || body.starts_with("---\r\n") || body.starts_with("+++") {
        out.push(Loss::Other);
    }
    let mut in_code = false;
    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_code = !in_code;
            continue;
        }
        if in_code || line.starts_with("    ") || line.starts_with('\t') {
            continue;
        }
        let mut rest = line;
        while let Some(at) = rest.find("![") {
            rest = &rest[at + 2..];
            // Export writes every picture as an `![alt](media-name)` placeholder.
            if rest.contains("](") {
                out.push(Loss::Pictures);
            }
        }
        if !html(line).is_empty() {
            out.push(Loss::WebContent);
        }
    }
    out
}
