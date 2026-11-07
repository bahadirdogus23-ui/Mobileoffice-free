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
    /// Clockwise degrees about the box's centre.
    pub rot: f32,
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
            let x = drawn_box(ctx.as_ref(), s);
            let kind = match &s.kind {
                ShapeKind::Picture { .. } => ObjectKind::Picture,
                ShapeKind::Table(_) => ObjectKind::Table,
                ShapeKind::Shape if s.text.is_some() => ObjectKind::Text,
                _ => ObjectKind::Other,
            };
            SlideObject { id: s.id.0, x: x.x as f32, y: x.y as f32, w: x.w as f32, h: x.h as f32, rot: x.rot as f32, kind }
        })
        .collect())
}

/// A shape's box as it is drawn: its own or inherited one, and for a table as tall as its rows
/// are drawn (they grow to fit their text).
fn drawn_box(ctx: Option<&deckcraft_model::resolve::Ctx>, s: &deckcraft_model::Shape) -> deckcraft_model::Xfrm {
    let mut x = match ctx {
        Some(c) => deckcraft_model::resolve::xfrm(c, s),
        None => s.xfrm.unwrap_or_default(),
    };
    if let (ShapeKind::Table(t), Some(c)) = (&s.kind, ctx) {
        let drawn: f64 = deckcraft_render::table_row_heights(c, t, &deckcraft_text::NoFields).iter().sum();
        if drawn.is_finite() && drawn > x.h {
            x.h = drawn;
        }
    }
    x
}

