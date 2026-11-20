//! Pictures under black-out marks keep everything outside the marks.
//!
//! PdfCraft's redaction clears the covered pixels of the images it can decode and write back
//! (Flate, LZW, CCITT, JBIG2 without masks), but removes every other image a mark touches
//! *whole*: a JPEG (`DCTDecode`, how almost every scanned page and phone photo is stored), JPEG
//! 2000, and any image with a soft mask. On a scanned page, one picture of the whole page, a small
//! mark therefore blanked the page.
//!
//! Here the engine still decides what a mark covers (it walks the content, forms and matrices),
//! but on stand-ins: before it runs, every image the marked pages draw is swapped, in place, for a
//! white grey picture of the same proportions that it can always clear. Afterwards the originals
//! are put back, so an image no mark touched is the very same object, bytes and all, and every
//! cleared stand-in tells which pixels of its original a mark covers. Those pixels of the original
//! are painted black (its soft mask made opaque there), and the result is stored in the cleared
//! copy the engine made, a JPEG again for a JPEG. An image the engine removed because a mark
//! covers it entirely stays removed: nothing of it was outside the mark.
//!
//! An image whose pixels can't be read here (JPEG 2000, a CMYK JPEG…) and that a mark covers only
//! in part refuses the whole redaction, before anything changes: removing it would lose what lies
//! outside the mark, keeping it would keep what lies under it.
//!
//! One image is decoded at a time; the stand-ins are a few kilobytes each.

use std::collections::HashSet;
use std::sync::Arc;

use pdfcraft_cos::{Dict, Document, ImageBudget, ObjRef, Object, Stream};

use crate::PdfError;

/// The key marking a stand-in, and the cleared copies the engine makes of it, with the index of
/// the image it stands in for. Never left in a saved file.
const PROBE: &[u8] = b"MobileOfficeRedactProbe";

/// Most pixels a stand-in has: larger images get a coarser one (each of its pixels then covers a
/// few of the image's, and a pixel touching a mark is blackened whole).
const PROBE_PIXELS: u64 = 16 << 20;

/// Quality of a JPEG written back after blackening (an untouched image is never written again).
const JPEG_QUALITY: u8 = 92;

/// Form XObjects followed below a page.
const MAX_FORM_DEPTH: usize = 16;
/// XObjects looked at on all the marked pages together.
const MAX_XOBJECTS: usize = 20_000;

struct Probe {
    at: ObjRef,
    original: Arc<Object>,
    /// The stand-in's size.
    pw: usize,
    ph: usize,
}

/// What a blackened pixel becomes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    /// The picture itself: black.
    Picture,
    /// A soft mask: opaque (the black shows, as the box drawn over it does).
    SoftMask,
    /// An explicit stencil mask (`/Mask` stream) or a stencil image: paints nothing.
    Stencil,
}

/// Applies every black-out mark of `doc` as [`pdfcraft_redact::apply`] does, keeping what lies
/// outside the marks of the pictures they touch. Returns how many marks were applied. On an error
/// `doc` may be half changed: callers work on a copy.
pub(crate) fn apply(doc: &mut Document) -> Result<usize, PdfError> {
    let probes = stand_in(doc)?;
    let report = pdfcraft_redact::apply(doc, None).map_err(crate::edit);
    // The originals go back whatever happened, so no stand-in can be saved.
    for p in &probes {
        doc.set(p.at, (*p.original).clone());
    }
    let report = report?;
    let copies: Vec<(ObjRef, usize)> = doc
        .scan_objects_subset(doc.modified_objects())
        .filter_map(|(r, o)| match &*o.ok()? {
            Object::Stream(s) => Some((r, usize::try_from(s.dict.int(PROBE)?).ok()?)),
            _ => None,
        })
        .collect();
    let mut budget = ImageBudget::new(doc);
    let mut refused = 0usize;
    for (copy, index) in copies {
        let Some(probe) = probes.get(index) else { continue };
        let covered = match &*doc.get(copy) {
            Object::Stream(s) => s.decoded_within(probe.pw * probe.ph + 4096).ok().filter(|d| d.len() >= probe.pw * probe.ph),
            _ => None,
        };
        let Some(covered) = covered else {
            refused += 1;
            continue;
        };
        let cover = Cover { mask: &covered, pw: probe.pw, ph: probe.ph };
        match blackened_picture(doc, &probe.original, &cover, &mut budget) {
            Some(new) => doc.set(copy, Object::Stream(new)),
            None => refused += 1,
        }
    }
    if refused > 0 {
        return Err(PdfError::Unredactable(refused as u32));
    }
    Ok(report.marks)
}

