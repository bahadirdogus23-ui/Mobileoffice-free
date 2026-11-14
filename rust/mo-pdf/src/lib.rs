//! PDF page tools over the PdfCraft engine: rotate, delete, move and extract pages, combine
//! files, put a hand-drawn signature, pen and highlighter strokes or notes on a page, add or remove password protection, and make a
//! smaller copy, and fill in forms. Viewing stays with Android's PdfRenderer (docs/adr/0003-pdf-goruntuleme.md); this bridge
//! edits the document's structure and writes a whole new file, never the opened one in place.

use std::sync::Arc;

use pdfcraft_annot as annot;
use pdfcraft_cos::{CosError, Document, SaveOptions};
use pdfcraft_organize as organize;

mod create;
mod form;
mod stamp;
mod text;
pub use create::images_to_pdf;
pub use form::{FieldKind, FormField};
pub use stamp::TextFont;

/// How typed text looks: `size` points, `font`, `color` 0–1 RGB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLook {
    pub size: f64,
    pub font: TextFont,
    pub color: [f64; 3],
}

/// A text typed on a page with [`PdfDoc::add_text`], found again for moving and restyling.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedText {
    /// Its place in the page's annotations (changes when annotations before it are removed).
    pub index: u32,
    /// Left, top, right, bottom as fractions of the displayed page (y down).
    pub rect: [f64; 4],
    pub text: String,
    pub size: f64,
    pub font: TextFont,
    pub color: [f64; 3],
}
pub use text::{Sensitive, fold, turkish_id_ok};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PdfError {
    #[error("{0}")]
    Open(String),
    #[error("the document is protected by a password")]
    NeedsPassword,
    #[error("the password is incorrect")]
    WrongPassword,
    #[error("{0}")]
    Edit(String),
    #[error("{0}")]
    Save(String),
}

/// One open PDF being edited.
pub struct PdfDoc {
    doc: Document,
    /// (page, index in its /Annots) of what [`PdfDoc::annotate`] added, newest last; cleared when
    /// pages move or go, which changes those positions.
    added: Vec<(usize, usize)>,
    /// The text layer of the document as it is now; dropped on every change.
    text: Option<pdfcraft_render::PageRenderer>,
    /// What PdfCraft repaired while opening the file (see [`PdfDoc::open_notes`]).
    open_notes: Vec<String>,
}

/// One place a search or a black-out found: its page and one rectangle per line, as fractions
/// of the displayed page (x right, y down): [left, top, right, bottom].
#[derive(Clone, Debug, PartialEq)]
pub struct TextHit {
    pub page: u32,
    pub rects: Vec<[f32; 4]>,
}

fn opening(e: CosError) -> PdfError {
    match e {
        CosError::NeedsPassword => PdfError::NeedsPassword,
        CosError::WrongPassword => PdfError::WrongPassword,
        e => PdfError::Open(e.to_string()),
    }
}

fn edit<E: std::fmt::Display>(e: E) -> PdfError {
    PdfError::Edit(e.to_string())
}

impl PdfDoc {
    pub fn open(bytes: Vec<u8>, password: Option<&str>) -> Result<PdfDoc, PdfError> {
        let doc = Document::open_with_password(Arc::new(bytes), password).map_err(opening)?;
        let open_notes = doc.repair_log().to_vec();
        Ok(PdfDoc { doc, added: Vec::new(), text: None, open_notes })
    }

    /// What PdfCraft had to repair to open the file, one English line each (a rebuilt
    /// cross-reference index, a missing header, skipped object numbers…); empty for a sound file.
    /// Taken as the file opened: later edits do not add to it. Whatever is saved is the repaired
    /// document, so the person is told before they keep a copy.
    pub fn open_notes(&self) -> &[String] {
        &self.open_notes
    }

    pub fn page_count(&self) -> Result<u32, PdfError> {
        Ok(organize::page_count(&self.doc).map_err(edit)? as u32)
    }

