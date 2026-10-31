//! New PDFs from pictures: one page per picture, either the picture's own size or an A4 page
//! (turned to match the picture) with the picture fitted inside a margin.

use pdfcraft_cos::{Dict, Object, PdfString, SaveOptions, Stream};

use crate::PdfError;

const A4: (f64, f64) = (595.28, 841.89);
/// About 1 cm around a picture on an A4 page.
const MARGIN: f64 = 28.35;

fn err<E: std::fmt::Display>(e: E) -> PdfError {
    PdfError::Edit(e.to_string())
}

/// A PDF of `images` (file name, bytes: JPEG, PNG, TIFF, GIF, BMP or JPEG 2000), in order.
/// Photos are turned upright from their camera orientation.
pub fn images_to_pdf(images: &[(String, Vec<u8>)], fit_a4: bool) -> Result<Vec<u8>, PdfError> {
    if images.is_empty() {
        return Err(PdfError::Edit("no pictures".into()));
    }
    let mut doc = if fit_a4 {
        let mut doc = pdfcraft_create::blank(A4.0, A4.1, images.len()).map_err(err)?;
        let pages = pdfcraft_annot::page_refs(&doc).map_err(err)?;
        for ((name, bytes), page) in images.iter().zip(pages) {
            let (image, (w, h)) = pdfcraft_create::image_xobject(&mut doc, name, bytes).map_err(err)?;
            let (pw, ph) = if w > h { (A4.1, A4.0) } else { A4 };
            let k = ((pw - 2.0 * MARGIN) / w.max(1e-3)).min((ph - 2.0 * MARGIN) / h.max(1e-3));
            let (dw, dh) = (w * k, h * k);
            let (x, y) = ((pw - dw) / 2.0, (ph - dh) / 2.0);
            let content =
                doc.add(Object::Stream(Stream::flate(Dict::new(), format!("q {dw:.3} 0 0 {dh:.3} {x:.3} {y:.3} cm /Im0 Do Q\n").as_bytes())));
            let mut xobjects = Dict::new();
            xobjects.set(b"Im0".to_vec(), Object::Ref(image));
            let mut resources = Dict::new();
            resources.set(b"XObject".to_vec(), Object::Dict(xobjects));
            doc.update_dict(page, |d| {
                d.set(b"MediaBox".to_vec(), Object::Array([0.0, 0.0, pw, ph].into_iter().map(Object::Real).collect()));
                d.set(b"Resources".to_vec(), Object::Dict(resources));
                d.set(b"Contents".to_vec(), Object::Ref(content));
            })
            .map_err(err)?;
        }
        doc
    } else {
        pdfcraft_create::from_images(images).map_err(err)?
    };
    // The document information names this app, not the engine (AGENTS.md: no ArtCraft names).
    let title = images[0].0.rsplit_once('.').map_or(images[0].0.as_str(), |(stem, _)| stem).to_string();
    let mut info = Dict::new();
    info.set(b"Title".to_vec(), PdfString::text(&title));
    info.set(b"Producer".to_vec(), PdfString::text("Mobile Office Free"));
    let info = doc.add(Object::Dict(info));
    doc.trailer_mut().set(b"Info".to_vec(), Object::Ref(info));
    pdfcraft_cos::write_full(&doc, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
}
