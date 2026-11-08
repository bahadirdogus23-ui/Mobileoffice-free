//! E-mails: EML (MIME, RFC 5322; also .mht web archives) and Outlook MSG. The page shows who
//! sent it, to whom, when and about what, then the message itself (its HTML with the pictures it
//! carries, or its text); attachments are listed and handed back to be opened.

use std::io::Read;

use mail_parser::{MessageParser, MimeHeaders};

use crate::{Attachment, Converted, body_of, data_uri, esc, image_mime, page, rewrite_src, text_to_html};

/// The most attachments kept, and the largest (bytes).
const MAX_ATTACHMENTS: usize = 100;
const MAX_ATTACHMENT: usize = 64 * 1024 * 1024;

/// The header table at the top of the page.
fn header_html(subject: &str, rows: &[(&str, String)]) -> String {
    let mut h = format!("<h1>{}</h1><table>", esc(if subject.trim().is_empty() { "(konusuz)" } else { subject }));
    for (label, value) in rows {
        if !value.trim().is_empty() {
            h.push_str(&format!("<tr><td><b>{}</b></td><td>{}</td></tr>", esc(label), esc(value)));
        }
    }
    h.push_str("</table><hr/>");
    h
}

fn attachments_html(list: &[Attachment]) -> String {
    if list.is_empty() {
        return String::new();
    }
    let mut h = String::from("<hr/><h2>Ekler</h2><ul>");
    for a in list {
        h.push_str(&format!("<li>{} ({})</li>", esc(&a.name), size_text(a.data.len())));
    }
    h.push_str("</ul>");
    h
}

fn size_text(n: usize) -> String {
    if n >= 1024 * 1024 { format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)) } else { format!("{} KB", n.div_ceil(1024)) }
}

