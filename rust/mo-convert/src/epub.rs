//! EPUB e-books: the chapters in reading order (the spine), with their pictures carried inside
//! the page, as one HTML document. DRM-protected books cannot be read and say so.

use std::io::Read;

use crate::{Converted, body_of, data_uri, esc, image_mime, page, rewrite_src};

/// What one part of the book may unpack to (32 MB) and the most chapters read.
const MAX_PART: u64 = 32 * 1024 * 1024;
const MAX_CHAPTERS: usize = 2000;

pub(crate) fn read(bytes: &[u8]) -> Result<Converted, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("EPUB: {e}"))?;
    let part = |zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str| -> Option<Vec<u8>> {
        let mut f = zip.by_name(name).ok()?;
        if f.size() > MAX_PART {
            return None;
        }
        let mut out = Vec::new();
        f.read_to_end(&mut out).ok()?;
        Some(out)
    };
    if zip.by_name("META-INF/encryption.xml").is_ok_and(|f| f.size() > 0) {
        // Font obfuscation also uses encryption.xml; only a book whose chapters are encrypted is refused below.
    }
    let container = part(&mut zip, "META-INF/container.xml").ok_or("EPUB: no container.xml")?;
    let container = String::from_utf8_lossy(&container).into_owned();
    let doc = roxmltree::Document::parse(&container).map_err(|e| format!("EPUB: {e}"))?;
    let opf_path =
        doc.descendants().find(|n| n.has_tag_name("rootfile")).and_then(|n| n.attribute("full-path")).ok_or("EPUB: no package file")?.to_string();
    let base = opf_path.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
    let opf = part(&mut zip, &opf_path).ok_or("EPUB: the package file is missing")?;
    let opf = String::from_utf8_lossy(&opf).into_owned();
    let opf = roxmltree::Document::parse(&opf).map_err(|e| format!("EPUB: {e}"))?;
    let title = opf.descendants().find(|n| n.tag_name().name() == "title").and_then(|n| n.text()).unwrap_or("E-kitap").trim().to_string();
    let author = opf.descendants().find(|n| n.tag_name().name() == "creator").and_then(|n| n.text()).map(str::trim).map(str::to_string);
    let manifest: Vec<(String, String, String)> = opf
        .descendants()
        .filter(|n| n.tag_name().name() == "item")
        .filter_map(|n| Some((n.attribute("id")?.to_string(), n.attribute("href")?.to_string(), n.attribute("media-type").unwrap_or("").to_string())))
        .collect();
    let spine: Vec<String> = opf
        .descendants()
        .filter(|n| n.tag_name().name() == "itemref")
        .filter(|n| n.attribute("linear") != Some("no"))
        .filter_map(|n| n.attribute("idref").map(str::to_string))
        .collect();

    let mut body = format!("<h1>{}</h1>", esc(&title));
    if let Some(a) = &author {
        body.push_str(&format!("<p><i>{}</i></p>", esc(a)));
    }
    let mut chapters = 0;
    let mut unreadable = 0;
    for id in spine.iter().take(MAX_CHAPTERS) {
        let Some((_, href, _)) = manifest.iter().find(|(i, _, _)| i == id) else { continue };
        let path = resolve(&base, href);
        let Some(data) = part(&mut zip, &path) else {
            unreadable += 1;
            continue;
        };
        let html = String::from_utf8_lossy(&data).into_owned();
        if !html.contains('<') {
            // Encrypted (DRM) chapters are not text.
            unreadable += 1;
            continue;
        }
        let dir = path.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
        let inner = body_of(&html);
        // Pictures go inside the page: the word engine reads no other files.
        let inner = rewrite_src(&inner, |src| {
            if src.starts_with("data:") {
                return None;
            }
            let p = resolve(&dir, src.split('#').next().unwrap_or(src));
            let data = part(&mut zip, &p)?;
            let mime = image_mime(&data)?;
            Some(data_uri(mime, &data))
        });
        if chapters > 0 {
            // Each chapter starts on a new page, as in the book.
            body.push_str("<p style=\"page-break-before: always\"></p>");
        }
        body.push_str(&inner);
        chapters += 1;
    }
    if chapters == 0 {
        return Err("EPUB: no readable chapters (the book may be protected with DRM)".into());
    }
    let mut notes = vec![format!("E-kitap: {chapters} bölüm okundu; yazı tipleri ve bazı biçimler kitabınkiyle aynı olmayabilir.")];
    if unreadable > 0 {
        notes.push(format!("{unreadable} bölüm okunamadı (korumalı ya da eksik)."));
    }
    Ok(Converted { html: page(&title, &body), notes, attachments: Vec::new() })
}

/// `href` (relative, maybe with "../" and %-escapes) against the folder `dir` ("OEBPS/text/").
fn resolve(dir: &str, href: &str) -> String {
    let href = percent_decode(href);
    if let Some(abs) = href.strip_prefix('/') {
        return abs.to_string();
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in href.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
