//! Finding text in the body of a document, for the viewer's search bar. Turkish readers type
//! without the special letters as often as with them, so the search ignores case and accents and
//! treats dotted and dotless i alike: "isik" finds "IŞIK" and "ışık".

use wordcraft_doc::{Pos, StoryRef};

use crate::WordDoc;

/// One highlighted rectangle of match number `index`, in page points.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub index: u32,
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// At most this many matches are returned.
pub const MAX_MATCHES: usize = 2_000;

/// The search form of one character (see the module notes).
pub fn fold(c: char) -> char {
    match c {
        'ı' | 'I' | 'İ' | 'î' | 'Î' | 'í' | 'Í' | 'ì' | 'Ì' | 'ï' | 'Ï' => 'i',
        'ç' | 'Ç' => 'c',
        'ğ' | 'Ğ' => 'g',
        'ö' | 'Ö' | 'ó' | 'Ó' | 'ò' | 'Ò' | 'ô' | 'Ô' | 'õ' | 'Õ' => 'o',
        'ş' | 'Ş' => 's',
        'ü' | 'Ü' | 'û' | 'Û' | 'ú' | 'Ú' | 'ù' | 'Ù' => 'u',
        'â' | 'Â' | 'á' | 'Á' | 'à' | 'À' | 'ä' | 'Ä' | 'ã' | 'Ã' => 'a',
        'é' | 'É' | 'è' | 'È' | 'ê' | 'Ê' | 'ë' | 'Ë' => 'e',
        'ñ' | 'Ñ' => 'n',
        _ => {
            let mut lower = c.to_lowercase();
            match (lower.next(), lower.next()) {
                (Some(l), None) => l,
                _ => c,
            }
        }
    }
}

impl WordDoc {
    /// The body text that starts on each page, paragraph by paragraph, for reading aloud.
    /// A paragraph that runs over a page break is counted on the page it starts on.
    pub fn page_texts(&self) -> Vec<String> {
        let mut out = vec![String::new(); self.layout.pages.len()];
        for path in self.doc.para_paths(StoryRef::Body) {
            let Some(p) = self.doc.para(StoryRef::Body, &path) else { continue };
            // Objects (pictures, fields) sit in the text as control characters; readers skip them.
            let text: String = p.text.chars().filter(|c| !c.is_control()).collect();
            if text.trim().is_empty() {
                continue;
            }
            let a = Pos::new(StoryRef::Body, path.clone(), 0);
            let b = Pos::new(StoryRef::Body, path.clone(), p.text.len());
            let page = self.layout.selection_rects(&self.doc, &a, &b, 0).first().map_or(0, |(pg, _)| *pg);
            if let Some(slot) = out.get_mut(page) {
                if !slot.is_empty() {
                    slot.push('\n');
                }
                slot.push_str(text.trim());
            }
        }
        out
    }

    /// Every place `query` appears in the body text, as rectangles on the pages.
    pub fn find(&self, query: &str) -> Vec<Hit> {
        let needle: Vec<char> = query.trim().chars().map(fold).collect();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut index = 0u32;
        for path in self.doc.para_paths(StoryRef::Body) {
            let Some(p) = self.doc.para(StoryRef::Body, &path) else { continue };
            // Folded characters and the byte offset each starts at in the paragraph.
            let (chars, offs): (Vec<char>, Vec<usize>) = p.text.char_indices().map(|(o, c)| (fold(c), o)).unzip();
            if chars.len() < needle.len() {
                continue;
            }
            let mut i = 0;
            while i + needle.len() <= chars.len() {
                if chars[i..i + needle.len()] != needle[..] {
                    i += 1;
                    continue;
                }
                let end = offs.get(i + needle.len()).copied().unwrap_or(p.text.len());
                let a = Pos::new(StoryRef::Body, path.clone(), offs[i]);
                let b = Pos::new(StoryRef::Body, path.clone(), end);
                for (page, r) in self.layout.selection_rects(&self.doc, &a, &b, 0) {
                    out.push(Hit { index, page: page as u32, x: r.x, y: r.y, w: r.w, h: r.h });
                }
                index += 1;
                if index as usize >= MAX_MATCHES {
                    return out;
                }
                i += needle.len();
            }
        }
        out
    }
}
