//! CAD viewing bridge (view only, by design): reads DXF and DWG with CADCraft, lists spaces and
//! layers, and renders any view — a centre, a scale and a size — to premultiplied RGBA, the
//! layout Android's ARGB_8888 `Bitmap` uses.
//!
//! Drawing a view is kept proportional to what it shows, not to the whole drawing:
//! - each space has one *overview* display list, built once with the level of detail the engine's
//!   own canvas uses for on-screen builds (short dashes and hatch patterns that would run together
//!   are drawn continuous or solid), serving every zoom up to about 6000 pixels across the space;
//! - closer in, model space draws from a *detail* list of only the objects around the view (the
//!   engine's spatial index finds them), at half-pixel curve detail, rebuilt when the view leaves
//!   it or the zoom changes by 2×;
//! - each frame keeps only the primitives the view shows, drops points closer than half a pixel,
//!   and draws primitives smaller than a pixel as one dot per pixel ([`frame_list`]).

use std::collections::HashMap;

use cadcraft_color::{Color, Rgb, aci_rgb};
use cadcraft_doc::{Drawing, Space};
use cadcraft_geom::{Bounds2, Vec2};
use cadcraft_render::raster::{RasterOptions, View};
use cadcraft_render::{DisplayList, Kind};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CadError {
    #[error("{0}")]
    Open(String),
    #[error("space {0} does not exist")]
    NoSpace(u32),
    #[error("layer {0} does not exist")]
    NoLayer(String),
    #[error("cannot draw a {0}×{1} view")]
    BadView(u32, u32),
}

/// A view picture: `width`×`height` premultiplied RGBA8, row by row.
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// A layer as the layer panel shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerInfo {
    pub name: String,
    pub visible: bool,
    /// The layer's colour as RGB.
    pub color: (u8, u8, u8),
}

/// World-space rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extents {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// A point an object snap found.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapPoint {
    pub x: f64,
    pub y: f64,
    /// "Endpoint", "Midpoint", "Center", "Intersection", "Quadrant", "Node" or "Insertion".
    pub kind: String,
}

/// The overview serves zooms up to this many pixels across the space's larger side; closer in,
/// model space draws from detail lists.
const OVERVIEW_PIXELS: f64 = 6000.0;

/// What a space is drawn from.
struct SpaceLists {
    /// The whole space, built once (until a layer is shown or hidden).
    overview: DisplayList,
    /// Pixels per drawing unit up to which the overview has enough detail.
    overview_max_scale: f64,
    /// Model space close up: the objects around the last view, for that zoom band.
    detail: Option<Detail>,
}

struct Detail {
    /// The area the list covers (the view it was built for, with a quarter of its larger side all round).
    window: Bounds2,
    /// ⌊log2(drawing units per pixel)⌋ the list was built for.
    band: i32,
    list: DisplayList,
}

pub struct CadDoc {
    drawing: Drawing,
    /// Space 0 is model space, then the paper-space layouts in tab order.
    spaces: Vec<(String, Space)>,
    lists: HashMap<u32, SpaceLists>,
    /// The primitives of the frame being drawn, reused from frame to frame.
    frame: DisplayList,
    /// Screen pixels already holding a sub-pixel primitive's dot (one bit each), reused.
    dots: Vec<u64>,
}

impl Drop for CadDoc {
    fn drop(&mut self) {
        // The engine caches the spatial index object snaps and detail lists use (keyed by the
        // drawing's entity store); without this it would outlive the document.
        cadcraft_engine::spatial::clear_cache();
    }
}

impl CadDoc {
    /// Opens DXF (ASCII or binary, R12–2018) or DWG (R13–2018) bytes; `name` is the file name.
    pub fn open(name: &str, bytes: &[u8]) -> Result<CadDoc, CadError> {
        let drawing = cadcraft_io::read(bytes, name).map_err(|e| CadError::Open(e.to_string()))?;
        let mut layouts: Vec<_> = drawing.layouts.iter().map(|l| (l.tab_order, l.name.clone())).collect();
        layouts.sort();
        let mut spaces = vec![("Model".to_string(), Space::Model)];
        // Paper layouts with nothing drawn on them (a new drawing's "Layout1", "Layout2") are left
        // out: a tab that opens on a blank sheet only looks broken.
        spaces.extend(layouts.into_iter().map(|(_, n)| (n.clone(), Space::Paper(n))).filter(|(_, sp)| !drawing.extents(sp).is_empty()));
        Ok(CadDoc { drawing, spaces, lists: HashMap::new(), frame: DisplayList::default(), dots: Vec::new() })
    }

    /// Notes from reading the file (e.g. what a DWG conversion could not keep).
    pub fn notes(&self) -> Vec<String> {
        self.drawing.read_notes.clone()
    }

    pub fn space_names(&self) -> Vec<String> {
        self.spaces.iter().map(|(n, _)| n.clone()).collect()
    }

