//! OpenDocument presentations (.odp: LibreOffice Impress, Pardus, Google Slides export) read into
//! the DeckCraft model, which has no reader for them: each page becomes a slide with its text
//! frames, pictures and basic shapes where they were. Looks beyond that (styles, animations,
//! charts) are not carried over; the import says so.

use std::io::Read;
use std::sync::Arc;

use deckcraft_model::{Presentation, Shape, ShapeId, ShapeKind, Xfrm};

use crate::DeckError;

/// Whether `bytes` is an OpenDocument presentation (a zip whose `mimetype` says so).
pub fn sniff(bytes: &[u8]) -> bool {
    // ODF stores `mimetype` first, uncompressed: its name and value sit right after the header.
    const MIME: &[u8] = b"application/vnd.oasis.opendocument.presentation";
    bytes.starts_with(b"PK\x03\x04") && bytes.get(30..38) == Some(b"mimetype") && {
        let tail = &bytes[38..bytes.len().min(38 + 80)];
        tail.windows(MIME.len()).any(|w| w == MIME)
    }
}

/// The presentation in `bytes`, and notes on what it could not keep.
pub fn import(bytes: &[u8]) -> Result<(Presentation, Vec<String>), DeckError> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| DeckError::Open(format!("ODP: {e}")))?;
    let read = |zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str| -> Option<Vec<u8>> {
        let mut f = zip.by_name(name).ok()?;
        // A bound on what one part may unpack to: a hostile file cannot fill the memory.
        if f.size() > MAX_PART {
            return None;
        }
        let mut out = Vec::new();
        f.read_to_end(&mut out).ok()?;
        Some(out)
    };
    let content = read(&mut zip, "content.xml").ok_or_else(|| DeckError::Open("ODP: no content.xml".into()))?;
    let content = String::from_utf8_lossy(&content).into_owned();
    let doc = roxmltree::Document::parse(&content).map_err(|e| DeckError::Open(format!("ODP: {e}")))?;
    let styles = read(&mut zip, "styles.xml").map(|b| String::from_utf8_lossy(&b).into_owned());
    let (sw, sh) = styles.as_deref().and_then(page_size).unwrap_or((28.0 / 2.54 * 72.0, 15.75 / 2.54 * 72.0));

    let mut pres = Presentation::default();
    pres.slide_size.width = sw;
    pres.slide_size.height = sh;
    // The blank layout (the last one when none is called so) for every slide.
    let layout = pres
        .masters
        .first()
        .and_then(|m| m.layouts.iter().find(|l| l.name.eq_ignore_ascii_case("blank")).or_else(|| m.layouts.last()))
        .map(|l| l.id)
        .ok_or_else(|| DeckError::Open("no slide layout".into()))?;
    pres.slides.clear();
    let mut skipped = 0usize;
    for page in doc.descendants().filter(|n| n.has_tag_name((DRAW, "page"))) {
        let mut slide = deckcraft_model::defaults::new_slide(&mut pres, layout).ok_or_else(|| DeckError::Open("too many slides".into()))?;
        slide.shapes.clear();
        if let Some(name) = page.attribute((DRAW, "name")) {
            slide.name = name.to_string();
        }
        for item in page.children().filter(|n| n.is_element()) {
            match shape_of(&mut pres, &mut zip, item, &read) {
                Some(shape) => slide.shapes.push(shape),
                None if !item.has_tag_name((PRESENTATION, "notes")) => skipped += 1,
                None => {}
            }
        }
        // Speaker notes: the text of the notes page's outline frame.
        if let Some(notes) = page.children().find(|n| n.has_tag_name((PRESENTATION, "notes"))) {
            let text: Vec<String> = notes
                .descendants()
                .filter(|n| n.has_tag_name((DRAW, "frame")) && n.attribute((PRESENTATION, "class")) == Some("notes"))
                .flat_map(paragraphs)
                .collect();
            if !text.is_empty() {
                slide.notes = deckcraft_model::text::TextBody::from_text(&text.join("\n"));
            }
        }
        pres.slides.push(Arc::new(slide));
    }
    if pres.slides.is_empty() {
        return Err(DeckError::Open("ODP: no slides".into()));
    }
    let mut notes = vec!["OpenDocument sunusu: metinler, resimler ve temel şekiller aktarıldı; biçimler ve animasyonlar aktarılmadı.".to_string()];
    if skipped > 0 {
        notes.push(format!("{skipped} nesne (grafik, tablo, gömülü nesne) gösterilemedi."));
    }
    Ok((pres, notes))
}