/// Swaps every image the marked pages draw for a stand-in; the probes, by index.
fn stand_in(doc: &mut Document) -> Result<Vec<Probe>, PdfError> {
    let marked: HashSet<usize> = pdfcraft_redact::marks(doc).iter().map(|m| m.page).collect();
    let pages = pdfcraft_organize::pages(doc).map_err(crate::edit)?;
    let mut images = Vec::new();
    let mut seen = HashSet::new();
    for (i, page) in pages.iter().enumerate() {
        if marked.contains(&i)
            && let Some(res) = page_resources(doc, page.obj)
        {
            collect_images(doc, &res, 0, &mut seen, &mut images);
        }
    }
    let mut probes = Vec::new();
    for at in images {
        let original = doc.get(at);
        let Object::Stream(s) = &*original else { continue };
        if !wanted(doc, s) {
            continue;
        }
        let (Some(w), Some(h)) = (dim(doc, &s.dict, b"Width"), dim(doc, &s.dict, b"Height")) else { continue };
        let (pw, ph) = probe_size(w, h);
        let mut dict = Dict::new();
        dict.set(b"Type".to_vec(), Object::name("XObject"));
        dict.set(b"Subtype".to_vec(), Object::name("Image"));
        dict.set(b"Width".to_vec(), Object::Int(pw as i64));
        dict.set(b"Height".to_vec(), Object::Int(ph as i64));
        dict.set(b"ColorSpace".to_vec(), Object::name("DeviceGray"));
        dict.set(b"BitsPerComponent".to_vec(), Object::Int(8));
        // A hidden layer hides the stand-in as it hid the image.
        if let Some(oc) = s.dict.get(b"OC") {
            dict.set(b"OC".to_vec(), oc.clone());
        }
        dict.set(PROBE.to_vec(), Object::Int(probes.len() as i64));
        let stand_in = Stream::flate(dict, &vec![0xFF; pw * ph]);
        doc.set(at, Object::Stream(stand_in));
        probes.push(Probe { at, original: original.clone(), pw, ph });
    }
    Ok(probes)
}

/// Whether the image is redacted through a stand-in. Those sharing JBIG2 globals are left to the
/// engine, which stores the other images sharing them again without them (the globals may hold
/// shapes from under the mark).
fn wanted(doc: &Document, s: &Stream) -> bool {
    if s.dict.name(b"Subtype") != Some(b"Image".as_slice()) {
        return false;
    }
    let parms = s.dict.get(b"DecodeParms").map(|p| doc.resolve(p));
    let globals = |o: &Object| doc.resolve(o).as_dict().is_some_and(|d| d.contains(b"JBIG2Globals"));
    !match parms.as_deref() {
        Some(Object::Array(a)) => a.iter().any(globals),
        Some(o) => globals(o),
        None => false,
    }
}

fn dim(doc: &Document, d: &Dict, key: &[u8]) -> Option<usize> {
    let v = doc.resolve(d.get(key)?).as_int()?;
    usize::try_from(v).ok().filter(|v| (1..=1 << 20).contains(v))
}

/// The stand-in's size for a `w` × `h` image: the same, or scaled down to [`PROBE_PIXELS`].
fn probe_size(w: usize, h: usize) -> (usize, usize) {
    let px = (w as u64) * (h as u64);
    if px <= PROBE_PIXELS {
        return (w, h);
    }
    let k = (PROBE_PIXELS as f64 / px as f64).sqrt();
    (((w as f64 * k) as usize).clamp(1, w), ((h as f64 * k) as usize).clamp(1, h))
}