    pub fn layers(&self) -> Vec<LayerInfo> {
        self.drawing
            .layers
            .iter()
            .map(|l| {
                let Rgb(r, g, b) = match l.color {
                    Color::True(c) => c,
                    Color::Index(i) => aci_rgb(i),
                    _ => aci_rgb(7),
                };
                LayerInfo { name: l.name.clone(), visible: l.on && !l.frozen, color: (r, g, b) }
            })
            .collect()
    }

    /// Shows or hides a layer (the file is never written: this is a viewer).
    pub fn set_layer_visible(&mut self, name: &str, visible: bool) -> Result<(), CadError> {
        let layer = self.drawing.layers.iter_mut().find(|l| l.name == name).ok_or_else(|| CadError::NoLayer(name.to_string()))?;
        layer.on = visible;
        if visible {
            layer.frozen = false;
        }
        self.lists.clear();
        Ok(())
    }

    /// The area the space's objects cover.
    pub fn extents(&mut self, space: u32) -> Result<Extents, CadError> {
        let b = self.lists(space)?.overview.bounds;
        if b.is_empty() {
            return Ok(Extents { min_x: 0.0, min_y: 0.0, max_x: 1.0, max_y: 1.0 });
        }
        Ok(Extents { min_x: b.min.x, min_y: b.min.y, max_x: b.max.x, max_y: b.max.y })
    }

    /// Draws `space` into a `width`×`height` picture whose centre is world point
    /// (`center_x`, `center_y`) at `scale` pixels per drawing unit, on a dark or a white sheet.
    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, space: u32, center_x: f64, center_y: f64, scale: f64, width: u32, height: u32, dark: bool) -> Result<Raster, CadError> {
        let view = View { center: Vec2::new(center_x, center_y), scale, width, height };
        self.lists(space)?;
        if width == 0 || height == 0 || width > 16384 || height > 16384 || !(scale.is_finite() && scale > 0.0) || !view.center.is_finite() {
            return Err(CadError::BadView(width, height));
        }
        let background = if dark { Rgb(33, 40, 48) } else { Rgb(255, 255, 255) };
        let opts = RasterOptions { background, ..RasterOptions::default() };
        let CadDoc { drawing, lists, frame, dots, .. } = self;
        let sl = lists.get_mut(&space).ok_or(CadError::NoSpace(space))?;
        let list = if space == 0 && scale > sl.overview_max_scale {
            let vis = view.world_bounds();
            let band = band(scale);
            if !sl.detail.as_ref().is_some_and(|d| d.band == band && d.window.contains_box(&vis)) {
                // Free the old list before building its successor.
                sl.detail = None;
                sl.detail = Some(detail(drawing, &vis, band));
            }
            sl.detail.as_ref().map_or(&sl.overview, |d| &d.list)
        } else {
            sl.detail = None;
            &sl.overview
        };
        frame_list(list, &view, frame, dots);
        let pm = cadcraft_render::raster::render(frame, &view, &opts).ok_or(CadError::BadView(width, height))?;
        Ok(Raster { width: pm.width(), height: pm.height(), pixels: pm.take() })
    }

    /// The object snap nearest world point (`x`, `y`) within `aperture` drawing units: an
    /// endpoint, midpoint, centre, intersection, quadrant, node or insertion point, as CAD
    /// programs snap to them. Its point and what it is ("Endpoint", "Midpoint", …).
    pub fn snap(&self, space: u32, x: f64, y: f64, aperture: f64) -> Result<Option<SnapPoint>, CadError> {
        use cadcraft_engine::snap::mode;
        let (_, sp) = self.spaces.get(space as usize).ok_or(CadError::NoSpace(space))?;
        let modes = mode::END | mode::MID | mode::CEN | mode::INT | mode::QUA | mode::NOD | mode::INS;
        Ok(cadcraft_engine::snap::osnap(&self.drawing, sp, Vec2::new(x, y), aperture, modes, None, false).map(|h| SnapPoint {
            x: h.point.x,
            y: h.point.y,
            kind: h.name.to_string(),
        }))
    }

    fn lists(&mut self, space: u32) -> Result<&mut SpaceLists, CadError> {
        let (_, sp) = self.spaces.get(space as usize).ok_or(CadError::NoSpace(space))?;
        let drawing = &self.drawing;
        Ok(self.lists.entry(space).or_insert_with(|| {
            let e = drawing.extents(sp);
            let size = if e.is_empty() { 1.0 } else { e.width().max(e.height()).max(1e-9) };
            // Detail for a view OVERVIEW_PIXELS across the space: curves within a pixel, dashes and
            // hatch pattern lines under two pixels drawn continuous or solid (the engine's
            // on-screen level of detail; without it every dash and pattern line of the whole
            // drawing would be stored).
            let max_scale = OVERVIEW_PIXELS / size;
            let opts = cadcraft_render::Options { tolerance: 1.0 / max_scale, min_dash: 2.0 / max_scale, transparency: true, ..Default::default() };
            SpaceLists { overview: cadcraft_render::build(drawing, sp, &opts), overview_max_scale: max_scale, detail: None }
        }))
    }
}

