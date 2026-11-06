//! Editing a presentation on DeckCraft's model: the text of a shape, and slides added,
//! duplicated, moved and deleted. The editing engine is left out of the app on purpose (it brings
//! the audio and video decoders); these few changes use the model's own helpers (`edit`,
//! `defaults`, `resolve`) the way the engine's commands do.

use std::collections::HashMap;
use std::sync::Arc;

use deckcraft_model::{Presentation, Shape, ShapeId, ShapeKind, SlideId};

use crate::DeckError;

fn edit_err(m: &str) -> DeckError {
    DeckError::Edit(m.to_string())
}

/// A shape with text on a slide: its id, its box in slide points and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextShape {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub text: String,
    /// For a table cell: its row and column in the table whose shape is `id`.
    pub cell: Option<(u32, u32)>,
}

/// The shapes of slide `index` that hold text (placeholders and text boxes), front-most last.
pub fn text_shapes(pres: &Presentation, index: usize) -> Result<Vec<TextShape>, DeckError> {
    let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
    let ctx = deckcraft_model::resolve::Ctx::for_slide(pres, slide);
    let mut out = Vec::new();
    for s in slide.shapes.iter().filter(|s| !s.hidden) {
        let x = match &ctx {
            Some(c) => deckcraft_model::resolve::xfrm(c, s),
            None => s.xfrm.unwrap_or_default(),
        };
        collect_text(s, x, &mut out);
    }
    Ok(out)
}

/// The text shapes in `s` (itself, or inside it when it is a group), `x` being where `s` is
/// on the slide. A group's children are placed through its child coordinate space.
fn collect_text(s: &deckcraft_model::Shape, x: deckcraft_model::Xfrm, out: &mut Vec<TextShape>) {
    match &s.kind {
        ShapeKind::Shape if s.text.is_some() => out.push(TextShape {
            id: s.id.0,
            x: x.x as f32,
            y: x.y as f32,
            w: x.w as f32,
            h: x.h as f32,
            text: s.text.as_ref().map(|t| t.text()).unwrap_or_default(),
            cell: None,
        }),
        // Every cell of a table, placed by the column widths and row heights scaled to the frame.
        ShapeKind::Table(t) => {
            let total_w: f64 = t.cols.iter().sum();
            let total_h: f64 = t.rows.iter().map(|r| r.height).sum();
            let (sx, sy) = (if total_w > 0.0 { x.w / total_w } else { 1.0 }, if total_h > 0.0 { x.h / total_h } else { 1.0 });
            let mut top = 0.0;
            for (ri, row) in t.rows.iter().enumerate() {
                let mut left = 0.0;
                for (ci, cell) in row.cells.iter().enumerate() {
                    let span_w: f64 = t.cols.iter().skip(ci).take(cell.grid_span.max(1) as usize).sum();
                    let span_h: f64 = t.rows.iter().skip(ri).take(cell.row_span.max(1) as usize).map(|r| r.height).sum();
                    if !cell.is_covered() {
                        out.push(TextShape {
                            id: s.id.0,
                            x: (x.x + left * sx) as f32,
                            y: (x.y + top * sy) as f32,
                            w: (span_w * sx) as f32,
                            h: (span_h * sy) as f32,
                            text: cell.text.text(),
                            cell: Some((ri as u32, ci as u32)),
                        });
                    }
                    left += t.cols.get(ci).copied().unwrap_or(0.0);
                }
                top += row.height;
            }
        }
        ShapeKind::Group { children, child } => {
            let (sx, sy) = (if child.w != 0.0 { x.w / child.w } else { 1.0 }, if child.h != 0.0 { x.h / child.h } else { 1.0 });
            for c in children.iter().filter(|c| !c.hidden) {
                let cx = c.xfrm.unwrap_or_default();
                let placed =
                    deckcraft_model::Xfrm { x: x.x + (cx.x - child.x) * sx, y: x.y + (cx.y - child.y) * sy, w: cx.w * sx, h: cx.h * sy, ..cx };
                collect_text(c, placed, out);
            }
        }
        _ => {}
    }
}

