//! CAD viewing bridge (view only, by design): reads DXF and DWG with CADCraft, lists spaces and
//! layers, and renders any view — a centre, a scale and a size — to premultiplied RGBA, the
//! layout Android's ARGB_8888 `Bitmap` uses. The display list of a space is built once and reused
//! for every pan and zoom.

use std::collections::HashMap;

use cadcraft_color::{Color, Rgb, aci_rgb};
use cadcraft_doc::{Drawing, Space};
use cadcraft_geom::Vec2;
use cadcraft_render::DisplayList;
use cadcraft_render::raster::{RasterOptions, View};

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

pub struct CadDoc {
    drawing: Drawing,
    /// Space 0 is model space, then the paper-space layouts in tab order.
    spaces: Vec<(String, Space)>,
    lists: HashMap<u32, DisplayList>,
}

impl CadDoc {
    /// Opens DXF (ASCII or binary, R12–2018) or DWG (R13–2018) bytes; `name` is the file name.
    pub fn open(name: &str, bytes: &[u8]) -> Result<CadDoc, CadError> {
        let drawing = cadcraft_io::read(bytes, name).map_err(|e| CadError::Open(e.to_string()))?;
        let mut layouts: Vec<_> = drawing.layouts.iter().map(|l| (l.tab_order, l.name.clone())).collect();
        layouts.sort();
        let mut spaces = vec![("Model".to_string(), Space::Model)];
        spaces.extend(layouts.into_iter().map(|(_, n)| (n.clone(), Space::Paper(n))));
        Ok(CadDoc { drawing, spaces, lists: HashMap::new() })
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
        let b = self.list(space)?.bounds;
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
        let background = if dark { Rgb(33, 40, 48) } else { Rgb(255, 255, 255) };
        let opts = RasterOptions { background, ..RasterOptions::default() };
        let pm = cadcraft_render::raster::render(self.list(space)?, &view, &opts).ok_or(CadError::BadView(width, height))?;
        Ok(Raster { width: pm.width(), height: pm.height(), pixels: pm.data().to_vec() })
    }

    fn list(&mut self, space: u32) -> Result<&DisplayList, CadError> {
        let (_, sp) = self.spaces.get(space as usize).ok_or(CadError::NoSpace(space))?;
        if !self.lists.contains_key(&space) {
            // Curve detail: about 1/4000 of the drawing's size, fine at any phone zoom.
            let e = self.drawing.extents(sp);
            let size = if e.is_empty() { 1.0 } else { e.width().max(e.height()) };
            let opts = cadcraft_render::Options { tolerance: (size / 4000.0).max(1e-6), transparency: true, ..Default::default() };
            let list = cadcraft_render::build(&self.drawing, sp, &opts);
            self.lists.insert(space, list);
        }
        Ok(&self.lists[&space])
    }
}

#[cfg(test)]
mod tests;
