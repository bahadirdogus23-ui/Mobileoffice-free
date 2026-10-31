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
}

/// The shapes of slide `index` that hold text (placeholders and text boxes), front-most last.
pub fn text_shapes(pres: &Presentation, index: usize) -> Result<Vec<TextShape>, DeckError> {
    let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
    let ctx = deckcraft_model::resolve::Ctx::for_slide(pres, slide);
    Ok(slide
        .shapes
        .iter()
        .filter(|s| !s.hidden && s.text.is_some() && matches!(s.kind, ShapeKind::Shape))
        .map(|s| {
            let x = match &ctx {
                Some(c) => deckcraft_model::resolve::xfrm(c, s),
                None => s.xfrm.unwrap_or_default(),
            };
            TextShape {
                id: s.id.0,
                x: x.x as f32,
                y: x.y as f32,
                w: x.w as f32,
                h: x.h as f32,
                text: s.text.as_ref().map(|t| t.text()).unwrap_or_default(),
            }
        })
        .collect())
}

/// Makes the text of shape `id` on slide `index` read `text` (paragraphs split by `\n`); text
/// that stays keeps its formatting, new text takes the formatting where it lands.
pub fn set_text(pres: &mut Presentation, index: usize, id: u32, text: &str) -> Result<(), DeckError> {
    let slide = pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?;
    let slide = Arc::make_mut(slide);
    let shape = slide.shapes.iter_mut().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such shape"))?;
    let body = shape.text.get_or_insert_with(Default::default);
    deckcraft_model::edit::replace_text(body, text);
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
