//! Readers for files the office engines have no reader for — e-books (EPUB), e-mails (EML and
//! Outlook MSG), contacts (vCard) and calendars (iCalendar) — each turned into one HTML page
//! that the word engine lays out, so they are read, searched, read aloud and printed like any
//! document. Nothing here writes: opening never changes a file.

mod calendar;
mod epub;
mod mail;
pub mod package;
pub mod text;
mod vcard;

/// A file turned into HTML: the page, notes on what was left out, and attachments to open.
#[derive(Debug, Default)]
pub struct Converted {
    pub html: String,
    pub notes: Vec<String>,
    pub attachments: Vec<Attachment>,
}

/// A file carried inside another (an e-mail attachment).
#[derive(Debug, Clone)]
pub struct Attachment {
    pub name: String,
    pub data: Vec<u8>,
}

/// What a file is, by its name and first bytes, when it is one of the kinds read here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Epub,
    Eml,
    Msg,
    VCard,
    Calendar,
}

/// The kind of `name` (and `bytes`), when this crate reads it.
pub fn kind_of(name: &str, bytes: &[u8]) -> Option<Kind> {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "epub" => Some(Kind::Epub),
        "eml" | "mht" | "mhtml" => Some(Kind::Eml),
        "msg" => Some(Kind::Msg),
        "vcf" | "vcard" => Some(Kind::VCard),
        "ics" | "ical" | "ifb" => Some(Kind::Calendar),
        _ => {
            let head = String::from_utf8_lossy(&bytes[..bytes.len().min(64)]).to_ascii_uppercase();
            if head.trim_start().starts_with("BEGIN:VCARD") {
                Some(Kind::VCard)
            } else if head.trim_start().starts_with("BEGIN:VCALENDAR") {
                Some(Kind::Calendar)
            } else {
                None
            }
        }
    }
}

/// `bytes` (a file called `name`) as one HTML page.
pub fn to_html(name: &str, bytes: &[u8]) -> Result<Converted, String> {
    match kind_of(name, bytes).ok_or_else(|| "not a file this reader knows".to_string())? {
        Kind::Epub => epub::read(bytes),
        Kind::Eml => mail::read_eml(bytes),
        Kind::Msg => mail::read_msg(bytes),
        Kind::VCard => Ok(vcard::read(bytes)),
        Kind::Calendar => Ok(calendar::read(bytes)),
    }
}

/// 8-bit text in the Windows Turkish code page (UTF-8 when it is valid UTF-8).
pub(crate) fn vcard_decode(bytes: &[u8]) -> String {
    vcard::decode_charset(bytes, Some("windows-1254"))
}

/// Text made safe to put in HTML.
pub(crate) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Plain text as HTML paragraphs (blank lines part them, single breaks are kept).
pub(crate) fn text_to_html(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    text.split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("<p>{}</p>", esc(p.trim_end()).replace('\n', "<br/>")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The inside of an HTML page's `<body>` (all of it when there is no body tag), with scripts,
/// styles and the head left out.
pub(crate) fn body_of(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<body").and_then(|i| lower[i..].find('>').map(|j| i + j + 1)).unwrap_or(0);
    let end = lower[start..].rfind("</body>").map(|i| start + i).unwrap_or(html.len());
    let mut body = html[start..end].to_string();
    // One pass per tag: a page of many thousand scripts takes no longer than its length.
    for tag in ["script", "style", "head"] {
        body = without_element(&body, tag);
    }
    body
}

/// `html` without its `<tag …>…</tag>` elements (to the end when one is not closed).
fn without_element(html: &str, tag: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let (open, close) = (format!("<{tag}"), format!("</{tag}>"));
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    while let Some(a) = lower[at..].find(&open).map(|i| at + i) {
        out.push_str(&html[at..a]);
        at = lower[a..].find(&close).map_or(html.len(), |j| a + j + close.len());
    }
    out.push_str(&html[at..]);
    out
}

/// Replaces each `src="…"` in `html` by what `map` gives for it (when it gives something).
pub(crate) fn rewrite_src(html: &str, mut map: impl FnMut(&str) -> Option<String>) -> String {
    // Lower-cased once: doing it again after every address made a page of many pictures slow.
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    // Copied to `out` up to `done`; the next `src=` is looked for from `from`.
    let (mut done, mut from) = (0, 0);
    while let Some(i) = lower[from..].find("src=").map(|i| from + i) {
        let Some(&q @ (b'"' | b'\'')) = html.as_bytes().get(i + 4) else {
            from = i + 4;
            continue;
        };
        let value_start = i + 5;
        let Some(len) = html[value_start..].find(char::from(q)) else { break };
        let value = &html[value_start..value_start + len];
        out.push_str(&html[done..value_start]);
        match map(value) {
            Some(new) => out.push_str(&new),
            None => out.push_str(value),
        }
        done = value_start + len;
        from = done;
    }
    out.push_str(&html[done..]);
    out
}

/// A picture as a `data:` address, for pages that carry their pictures inside.
pub(crate) fn data_uri(mime: &str, data: &[u8]) -> String {
    use base64::Engine;
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(data))
}

/// The picture type of `data` by its first bytes.
pub(crate) fn image_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if data.starts_with(&[0xFF, 0xD8]) {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF8") {
        Some("image/gif")
    } else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// The page around a body: a title and the body, in the word engine's HTML.
pub(crate) fn page(title: &str, body: &str) -> String {
    format!("<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body>{body}</body></html>", esc(title))
}

#[cfg(test)]
mod robustness;
#[cfg(test)]
mod tests;

#[cfg(test)]
#[global_allocator]
static ALLOC: mo_fuzz::CountingAlloc = mo_fuzz::CountingAlloc;