/// Makes the text of shape `id` on slide `index` read `text` (paragraphs split by `\n`); text
/// that stays keeps its formatting, new text takes the formatting where it lands.
pub fn set_text(pres: &mut Presentation, index: usize, id: u32, text: &str) -> Result<(), DeckError> {
    let slide = pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?;
    let slide = Arc::make_mut(slide);
    let shape = find_shape(&mut slide.shapes, id).ok_or_else(|| edit_err("no such shape"))?;
    let body = shape.text.get_or_insert_with(Default::default);
    deckcraft_model::edit::replace_text(body, text);
    Ok(())
}

/// Shape `id` among `shapes` or inside their groups.
fn find_shape(shapes: &mut [deckcraft_model::Shape], id: u32) -> Option<&mut deckcraft_model::Shape> {
    for s in shapes {
        if s.id.0 == id {
            return Some(s);
        }
        if let ShapeKind::Group { children, .. } = &mut s.kind
            && let Some(found) = find_shape(children, id)
        {
            return Some(found);
        }
    }
    None
}

/// What kind of object a slide holds, for the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    Text,
    Picture,
    Table,
    Other,
}

/// One object on a slide (top level: a group counts as one), where it is in points.
#[derive(Clone, Debug, PartialEq)]
pub struct SlideObject {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub kind: ObjectKind,
}

/// The objects of slide `index` that can be moved, front-most last.
pub fn objects(pres: &Presentation, index: usize) -> Result<Vec<SlideObject>, DeckError> {
    let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
    let ctx = deckcraft_model::resolve::Ctx::for_slide(pres, slide);
    Ok(slide
        .shapes
        .iter()
        .filter(|s| !s.hidden && !s.locked)
        .map(|s| {
            let x = match &ctx {
                Some(c) => deckcraft_model::resolve::xfrm(c, s),
                None => s.xfrm.unwrap_or_default(),
            };
            let kind = match &s.kind {
                ShapeKind::Picture { .. } => ObjectKind::Picture,
                ShapeKind::Table(_) => ObjectKind::Table,
                ShapeKind::Shape if s.text.is_some() => ObjectKind::Text,
                _ => ObjectKind::Other,
            };
            SlideObject { id: s.id.0, x: x.x as f32, y: x.y as f32, w: x.w as f32, h: x.h as f32, kind }
        })
        .collect())
}

/// Moves object `id` on slide `index` by (`dx`, `dy`) points and scales it by `scale` about its
/// centre (1 keeps its size). A placeholder that took its box from the layout gets its own.
pub fn place_object(pres: &mut Presentation, index: usize, id: u32, dx: f64, dy: f64, scale: f64) -> Result<(), DeckError> {
    let current = {
        let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
        let shape = slide.shapes.iter().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
        match deckcraft_model::resolve::Ctx::for_slide(pres, slide) {
            Some(c) => deckcraft_model::resolve::xfrm(&c, shape),
            None => shape.xfrm.unwrap_or_default(),
        }
    };
    let scale = scale.clamp(0.05, 20.0);
    let (w, h) = (current.w * scale, current.h * scale);
    let moved = deckcraft_model::Xfrm { x: current.x + dx - (w - current.w) / 2.0, y: current.y + dy - (h - current.h) / 2.0, w, h, ..current };
    let slide = Arc::make_mut(pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?);
    let shape = slide.shapes.iter_mut().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
    // A group's children are laid out in its own space, so its frame is enough; a table's grid
    // carries its own sizes, which scale with it.
    if let ShapeKind::Table(t) = &mut shape.kind
        && scale != 1.0
    {
        t.cols.iter_mut().for_each(|c| *c *= scale);
        t.rows.iter_mut().for_each(|r| r.height *= scale);
    }
    shape.xfrm = Some(moved);
    Ok(())
}