/// ⌊log2(drawing units per pixel)⌋ at `scale` pixels per unit: zooms within a band share a list.
fn band(scale: f64) -> i32 {
    (1.0 / scale).log2().floor().clamp(-1000.0, 1000.0) as i32
}

/// The model-space objects around view `vis` drawn for zoom band `band`: half a pixel of curve
/// detail, two-pixel dashes and hatch lines only inside the window (as the engine's canvas builds
/// on screen). The engine's spatial index picks the objects; a drawing too small for one is
/// built whole.
fn detail(d: &Drawing, vis: &Bounds2, band: i32) -> Detail {
    let window = vis.expand(vis.width().max(vis.height()) * 0.25);
    let px = 2f64.powi(band);
    let opts = cadcraft_render::Options { tolerance: px * 0.5, min_dash: px * 2.0, transparency: true, view: Some(window), ..Default::default() };
    let index = cadcraft_engine::spatial::index(d, &Space::Model);
    let near = index.as_ref().and_then(|ix| ix.query(&window, true));
    let list = match (&index, near) {
        (Some(ix), Some(near)) => cadcraft_render::build_entities(d, near.iter().filter_map(|&i| ix.entity(i)).map(|e| &**e), &opts),
        _ => cadcraft_render::build_entities(d, d.model.iter().map(|e| &**e), &opts),
    };
    Detail { window, band, list }
}

/// Fills `out` with what `view` shows of `list`, ready for the rasteriser: primitives off screen
/// are left out, polyline points closer than half a pixel to the last one kept are dropped, and
/// a primitive smaller than a pixel becomes a one-pixel dot, at most one per screen pixel (the
/// first one drawn there). At an overview zoom most of a large drawing is such dots: drawing them
/// all took most of a frame's time, for the same picture.
fn frame_list(list: &DisplayList, view: &View, out: &mut DisplayList, dots: &mut Vec<u64>) {
    out.prims.clear();
    out.verts.clear();
    out.tris.clear();
    out.bounds = list.bounds;
    let (w, h) = (view.width as usize, view.height as usize);
    dots.clear();
    dots.resize((w * h).div_ceil(64), 0);
    let s = view.scale;
    let px = 1.0 / s;
    // Margin: a hairline is drawn about a pixel wide around its path.
    let vis = view.world_bounds().expand(2.0 * px);
    let half_px2 = 0.25 * px * px;
    let (left, top) = (view.center.x - w as f64 / 2.0 * px, view.center.y + h as f64 / 2.0 * px);
    for p in &list.prims {
        let pts = list.points(p);
        if pts.is_empty() {
            continue;
        }
        if matches!(p.kind, Kind::Infinite { .. }) {
            push(out, p, pts);
            continue;
        }
        let b = Bounds2::from_points(pts.iter().copied());
        if !b.intersects(&vis) {
            continue;
        }
        if p.kind != Kind::Mask && b.width() < px && b.height() < px {
            let c = b.center();
            let (x, y) = ((c.x - left) * s, (top - c.y) * s);
            if !(x >= 0.0 && y >= 0.0 && x < w as f64 && y < h as f64) {
                continue;
            }
            let bit = y as usize * w + x as usize;
            let (word, mask) = (bit / 64, 1u64 << (bit % 64));
            if dots[word] & mask != 0 {
                continue;
            }
            dots[word] |= mask;
            let mut q = *p;
            q.kind = Kind::Polyline;
            push(out, &q, &[Vec2::new(c.x - 0.5 * px, c.y), Vec2::new(c.x + 0.5 * px, c.y)]);
            continue;
        }
        match p.kind {
            Kind::Polyline if pts.len() > 2 => {
                let start = out.verts.len();
                let mut last = pts[0];
                out.verts.push(last);
                for &q in &pts[1..pts.len() - 1] {
                    if (q - last).len2() >= half_px2 {
                        out.verts.push(q);
                        last = q;
                    }
                }
                out.verts.push(pts[pts.len() - 1]);
                let mut q = *p;
                q.start = start as u32;
                q.len = (out.verts.len() - start) as u32;
                out.prims.push(q);
            }
            _ => push(out, p, pts),
        }
    }
}

/// Appends primitive `p` with points `pts` to `out`.
fn push(out: &mut DisplayList, p: &cadcraft_render::DPrim, pts: &[Vec2]) {
    let mut q = *p;
    let dst = if matches!(p.kind, Kind::Tris | Kind::Mask) { &mut out.tris } else { &mut out.verts };
    q.start = dst.len() as u32;
    q.len = pts.len() as u32;
    dst.extend_from_slice(pts);
    out.prims.push(q);
}

#[cfg(test)]
mod tests;