/// A page's resources, inherited ones included.
fn page_resources(doc: &Document, page: ObjRef) -> Option<Dict> {
    let mut node = page;
    for _ in 0..64 {
        let d = doc.get(node).as_dict()?.clone();
        if let Some(r) = d.get(b"Resources") {
            return doc.resolve(r).as_dict().cloned();
        }
        node = d.reference(b"Parent")?;
    }
    None
}

/// Every image `res` lists, and those of the forms it lists (a form without resources of its own
/// uses these).
fn collect_images(doc: &Document, res: &Dict, depth: usize, seen: &mut HashSet<ObjRef>, out: &mut Vec<ObjRef>) {
    let Some(xobjects) = res.get(b"XObject").and_then(|x| doc.resolve(x).as_dict().cloned()) else { return };
    for (_, v) in xobjects.iter() {
        let Some(r) = v.as_ref() else { continue };
        if seen.len() >= MAX_XOBJECTS || !seen.insert(r) {
            continue;
        }
        let obj = doc.get(r);
        let Object::Stream(s) = &*obj else { continue };
        match s.dict.name(b"Subtype") {
            Some(b"Image") => out.push(r),
            Some(b"Form") if depth < MAX_FORM_DEPTH => {
                let own = s.dict.get(b"Resources").and_then(|r| doc.resolve(r).as_dict().cloned());
                collect_images(doc, own.as_ref().unwrap_or(res), depth + 1, seen, out);
            }
            _ => {}
        }
    }
}

/// Which pixels of a stand-in a mark covers: its cleared samples are 0.
struct Cover<'a> {
    mask: &'a [u8],
    pw: usize,
    ph: usize,
}

impl Cover<'_> {
    /// Calls `f` with each row of a `w` × `h` grid laid over the same picture that a mark
    /// touches, and which of its pixels it touches: those overlapping a covered stand-in pixel.
    fn rows(&self, w: usize, h: usize, mut f: impl FnMut(usize, &[bool])) {
        let (pw, ph) = (self.pw as u64, self.ph as u64);
        let span = |i: usize, n: usize, p: u64| {
            let (i, n) = (i as u64, n as u64);
            let a = i * p / n;
            let b = ((i + 1) * p).div_ceil(n).saturating_sub(1).max(a);
            (a as usize, (b as usize).min(p as usize - 1))
        };
        let cols: Vec<(usize, usize)> = (0..w).map(|i| span(i, w, pw)).collect();
        let mut any = vec![false; self.pw];
        let mut prefix = vec![0u32; self.pw + 1];
        let mut row = vec![false; w];
        for j in 0..h {
            let (r0, r1) = span(j, h, ph);
            any.fill(false);
            let mut hit = false;
            for r in r0..=r1 {
                for (a, &m) in any.iter_mut().zip(&self.mask[r * self.pw..(r + 1) * self.pw]) {
                    if m == 0 {
                        *a = true;
                        hit = true;
                    }
                }
            }
            if !hit {
                continue;
            }
            for (k, &a) in any.iter().enumerate() {
                prefix[k + 1] = prefix[k] + u32::from(a);
            }
            for (cell, &(c0, c1)) in row.iter_mut().zip(&cols) {
                *cell = prefix[c1 + 1] > prefix[c0];
            }
            f(j, &row);
        }
    }
}