/// Slide `index` with a new text box reading `text`, in the middle of the slide; returns its id.
pub fn add_text_box(pres: &mut Presentation, index: usize, text: &str) -> Result<u32, DeckError> {
    let (sw, sh) = slide_size_pt(pres);
    let (w, h) = (sw * 0.5, sh * 0.15);
    let shape = deckcraft_model::Shape {
        name: "TextBox".into(),
        xfrm: Some(deckcraft_model::Xfrm { x: (sw - w) / 2.0, y: (sh - h) / 2.0, w, h, ..Default::default() }),
        text_box: true,
        text: Some(deckcraft_model::text::TextBody::from_text(text)),
        ..Default::default()
    };
    push_shape(pres, index, shape)
}

/// Slide `index` with picture `data` (PNG or JPEG, `width`×`height` pixels) in its middle,
/// as large as fits in two thirds of the slide; returns its id.
pub fn add_picture(pres: &mut Presentation, index: usize, data: Vec<u8>, width: u32, height: u32) -> Result<u32, DeckError> {
    let content_type = if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if data.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg"
    } else {
        return Err(edit_err("only PNG and JPEG pictures can be added"));
    };
    if pres.slides.get(index).is_none() {
        return Err(DeckError::NoSlide(index as u32));
    }
    let name = if content_type == "image/png" { "picture.png" } else { "picture.jpeg" };
    let media = pres.add_media(name, content_type, data).ok_or_else(|| edit_err("the presentation cannot grow further"))?;
    let (sw, sh) = slide_size_pt(pres);
    let (iw, ih) = (f64::from(width.max(1)), f64::from(height.max(1)));
    let k = (sw * 2.0 / 3.0 / iw).min(sh * 2.0 / 3.0 / ih);
    let (w, h) = (iw * k, ih * k);
    let shape = deckcraft_model::Shape {
        name: "Picture".into(),
        xfrm: Some(deckcraft_model::Xfrm { x: (sw - w) / 2.0, y: (sh - h) / 2.0, w, h, ..Default::default() }),
        kind: ShapeKind::Picture { fill: deckcraft_model::style::PictureFill { media, ..Default::default() } },
        ..Default::default()
    };
    push_shape(pres, index, shape)
}

fn slide_size_pt(pres: &Presentation) -> (f64, f64) {
    (pres.slide_size.width, pres.slide_size.height)
}

fn push_shape(pres: &mut Presentation, index: usize, mut shape: deckcraft_model::Shape) -> Result<u32, DeckError> {
    let id = pres.alloc_id().ok_or_else(|| edit_err("the presentation cannot grow further"))?;
    shape.id = deckcraft_model::ShapeId(id);
    shape.name = format!("{} {id}", shape.name);
    let slide = Arc::make_mut(pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?);
    slide.shapes.push(shape);
    Ok(id)
}

/// Removes object `id` from slide `index`, with its animations.
pub fn delete_object(pres: &mut Presentation, index: usize, id: u32) -> Result<(), DeckError> {
    let slide = Arc::make_mut(pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?);
    let before = slide.shapes.len();
    slide.shapes.retain(|s| s.id.0 != id);
    if slide.shapes.len() == before {
        return Err(edit_err("no such object"));
    }
    slide.animations.retain(|a| a.shape.0 != id);
    Ok(())
}

/// Makes cell (`row`, `col`) of the table shape `id` on slide `index` read `text`.
pub fn set_cell_text(pres: &mut Presentation, index: usize, id: u32, row: u32, col: u32, text: &str) -> Result<(), DeckError> {
    let slide = pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?;
    let slide = Arc::make_mut(slide);
    let shape = find_shape(&mut slide.shapes, id).ok_or_else(|| edit_err("no such shape"))?;
    let ShapeKind::Table(t) = &mut shape.kind else { return Err(edit_err("not a table")) };
    let cell = t.rows.get_mut(row as usize).and_then(|r| r.cells.get_mut(col as usize)).ok_or_else(|| edit_err("no such cell"))?;
    deckcraft_model::edit::replace_text(&mut cell.text, text);
    Ok(())
}

