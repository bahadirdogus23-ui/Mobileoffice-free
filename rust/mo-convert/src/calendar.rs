//! iCalendar files (.ics): the events and to-dos in date order, each with its time, place,
//! organiser, repetition and description.

use crate::vcard::{Prop, props, unescape};
use crate::{Converted, esc, page};

/// "20261108T093000Z", "20261108T093000" or "20261108" as Turkish date and time, with what it
/// sorts by.
fn when(p: &Prop) -> (String, String) {
    let v = p.value.trim();
    let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return (v.to_string(), v.to_string());
    }
    let (y, m, d) = (&digits[0..4], &digits[4..6], &digits[6..8]);
    let mut text = format!("{d}.{m}.{y}");
    if digits.len() >= 12 {
        text.push_str(&format!(" {}:{}", &digits[8..10], &digits[10..12]));
        if v.ends_with('Z') {
            text.push_str(" (UTC)");
        } else if let Some((_, tz)) = p.params.iter().find(|(k, _)| k == "TZID") {
            text.push_str(&format!(" ({tz})"));
        }
    }
    (text, digits)
}

/// A repetition rule ("FREQ=WEEKLY;BYDAY=MO") in a few Turkish words.
fn repeat(rule: &str) -> String {
    let mut out = Vec::new();
    for part in rule.split(';') {
        match part.split_once('=') {
            Some(("FREQ", f)) => out.push(
                match f {
                    "DAILY" => "her gün",
                    "WEEKLY" => "her hafta",
                    "MONTHLY" => "her ay",
                    "YEARLY" => "her yıl",
                    other => other,
                }
                .to_string(),
            ),
            Some(("INTERVAL", n)) if n != "1" => out.push(format!("{n} aralıkla")),
            Some(("COUNT", n)) => out.push(format!("{n} kez")),
            Some(("UNTIL", u)) => {
                let d: String = u.chars().filter(|c| c.is_ascii_digit()).collect();
                if d.len() >= 8 {
                    out.push(format!("{}.{}.{} tarihine kadar", &d[6..8], &d[4..6], &d[0..4]));
                }
            }
            _ => {}
        }
    }
    out.join(", ")
}

pub(crate) fn read(bytes: &[u8]) -> Converted {
    let mut items: Vec<(String, String)> = Vec::new(); // (sort key, html)
    let mut current: Option<(String, Vec<Prop>)> = None;
    let mut depth_other = 0;
    for p in props(bytes) {
        let upper = p.value.trim().to_ascii_uppercase();
        match p.name.as_str() {
            "BEGIN" if (upper == "VEVENT" || upper == "VTODO") && current.is_none() => current = Some((upper, Vec::new())),
            // Alarms and the like inside an event are not shown.
            "BEGIN" if current.is_some() => depth_other += 1,
            "END" if depth_other > 0 => depth_other -= 1,
            "END" if upper == "VEVENT" || upper == "VTODO" => {
                if let Some((kind, props)) = current.take() {
                    items.push(item_html(&kind, &props));
                }
            }
            _ => {
                if let Some((_, list)) = current.as_mut()
                    && depth_other == 0
                {
                    list.push(p);
                }
            }
        }
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let title = match items.len() {
        0 => "Takvim".to_string(),
        1 => "1 etkinlik".to_string(),
        n => format!("{n} etkinlik"),
    };
    let body = if items.is_empty() {
        "<p>Bu dosyada etkinlik bulunamadı.</p>".to_string()
    } else {
        items.into_iter().map(|(_, h)| h).collect::<Vec<_>>().join("\n")
    };
    Converted { html: page(&title, &format!("<h1>{}</h1>{body}", esc(&title))), ..Default::default() }
}

fn item_html(kind: &str, props: &[Prop]) -> (String, String) {
    let get = |n: &str| props.iter().find(|p| p.name == n);
    let summary = get("SUMMARY").map(|p| unescape(&p.value)).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "(başlıksız)".into());
    let (start, key) = get("DTSTART").map(when).unwrap_or_default();
    let mut rows = String::new();
    let mut row = |label: &str, value: &str| {
        if !value.trim().is_empty() {
            rows.push_str(&format!("<tr><td><b>{}</b></td><td>{}</td></tr>", esc(label), esc(value.trim()).replace('\n', "<br/>")));
        }
    };
    if kind == "VTODO" {
        row("Tür", "Yapılacak");
    }
    row("Başlangıç", &start);
    if let Some(end) = get("DTEND").or_else(|| get("DUE")).map(when) {
        row(if kind == "VTODO" { "Son tarih" } else { "Bitiş" }, &end.0);
    }
    if let Some(l) = get("LOCATION") {
        row("Yer", &unescape(&l.value));
    }
    if let Some(o) = get("ORGANIZER") {
        let name = o.params.iter().find(|(k, _)| k == "CN").map(|(_, v)| v.clone());
        let mail = o.value.trim_start_matches("mailto:").trim_start_matches("MAILTO:").to_string();
        row("Düzenleyen", &name.map(|n| format!("{n} <{mail}>")).unwrap_or(mail));
    }
    let attendees: Vec<String> = props
        .iter()
        .filter(|p| p.name == "ATTENDEE")
        .map(|p| {
            p.params.iter().find(|(k, _)| k == "CN").map(|(_, v)| v.clone()).unwrap_or_else(|| p.value.trim_start_matches("mailto:").to_string())
        })
        .collect();
    if !attendees.is_empty() {
        row("Katılımcılar", &attendees.join(", "));
    }
    if let Some(r) = get("RRULE") {
        row("Tekrar", &repeat(&r.value));
    }
    if let Some(s) = get("STATUS") {
        row(
            "Durum",
            match s.value.trim() {
                "CONFIRMED" => "onaylandı",
                "TENTATIVE" => "kesin değil",
                "CANCELLED" => "iptal edildi",
                "COMPLETED" => "tamamlandı",
                other => other,
            },
        );
    }
    if let Some(d) = get("DESCRIPTION") {
        row("Açıklama", &unescape(&d.value));
    }
    (key, format!("<h2>{}</h2><table border=\"1\">{rows}</table>", esc(&summary)))
}