pub(crate) fn read_eml(bytes: &[u8]) -> Result<Converted, String> {
    let msg = MessageParser::default().parse(bytes).ok_or("E-posta okunamadı")?;
    let addr = |a: Option<&mail_parser::Address>| -> String {
        a.map(|list| {
            list.iter()
                .map(|m| match (m.name(), m.address()) {
                    (Some(n), Some(e)) => format!("{n} <{e}>"),
                    (None, Some(e)) => e.to_string(),
                    (Some(n), None) => n.to_string(),
                    (None, None) => String::new(),
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
    };
    let date = msg
        .date()
        .map(|d| {
            let sign = if d.tz_before_gmt { '-' } else { '+' };
            format!("{:02}.{:02}.{} {:02}:{:02} (GMT{sign}{:02}:{:02})", d.day, d.month, d.year, d.hour, d.minute, d.tz_hour, d.tz_minute)
        })
        .unwrap_or_default();
    let subject = msg.subject().unwrap_or("").to_string();
    let head = header_html(&subject, &[("Kimden", addr(msg.from())), ("Kime", addr(msg.to())), ("Bilgi", addr(msg.cc())), ("Tarih", date)]);

    // Attachments, and the pictures the HTML points at by content id.
    let mut attachments = Vec::new();
    let mut inline: Vec<(String, String)> = Vec::new();
    for part in msg.attachments().take(MAX_ATTACHMENTS) {
        let data = part.contents();
        if data.len() > MAX_ATTACHMENT {
            continue;
        }
        if let (Some(cid), Some(mime)) = (part.content_id(), image_mime(data)) {
            inline.push((cid.trim_matches(['<', '>']).to_string(), data_uri(mime, data)));
            if part.attachment_name().is_none() {
                continue;
            }
        }
        let name = part.attachment_name().map(str::to_string).unwrap_or_else(|| format!("ek-{}", attachments.len() + 1));
        attachments.push(Attachment { name, data: data.to_vec() });
    }
    let body = match msg.body_html(0) {
        Some(html) => rewrite_src(&body_of(&html), |src| {
            let cid = src.strip_prefix("cid:")?;
            inline.iter().find(|(c, _)| c == cid).map(|(_, uri)| uri.clone())
        }),
        None => text_to_html(&msg.body_text(0).unwrap_or_default()),
    };
    let html = page(&subject, &format!("{head}{body}{}", attachments_html(&attachments)));
    let mut notes = vec!["E-posta: uzaktaki resimler gösterilmez (uygulama internete bağlanmaz).".to_string()];
    if !attachments.is_empty() {
        notes.push(format!("{} ek var; Ekler menüsünden açılabilir.", attachments.len()));
    }
    Ok(Converted { html, notes, attachments })
}

/// An Outlook .msg (a compound file of MAPI properties): subject, people, date, body and attachments.
pub(crate) fn read_msg(bytes: &[u8]) -> Result<Converted, String> {
    let mut file = cfb::CompoundFile::open(std::io::Cursor::new(bytes)).map_err(|e| format!("MSG: {e}"))?;
    let mut stream = |path: &str| -> Option<Vec<u8>> {
        let mut s = file.open_stream(path).ok()?;
        let mut out = Vec::new();
        (&mut s).take(MAX_ATTACHMENT as u64).read_to_end(&mut out).ok()?;
        Some(out)
    };
    let utf16 = |b: Vec<u8>| -> String {
        let units: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
        String::from_utf16_lossy(&units).trim_end_matches('\0').to_string()
    };
    // A string property: Unicode (001F) or 8-bit (001E, Windows Turkish).
    let mut prop = |id: &str| -> Option<String> {
        if let Some(b) = stream(&format!("/__substg1.0_{id}001F")) {
            return Some(utf16(b));
        }
        stream(&format!("/__substg1.0_{id}001E")).map(|b| crate::vcard_decode(&b))
    };
    let subject = prop("0037").unwrap_or_default();
    let from = match (prop("0C1A"), prop("5D01").or_else(|| prop("0C1F"))) {
        (Some(n), Some(e)) if !e.is_empty() && e != n => format!("{n} <{e}>"),
        (Some(n), _) => n,
        (None, Some(e)) => e,
        _ => String::new(),
    };
    let to = prop("0E04").unwrap_or_default();
    let cc = prop("0E03").unwrap_or_default();
    let text = prop("1000");
    let html = stream("/__substg1.0_10130102").map(|b| String::from_utf8_lossy(&b).into_owned());
    let date = stream("/__properties_version1.0").and_then(|p| submit_time(&p)).unwrap_or_default();

    // Attachments: one storage each.
    let mut attachments = Vec::new();
    let mut inline: Vec<(String, String)> = Vec::new();
    let storages: Vec<String> = file
        .read_root_storage()
        .filter(|e| e.is_storage() && e.name().starts_with("__attach_version1.0_"))
        .map(|e| e.path().to_string_lossy().into_owned())
        .take(MAX_ATTACHMENTS)
        .collect();
    for dir in storages {
        let mut s = |name: &str| -> Option<Vec<u8>> {
            let mut st = file.open_stream(format!("{dir}/{name}")).ok()?;
            let mut out = Vec::new();
            (&mut st).take(MAX_ATTACHMENT as u64).read_to_end(&mut out).ok()?;
            Some(out)
        };
        let Some(data) = s("__substg1.0_37010102") else { continue };
        let name = s("__substg1.0_3707001F").or_else(|| s("__substg1.0_3704001F")).map(utf16).filter(|n| !n.is_empty());
        if let (Some(cid), Some(mime)) = (s("__substg1.0_3712001F").map(utf16), image_mime(&data)) {
            inline.push((cid, data_uri(mime, &data)));
        }
        attachments.push(Attachment { name: name.unwrap_or_else(|| format!("ek-{}", attachments.len() + 1)), data });
    }
    let head = header_html(&subject, &[("Kimden", from), ("Kime", to), ("Bilgi", cc), ("Tarih", date)]);
    let body = match html.filter(|h| !h.trim().is_empty()) {
        Some(h) => rewrite_src(&body_of(&h), |src| {
            let cid = src.strip_prefix("cid:")?;
            inline.iter().find(|(c, _)| c == cid).map(|(_, uri)| uri.clone())
        }),
        None => text_to_html(&text.unwrap_or_default()),
    };
    let notes = vec!["Outlook iletisi (.msg): metin, kişiler ve ekler aktarıldı; RTF biçimli gövdeler düz metin olarak gösterilir.".to_string()];
    Ok(Converted { html: page(&subject, &format!("{head}{body}{}", attachments_html(&attachments))), notes, attachments })
}

/// The message's submit time (PR_CLIENT_SUBMIT_TIME, 0x0039, a FILETIME) from the top-level
/// property stream (a 32-byte header, then 16-byte entries).
fn submit_time(p: &[u8]) -> Option<String> {
    for entry in p.get(32..)?.as_chunks::<16>().0 {
        let tag = u32::from_le_bytes(entry[0..4].try_into().ok()?);
        if tag == 0x0039_0040 || tag == 0x0E06_0040 {
            let ft = u64::from_le_bytes(entry[8..16].try_into().ok()?);
            // 100 ns steps since 1601 → seconds since 1970.
            let secs = (ft / 10_000_000).checked_sub(11_644_473_600)? as i64;
            let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
            let (y, m, d) = civil(days);
            return Some(format!("{d:02}.{m:02}.{y} {:02}:{:02} (UTC)", rem / 3600, rem % 3600 / 60));
        }
    }
    None
}

/// Year, month, day of `days` since 1970-01-01 (proleptic Gregorian).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}
