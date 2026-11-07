//! Old binary PowerPoint (.ppt, .pps; PowerPoint 97–2003) read for its words, which DeckCraft
//! has no reader for: each slide's title and body text, its speaker notes and the slide size,
//! from the "PowerPoint Document" stream's text lists ([MS-PPT] SlideListWithText). Pictures,
//! shapes and formatting are not carried over; the import says so.

use std::io::Read;
use std::sync::Arc;

use deckcraft_model::{Presentation, Shape, ShapeId, Xfrm};

use crate::DeckError;

/// Whether `bytes` is a compound file holding a PowerPoint 97–2003 document.
pub fn sniff(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1])
        && cfb::CompoundFile::open(std::io::Cursor::new(bytes)).is_ok_and(|f| f.exists("/PowerPoint Document"))
}

/// One slide's words: its title, its other text blocks and its notes.
#[derive(Default)]
struct SlideText {
    title: Vec<String>,
    body: Vec<String>,
    notes: Vec<String>,
}

/// The presentation in `bytes` (its words only), and notes on what it could not keep.
pub fn import(bytes: &[u8]) -> Result<(Presentation, Vec<String>), DeckError> {
    let mut file = cfb::CompoundFile::open(std::io::Cursor::new(bytes)).map_err(|e| DeckError::Open(format!("PPT: {e}")))?;
    let mut doc = Vec::new();
    file.open_stream("/PowerPoint Document")
        .map_err(|e| DeckError::Open(format!("PPT: {e}")))?
        .take(MAX_STREAM)
        .read_to_end(&mut doc)
        .map_err(|e| DeckError::Open(format!("PPT: {e}")))?;
    let mut size = None;
    let mut slides: Vec<SlideText> = Vec::new();
    let mut notes: Vec<Vec<String>> = Vec::new();
    walk(&doc, 0, &mut |rec_type, instance, body, list| {
        match rec_type {
            // DocumentAtom: the slide size in master units (576 per inch).
            DOCUMENT_ATOM if size.is_none() && body.len() >= 8 => {
                let w = i32::from_le_bytes(body[0..4].try_into().unwrap_or_default());
                let h = i32::from_le_bytes(body[4..8].try_into().unwrap_or_default());
                if w > 0 && h > 0 {
                    size = Some((f64::from(w) / 8.0, f64::from(h) / 8.0));
                }
            }
            // A new slide (or notes page) in a SlideListWithText.
            SLIDE_PERSIST_ATOM => match list {
                Some(0) => slides.push(SlideText::default()),
                Some(2) => notes.push(Vec::new()),
                _ => {}
            },
            TEXT_CHARS_ATOM | TEXT_BYTES_ATOM => {
                let text = if rec_type == TEXT_CHARS_ATOM {
                    String::from_utf16_lossy(&body.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect::<Vec<_>>())
                } else {
                    body.iter().map(|&b| char::from(b)).collect()
                };
                // PowerPoint ends paragraphs with a carriage return; a vertical tab breaks a line.
                let text = text.replace(['\r', '\u{000B}'], "\n");
                let text = text.trim_end_matches('\n').to_string();
                if text.trim().is_empty() {
                    return;
                }
                match list {
                    Some(0) => {
                        if let Some(s) = slides.last_mut() {
                            // The text header before it said whether this is the title.
                            if instance == 0 && s.title.is_empty() && s.body.is_empty() {
                                s.title.push(text);
                            } else {
                                s.body.push(text);
                            }
                        }
                    }
                    Some(2) => {
                        if let Some(n) = notes.last_mut() {
                            n.push(text);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
    if slides.is_empty() {
        return Err(DeckError::Open("PPT: no slides with text were found".into()));
    }
    for (s, n) in slides.iter_mut().zip(notes) {
        s.notes = n;
    }
    let (sw, sh) = size.unwrap_or((720.0, 540.0));
    let mut pres = Presentation::default();
    pres.slide_size.width = sw;
    pres.slide_size.height = sh;
    let layout = pres
        .masters
        .first()
        .and_then(|m| m.layouts.iter().find(|l| l.name.eq_ignore_ascii_case("blank")).or_else(|| m.layouts.last()))
        .map(|l| l.id)
        .ok_or_else(|| DeckError::Open("no slide layout".into()))?;
    pres.slides.clear();
    for s in slides {
        let mut slide = deckcraft_model::defaults::new_slide(&mut pres, layout).ok_or_else(|| DeckError::Open("too many slides".into()))?;
        slide.shapes.clear();
        // The title across the top, the rest below it, as PowerPoint's Title and Content layout.
        let boxes = [
            (s.title, Xfrm { x: sw * 0.06, y: sh * 0.05, w: sw * 0.88, h: sh * 0.18, ..Default::default() }, 32.0),
            (s.body, Xfrm { x: sw * 0.06, y: sh * 0.27, w: sw * 0.88, h: sh * 0.66, ..Default::default() }, 20.0),
        ];
        for (texts, xfrm, size_pt) in boxes {
            if texts.is_empty() {
                continue;
            }
            let id = ShapeId(pres.alloc_id().ok_or_else(|| DeckError::Open("too many shapes".into()))?);
            let mut body = deckcraft_model::text::TextBody::from_text(&texts.join("\n"));
            for p in &mut body.paragraphs {
                for r in &mut p.runs {
                    r.props.size = Some(size_pt);
                }
            }
            slide.shapes.push(Shape {
                id,
                name: format!("TextBox {}", id.0),
                xfrm: Some(xfrm),
                text_box: true,
                text: Some(body),
                ..Default::default()
            });
        }
        if !s.notes.is_empty() {
            slide.notes = deckcraft_model::text::TextBody::from_text(&s.notes.join("\n"));
        }
        pres.slides.push(Arc::new(slide));
    }
    Ok((
        pres,
        vec!["Eski PowerPoint (.ppt) sunusu: slaytların metinleri ve notları aktarıldı; resimler, şekiller ve biçimler aktarılmadı.".to_string()],
    ))
}

/// Calls `visit(type, instance, body, list)` for every record in `data` (depth first). `list`
/// is the instance of the SlideListWithText the record is in (0 slides, 1 masters, 2 notes).
fn walk(data: &[u8], depth: usize, visit: &mut Visit) {
    fn go(data: &[u8], depth: usize, list: Option<u16>, visit: &mut Visit) {
        let mut at = 0usize;
        // A text header's type (0 title) carries over to the text atom right after it.
        let mut text_type = 1u16;
        while at + 8 <= data.len() {
            let ver_inst = u16::from_le_bytes([data[at], data[at + 1]]);
            let rec_type = u16::from_le_bytes([data[at + 2], data[at + 3]]);
            let len = u32::from_le_bytes([data[at + 4], data[at + 5], data[at + 6], data[at + 7]]) as usize;
            let start = at + 8;
            let Some(end) = start.checked_add(len).filter(|e| *e <= data.len()) else { break };
            let body = &data[start..end];
            let ver = ver_inst & 0x0F;
            let instance = ver_inst >> 4;
            if ver == 0x0F {
                // A container: its records inside (bounded depth, so a hostile file cannot recurse forever).
                if depth < 16 {
                    let inner = if rec_type == SLIDE_LIST_WITH_TEXT { Some(instance) } else { list };
                    go(body, depth + 1, inner, visit);
                }
            } else if rec_type == TEXT_HEADER_ATOM && body.len() >= 4 {
                text_type = u32::from_le_bytes([body[0], body[1], body[2], body[3]]) as u16;
            } else if rec_type == TEXT_CHARS_ATOM || rec_type == TEXT_BYTES_ATOM {
                // Title (0) and centred title (6) both count as the title.
                let kind = if text_type == 0 || text_type == 6 { 0 } else { 1 };
                visit(rec_type, kind, body, list);
                text_type = 1;
            } else {
                visit(rec_type, instance, body, list);
            }
            at = end;
        }
    }
    go(data, depth, None, visit);
}

/// What [`walk`] calls for each record: its type, instance (or text kind), body and the text list it is in.
type Visit<'a> = dyn FnMut(u16, u16, &[u8], Option<u16>) + 'a;

const DOCUMENT_ATOM: u16 = 0x03E9;
const SLIDE_PERSIST_ATOM: u16 = 0x03F3;
const SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0;
const TEXT_HEADER_ATOM: u16 = 0x0F9F;
const TEXT_CHARS_ATOM: u16 = 0x0FA0;
const TEXT_BYTES_ATOM: u16 = 0x0FA8;

/// The most of the document stream read (256 MB).
const MAX_STREAM: u64 = 256 * 1024 * 1024;