/// `original` with the covered pixels blackened (its soft mask or stencil mask treated alike),
/// as a stream to store in the engine's cleared copy; `None` if a part can't be read.
fn blackened_picture(doc: &mut Document, original: &Object, cover: &Cover<'_>, budget: &mut ImageBudget) -> Option<Stream> {
    let Object::Stream(s) = original else { return None };
    let stencil = matches!(s.dict.get(b"ImageMask").map(|v| doc.resolve(v)).as_deref(), Some(Object::Bool(true)));
    let mut new = blackened(doc, s, cover, if stencil { Role::Stencil } else { Role::Picture }, budget)?;
    for (key, role) in [(b"SMask".as_slice(), Role::SoftMask), (b"Mask", Role::Stencil)] {
        let Some(r) = s.dict.get(key).and_then(Object::as_ref) else { continue };
        let mask = doc.get(r);
        let Object::Stream(m) = &*mask else { continue };
        let cleared = blackened(doc, m, cover, role, budget)?;
        let nr = doc.add(Object::Stream(cleared));
        new.dict.set(key.to_vec(), Object::Ref(nr));
    }
    Some(new)
}

/// The names of `s`'s filters.
fn filter_names(s: &Stream) -> Vec<Vec<u8>> {
    match s.dict.get(b"Filter") {
        Some(Object::Name(n)) => vec![n.clone()],
        Some(Object::Array(a)) => a.iter().filter_map(Object::as_name).map(<[u8]>::to_vec).collect(),
        _ => Vec::new(),
    }
}

/// The dictionary of a blackened copy of `s`: what could still hold the old pixels (alternate
/// images, data in another file) goes.
fn cleaned(s: &Stream) -> Stream {
    let mut dict = s.dict.clone();
    for key in [b"Alternates".as_slice(), b"F", b"FFilter", b"FDecodeParms", b"DP", b"Length", b"DL", PROBE] {
        dict.remove(key);
    }
    Stream { dict, raw: s.raw.clone() }
}

/// One image stream (a picture or a mask) with the covered pixels set for `role`.
fn blackened(doc: &Document, s: &Stream, cover: &Cover<'_>, role: Role, budget: &mut ImageBudget) -> Option<Stream> {
    let names = filter_names(s);
    if names.iter().any(|n| matches!(n.as_slice(), b"DCTDecode" | b"DCT")) {
        return blackened_jpeg(doc, s, cover, role, budget);
    }
    if names.iter().any(|n| n.as_slice() == b"JPXDecode") {
        return None;
    }
    blackened_samples(doc, s, cover, role, budget)
}

/// A JPEG, grey or RGB, decoded, blackened and stored as a JPEG again.
fn blackened_jpeg(doc: &Document, s: &Stream, cover: &Cover<'_>, role: Role, budget: &mut ImageBudget) -> Option<Stream> {
    let mut samples = pdfcraft_optimize::decode_samples(doc, s, budget).ok()?;
    let (w, h, n) = (samples.width as usize, samples.height as usize, samples.components);
    let value = if role == Role::Picture { 0 } else { 255 };
    cover.rows(w, h, |j, row| {
        for (i, _) in row.iter().enumerate().filter(|(_, c)| **c) {
            let at = (j * w + i) * n;
            samples.data[at..at + n].fill(value);
        }
    });
    pdfcraft_optimize::encode_samples(&cleaned(s), &samples, JPEG_QUALITY)
}

/// Components of colour space `cs`, and whether its zero is "no ink" (so black is all ones).
fn colour_space(doc: &Document, cs: &Object) -> Option<(usize, bool)> {
    let cs = doc.resolve(cs);
    let (name, array) = match &*cs {
        Object::Name(n) => (n.clone(), None),
        Object::Array(a) => (a.first().map(|f| doc.resolve(f)).and_then(|f| f.as_name().map(<[u8]>::to_vec))?, Some(a.clone())),
        _ => return None,
    };
    Some(match name.as_slice() {
        b"DeviceGray" | b"G" | b"CalGray" | b"Indexed" | b"I" => (1, false),
        b"Separation" => (1, true),
        b"DeviceRGB" | b"RGB" | b"CalRGB" | b"Lab" => (3, false),
        b"DeviceCMYK" | b"CMYK" => (4, true),
        b"DeviceN" => (doc.resolve(array?.get(1)?).as_array()?.len(), true),
        b"ICCBased" => {
            let profile = doc.resolve(array?.get(1)?);
            let Object::Stream(p) = &*profile else { return None };
            let n = usize::try_from(doc.resolve(p.dict.get(b"N")?).as_int()?).ok().filter(|n| matches!(n, 1 | 3 | 4))?;
            (n, n == 4)
        }
        _ => return None,
    })
}

