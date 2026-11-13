//! Whether a zip package (DOCX, ODT, PPTX) can be read whole: every entry inflates with a good
//! checksum, and every XML part an engine parses is well-formed to its end and within that
//! engine's limits.
//!
//! The engines are lenient on purpose: a damaged entry is skipped, an XML part cut short is read
//! up to the cut, an unreadable secondary part (styles, notes, comments, a slide master…) is left
//! out, each with no more than a log line. The document still opens, but saving it over the
//! original file would make the loss permanent (AGENTS.md invariant 4). The bridges run this
//! check on opening so the app can offer only "Save as" for such a file.

use std::io::{Cursor, Read};

/// The limits an engine's XML reader puts on one part, and how lenient it is.
#[derive(Clone, Copy, Debug)]
pub struct XmlLimits {
    /// Most elements (start and empty tags) in one part.
    pub max_elements: usize,
    /// Deepest element nesting.
    pub max_depth: usize,
    /// Most attributes on one element (not counting namespace declarations), when limited.
    pub max_attrs: Option<usize>,
    /// Largest part, in bytes.
    pub max_bytes: u64,
    /// End tags that close nothing are skipped instead of failing the part.
    pub allow_unmatched_ends: bool,
}

/// Whether an engine with `limits` reads `xml` whole: no syntax error before the root element
/// ends, every element closed, and within the limits. End tag names are not compared, as the
/// engines do not compare them either.
pub fn xml_reads_whole(xml: &[u8], limits: &XmlLimits) -> bool {
    if xml.len() as u64 > limits.max_bytes {
        return false;
    }
    let transcoded = utf16_to_utf8(xml);
    let bytes = match &transcoded {
        Some(s) => s.as_bytes(),
        None => xml.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(xml),
    };
    let mut r = quick_xml::Reader::from_reader(bytes);
    {
        let c = r.config_mut();
        c.trim_text(false);
        c.check_end_names = false;
        c.allow_unmatched_ends = limits.allow_unmatched_ends;
        c.expand_empty_elements = false;
    }
    let mut depth = 0usize;
    let mut elements = 0usize;
    let mut root = false;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let Ok(ev) = r.read_event_into(&mut buf) else { return false };
        use quick_xml::events::Event;
        let (start, empty) = match &ev {
            Event::Start(e) => (Some(e), false),
            Event::Empty(e) => (Some(e), true),
            _ => (None, false),
        };
        if let Some(e) = start.filter(|_| !root || depth > 0) {
            elements += 1;
            if elements > limits.max_elements {
                return false;
            }
            if let Some(max) = limits.max_attrs {
                let attrs = e.attributes().with_checks(false).flatten().filter(|a| !a.key.as_ref().starts_with(b"xmlns")).count();
                if attrs > max {
                    return false;
                }
            }
            root = true;
            if !empty {
                if depth >= limits.max_depth {
                    return false;
                }
                depth += 1;
            } else if depth == 0 {
                // An empty root element: the whole part.
                return true;
            }
            continue;
        }
        match ev {
            Event::End(_) if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    // The engines stop at the root element's end.
                    return true;
                }
            }
            Event::Text(t) if depth > 0 => {
                if t.decode().is_err() {
                    return false;
                }
            }
            // Cut short: the engines close what is open and keep only what came before.
            Event::Eof => return false,
            _ => {}
        }
    }
}

/// The entries of the zip `bytes` that an engine would skip or read only in part: those that do
/// not inflate (a bad checksum, a damaged stream, an unknown method) and the XML parts `is_xml`
/// selects (by lower-case name) that [`xml_reads_whole`] refuses. A package that is not a zip at
/// all is reported as one entry named `*`.
pub fn damaged_entries(bytes: &[u8], is_xml: impl Fn(&str) -> bool, limits: &XmlLimits) -> Vec<String> {
    let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(bytes)) else { return vec!["*".to_string()] };
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut f = match zip.by_index(i) {
            Ok(f) => f,
            Err(_) => {
                out.push(format!("#{i}"));
                continue;
            }
        };
        if f.is_dir() {
            continue;
        }
        let name = f.name().trim_start_matches('/').to_string();
        let ok = if is_xml(&name.to_ascii_lowercase()) {
            let mut xml = Vec::new();
            // Reading to the end checks the entry's CRC.
            match (&mut f).take(limits.max_bytes.saturating_add(1)).read_to_end(&mut xml) {
                Ok(_) => xml_reads_whole(&xml, limits),
                Err(_) => false,
            }
        } else {
            std::io::copy(&mut f, &mut std::io::sink()).is_ok()
        };
        if !ok {
            out.push(name);
        }
    }
    out
}

