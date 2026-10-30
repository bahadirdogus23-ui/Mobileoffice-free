//! The text on PDF pages, with where each letter is: search that ignores Turkish dotted and
//! dotless i and accents, and finding personal data (Turkish ID numbers, IBANs, phone numbers,
//! e-mail addresses) to black out.

use std::ops::Range;

/// What can be found to black out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sensitive {
    /// T.C. kimlik numarası: 11 digits whose check digits are right.
    TurkishId,
    /// A Turkish IBAN: TR and 24 digits, spaces allowed between groups.
    Iban,
    /// A Turkish phone number: 10 digits starting 2–5, after 0, 90 or +90 if any.
    Phone,
    Email,
}

/// `c` for comparing: lower case, Turkish ı I İ as i, accents dropped (one char for one char,
/// so positions in the folded text are positions in the page text).
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

/// Where `query` occurs in `text`, ignoring case, Turkish i forms and accents.
pub fn find_folded(text: &[char], query: &str) -> Vec<Range<usize>> {
    let q: Vec<char> = query.trim().chars().map(fold).collect();
    if q.is_empty() {
        return Vec::new();
    }
    let t: Vec<char> = text.iter().map(|c| fold(*c)).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + q.len() <= t.len() {
        if t[i..i + q.len()] == q[..] {
            out.push(i..i + q.len());
            i += q.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Where `kind` occurs in `text`.
pub fn find_sensitive(kind: Sensitive, text: &[char]) -> Vec<Range<usize>> {
    match kind {
        Sensitive::TurkishId => {
            digit_runs(text, &[]).into_iter().filter(|(r, d)| d.len() == 11 && r.len() == 11 && turkish_id_ok(d)).map(|(r, _)| r).collect()
        }
        Sensitive::Iban => iban(text),
        Sensitive::Phone => digit_runs(text, &[' ', '-', '.', '(', ')'])
            .into_iter()
            .filter_map(|(r, d)| {
                let d: &[u8] = &d;
                let national = match d.len() {
                    10 => d,
                    11 if d[0] == 0 => &d[1..],
                    12 if d[..2] == [9, 0] => &d[2..],
                    _ => return None,
                };
                (2..=5).contains(&national[0]).then_some(r)
            })
            // "+90 …": take the plus too.
            .map(|r| if r.start > 0 && text[r.start - 1] == '+' { r.start - 1..r.end } else { r })
            .collect(),
        Sensitive::Email => pdfcraft_redact::patterns::find(pdfcraft_redact::patterns::Pattern::Email, text),
    }
}

/// Runs of digits, joined across single `seps` between digits, that no other digit or letter
/// touches: (range, the digits).
fn digit_runs(text: &[char], seps: &[char]) -> Vec<(Range<usize>, Vec<u8>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if !text[i].is_ascii_digit() || (i > 0 && text[i - 1].is_alphanumeric()) {
            i += 1;
            continue;
        }
        let start = i;
        let mut digits = Vec::new();
        let mut end = i;
        while i < text.len() {
            let c = text[i];
            if let Some(d) = c.to_digit(10).filter(|_| c.is_ascii_digit()) {
                digits.push(d as u8);
                i += 1;
                end = i;
            } else if (seps.contains(&c) && text.get(i + 1).is_some_and(|n| n.is_ascii_digit() || (c == ' ' && *n == '(')))
                || (seps.contains(&'(') && matches!(c, '(' | ')'))
            {
                i += 1;
            } else {
                break;
            }
            if digits.len() > 14 {
                break;
            }
        }
        if !text.get(end).is_some_and(|c| c.is_alphanumeric()) {
            out.push((start..end, digits));
        }
        i = end.max(start + 1);
    }
    out
}

/// The two check digits of a T.C. kimlik numarası.
pub fn turkish_id_ok(d: &[u8]) -> bool {
    if d.len() != 11 || d[0] == 0 {
        return false;
    }
    let n = |i: usize| i32::from(d[i]);
    let odd = n(0) + n(2) + n(4) + n(6) + n(8);
    let even = n(1) + n(3) + n(5) + n(7);
    let tenth = (odd * 7 - even).rem_euclid(10);
    tenth == n(9) && (0..10).map(n).sum::<i32>() % 10 == n(10)
}

fn iban(text: &[char]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < text.len() {
        let starts = text[i].eq_ignore_ascii_case(&'T') && text[i + 1].eq_ignore_ascii_case(&'R') && (i == 0 || !text[i - 1].is_alphanumeric());
        if !starts {
            i += 1;
            continue;
        }
        let mut j = i + 2;
        let mut digits = 0;
        let mut end = j;
        while j < text.len() && digits < 24 {
            if text[j].is_ascii_digit() {
                digits += 1;
                j += 1;
                end = j;
            } else if text[j] == ' ' && text.get(j + 1).is_some_and(char::is_ascii_digit) {
                j += 1;
            } else {
                break;
            }
        }
        if digits == 24 && !text.get(end).is_some_and(|c| c.is_alphanumeric()) {
            out.push(i..end);
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn found(kind: Sensitive, s: &str) -> Vec<String> {
        let t = chars(s);
        find_sensitive(kind, &t).into_iter().map(|r| t[r].iter().collect()).collect()
    }

    #[test]
    fn search_ignores_turkish_i_and_accents() {
        let t = chars("İSTANBUL'da ıslak Çiçek");
        assert_eq!(find_folded(&t, "istanbul").len(), 1);
        assert_eq!(find_folded(&t, "ISLAK").len(), 1);
        assert_eq!(find_folded(&t, "cicek"), vec![18..23]);
        assert!(find_folded(&t, "  ").is_empty());
    }

    #[test]
    fn finds_valid_turkish_ids_only() {
        // 10000000146 is a well-known valid test number.
        assert_eq!(found(Sensitive::TurkishId, "TC: 10000000146, yanlış: 10000000147"), ["10000000146"]);
        assert!(found(Sensitive::TurkishId, "A10000000146 100000001460").is_empty());
    }

    #[test]
    fn finds_ibans_with_or_without_spaces() {
        assert_eq!(
            found(Sensitive::Iban, "IBAN: TR33 0006 1005 1978 6457 8413 26 ve TR330006100519786457841326."),
            ["TR33 0006 1005 1978 6457 8413 26", "TR330006100519786457841326"]
        );
        assert!(found(Sensitive::Iban, "TR33 0006 1005").is_empty());
    }

    #[test]
    fn finds_turkish_phone_numbers() {
        assert_eq!(
            found(Sensitive::Phone, "Tel: 0532 123 45 67, +90 (212) 555-12-34; 5321234567. Sipariş 123456"),
            ["0532 123 45 67", "+90 (212) 555-12-34", "5321234567"]
        );
    }

    #[test]
    fn finds_email_addresses() {
        assert_eq!(found(Sensitive::Email, "yaz: ali.veli@ornek.com.tr lütfen"), ["ali.veli@ornek.com.tr"]);
    }
}