/// Any other image (Flate, LZW, CCITT fax, JBIG2…): its samples decoded as stored, the covered
/// pixels' bits set, and stored losslessly (Group 4 for a one-bit image when that is smaller).
fn blackened_samples(doc: &Document, s: &Stream, cover: &Cover<'_>, role: Role, budget: &mut ImageBudget) -> Option<Stream> {
    let d = &s.dict;
    let (w, h) = (dim(doc, d, b"Width")?, dim(doc, d, b"Height")?);
    if (w as u64) * (h as u64) > pdfcraft_cos::MAX_REDACTED_IMAGE_PIXELS {
        return None;
    }
    // Group 3 coded on byte boundaries is drawn by the renderer as other rows than it decodes
    // to here (the engine refuses it for the same reason).
    if doc.fax_params(s).is_some_and(|p| p.k >= 0 && p.encoded_byte_align) {
        return None;
    }
    let stencil = role == Role::Stencil;
    let (ncomp, bpc, subtractive) = if stencil {
        (1, 1, false)
    } else {
        let (n, sub) = colour_space(doc, d.get(b"ColorSpace")?)?;
        (n, usize::try_from(doc.resolve(d.get(b"BitsPerComponent")?).as_int()?).ok()?, sub)
    };
    if !matches!(bpc, 1 | 2 | 4 | 8 | 16) {
        return None;
    }
    let bits = ncomp * bpc;
    let row_bytes = (w * bits).div_ceil(8);
    let bytes = row_bytes.checked_mul(h)?;
    let mut data = budget.decode(doc, s, bytes.checked_add(h)?.checked_mul(3)?.checked_add(4096)?).ok()?;
    if data.len() < bytes {
        return None;
    }
    data.truncate(bytes);
    // A decode array starting [1 0] swaps what 0 and all ones mean.
    let inverted = d.get(b"Decode").map(|a| doc.resolve(a)).and_then(|a| {
        let a = a.as_array()?.clone();
        Some(doc.resolve(a.first()?).as_f64()? > doc.resolve(a.get(1)?).as_f64()?)
    }) == Some(true);
    let ones = match role {
        Role::Picture => subtractive,
        Role::SoftMask => true,
        // A stencil sample of 0 paints (ISO 32000-2 §8.9.6.2).
        Role::Stencil => true,
    } != inverted;
    cover.rows(w, h, |j, row| {
        for (i, _) in row.iter().enumerate().filter(|(_, c)| **c) {
            let start = j * row_bytes * 8 + i * bits;
            for bit in start..start + bits {
                let (byte, mask) = (bit / 8, 0x80u8 >> (bit % 8));
                if ones {
                    data[byte] |= mask;
                } else {
                    data[byte] &= !mask;
                }
            }
        }
    });
    let mut dict = cleaned(s).dict;
    dict.remove(b"DecodeParms");
    let flate = Stream::flate(dict.clone(), &data);
    if bits == 1
        && let (Ok(columns), Ok(rows)) = (u32::try_from(w), u32::try_from(h))
        && let Ok(g4) = pdfcraft_filters::encode_g4(&data, columns, rows)
        && g4.len() < flate.raw.len()
    {
        let mut parms = Dict::new();
        parms.set(b"K".to_vec(), Object::Int(-1));
        parms.set(b"Columns".to_vec(), Object::Int(i64::from(columns)));
        parms.set(b"Rows".to_vec(), Object::Int(i64::from(rows)));
        dict.set(b"Filter".to_vec(), Object::name("CCITTFaxDecode"));
        dict.set(b"DecodeParms".to_vec(), Object::Dict(parms));
        return Some(Stream::from_raw(dict, g4));
    }
    Some(flate)
}

#[cfg(test)]
mod tests;
