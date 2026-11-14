//! Finding text in the body of a document, for the viewer's search bar. Turkish readers type
//! without the special letters as often as with them, so the search ignores case and accents and
//! treats dotted and dotless i alike: "isik" finds "IŞIK" and "ışık".

use wordcraft_doc::{Path, Pos, StoryRef};

use crate::{WordDoc, WordError};

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
        for path in self.doc().para_paths(StoryRef::Body) {
            if let Some(slot) = out.get_mut(self.start_page(&path)) {
                self.push_para_text(&path, slot);
            }
        }
        out
    }

    /// The body text that starts on `page` alone, as [`Self::page_texts`] gives it for that page,
    /// for a screen reader: only that page's paragraphs are copied, and each paragraph is placed
    /// through the layout's index rather than by measuring its lines, so it stays cheap to ask
    /// again while the person types.
    pub fn page_text(&self, page: u32) -> Result<String, WordError> {
        if page as usize >= self.layout.pages.len() {
            return Err(WordError::NoPage(page));
        }
        let mut out = String::new();
        for path in self.doc().para_paths(StoryRef::Body) {
            if self.start_page(&path) == page as usize {
                self.push_para_text(&path, &mut out);
            }
        }
        Ok(out)
    }

    /// The page a body paragraph starts on: where its first piece is placed (0 when it is not).
    fn start_page(&self, path: &Path) -> usize {
        self.layout.index.get(&(StoryRef::Body, path.clone())).and_then(|pieces| pieces.first()).map_or(0, |(page, _)| *page)
    }

    /// Adds body paragraph `path`'s text to `out` on a line of its own; nothing for an empty one.
    fn push_para_text(&self, path: &Path, out: &mut String) {
        let Some(p) = self.doc().para(StoryRef::Body, path) else { return };
        // Objects (pictures, fields) sit in the text as control characters; readers skip them
        // (tabs stay: they separate words).
        let text: String = p.text.chars().filter(|c| !c.is_control() || *c == '\t').collect();
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(text);
    }

    /// Every place `query` appears in the body text, as rectangles on the pages.
    pub fn find(&self, query: &str) -> Vec<Hit> {
        let needle: Vec<char> = query.trim().chars().map(fold).collect();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut index = 0u32;
        for path in self.doc().para_paths(StoryRef::Body) {
            let Some(p) = self.doc().para(StoryRef::Body, &path) else { continue };
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
                for (page, r) in self.layout.selection_rects(self.doc(), &a, &b, 0) {
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