/// The slide shape for one item on a page (a frame, a shape), or None for what is not read.
fn shape_of<'a>(
    pres: &mut Presentation,
    zip: &mut zip::ZipArchive<std::io::Cursor<&'a [u8]>>,
    n: roxmltree::Node,
    read: &ReadPart<'a>,
) -> Option<Shape> {
    let xfrm = box_of(n)?;
    let id = ShapeId(pres.alloc_id()?);
    let tag = n.tag_name().name();
    match tag {
        "frame" => {
            // A picture, or a text box.
            if let Some(img) = n.children().find(|c| c.has_tag_name((DRAW, "image"))) {
                let href = img.attribute((XLINK, "href"))?;
                let data = read(zip, href.trim_start_matches("./"))?;
                let mime = if data.starts_with(&[0x89, b'P', b'N', b'G']) {
                    "image/png"
                } else if data.starts_with(&[0xFF, 0xD8]) {
                    "image/jpeg"
                } else if data.starts_with(b"GIF8") {
                    "image/gif"
                } else {
                    return None;
                };
                let name = href.rsplit('/').next().unwrap_or("picture");
                let media = pres.add_media(name, mime, data)?;
                return Some(Shape {
                    id,
                    name: format!("Picture {}", id.0),
                    xfrm: Some(xfrm),
                    kind: ShapeKind::Picture { fill: deckcraft_model::style::PictureFill { media, ..Default::default() } },
                    ..Default::default()
                });
            }
            let text = n.children().find(|c| c.has_tag_name((DRAW, "text-box")))?;
            let paras = paragraphs(text);
            Some(Shape {
                id,
                name: format!("TextBox {}", id.0),
                xfrm: Some(xfrm),
                text_box: true,
                text: Some(deckcraft_model::text::TextBody::from_text(&paras.join("\n"))),
                ..Default::default()
            })
        }
        "rect" | "ellipse" | "custom-shape" => {
            let preset = match tag {
                "ellipse" => "ellipse",
                "custom-shape" => {
                    match n.children().find(|c| c.has_tag_name((DRAW, "enhanced-geometry"))).and_then(|g| g.attribute((DRAW, "type"))) {
                        Some("ellipse") => "ellipse",
                        Some("round-rectangle") => "roundRect",
                        Some("isosceles-triangle") => "triangle",
                        Some("right-arrow") => "rightArrow",
                        Some("star5") => "star5",
                        _ => "rect",
                    }
                }
                _ => "rect",
            };
            let paras = paragraphs(n);
            Some(Shape {
                id,
                name: format!("Shape {}", id.0),
                xfrm: Some(xfrm),
                geom: deckcraft_model::Geom::Preset { name: preset.into(), adj: vec![] },
                // LibreOffice's default shape look: a light blue fill and a darker outline.
                fill: Some(deckcraft_model::Fill::Solid { color: rgb(0x72_9F_CF) }),
                line: Some(deckcraft_model::Line {
                    fill: Some(deckcraft_model::Fill::Solid { color: rgb(0x34_65_A4) }),
                    width: Some(1.0),
                    ..Default::default()
                }),
                text: Some(deckcraft_model::text::TextBody::from_text(&paras.join("\n"))),
                ..Default::default()
            })
        }
        _ => None,
    }
}

fn rgb(c: u32) -> deckcraft_model::ColorRef {
    deckcraft_model::ColorRef::rgb(deckcraft_model::Rgba { r: (c >> 16) as u8, g: (c >> 8) as u8, b: c as u8, a: 255 })
}

/// An item's box in points from its `svg:x`, `svg:y`, `svg:width`, `svg:height`.
fn box_of(n: roxmltree::Node) -> Option<Xfrm> {
    let get = |name: &str| n.attribute((SVG, name)).and_then(length);
    let (w, h) = (get("width")?, get("height")?);
    Some(Xfrm { x: get("x").unwrap_or(0.0), y: get("y").unwrap_or(0.0), w, h, ..Default::default() })
}

/// An ODF length ("2.5cm", "10mm", "1in", "12pt", "100px") in points.
fn length(v: &str) -> Option<f64> {
    let v = v.trim();
    let split = v.find(|c: char| c.is_ascii_alphabetic() || c == '%')?;
    let (num, unit) = v.split_at(split);
    let n: f64 = num.trim().parse().ok()?;
    let pt = match unit {
        "cm" => n / 2.54 * 72.0,
        "mm" => n / 25.4 * 72.0,
        "in" => n * 72.0,
        "pt" => n,
        "pc" => n * 12.0,
        "px" => n * 0.75,
        _ => return None,
    };
    pt.is_finite().then_some(pt)
}

/// The slide size from `styles.xml`: the first page layout's width and height, in points.
fn page_size(styles: &str) -> Option<(f64, f64)> {
    let doc = roxmltree::Document::parse(styles).ok()?;
    let props = doc.descendants().find(|n| n.has_tag_name((STYLE, "page-layout-properties")))?;
    let w = props.attribute((FO, "page-width")).and_then(length)?;
    let h = props.attribute((FO, "page-height")).and_then(length)?;
    (w > 1.0 && h > 1.0).then_some((w, h))
}

/// The paragraphs (`text:p`, `text:h`, in lists too) under `n`, as plain text.
fn paragraphs(n: roxmltree::Node) -> Vec<String> {
    n.descendants().filter(|d| d.has_tag_name((TEXT, "p")) || d.has_tag_name((TEXT, "h"))).map(paragraph_text).collect()
}

/// A paragraph's text: its spans, with `text:s` spaces, tabs and line breaks.
fn paragraph_text(p: roxmltree::Node) -> String {
    let mut out = String::new();
    fn walk(n: roxmltree::Node, out: &mut String) {
        for c in n.children() {
            if c.is_text() {
                out.push_str(c.text().unwrap_or(""));
            } else if c.has_tag_name((TEXT, "s")) {
                let count: usize = c.attribute((TEXT, "c")).and_then(|v| v.parse().ok()).unwrap_or(1);
                out.extend(std::iter::repeat_n(' ', count.min(100)));
            } else if c.has_tag_name((TEXT, "tab")) {
                out.push('\t');
            } else if c.has_tag_name((TEXT, "line-break")) {
                out.push('\u{000B}');
            } else if c.is_element() {
                walk(c, out);
            }
        }
    }
    walk(p, &mut out);
    out
}

/// Reads one part of the zip (None when it is missing or too big).
type ReadPart<'a> = dyn Fn(&mut zip::ZipArchive<std::io::Cursor<&'a [u8]>>, &str) -> Option<Vec<u8>>;

/// What one zip part may unpack to (64 MB).
const MAX_PART: u64 = 64 * 1024 * 1024;

const DRAW: &str = "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
const TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const SVG: &str = "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0";
const STYLE: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
const FO: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
const XLINK: &str = "http://www.w3.org/1999/xlink";
const PRESENTATION: &str = "urn:oasis:names:tc:opendocument:xmlns:presentation:1.0";
