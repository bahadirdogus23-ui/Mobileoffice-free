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

/// The message with its top-level header block in UTF-8 when it is raw 8-bit text (old mail
/// programs wrote "Subject: Toplantı" in Windows-1254 without RFC 2047 encoded words; the parser
/// reads raw header bytes as UTF-8 only).
fn headers_in_utf8(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").or_else(|| bytes.windows(2).position(|w| w == b"\n\n"));
    let Some(end) = end else { return bytes.into() };
    let head = &bytes[..end];
    if head.is_ascii() || std::str::from_utf8(head).is_ok() {
        return bytes.into();
    }
    let mut out = crate::text::decode_any(head).into_bytes();
    out.extend_from_slice(&bytes[end..]);
    out.into()
}

/// The text of a text or HTML part read again from its bytes when the parser's reading cannot be
/// trusted for Turkish: the part names no charset (the parser then reads UTF-8 and turns
/// Windows-1254 letters into "�"; an HTML part may name it in a `<meta>` instead) or names
/// ISO-8859-9 (which Turkish Windows programs write while using Windows-1254's quotes and
/// dashes, dropped as control characters otherwise). `None` keeps the parser's text.
fn reread_part(msg: &mail_parser::Message<'_>, part: &mail_parser::MessagePart<'_>) -> Option<String> {
    use mail_parser::decoders::{base64::base64_decode, quoted_printable::quoted_printable_decode};
    let html = matches!(part.body, mail_parser::PartType::Html(_));
    if !html && !matches!(part.body, mail_parser::PartType::Text(_)) {
        return None;
    }
    let charset = part.content_type().and_then(|c| c.attribute("charset")).map(str::to_ascii_lowercase);
    let declared = charset.as_deref().and_then(|c| encoding_rs::Encoding::for_label(c.trim().as_bytes()));
    if charset.is_some() && declared.is_none_or(|e| e != encoding_rs::WINDOWS_1254) {
        return None;
    }
    let raw = msg.raw_message().get(part.offset_body as usize..part.offset_end as usize)?;
    let body = match part.encoding {
        mail_parser::Encoding::QuotedPrintable => quoted_printable_decode(raw)?,
        mail_parser::Encoding::Base64 => base64_decode(raw)?,
        mail_parser::Encoding::None => raw.to_vec(),
    };
    if let Some(enc) = declared {
        return Some(enc.decode_without_bom_handling(&body).0.into_owned());
    }
    if html && let Some(enc) = crate::text::html_meta_charset(&body) {
        return Some(enc.decode_without_bom_handling(&body).0.into_owned());
    }
    crate::text::decode_undeclared(&body)
}

pub(crate) fn read_eml(bytes: &[u8]) -> Result<Converted, String> {
    let bytes = headers_in_utf8(bytes);
    let msg = MessageParser::default().parse(&bytes[..]).ok_or("E-posta okunamadı")?;
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
    let part = msg.html_part(0);
    let reread = part.and_then(|p| reread_part(&msg, p));
    let html = match (part.map(|p| &p.body), reread) {
        (Some(mail_parser::PartType::Html(_)), Some(html)) => Some(html.into()),
        (Some(mail_parser::PartType::Text(_)), Some(text)) => Some(text_to_html(&text).into()),
        _ => msg.body_html(0),
    };
    let body = match html {
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
    let props = stream("/__properties_version1.0").unwrap_or_default();
    // 8-bit strings are in the message's code page (PR_MESSAGE_CODEPAGE), its HTML body in the
    // internet code page (PR_INTERNET_CPID); either may be missing.
    let code_page = long_prop(&props, 0x3FFD).and_then(code_page_encoding);
    let internet_cp = long_prop(&props, 0x3FDE).and_then(code_page_encoding);
    let narrow = move |b: &[u8]| -> String {
        let b = b.split(|c| *c == 0).next().unwrap_or_default();
        match code_page {
            Some(enc) => crate::text::decode_declared(b, enc),
            None => crate::vcard_decode(b),
        }
    };
    // A string property: Unicode (001F) or 8-bit (001E).
    let mut prop = |id: &str| -> Option<String> {
        if let Some(b) = stream(&format!("/__substg1.0_{id}001F")) {
            return Some(utf16(b));
        }
        stream(&format!("/__substg1.0_{id}001E")).map(|b| narrow(&b))
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
    let html = stream("/__substg1.0_10130102").map(|b| match internet_cp.or_else(|| crate::text::html_meta_charset(&b)) {
        Some(enc) => crate::text::decode_declared(&b, enc),
        None => crate::text::decode_any(&b),
    });
    let date = submit_time(&props).unwrap_or_default();

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
        let name = s("__substg1.0_3707001F")
            .or_else(|| s("__substg1.0_3704001F"))
            .map(utf16)
            .or_else(|| s("__substg1.0_3707001E").or_else(|| s("__substg1.0_3704001E")).map(|b| narrow(&b)))
            .filter(|n| !n.is_empty());
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

/// A 32-bit integer property (type 0003) `id` from the top-level property stream (a 32-byte
/// header, then 16-byte entries: tag, flags, value).
fn long_prop(p: &[u8], id: u16) -> Option<u32> {
    let tag = (u32::from(id) << 16) | 0x0003;
    p.get(32..)?
        .as_chunks::<16>()
        .0
        .iter()
        .find(|e| u32::from_le_bytes([e[0], e[1], e[2], e[3]]) == tag)
        .map(|e| u32::from_le_bytes([e[8], e[9], e[10], e[11]]))
}

/// The text encoding of a Windows code page number, when it is one text uses.
fn code_page_encoding(cp: u32) -> Option<&'static encoding_rs::Encoding> {
    let label = match cp {
        65001 => "utf-8".to_string(),
        1200 => "utf-16le".to_string(),
        874 | 1250..=1258 => format!("windows-{cp}"),
        28591..=28599 | 28603 | 28605 => format!("iso-8859-{}", cp - 28590),
        20866 => "koi8-r".to_string(),
        932 => "shift_jis".to_string(),
        936 => "gbk".to_string(),
        949 => "euc-kr".to_string(),
        950 => "big5".to_string(),
        51932 => "euc-jp".to_string(),
        54936 => "gb18030".to_string(),
        _ => return None,
    };
    encoding_rs::Encoding::for_label(label.as_bytes())
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