/// Moves object `id` on slide `index` by (`dx`, `dy`) points, scales it by `scale` about its
/// centre (1 keeps its size) and turns it `rotate` degrees clockwise. A placeholder that took its
/// box from the layout gets its own.
pub fn place_object(pres: &mut Presentation, index: usize, id: u32, dx: f64, dy: f64, scale: f64, rotate: f64) -> Result<(), DeckError> {
    let current = {
        let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
        let shape = slide.shapes.iter().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
        drawn_box(deckcraft_model::resolve::Ctx::for_slide(pres, slide).as_ref(), shape)
    };
    let scale = scale.clamp(0.05, 20.0);
    // Sized from a corner, a text box's letters grow and shrink with it, as in design apps.
    let sizes = if scale != 1.0 { run_sizes(pres, index, id)? } else { Vec::new() };
    let (w, h) = (current.w * scale, current.h * scale);
    let moved = deckcraft_model::Xfrm {
        x: current.x + dx - (w - current.w) / 2.0,
        y: current.y + dy - (h - current.h) / 2.0,
        w,
        h,
        rot: (current.rot + rotate).rem_euclid(360.0),
        ..current
    };
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
    if scale != 1.0 && matches!(shape.kind, ShapeKind::Shape) {
        scale_text(shape, &sizes, scale);
    }
    let mut moved = moved;
    if let ShapeKind::Table(t) = &shape.kind {
        // A table keeps its rows' own (minimum) heights as its height: a taller box would
        // stretch the rows when drawn.
        moved.h = t.rows.iter().map(|r| r.height).sum::<f64>().max(1.0);
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

/// Slide `index` with a new shape (`preset` is a PowerPoint shape name such as "rect",
/// "ellipse", "rightArrow") filled with 0xRRGGBB, in the middle of the slide; returns its id.
/// It holds an empty text body, so text can be typed into it.
pub fn add_shape(pres: &mut Presentation, index: usize, preset: &str, fill: u32) -> Result<u32, DeckError> {
    const SHAPES: [&str; 8] = ["rect", "roundRect", "ellipse", "triangle", "rightArrow", "star5", "heart", "wedgeRoundRectCallout"];
    if !SHAPES.contains(&preset) {
        return Err(edit_err("unknown shape"));
    }
    let (sw, sh) = slide_size_pt(pres);
    let side = sh * 0.3;
    let (w, h) = if matches!(preset, "rightArrow" | "rect" | "roundRect" | "wedgeRoundRectCallout") { (side * 1.6, side) } else { (side, side) };
    let color = deckcraft_model::ColorRef::rgb(deckcraft_model::Rgba { r: (fill >> 16) as u8, g: (fill >> 8) as u8, b: fill as u8, a: 255 });
    let shape = deckcraft_model::Shape {
        name: "Shape".into(),
        xfrm: Some(deckcraft_model::Xfrm { x: (sw - w) / 2.0, y: (sh - h) / 2.0, w, h, ..Default::default() }),
        geom: deckcraft_model::Geom::Preset { name: preset.into(), adj: vec![] },
        fill: Some(deckcraft_model::Fill::Solid { color }),
        line: Some(deckcraft_model::Line { fill: Some(deckcraft_model::Fill::None), ..Default::default() }),
        text: Some(deckcraft_model::text::TextBody::from_text("")),
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

/// A copy of object `id` on slide `index`, a little down and to the right of it; returns its id.
pub fn duplicate_object(pres: &mut Presentation, index: usize, id: u32) -> Result<u32, DeckError> {
    let (mut copy, at) = {
        let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
        let shape = slide.shapes.iter().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
        let at = match deckcraft_model::resolve::Ctx::for_slide(pres, slide) {
            Some(c) => deckcraft_model::resolve::xfrm(&c, shape),
            None => shape.xfrm.unwrap_or_default(),
        };
        (shape.clone(), at)
    };
    // A copy is no placeholder: it keeps its own box and look.
    copy.ph = None;
    copy.xfrm = Some(deckcraft_model::Xfrm { x: at.x + 18.0, y: at.y + 18.0, ..at });
    renumber_children(pres, &mut copy)?;
    let name = copy.name.split(' ').next().unwrap_or("Shape").to_string();
    copy.name = name;
    push_shape(pres, index, copy)
}

/// Fresh ids for the shapes inside a group being copied.
fn renumber_children(pres: &mut Presentation, shape: &mut deckcraft_model::Shape) -> Result<(), DeckError> {
    if let ShapeKind::Group { children, .. } = &mut shape.kind {
        for child in children.iter_mut() {
            child.id = ShapeId(pres.alloc_id().ok_or_else(|| edit_err("the presentation cannot grow further"))?);
            renumber_children(pres, child)?;
        }
    }
    Ok(())
}

/// Object `id` brought in front of everything on slide `index`, or sent behind everything.
pub fn arrange_object(pres: &mut Presentation, index: usize, id: u32, to_front: bool) -> Result<(), DeckError> {
    let slide = Arc::make_mut(pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?);
    let at = slide.shapes.iter().position(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
    let shape = slide.shapes.remove(at);
    if to_front {
        slide.shapes.push(shape);
    } else {
        slide.shapes.insert(0, shape);
    }
    Ok(())
}

/// A change to the look of an object's text, or to its box.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectStyle {
    /// Every letter `step` points larger (smaller when negative).
    TextSize(f64),
    Bold(bool),
    Italic(bool),
    /// Text colour as 0xRRGGBB.
    TextColor(u32),
    /// 0 left, 1 centre, 2 right, 3 justified.
    Align(u8),
    /// The box filled with 0xRRGGBB, or left clear (None).
    Fill(Option<u32>),
    /// Every letter in this font family (Latin, East Asian and complex scripts alike).
    Font(String),
}

/// Applies `style` to object `id` on slide `index` (all of its text for the text changes).
pub fn style_object(pres: &mut Presentation, index: usize, id: u32, style: ObjectStyle) -> Result<(), DeckError> {
    // The size each run shows now (from the run, its placeholder, layout and master), for steps.
    let sizes = run_sizes(pres, index, id)?;
    let rgb = |c: u32| deckcraft_model::ColorRef::rgb(deckcraft_model::Rgba { r: (c >> 16) as u8, g: (c >> 8) as u8, b: c as u8, a: 255 });
    let slide = Arc::make_mut(pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?);
    let shape = slide.shapes.iter_mut().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
    if let ObjectStyle::Fill(fill) = style {
        shape.fill = Some(match fill {
            Some(c) => deckcraft_model::Fill::Solid { color: rgb(c) },
            None => deckcraft_model::Fill::None,
        });
        return Ok(());
    }
    let body = shape.text.as_mut().ok_or_else(|| edit_err("this object has no text"))?;
    for (pi, para) in body.paragraphs.iter_mut().enumerate() {
        if let ObjectStyle::Align(a) = style {
            para.props.align = Some(match a {
                1 => deckcraft_model::text::Align::Center,
                2 => deckcraft_model::text::Align::Right,
                3 => deckcraft_model::text::Align::Justify,
                _ => deckcraft_model::text::Align::Left,
            });
            continue;
        }
        for (ri, run) in para.runs.iter_mut().enumerate() {
            let p = &mut run.props;
            match &style {
                &ObjectStyle::TextSize(step) => {
                    let now = sizes.get(pi).and_then(|r| r.get(ri)).copied().unwrap_or(18.0);
                    p.size = Some((now + step).clamp(4.0, 400.0));
                }
                &ObjectStyle::Bold(on) => p.bold = Some(on),
                &ObjectStyle::Italic(on) => p.italic = Some(on),
                &ObjectStyle::TextColor(c) => p.fill = Some(deckcraft_model::Fill::Solid { color: rgb(c) }),
                ObjectStyle::Font(name) => {
                    p.font = Some(name.clone());
                    p.font_ea = Some(name.clone());
                    p.font_cs = Some(name.clone());
                }
                ObjectStyle::Align(_) | ObjectStyle::Fill(_) => {}
            }
        }
    }
    Ok(())
}

/// Whether the first letter of object `id`'s text is bold (as shown).
pub fn text_is_bold(pres: &Presentation, index: usize, id: u32) -> bool {
    let Some(slide) = pres.slides.get(index) else { return false };
    let Some(shape) = slide.shapes.iter().find(|s| s.id.0 == id) else { return false };
    let Some(ctx) = deckcraft_model::resolve::Ctx::for_slide(pres, slide) else { return false };
    let Some(body) = &shape.text else { return false };
    body.paragraphs
        .iter()
        .find_map(|p| p.runs.first().map(|r| deckcraft_model::resolve::run(&ctx, shape, p, &r.props).bold.unwrap_or(false)))
        .unwrap_or(false)
}

/// Slide `index`'s speaker notes made to read `text` (`\n` between paragraphs).
pub fn set_notes(pres: &mut Presentation, index: usize, text: &str) -> Result<(), DeckError> {
    let slide = Arc::make_mut(pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?);
    slide.notes = deckcraft_model::text::TextBody::from_text(text);
    Ok(())
}

/// The size (points) each run of object `id`'s text shows now, paragraph by paragraph: its own,
/// or what its placeholder, layout and master give it.
fn run_sizes(pres: &Presentation, index: usize, id: u32) -> Result<Vec<Vec<f64>>, DeckError> {
    let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
    let shape = slide.shapes.iter().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
    let ctx = deckcraft_model::resolve::Ctx::for_slide(pres, slide);
    Ok(shape
        .text
        .as_ref()
        .map(|body| {
            body.paragraphs
                .iter()
                .map(|p| {
                    p.runs
                        .iter()
                        .map(|r| {
                            ctx.as_ref().and_then(|c| deckcraft_model::resolve::run(c, shape, p, &r.props).size).or(r.props.size).unwrap_or(18.0)
                        })
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Every run of `shape`'s text set to its size in `sizes` times `factor`.
fn scale_text(shape: &mut deckcraft_model::Shape, sizes: &[Vec<f64>], factor: f64) {
    if let Some(body) = shape.text.as_mut() {
        for (pi, para) in body.paragraphs.iter_mut().enumerate() {
            for (ri, run) in para.runs.iter_mut().enumerate() {
                let now = sizes.get(pi).and_then(|r| r.get(ri)).copied().unwrap_or(18.0);
                run.props.size = Some(((now * factor) * 2.0).round().clamp(2.0, 800.0) / 2.0);
            }
        }
    }
}

/// Object `id` on slide `index` given the box (`x`, `y`, `w`, `h`) in points, keeping its turn:
/// a text box's text flows anew in it (its letters keep their size), a picture or shape is
/// stretched, a table's grid follows.
pub fn set_object_box(pres: &mut Presentation, index: usize, id: u32, x: f64, y: f64, w: f64, h: f64) -> Result<(), DeckError> {
    if !(w.is_finite() && h.is_finite() && x.is_finite() && y.is_finite()) || w < 1.0 || h < 1.0 {
        return Err(edit_err("bad box"));
    }
    let current = {
        let slide = pres.slides.get(index).ok_or(DeckError::NoSlide(index as u32))?;
        let shape = slide.shapes.iter().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
        drawn_box(deckcraft_model::resolve::Ctx::for_slide(pres, slide).as_ref(), shape)
    };
    let slide = Arc::make_mut(pres.slides.get_mut(index).ok_or(DeckError::NoSlide(index as u32))?);
    let shape = slide.shapes.iter_mut().find(|s| s.id.0 == id).ok_or_else(|| edit_err("no such object"))?;
    let mut h = h;
    if let ShapeKind::Table(t) = &mut shape.kind {
        // A table's columns take the new width; its rows keep their (minimum) heights and grow
        // to fit their text when drawn, so the height asked for (as drawn) is not imposed.
        let kx = w / current.w.max(1.0);
        t.cols.iter_mut().for_each(|c| *c *= kx);
        h = t.rows.iter().map(|r| r.height).sum::<f64>().max(1.0);
    }
    shape.xfrm = Some(deckcraft_model::Xfrm { x, y, w, h, ..current });
    Ok(())
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
