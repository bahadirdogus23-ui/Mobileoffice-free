//! vCard contacts (.vcf, versions 2.1, 3.0 and 4.0): each contact as a heading and a table of
//! its phones, e-mails, addresses and the rest. Old phones write 2.1 with quoted-printable
//! text in their own charset; that is decoded too.

use crate::{Converted, esc, page};

/// One property line: its name, parameters (upper-cased keys) and value.
pub(crate) struct Prop {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: String,
}

/// The logical lines of a vCard or iCalendar file (folded lines joined), as properties.
pub(crate) fn props(bytes: &[u8]) -> Vec<Prop> {
    let text = decode_text(bytes);
    // Unfold: a line starting with a space or tab continues the previous one; a quoted-printable
    // line ending in '=' continues on the next.
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if (line.starts_with(' ') || line.starts_with('\t')) && !lines.is_empty() {
            lines.last_mut().unwrap().push_str(&line[1..]);
        } else if lines.last().is_some_and(|l| l.ends_with('=') && l.to_ascii_uppercase().contains("QUOTED-PRINTABLE")) {
            let last = lines.last_mut().unwrap();
            last.pop();
            last.push_str(line);
        } else {
            lines.push(line.to_string());
        }
    }
    lines
        .into_iter()
        .filter_map(|l| {
            let colon = find_value_colon(&l)?;
            let (head, value) = (&l[..colon], &l[colon + 1..]);
            let mut parts = head.split(';');
            // A group prefix ("item1.TEL") is dropped.
            let name = parts.next()?.rsplit('.').next()?.to_ascii_uppercase();
            let params: Vec<(String, String)> = parts
                .map(|p| match p.split_once('=') {
                    Some((k, v)) => (k.to_ascii_uppercase(), v.trim_matches('"').to_string()),
                    // vCard 2.1 writes bare types: "TEL;CELL:…".
                    None => ("TYPE".into(), p.to_string()),
                })
                .collect();
            let qp = params.iter().any(|(k, v)| k == "ENCODING" && v.eq_ignore_ascii_case("QUOTED-PRINTABLE"));
            let charset = params.iter().find(|(k, _)| k == "CHARSET").map(|(_, v)| v.clone());
            let value = if qp { decode_qp(value, charset.as_deref()) } else { value.to_string() };
            Some(Prop { name, params, value })
        })
        .collect()
}

/// The colon that starts a property's value (not one inside a quoted parameter).
fn find_value_colon(line: &str) -> Option<usize> {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ':' if !quoted => return Some(i),
            _ => {}
        }
    }
    None
}

/// The file's text: UTF-8 or UTF-16 (with or without a byte order mark), else the legacy
/// encoding it looks like, Turkish first (see [`crate::text`]).
fn decode_text(bytes: &[u8]) -> String {
    crate::text::decode_any(bytes)
}

/// Bytes in a named charset (UTF-8 first, as phones often write it whatever they declare);
/// without a known name, the legacy encoding they look like, Turkish first. Windows-1254's
/// quotes and dashes (0x80–0x9F) are read as such, not as control characters.
pub(crate) fn decode_charset(bytes: &[u8], charset: Option<&str>) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    charset.and_then(|cs| crate::text::decode_label(bytes, cs)).unwrap_or_else(|| crate::text::decode_any(bytes))
}

/// Quoted-printable text ("=C5=9E") decoded in `charset`.
fn decode_qp(value: &str, charset: Option<&str>) -> String {
    let b = value.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(v) = hex {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    decode_charset(&out, charset)
}

/// A value with vCard's escapes undone (\n, \, \; \\).
pub(crate) fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n' | 'N') => out.push('\n'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The kinds a property is marked with ("cell", "work"…), in Turkish.
fn kinds(p: &Prop) -> String {
    let mut out: Vec<&str> = Vec::new();
    for (k, v) in &p.params {
        if k != "TYPE" {
            continue;
        }
        for t in v.split(',') {
            let word = match t.to_ascii_lowercase().as_str() {
                "cell" | "mobile" => "cep",
                "work" => "iş",
                "home" => "ev",
                "fax" => "faks",
                "voice" | "pref" | "internet" | "x400" => continue,
                _ => continue,
            };
            if !out.contains(&word) {
                out.push(word);
            }
        }
    }
    out.join(", ")
}

pub(crate) fn read(bytes: &[u8]) -> Converted {
    let mut body = String::new();
    let mut count = 0;
    let mut card: Vec<Prop> = Vec::new();
    let mut inside = false;
    for p in props(bytes) {
        match (p.name.as_str(), p.value.to_ascii_uppercase().as_str()) {
            ("BEGIN", "VCARD") => {
                inside = true;
                card.clear();
            }
            ("END", "VCARD") => {
                inside = false;
                count += 1;
                body.push_str(&contact_html(&card));
            }
            _ if inside => card.push(p),
            _ => {}
        }
    }
    if count == 0 {
        body.push_str("<p>Bu dosyada kişi bulunamadı.</p>");
    }
    let title = if count == 1 { "Kişi".to_string() } else { format!("{count} kişi") };
    Converted { html: page(&title, &format!("<h1>{}</h1>{body}", esc(&title))), ..Default::default() }
}

fn contact_html(card: &[Prop]) -> String {
    let get = |n: &str| card.iter().find(|p| p.name == n).map(|p| unescape(&p.value));
    let name = get("FN").filter(|s| !s.trim().is_empty()).or_else(|| {
        // N: family;given;middle;prefix;suffix
        get("N").map(|n| {
            let parts: Vec<&str> = n.split(';').collect();
            let given = parts.get(1).copied().unwrap_or("");
            let family = parts.first().copied().unwrap_or("");
            format!("{given} {family}").trim().to_string()
        })
    });
    let mut rows = String::new();
    let mut row = |label: &str, value: &str| {
        if !value.trim().is_empty() {
            rows.push_str(&format!("<tr><td><b>{}</b></td><td>{}</td></tr>", esc(label), esc(value.trim()).replace('\n', "<br/>")));
        }
    };
    if let Some(org) = get("ORG") {
        row("Kurum", &org.replace(';', " · "));
    }
    if let Some(t) = get("TITLE") {
        row("Unvan", &t);
    }
    for p in card.iter().filter(|p| p.name == "TEL") {
        let k = kinds(p);
        row(&if k.is_empty() { "Telefon".to_string() } else { format!("Telefon ({k})") }, unescape(&p.value).trim_start_matches("tel:"));
    }
    for p in card.iter().filter(|p| p.name == "EMAIL") {
        row("E-posta", &unescape(&p.value));
    }
    for p in card.iter().filter(|p| p.name == "ADR") {
        // ADR: pobox;ext;street;city;region;code;country
        let v = unescape(&p.value);
        let parts: Vec<&str> = v.split(';').map(str::trim).filter(|s| !s.is_empty()).collect();
        let k = kinds(p);
        row(&if k.is_empty() { "Adres".to_string() } else { format!("Adres ({k})") }, &parts.join(", "));
    }
    if let Some(b) = get("BDAY") {
        row("Doğum günü", &b);
    }
    for p in card.iter().filter(|p| p.name == "URL") {
        row("Web", &unescape(&p.value));
    }
    if let Some(n) = get("NOTE") {
        row("Not", &n);
    }
    format!("<h2>{}</h2><table border=\"1\">{rows}</table>", esc(name.as_deref().unwrap_or("(adsız)")))
}