    /// Turns `pages` (0-based) by `degrees` (a multiple of 90, clockwise).
    pub fn rotate(&mut self, pages: &[u32], degrees: i32) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        organize::rotate_pages(&mut self.doc, &indices(pages), i64::from(degrees)).map_err(edit)
    }

    /// The page's /Rotate, 0–270.
    pub fn rotation(&self, page: u32) -> Result<i32, PdfError> {
        Ok(organize::page_rotation(&self.doc, page as usize).map_err(edit)? as i32)
    }

    /// Removes `pages`; a document keeps at least one page.
    pub fn delete(&mut self, pages: &[u32]) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        if pages.len() as u32 >= self.page_count()? {
            return Err(PdfError::Edit("a PDF needs at least one page".into()));
        }
        self.added.clear();
        organize::delete_pages(&mut self.doc, &indices(pages)).map_err(edit)
    }

    /// Moves `pages` so the first of them lands at position `to` (0-based, in the new order).
    pub fn move_pages(&mut self, pages: &[u32], to: u32) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        self.added.clear();
        organize::move_pages(&mut self.doc, &indices(pages), to as usize).map_err(edit)
    }

    /// Appends all pages of another PDF (fields renamed where names clash).
    pub fn append(&mut self, other: Vec<u8>, password: Option<&str>) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        let src = Document::open_with_password(Arc::new(other), password).map_err(opening)?;
        let n = organize::page_count(&src).map_err(edit)?;
        let at = organize::page_count(&self.doc).map_err(edit)?;
        let all: Vec<usize> = (0..n).collect();
        organize::import_pages_with(&mut self.doc, &src, &all, at, organize::FieldConflicts::Rename).map_err(edit)?;
        Ok(())
    }

    /// A new PDF with only `pages`, in the given order; this document is unchanged.
    pub fn extract(&self, pages: &[u32]) -> Result<Vec<u8>, PdfError> {
        self.refuse_restricted()?;
        // A copy keeps the password, if any: the part stays as protected as the whole.
        let mut out = self.doc.clone();
        let n = organize::page_count(&out).map_err(edit)?;
        let keep = indices(pages);
        if keep.is_empty() || keep.iter().any(|p| *p >= n) {
            return Err(PdfError::Edit("no such page".into()));
        }
        // Copy the chosen pages to the end in order, then drop the originals.
        let copies: Vec<usize> = keep.clone();
        organize::duplicate_pages(&mut out, &copies, Some(n)).map_err(edit)?;
        organize::delete_pages(&mut out, &(0..n).collect::<Vec<_>>()).map_err(edit)?;
        pdfcraft_cos::write_full(&out, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }

    /// Puts a hand-drawn signature on `page` (0-based). Each stroke is a list of points as
    /// fractions of the page as it is displayed (x right, y down, 0–1), so the caller need not know
    /// the page's boxes or rotation.
    pub fn sign(&mut self, page: u32, strokes: &[Vec<[f64; 2]>]) -> Result<(), PdfError> {
        self.text = None;
        let strokes = self.user_strokes(page, strokes)?;
        let shape = annot::Shape::Signature { strokes };
        let style = annot::Style { width: 1.5, ..annot::Style::default_for(&shape) };
        self.annotate(page, shape, style, String::new())
    }

    /// Pen strokes on `page` (points as in [`PdfDoc::sign`]): `color` 0–1 RGB, `width` in points,
    /// `opacity` 0–1 (a highlighter is wide, yellow and see-through).
    pub fn draw(&mut self, page: u32, strokes: &[Vec<[f64; 2]>], color: [f64; 3], width: f64, opacity: f64) -> Result<(), PdfError> {
        self.text = None;
        let strokes = self.user_strokes(page, strokes)?;
        let style = annot::Style { color, opacity, width, fill: None };
        self.annotate(page, annot::Shape::Ink { strokes }, style, String::new())
    }

    /// Marks text on `page` over `rects` (fractions of the displayed page: left, top, right,
    /// bottom, y down, as [`PdfDoc::text_between`] gives them): `kind` 0 highlights it, 1
    /// underlines it, 2 strikes it through; `color` is 0–1 RGB.
    pub fn mark_text(&mut self, page: u32, rects: &[[f64; 4]], kind: u8, color: [f64; 3]) -> Result<(), PdfError> {
        if rects.is_empty() {
            return Ok(());
        }
        self.text = None;
        // Each rectangle's corners in PDF's quad order: upper left, upper right, lower left, lower right.
        let corners: Vec<Vec<[f64; 2]>> = rects.iter().map(|r| vec![[r[0], r[1]], [r[2], r[1]], [r[0], r[3]], [r[2], r[3]]]).collect();
        let user = self.user_strokes(page, &corners)?;
        let quads: Vec<[f64; 8]> = user.iter().map(|c| [c[0][0], c[0][1], c[1][0], c[1][1], c[2][0], c[2][1], c[3][0], c[3][1]]).collect();
        let markup = match kind {
            1 => annot::Markup::Underline,
            2 => annot::Markup::StrikeOut,
            _ => annot::Markup::Highlight,
        };
        let shape = annot::Shape::TextMarkup { kind: markup, quads };
        let style = annot::Style { color, ..annot::Style::default_for(&shape) };
        self.annotate(page, shape, style, String::new())
    }

    /// A sticky note holding `text`, its corner at `at` (a fraction of the displayed page).
    pub fn note(&mut self, page: u32, at: [f64; 2], text: &str) -> Result<(), PdfError> {
        self.text = None;
        let [x, y] = self.user_strokes(page, &[vec![at]])?[0][0];
        let shape = annot::Shape::Note { at: [x, y], icon: annot::NoteIcon::Comment };
        let style = annot::Style::default_for(&shape);
        self.annotate(page, shape, style, text.to_string())
    }

    /// Text typed on `page`, its top-left corner at `at` (a fraction of the displayed page), in
    /// `size` points, `font` and `color` (0–1 RGB). Shown and printed as typed, Turkish letters too.
    pub fn add_text(&mut self, page: u32, at: [f64; 2], text: &str, size: f64, font: TextFont, color: [f64; 3]) -> Result<(), PdfError> {
        self.text = None;
        let [x, y] = self.user_strokes(page, &[vec![at]])?[0][0];
        let index = stamp::add_text(&mut self.doc, page as usize, [x, y], text, size, font, color)?;
        self.added.push((page as usize, index));
        Ok(())
    }

    /// Lays recognised `words` (each its text and box as fractions of the displayed page: left,
    /// top, right, bottom) over `page` as invisible text, so a scanned page can be searched and
    /// copied while it looks the same. Returns how many words were written.
    pub fn add_ocr_words(&mut self, page: u32, words: &[(String, [f64; 4])]) -> Result<u32, PdfError> {
        self.refuse_restricted()?;
        self.text = None;
        let p = page as usize;
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let crop = boxes.get(p).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let turn = organize::page_rotation(&self.doc, p).map_err(edit)?;
        let words: Vec<stamp::OcrWord> = words
            .iter()
            .map(|(text, [l, t, r, b])| stamp::OcrWord {
                text: text.clone(),
                bottom_left: user_point(crop, turn, [*l, *b]),
                bottom_right: user_point(crop, turn, [*r, *b]),
                top_left: user_point(crop, turn, [*l, *t]),
            })
            .collect();
        Ok(stamp::add_invisible_words(&mut self.doc, p, &words)? as u32)
    }

    /// The texts typed with [`PdfDoc::add_text`] on `page`, to select, move and restyle: their
    /// boxes as fractions of the displayed page (left, top, right, bottom; y down).
    pub fn typed_texts(&self, page: u32) -> Result<Vec<TypedText>, PdfError> {
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let crop = boxes.get(page as usize).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let turn = organize::page_rotation(&self.doc, page as usize).map_err(edit)?;
        Ok(stamp::typed_on(&self.doc, page as usize)
            .into_iter()
            .map(|t| {
                let a = display_point(crop, turn, [t.rect[0], t.rect[1]]);
                let b = display_point(crop, turn, [t.rect[2], t.rect[3]]);
                TypedText {
                    index: t.index as u32,
                    rect: [a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])],
                    text: t.text,
                    size: t.size,
                    font: t.font,
                    color: t.color,
                }
            })
            .collect())
    }

    /// Typed text `index` on `page` moved to `at` (its top-left, a fraction of the displayed page)
    /// and given `text`, `size`, `font` and `color`. One undo step (it was one annotation).
    pub fn update_text(&mut self, page: u32, index: u32, at: [f64; 2], text: &str, look: TextLook) -> Result<(), PdfError> {
        let TextLook { size, font, color } = look;
        self.refuse_restricted()?;
        let (page_i, index) = (page as usize, index as usize);
        if !stamp::typed_on(&self.doc, page_i).iter().any(|t| t.index == index) {
            return Err(PdfError::Edit("that is not a text typed here".into()));
        }
        let [x, y] = self.user_strokes(page, &[vec![at]])?[0][0];
        // Building an appearance can fail (for example for empty text). Keep both the
        // original annotation and undo entries intact until the replacement is complete.
        let mut next = self.doc.clone();
        annot::delete_annotation(&mut next, page_i, index).map_err(edit)?;
        let new = stamp::add_text(&mut next, page_i, [x, y], text, size, font, color)?;
        self.doc = next;
        self.text = None;
        self.forget_added(page_i, index);
        self.added.push((page_i, new));
        Ok(())
    }

    /// Typed text `index` on `page` removed.
    pub fn delete_text(&mut self, page: u32, index: u32) -> Result<(), PdfError> {
        self.refuse_restricted()?;
        let (page_i, index) = (page as usize, index as usize);
        if !stamp::typed_on(&self.doc, page_i).iter().any(|t| t.index == index) {
            return Err(PdfError::Edit("that is not a text typed here".into()));
        }
        self.text = None;
        annot::delete_annotation(&mut self.doc, page_i, index).map_err(edit)?;
        self.forget_added(page_i, index);
        Ok(())
    }

    /// After annotation `index` on `page` went: it is no longer undoable, and the later ones on
    /// that page moved up by one.
    fn forget_added(&mut self, page: usize, index: usize) {
        self.added.retain(|(p, i)| !(*p == page && *i == index));
        for (p, i) in &mut self.added {
            if *p == page && *i > index {
                *i -= 1;
            }
        }
    }

    /// Removes the last signature, drawing or note added since the document was opened (or
    /// since pages were last moved or removed); false when there is none. What the file
    /// already had is never removed.
    pub fn undo_annotation(&mut self) -> Result<bool, PdfError> {
        self.text = None;
        let Some((page, index)) = self.added.pop() else { return Ok(false) };
        annot::delete_annotation(&mut self.doc, page, index).map_err(edit)?;
        Ok(true)
    }

    /// Strokes given as fractions of the displayed page, in `page`'s user space.
    fn user_strokes(&self, page: u32, strokes: &[Vec<[f64; 2]>]) -> Result<Vec<Vec<[f64; 2]>>, PdfError> {
        self.refuse_restricted()?;
        let page = page as usize;
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let crop = boxes.get(page).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let turn = organize::page_rotation(&self.doc, page).map_err(edit)?;
        let strokes: Vec<Vec<[f64; 2]>> =
            strokes.iter().filter(|s| !s.is_empty()).map(|s| s.iter().map(|p| user_point(crop, turn, *p)).collect()).collect();
        if strokes.is_empty() {
            return Err(PdfError::Edit("the drawing is empty".into()));
        }
        Ok(strokes)
    }

    fn annotate(&mut self, page: u32, shape: annot::Shape, style: annot::Style, contents: String) -> Result<(), PdfError> {
        let new = annot::NewAnnotation { page: page as usize, shape, style, contents, author: String::new() };
        let meta = annot::Meta { date: None, id: String::new() };
        let index = annot::add_annotation(&mut self.doc, &new, &meta).map_err(edit)?;
        self.added.push((page as usize, index));
        Ok(())
    }

    /// Whether the file written by [`PdfDoc::save`] asks for a password.
    pub fn is_protected(&self) -> bool {
        self.doc.output_handler().is_some()
    }

    /// Whether the password it was opened with keeps it from being changed: the owner set
    /// restrictions (no editing, printing…) and only the owner's password lifts them.
    pub fn is_restricted(&self) -> bool {
        self.doc.permissions().is_some_and(|p| {
            const ALL: i32 = 0b1111_0011_1100; // bits 3–6 and 9–12 (§7.6.4.2, Table 22)
            !p.owner && p.bits & ALL != ALL
        })
    }

    /// Protects the next save with `password` (AES-256), every use allowed once it is open.
    /// `seed` is 32 random bytes for the keys.
    pub fn protect(&mut self, password: &str, seed: [u8; 32]) -> Result<(), PdfError> {
        self.text = None;
        if password.is_empty() {
            return Err(PdfError::Edit("the password is empty".into()));
        }
        self.refuse_restricted()?;
        let params = pdfcraft_cos::NewEncryption {
            algorithm: pdfcraft_cos::Algorithm::Aes256,
            user_password: password,
            owner_password: password,
            permissions: -4, // every bit allowed (§7.6.4.2)
            encrypt_metadata: true,
            seed,
        };
        self.doc.set_encryption(&params).map_err(edit)
    }

    /// The next save asks for no password.
    pub fn unprotect(&mut self) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_restricted()?;
        self.doc.remove_encryption();
        Ok(())
    }

    fn refuse_restricted(&self) -> Result<(), PdfError> {
        if self.is_restricted() {
            return Err(PdfError::Edit("the owner of this PDF does not allow changing it".into()));
        }
        Ok(())
    }

    /// The document without its password, for showing on screen (never written to the person's
    /// files): Android's renderer cannot open encrypted files.
    ///
    /// Android's renderer draws no annotations, so in this copy every comment, signature and form
    /// field with an appearance is drawn into its page (page by page: a page that cannot be is
    /// shown as it is).
    pub fn display_copy(&self) -> Result<Vec<u8>, PdfError> {
        if !self.is_protected() && self.doc.security().is_none() && !self.has_drawn_annotations() {
            return self.save();
        }
        let mut copy = self.doc.clone();
        copy.remove_encryption();
        let pages = annot::page_refs(&copy).map(|p| p.len()).unwrap_or(0);
        for page in 0..pages {
            let before = copy.clone();
            if pdfcraft_edit::flatten(&mut copy, &[page], true, true).is_err() {
                copy = before;
            }
        }
        pdfcraft_cos::write_full(&copy, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }

    /// Whether Android's renderer would leave something out: an annotation other than a link or
    /// a pop-up that has an appearance. The viewer then shows [`PdfDoc::display_copy`].
    pub fn has_drawn_annotations(&self) -> bool {
        let Ok(pages) = annot::page_refs(&self.doc) else { return false };
        pages.iter().any(|p| {
            let Some(list) = self.doc.get(*p).as_dict().and_then(|d| d.get(b"Annots").cloned()) else { return false };
            let list = self.doc.resolve(&list);
            list.as_array().is_some_and(|a| {
                a.iter().any(|o| {
                    let d = self.doc.resolve(o);
                    d.as_dict().is_some_and(|d| d.get(b"AP").is_some() && !matches!(d.name(b"Subtype"), Some(b"Link" | b"Popup")))
                })
            })
        })
    }

    /// A smaller copy: pictures resampled and recompressed (to 150 ppi, or 96 ppi `for_screen`),
    /// thumbnails dropped, streams compressed. This document is unchanged.
    pub fn compress(&self, for_screen: bool) -> Result<Vec<u8>, PdfError> {
        let preset = if for_screen { pdfcraft_optimize::Preset::Mobile } else { pdfcraft_optimize::Preset::Standard };
        let mut copy = self.doc.clone();
        pdfcraft_optimize::optimize(&mut copy, &preset.settings()).map_err(edit)?;
        pdfcraft_cos::write_full(&copy, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }

    /// The form's fields, in document order (none when there is no form).
    /// Finds where a printed form expects answers (empty boxes, lines to write on, small squares
    /// to tick, underscore runs) and adds a fillable field for each, named from its label, as
    /// Acrobat's "Prepare form" does: for forms printed from a browser or a word processor, which
    /// keep the boxes but lose the fields. Fields already there are left alone. Returns how many
    /// were added.
    pub fn detect_fields(&mut self) -> Result<u32, PdfError> {
        self.refuse_unfillable()?;
        use pdfcraft_forms::detect;
        let n = self.page_count()?;
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let sizes: Vec<(f32, f32)> = (0..n).map(|p| self.view_size(p)).collect::<Result<_, _>>()?;
        let turns: Vec<i64> = (0..n as usize).map(|p| organize::page_rotation(&self.doc, p)).collect::<Result<_, _>>().map_err(edit)?;
        let existing_fields = pdfcraft_forms::fields(&self.doc);
        let mut taken: Vec<String> = existing_fields.iter().map(|f| f.name.clone()).collect();
        let mut found: Vec<(usize, detect::Candidate)> = Vec::new();
        for page in 0..n as usize {
            let (w, h) = sizes[page];
            let crop = boxes[page][1];
            let to_user = |x: f32, y: f32| user_point(crop, turns[page], [f64::from(x / w), f64::from(y / h)]);
            let layer = self.text_layer()?;
            let words = match layer.filtered_text(page, Default::default()) {
                Ok(text) => page_words(&text)
                    .into_iter()
                    .map(|(t, r)| {
                        let (a, b) = (to_user(r[0], r[1]), to_user(r[2], r[3]));
                        detect::Word { text: t, rect: [a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])] }
                    })
                    .collect(),
                Err(_) => Vec::new(),
            };
            // Shapes from the drawing's paths, and from a picture of the page for those the
            // paths don't give as plain rectangles (rounded boxes, as browsers draw them).
            let picture = layer.render(pdfcraft_render::RenderRequest { page, scale: DETECT_SCALE, ..Default::default() });
            let mut shapes = detect::page_shapes(&self.doc, page);
            if picture.error.is_none() {
                let seen = detect::raster::shapes(&picture.rgba, picture.width, picture.height, f64::from(DETECT_SCALE));
                // The picture's frame is the displayed page, y up from its bottom.
                let to_user_up = |r: [f64; 4]| {
                    let a = to_user(r[0] as f32, h - r[1] as f32);
                    let b = to_user(r[2] as f32, h - r[3] as f32);
                    [a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])]
                };
                shapes.boxes.extend(seen.boxes.into_iter().map(to_user_up));
                shapes.rules.extend(seen.rules.into_iter().map(to_user_up));
            }
            let existing: Vec<[f64; 4]> =
                existing_fields.iter().flat_map(|f| f.widgets.iter()).filter(|wd| wd.page == Some(page)).map(|wd| wd.rect).collect();
            for mut c in detect::detect(&words, &shapes, &existing, &taken) {
                // Detection names a field from the text beside it; a box whose label is above it
                // (a heading over a text area) gets a generic name: take the line above instead.
                if is_generic_name(&c.name)
                    && let Some(label) = label_above(&words, c.rect)
                    && !taken.contains(&label)
                {
                    c.name = label;
                }
                taken.push(c.name.clone());
                found.push((page, c));
            }
        }
        let mut added = 0;
        for (page, c) in found {
            let tall = c.rect[3] - c.rect[1];
            let (kind, comb) = match c.kind {
                detect::Kind::Text => (pdfcraft_forms::NewField::Text { multiline: tall > 30.0 }, None),
                detect::Kind::CheckBox => (pdfcraft_forms::NewField::CheckBox, None),
                detect::Kind::Comb { cells } => (pdfcraft_forms::NewField::Text { multiline: false }, Some(cells)),
            };
            let Ok(name) = pdfcraft_forms::add_field(&mut self.doc, page, c.rect, &kind, Some(&c.name)) else { continue };
            // The page already draws the box or line: the field gets no border or fill of its own.
            let mut props = pdfcraft_forms::FieldProps {
                appearance: Some(pdfcraft_forms::LookPatch { border: Some(None), fill: Some(None), ..Default::default() }),
                ..Default::default()
            };
            if let Some(cells) = comb {
                props.max_len = Some(Some(cells));
                props.flags = vec![(pdfcraft_forms::flags::COMB, true)];
            }
            let _ = pdfcraft_forms::set_props(&mut self.doc, &name, &props);
            added += 1;
        }
        if added > 0 {
            self.text = None;
        }
        Ok(added)
    }

    /// What the file holds of a form, in one line, for when no field is found.
    pub fn form_diagnosis(&self) -> String {
        form::diagnosis(&self.doc)
    }

    pub fn fields(&self) -> Vec<FormField> {
        form::fields(&self.doc)
    }

    /// [`Self::fields`], where a field with neither a telling tooltip nor a telling name ("text_1",
    /// "Text2", "undefined") is labelled with the words printed beside or above it.
    pub fn labelled_fields(&mut self) -> Vec<FormField> {
        let mut fields = self.fields();
        for f in fields.iter_mut().filter(|f| needs_label(&f.label) && !needs_label(&f.name)) {
            f.label = f.name.clone();
        }
        if !fields.iter().any(|f| needs_label(&f.label)) {
            return fields;
        }
        let raw = pdfcraft_forms::fields(&self.doc);
        let mut words: std::collections::HashMap<usize, Vec<pdfcraft_forms::detect::Word>> = std::collections::HashMap::new();
        for f in fields.iter_mut().filter(|f| needs_label(&f.label)) {
            let Some(widget) = raw.iter().find(|r| r.name == f.name).and_then(|r| r.widgets.iter().find(|w| w.page.is_some())) else {
                continue;
            };
            let page = widget.page.expect("filtered");
            if let std::collections::hash_map::Entry::Vacant(e) = words.entry(page) {
                e.insert(self.user_words(page).unwrap_or_default());
            }
            let on_page = &words[&page];
            if let Some(label) = label_beside(on_page, widget.rect).or_else(|| label_above(on_page, widget.rect)) {
                f.label = label;
            }
        }
        fields
    }

    /// The words of `page` with their boxes in user space (points, y up).
    fn user_words(&mut self, page: usize) -> Result<Vec<pdfcraft_forms::detect::Word>, PdfError> {
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let crop = boxes.get(page).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let turn = organize::page_rotation(&self.doc, page).map_err(edit)?;
        let (w, h) = self.view_size(page as u32)?;
        let text = self.text_layer()?.filtered_text(page, Default::default()).map_err(|e| PdfError::Edit(e.to_string()))?;
        let to_user = |x: f32, y: f32| user_point(crop, turn, [f64::from(x / w), f64::from(y / h)]);
        Ok(page_words(&text)
            .into_iter()
            .map(|(t, r)| {
                let (a, b) = (to_user(r[0], r[1]), to_user(r[2], r[3]));
                pdfcraft_forms::detect::Word { text: t, rect: [a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])] }
            })
            .collect())
    }

    pub fn set_text(&mut self, name: &str, text: &str) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_unfillable()?;
        form::set_text(&mut self.doc, name, text)
    }

    pub fn set_check(&mut self, name: &str, on: bool) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_unfillable()?;
        form::set_check(&mut self.doc, name, on)
    }

    /// Picks `value` (an export value from [`FormField::options`]; empty for none).
    pub fn set_choice(&mut self, name: &str, value: &str) -> Result<(), PdfError> {
        self.text = None;
        self.refuse_unfillable()?;
        form::set_choice(&mut self.doc, name, value)
    }

    fn refuse_unfillable(&self) -> Result<(), PdfError> {
        if self.is_restricted() && self.doc.permissions().is_some_and(|p| !p.fill_forms()) {
            return Err(PdfError::Edit("the owner of this PDF does not allow filling in its form".into()));
        }
        Ok(())
    }

    /// Every place `query` occurs, ignoring case, Turkish i forms and accents (at most `limit`).
    pub fn find_text(&mut self, query: &str, limit: usize) -> Result<Vec<TextHit>, PdfError> {
        let query = query.to_string();
        self.hits(limit, move |chars| text::find_folded(chars, &query))
    }

    /// The text between two points of `page` (fractions of the displayed page, y down), in
    /// reading order, as a finger drags across it: the text and its line boxes (fractions).
    pub fn text_between(&mut self, page: u32, from: [f64; 2], to: [f64; 2]) -> Result<(String, Vec<[f32; 4]>), PdfError> {
        let (w, h) = self.view_size(page)?;
        let layer = self.text_layer()?;
        let text = layer.filtered_text(page as usize, Default::default()).map_err(PdfError::Edit)?;
        let at = |p: [f64; 2]| text.nearest(p[0] as f32 * w, p[1] as f32 * h);
        let (Some(a), Some(b)) = (at(from), at(to)) else { return Ok((String::new(), Vec::new())) };
        let range = a.min(b)..a.max(b) + 1;
        let rects = text.line_rects(range.clone()).into_iter().map(|r| [r[0] / w, r[1] / h, r[2] / w, r[3] / h]).collect();
        Ok((text.text_of(range), rects))
    }

    /// The text of `page` in reading order (for reading aloud and copying).
    pub fn page_text(&mut self, page: u32) -> Result<String, PdfError> {
        let layer = self.text_layer()?;
        Ok(layer.filtered_text(page as usize, Default::default()).map_err(PdfError::Edit)?.plain_text())
    }

    /// Marks for blacking out every place of `kinds` and of `words` (whole text, matched like a
    /// search). Nothing is removed until [`PdfDoc::apply_redactions`]. Returns the places marked.
    pub fn mark_redactions(&mut self, kinds: &[Sensitive], words: &[String]) -> Result<Vec<TextHit>, PdfError> {
        self.refuse_restricted()?;
        let kinds = kinds.to_vec();
        let words: Vec<String> = words.iter().map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect();
        let hits = self.hits(usize::MAX, move |chars| {
            let mut all: Vec<std::ops::Range<usize>> = kinds.iter().flat_map(|k| text::find_sensitive(*k, chars)).collect();
            all.extend(words.iter().flat_map(|w| text::find_folded(chars, w)));
            // One mark where finds overlap ("Şule" inside "sule@ornek.com").
            all.sort_by_key(|r| (r.start, std::cmp::Reverse(r.end)));
            let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
            for r in all {
                match merged.last_mut() {
                    Some(last) if r.start < last.end => last.end = last.end.max(r.end),
                    _ => merged.push(r),
                }
            }
            merged
        })?;
        for hit in &hits {
            let quads: Vec<[f64; 8]> = hit
                .rects
                .iter()
                .map(|r| {
                    let mut q = [0.0; 8];
                    for (i, p) in [[r[0], r[1]], [r[2], r[1]], [r[0], r[3]], [r[2], r[3]]].into_iter().enumerate() {
                        let [x, y] = self.user_strokes(hit.page, &[vec![[f64::from(p[0]), f64::from(p[1])]]]).map(|s| s[0][0])?;
                        q[2 * i] = x;
                        q[2 * i + 1] = y;
                    }
                    Ok(q)
                })
                .collect::<Result<_, PdfError>>()?;
            let shape = annot::Shape::Redact { quads, overlay: String::new(), look: Default::default() };
            let style = annot::Style::default_for(&shape);
            self.annotate(hit.page, shape, style, String::new())?;
        }
        self.text = None;
        Ok(hits)
    }

    /// Drops every black-out mark not yet applied (the text under them stays).
    pub fn clear_redaction_marks(&mut self) -> Result<u32, PdfError> {
        self.text = None;
        let n = pdfcraft_redact::clear_marks(&mut self.doc, None).map_err(edit)?;
        // Undo positions no longer hold once marks are gone.
        self.added.clear();
        Ok(n as u32)
    }

    /// Black-out marks waiting to be applied.
    pub fn redaction_marks(&self) -> u32 {
        pdfcraft_redact::marks(&self.doc).len() as u32
    }

    /// Removes the text, pictures and drawings under every black-out mark for good and paints the
    /// places black. Returns how many marks were applied.
    pub fn apply_redactions(&mut self) -> Result<u32, PdfError> {
        self.refuse_restricted()?;
        self.text = None;
        let report = pdfcraft_redact::apply(&mut self.doc, None).map_err(edit)?;
        // Undo positions no longer hold: the marks are gone.
        self.added.clear();
        Ok(report.marks as u32)
    }

    /// Matches of `matcher` over each page's text, as page fractions.
    fn hits(&mut self, limit: usize, matcher: impl Fn(&[char]) -> Vec<std::ops::Range<usize>>) -> Result<Vec<TextHit>, PdfError> {
        let n = self.page_count()?;
        let sizes: Vec<(f32, f32)> = (0..n).map(|p| self.view_size(p)).collect::<Result<_, _>>()?;
        let layer = self.text_layer()?;
        let mut out = Vec::new();
        for page in 0..n {
            // A page whose text can't be read is skipped, not an error for the whole document.
            let Ok(text) = layer.filtered_text(page as usize, Default::default()) else { continue };
            let (w, h) = sizes[page as usize];
            for range in text.find_with(&matcher) {
                let rects = text.line_rects(range).into_iter().map(|r| [r[0] / w, r[1] / h, r[2] / w, r[3] / h]).collect::<Vec<_>>();
                if !rects.is_empty() {
                    out.push(TextHit { page, rects });
                    if out.len() >= limit {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }

    /// The page as displayed, in points: its crop box turned by /Rotate.
    fn view_size(&self, page: u32) -> Result<(f32, f32), PdfError> {
        let boxes = organize::page_boxes(&self.doc).map_err(edit)?;
        let [x0, y0, x1, y1] = boxes.get(page as usize).ok_or_else(|| PdfError::Edit("no such page".into()))?[1];
        let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
        Ok(if organize::page_rotation(&self.doc, page as usize).map_err(edit)?.rem_euclid(180) == 90 { (h, w) } else { (w, h) })
    }

    fn text_layer(&mut self) -> Result<&mut pdfcraft_render::PageRenderer, PdfError> {
        if self.text.is_none() {
            let bytes = self.display_copy()?;
            self.text = Some(pdfcraft_render::PageRenderer::new(Arc::new(bytes), Default::default()));
        }
        Ok(self.text.as_mut().expect("just set"))
    }

    /// The whole document written afresh (compressed object streams, a new cross-reference).
    pub fn save(&self) -> Result<Vec<u8>, PdfError> {
        pdfcraft_cos::write_full(&self.doc, &SaveOptions::default()).map_err(|e| PdfError::Save(e.to_string()))
    }
}

/// Whether detection fell back to a numbered name ("Text1", "Check Box2").
fn is_generic_name(name: &str) -> bool {
    let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
    stem.len() < name.len() && matches!(stem.trim(), "Text" | "Check Box" | "CheckBox" | "Field")
}

/// Whether a field's name tells a person nothing: a form tool's default ("text_1", "Text2",
/// "Text Field 3", "undefined", "Check Box1").
fn needs_label(name: &str) -> bool {
    let last = name.rsplit('.').next().unwrap_or(name);
    let stem: String =
        last.chars().filter(|c| !c.is_ascii_digit() && !matches!(c, '_' | '-' | ' ' | '#' | '[' | ']')).collect::<String>().to_lowercase();
    matches!(
        stem.as_str(),
        "" | "text"
            | "textfield"
            | "textbox"
            | "field"
            | "fill"
            | "untitled"
            | "undefined"
            | "checkbox"
            | "check"
            | "box"
            | "input"
            | "radio"
            | "radiobutton"
            | "dropdown"
            | "combobox"
            | "list"
            | "listbox"
            | "metin"
            | "alan"
            | "onaykutusu"
    )
}

/// The words printed on the same line as `rect` (user space, y up): the run just to its left
/// (a label before a blank), else the run just to its right (a check box's caption).
fn label_beside(words: &[pdfcraft_forms::detect::Word], rect: [f64; 4]) -> Option<String> {
    let mid = (rect[1] + rect[3]) / 2.0;
    let mut line: Vec<&pdfcraft_forms::detect::Word> = words.iter().filter(|w| w.rect[1] - 2.0 <= mid && w.rect[3] + 2.0 >= mid).collect();
    line.sort_by(|a, b| a.rect[0].total_cmp(&b.rect[0]));
    let tidy = |run: Vec<&pdfcraft_forms::detect::Word>| {
        let text = run.iter().map(|w| w.text.as_str()).filter(|t| !t.chars().all(|c| c == '_')).collect::<Vec<_>>().join(" ");
        let text = text.trim().trim_end_matches(':').trim().to_string();
        (text.chars().any(char::is_alphanumeric) && text.chars().count() <= 60).then_some(text)
    };
    // Left: walk back from the field while the gaps stay small.
    let mut left: Vec<&pdfcraft_forms::detect::Word> = Vec::new();
    let mut edge = rect[0];
    for w in line.iter().rev().filter(|w| w.rect[2] <= rect[0] + 2.0) {
        let gap = edge - w.rect[2];
        if gap > if left.is_empty() { 120.0 } else { 14.0 } {
            break;
        }
        left.insert(0, w);
        edge = w.rect[0];
    }
    if let Some(t) = tidy(left) {
        return Some(t);
    }
    let mut right: Vec<&pdfcraft_forms::detect::Word> = Vec::new();
    let mut edge = rect[2];
    for w in line.iter().filter(|w| w.rect[0] >= rect[2] - 2.0) {
        let gap = w.rect[0] - edge;
        if gap > if right.is_empty() { 30.0 } else { 14.0 } {
            break;
        }
        right.push(w);
        edge = w.rect[2];
    }
    tidy(right)
}

/// The line of words just above `rect` (user space, y up) that overlaps it across: the nearest
/// within [`LABEL_REACH`] (a text area's field may sit on its bottom line, well below its label). Its words joined in reading order.
fn label_above(words: &[pdfcraft_forms::detect::Word], rect: [f64; 4]) -> Option<String> {
    let above: Vec<&pdfcraft_forms::detect::Word> = words
        .iter()
        .filter(|w| w.rect[1] >= rect[3] - 2.0 && w.rect[1] - rect[3] < LABEL_REACH && w.rect[0] < rect[2] && w.rect[2] > rect[0] - 4.0)
        .collect();
    // The nearest line with a word in it (a lone "/" or "*" does not make a line).
    let nearest = above.iter().filter(|w| w.text.chars().any(char::is_alphanumeric)).map(|w| w.rect[1]).fold(f64::INFINITY, f64::min);
    let mut line: Vec<&&pdfcraft_forms::detect::Word> = above.iter().filter(|w| (w.rect[1] - nearest).abs() < 3.0).collect();
    line.sort_by(|a, b| a.rect[0].total_cmp(&b.rect[0]));
    let text = line.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
    let text = text.trim().trim_end_matches(':').trim().to_string();
    (!text.is_empty() && text.chars().count() <= 60).then_some(text)
}

/// How far above a field (points) its label is looked for.
const LABEL_REACH: f64 = 72.0;

/// Pixels per point of the page picture field detection looks for boxes in.
const DETECT_SCALE: f32 = 2.0;

/// A page's words in reading order with their boxes in view space (points, y down); runs of
/// underscores (a form's blanks) are words of their own.
fn page_words(text: &pdfcraft_render::PageText) -> Vec<(String, [f32; 4])> {
    let mut words: Vec<(String, [f32; 4])> = Vec::new();
    let mut last_line = u32::MAX;
    let mut gap = false;
    let mut last_under = false;
    for (i, g) in text.glyphs.iter().enumerate() {
        let line = text.line_of.get(i).copied().unwrap_or(0);
        if g.text.trim().is_empty() {
            gap = true;
            continue;
        }
        let under = g.text.chars().all(|c| c == '_');
        let breaks = words.is_empty() || gap || line != last_line || text.space_before.get(i).copied().unwrap_or(false) || last_under != under;
        gap = false;
        last_line = line;
        match words.last_mut() {
            Some((w, r)) if !breaks => {
                w.push_str(&g.text);
                r[0] = r[0].min(g.rect[0]);
                r[1] = r[1].min(g.rect[1]);
                r[2] = r[2].max(g.rect[2]);
                r[3] = r[3].max(g.rect[3]);
            }
            _ => {
                words.push((g.text.clone(), g.rect));
                last_under = under;
            }
        }
    }
    words
}

/// A point given as a fraction of the displayed page (y down) in the page's user space: the crop
/// box turned by the page's /Rotate (clockwise).
fn user_point(crop: [f64; 4], turn: i64, [u, v]: [f64; 2]) -> [f64; 2] {
    let [x0, y0, x1, y1] = crop;
    let (w, h) = (x1 - x0, y1 - y0);
    match turn.rem_euclid(360) {
        90 => [x0 + v * w, y0 + u * h],
        180 => [x1 - u * w, y0 + v * h],
        270 => [x1 - v * w, y1 - u * h],
        _ => [x0 + u * w, y1 - v * h],
    }
}

/// The inverse of [`user_point`]: a user-space point as a fraction of the displayed page.
fn display_point(crop: [f64; 4], turn: i64, [x, y]: [f64; 2]) -> [f64; 2] {
    let [x0, y0, x1, y1] = crop;
    let (w, h) = ((x1 - x0).max(1e-9), (y1 - y0).max(1e-9));
    match turn.rem_euclid(360) {
        90 => [(y - y0) / h, (x - x0) / w],
        180 => [(x1 - x) / w, (y - y0) / h],
        270 => [(y1 - y) / h, (x1 - x) / w],
        _ => [(x - x0) / w, (y1 - y) / h],
    }
}

fn indices(pages: &[u32]) -> Vec<usize> {
    pages.iter().map(|p| *p as usize).collect()
}

#[cfg(test)]
mod tests;