/// A new slide after `index`, with the same layout and empty placeholders. Returns its index.
pub fn new_slide(pres: &mut Presentation, index: usize) -> Result<usize, DeckError> {
    let layout = pres.slides.get(index).map(|s| s.layout).ok_or(DeckError::NoSlide(index as u32))?;
    let slide = deckcraft_model::defaults::new_slide(pres, layout).ok_or_else(|| edit_err("the presentation has no room for more slides"))?;
    let id = slide.id;
    let at = index + 1;
    pres.slides.insert(at, Arc::new(slide));
    add_to_section_after(pres, index, id);
    Ok(at)
}

/// A copy of slide `index` right after it (new ids, animations following them). Returns its index.
pub fn duplicate_slide(pres: &mut Presentation, index: usize) -> Result<usize, DeckError> {
    let mut copy = pres.slides.get(index).map(|s| s.as_ref().clone()).ok_or(DeckError::NoSlide(index as u32))?;
    copy.id = SlideId(pres.alloc_id().ok_or_else(|| edit_err("the presentation has no room for more slides"))?);
    let mut map = HashMap::new();
    reid(pres, &mut copy.shapes, &mut map, 0)?;
    for a in &mut copy.animations {
        a.shape = map.get(&a.shape).copied().unwrap_or(a.shape);
        a.trigger = a.trigger.and_then(|t| map.get(&t).copied());
    }
    let id = copy.id;
    let at = index + 1;
    pres.slides.insert(at, Arc::new(copy));
    add_to_section_after(pres, index, id);
    Ok(at)
}

/// Removes slide `index`; a presentation keeps at least one slide.
pub fn delete_slide(pres: &mut Presentation, index: usize) -> Result<(), DeckError> {
    if pres.slides.len() < 2 {
        return Err(edit_err("a presentation needs at least one slide"));
    }
    if index >= pres.slides.len() {
        return Err(DeckError::NoSlide(index as u32));
    }
    let gone = pres.slides.remove(index).id;
    for sec in &mut pres.sections {
        sec.slides.retain(|s| *s != gone);
    }
    Ok(())
}

/// Moves slide `from` to position `to`.
pub fn move_slide(pres: &mut Presentation, from: usize, to: usize) -> Result<(), DeckError> {
    if from >= pres.slides.len() || to >= pres.slides.len() {
        return Err(DeckError::NoSlide(from.max(to) as u32));
    }
    let s = pres.slides.remove(from);
    pres.slides.insert(to, s);
    Ok(())
}

/// New ids for `shapes` and their children; connectors follow their ends.
fn reid(pres: &mut Presentation, shapes: &mut [Shape], map: &mut HashMap<ShapeId, ShapeId>, depth: usize) -> Result<(), DeckError> {
    if depth > 64 {
        return Ok(());
    }
    for s in shapes.iter_mut() {
        let id = ShapeId(pres.alloc_id().ok_or_else(|| edit_err("the presentation has no room for more shapes"))?);
        map.insert(s.id, id);
        s.id = id;
        if let ShapeKind::Group { children, .. } = &mut s.kind {
            reid(pres, children, map, depth + 1)?;
        }
    }
    if depth == 0 {
        fix_connectors(shapes, map);
    }
    Ok(())
}

fn fix_connectors(shapes: &mut [Shape], map: &HashMap<ShapeId, ShapeId>) {
    for s in shapes {
        match &mut s.kind {
            ShapeKind::Connector { start, end } => {
                for e in [start, end] {
                    *e = e.and_then(|(id, site)| map.get(&id).map(|n| (*n, site)));
                }
            }
            ShapeKind::Group { children, .. } => fix_connectors(children, map),
            _ => {}
        }
    }
}

/// A new slide joins the section of the slide it follows.
fn add_to_section_after(pres: &mut Presentation, index: usize, id: SlideId) {
    let Some(prev) = pres.slides.get(index).map(|s| s.id) else { return };
    for sec in &mut pres.sections {
        if let Some(p) = sec.slides.iter().position(|s| *s == prev) {
            sec.slides.insert(p + 1, id);
            return;
        }
    }
}