/// UTF-16 text with a byte-order mark, as UTF-8 (as the engines read such parts).
fn utf16_to_utf8(bytes: &[u8]) -> Option<String> {
    let (le, rest) = match bytes {
        [0xFF, 0xFE, rest @ ..] => (true, rest),
        [0xFE, 0xFF, rest @ ..] => (false, rest),
        _ => return None,
    };
    let units = rest.as_chunks::<2>().0.iter().map(|c| if le { u16::from_le_bytes(*c) } else { u16::from_be_bytes(*c) });
    Some(char::decode_utf16(units).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const LIMITS: XmlLimits = XmlLimits { max_elements: 10, max_depth: 3, max_attrs: Some(2), max_bytes: 1 << 20, allow_unmatched_ends: false };

    #[test]
    fn whole_xml_is_told_from_cut_or_damaged_xml() {
        assert!(xml_reads_whole(b"<?xml version=\"1.0\"?><a x=\"1\"><b>t</b><c/></a>", &LIMITS));
        assert!(xml_reads_whole(b"\xEF\xBB\xBF<a/>", &LIMITS));
        // Misnamed end tags pass, as in the engines; what follows the root is ignored.
        assert!(xml_reads_whole(b"<a><b></c></a>junk<", &LIMITS));
        // Cut short, broken, empty, past the limits.
        assert!(!xml_reads_whole(b"<a><b>text", &LIMITS));
        assert!(!xml_reads_whole(b"<a><b x=\"1></b></a>", &LIMITS));
        assert!(!xml_reads_whole(b"", &LIMITS));
        assert!(xml_reads_whole(b"<a><b><c><d/></c></b></a>", &LIMITS));
        assert!(!xml_reads_whole(b"<a><b><c><d></d></c></b></a>", &LIMITS));
        assert!(!xml_reads_whole(b"<a x=\"1\" y=\"2\" z=\"3\"/>", &LIMITS));
        assert!(xml_reads_whole(b"<a xmlns=\"u\" xmlns:p=\"v\" x=\"1\" y=\"2\"/>", &LIMITS));
        assert!(!xml_reads_whole(&[b"<a>".as_slice(), &b"<b/>".repeat(10), b"</a>"].concat(), &LIMITS));
        // UTF-16 with a byte-order mark.
        let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain("<a>ş</a>".encode_utf16().flat_map(u16::to_le_bytes)).collect();
        assert!(xml_reads_whole(&utf16, &LIMITS));
    }

    fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn damaged_entries_are_named() {
        let good = zip_of(&[("a.xml", b"<a/>"), ("b.bin", b"0123456789")]);
        assert!(damaged_entries(&good, |n| n.ends_with(".xml"), &LIMITS).is_empty());

        let cut = zip_of(&[("a.xml", b"<a><b>"), ("b.bin", b"0123456789")]);
        assert_eq!(damaged_entries(&cut, |n| n.ends_with(".xml"), &LIMITS), ["a.xml"]);
        // Not selected as XML: only its checksum matters.
        assert!(damaged_entries(&cut, |_| false, &LIMITS).is_empty());

        // One byte of the stored data changed: its checksum no longer matches.
        let mut bad = good.clone();
        let at = bad.windows(10).position(|w| w == b"0123456789").unwrap();
        bad[at] = b'X';
        assert_eq!(damaged_entries(&bad, |n| n.ends_with(".xml"), &LIMITS), ["b.bin"]);

        assert_eq!(damaged_entries(b"not a zip", |_| true, &LIMITS), ["*"]);
    }
}
