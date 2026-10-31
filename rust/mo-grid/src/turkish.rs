//! Turkish spreadsheet conventions at the edges of the engine, as Excel in Turkish has them: a
//! decimal comma and a dot between thousands ("1.250,5"), `;` between a function's arguments, and
//! Turkish function names (`=TOPLA(A1;A2)`). The engine, its files and other programs keep the
//! English forms; these functions turn what a person types into them and back for showing.

/// Turkish function names and the English ones the engine and XLSX files use.
const NAMES: &[(&str, &str)] = &[
    ("TOPLA", "SUM"),
    ("ORTALAMA", "AVERAGE"),
    ("EĞER", "IF"),
    ("EĞERHATA", "IFERROR"),
    ("EĞERSAY", "COUNTIF"),
    ("ÇOKEĞERSAY", "COUNTIFS"),
    ("ETOPLA", "SUMIF"),
    ("ÇOKETOPLA", "SUMIFS"),
    ("EĞERORTALAMA", "AVERAGEIF"),
    ("DÜŞEYARA", "VLOOKUP"),
    ("YATAYARA", "HLOOKUP"),
    ("İNDİS", "INDEX"),
    ("KAÇINCI", "MATCH"),
    ("BUGÜN", "TODAY"),
    ("ŞİMDİ", "NOW"),
    ("TARİH", "DATE"),
    ("YIL", "YEAR"),
    ("AY", "MONTH"),
    ("GÜN", "DAY"),
    ("MAK", "MAX"),
    ("MİN", "MIN"),
    ("BAĞ_DEĞ_SAY", "COUNT"),
    ("BAĞ_DEĞ_DOLU_SAY", "COUNTA"),
    ("BOŞLUKSAY", "COUNTBLANK"),
    ("YUVARLA", "ROUND"),
    ("AŞAĞIYUVARLA", "ROUNDDOWN"),
    ("YUKARIYUVARLA", "ROUNDUP"),
    ("TAMSAYI", "INT"),
    ("MUTLAK", "ABS"),
    ("KAREKÖK", "SQRT"),
    ("ÇARPIM", "PRODUCT"),
    ("TOPLAÇARPIM", "SUMPRODUCT"),
    ("ORTANCA", "MEDIAN"),
    ("VE", "AND"),
    ("YADA", "OR"),
    ("DEĞİL", "NOT"),
    ("BİRLEŞTİR", "CONCATENATE"),
    ("UZUNLUK", "LEN"),
    ("SOLDAN", "LEFT"),
    ("SAĞDAN", "RIGHT"),
    ("PARÇAAL", "MID"),
    ("BÜYÜKHARF", "UPPER"),
    ("KÜÇÜKHARF", "LOWER"),
    ("KIRP", "TRIM"),
    ("METNEÇEVİR", "TEXT"),
    ("DEĞER", "VALUE"),
    ("YERİNEKOY", "SUBSTITUTE"),
    ("BUL", "FIND"),
    ("MOD", "MOD"),
    ("ÜS", "POWER"),
    ("PMT", "PMT"),
    ("DEVRE_FAİZORANI", "RATE"),
];

/// Logical constants, written without parentheses.
const CONSTANTS: &[(&str, &str)] = &[("DOĞRU", "TRUE"), ("YANLIŞ", "FALSE")];

/// Function names a person may type, for suggestions: the Turkish names when `turkish`, else the
/// English ones, in alphabetical order.
pub fn function_names(turkish: bool) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = NAMES.iter().map(|(t, e)| if turkish { *t } else { *e }).collect();
    if turkish {
        names.sort_by_key(|n| collation_key(n));
    } else {
        names.sort_unstable();
    }
    names.dedup();
    names
}

/// Upper case the Turkish way (i → İ, ı → I), so "eğer" and "EĞER" are the same name.
fn upper_tr(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'i' => vec!['İ'],
            'ı' => vec!['I'],
            c => c.to_uppercase().collect(),
        })
        .collect()
}

fn english_name(tr: &str) -> Option<&'static str> {
    let up = upper_tr(tr);
    NAMES.iter().chain(CONSTANTS).find(|(t, _)| *t == up).map(|(_, e)| *e)
}

fn turkish_name(en: &str) -> Option<&'static str> {
    let up = en.to_ascii_uppercase();
    NAMES.iter().chain(CONSTANTS).find(|(_, e)| *e == up).map(|(t, _)| *t)
}

/// What a person typed in Turkish style as the engine takes it: a formula with English names,
/// `,` between arguments and decimal points, or a number with a decimal point. Text, dates
/// ("31.10.2026") and anything else stay as typed.
pub fn to_engine(input: &str) -> String {
    if let Some(body) = input.strip_prefix('=') {
        return format!("={}", formula(body, true));
    }
    number_to_engine(input).unwrap_or_else(|| input.to_string())
}

/// What the engine stores, shown for editing in Turkish style.
pub fn from_engine(stored: &str) -> String {
    if let Some(body) = stored.strip_prefix('=') {
        return format!("={}", formula(body, false));
    }
    let number = stored.strip_suffix('%').unwrap_or(stored);
    if !number.is_empty() && number.parse::<f64>().is_ok() && !number.contains(['e', 'E']) {
        return stored.replace('.', ",");
    }
    stored.to_string()
}

/// "1.250,5", "1250,5", "-12,5%" as "1250.5", "1250.5", "-12.5%"; None for anything else.
fn number_to_engine(input: &str) -> Option<String> {
    let t = input.trim();
    let (body, percent) = match t.strip_suffix('%') {
        Some(b) => (b.trim_end(), "%"),
        None => (t, ""),
    };
    let (sign, digits) = match body.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", body),
    };
    let (whole, frac) = match digits.split_once(',') {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    if whole.is_empty() || frac.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    // Thousands dots must group by three after the first group ("1.250.000"), so that a date
    // like "31.10.2026" is never read as a number.
    let groups: Vec<&str> = whole.split('.').collect();
    let grouped = groups.len() > 1;
    let ok = groups.iter().all(|g| !g.is_empty() && g.bytes().all(|b| b.is_ascii_digit()))
        && (!grouped || (groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3)));
    if !ok || (frac.is_none() && !grouped) {
        // Plain digits need no change; let the engine read them as typed.
        return None;
    }
    let mut out = format!("{sign}{}", groups.concat());
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    out.push_str(percent);
    Some(out)
}

/// A formula body between the Turkish and English forms (`to_english` picks the direction):
/// function names, argument separators and decimal separators, leaving text in quotes alone.
fn formula(body: &str, to_english: bool) -> String {
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    let digit = |j: usize| chars.get(j).is_some_and(char::is_ascii_digit);
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            // A string literal, with "" for a quote inside it.
            out.push(c);
            i += 1;
            while i < chars.len() {
                out.push(chars[i]);
                if chars[i] == '"' {
                    if chars.get(i + 1) == Some(&'"') {
                        out.push('"');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let call = chars.get(i) == Some(&'(');
            let renamed = if to_english { english_name(&word) } else { turkish_name(&word) };
            let constant = |w: &str| CONSTANTS.iter().any(|(t, e)| upper_tr(w) == *t || w.eq_ignore_ascii_case(e));
            match renamed {
                Some(name) if call || constant(&word) => out.push_str(name),
                _ => out.push_str(&word),
            }
            continue;
        }
        let between_digits = i > 0 && digit(i - 1) && digit(i + 1);
        match (c, to_english) {
            (';', true) => out.push(','),
            (',', true) if between_digits => out.push('.'),
            (',', false) => out.push(';'),
            ('.', false) if between_digits => out.push(','),
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

/// The Turkish alphabet, for sorting: ç after c, ğ after g, ı before i, ö after o, ş after s,
/// ü after u.
const ALPHABET: &str = "abcçdefgğhıijklmnoöprsştuüvwxyz";

/// A sort key that orders text as a Turkish reader expects (case ignored).
pub fn collation_key(text: &str) -> Vec<u32> {
    text.chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .map(|c| match ALPHABET.chars().position(|a| a == c) {
            // Letters after digits and punctuation, in alphabet order.
            Some(p) => 0x1_0000 + p as u32,
            None if c.is_alphabetic() => 0x2_0000 + c as u32,
            None => c as u32,
        })
        .collect()
}

/// A number the engine formatted ("1,250.50", "12.5%") with Turkish separators ("1.250,50").
pub fn number_display(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '.' => ',',
            ',' => '.',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_typed_the_turkish_way() {
        assert_eq!(to_engine("1250,5"), "1250.5");
        assert_eq!(to_engine("1.250,75"), "1250.75");
        assert_eq!(to_engine("1.250.000"), "1250000");
        assert_eq!(to_engine("-12,5%"), "-12.5%");
        assert_eq!(to_engine("42"), "42");
        // Dates and text are not numbers.
        assert_eq!(to_engine("31.10.2026"), "31.10.2026");
        assert_eq!(to_engine("12.5"), "12.5");
        assert_eq!(to_engine("Ankara, Çankaya"), "Ankara, Çankaya");
        assert_eq!(from_engine("1250.5"), "1250,5");
        assert_eq!(from_engine("Merhaba. Nasılsın?"), "Merhaba. Nasılsın?");
    }

    #[test]
    fn formulas_typed_the_turkish_way() {
        assert_eq!(to_engine("=TOPLA(A1;A2)"), "=SUM(A1,A2)");
        assert_eq!(to_engine("=eğer(B2>1000,5;\"Yüksek; çok\";DOĞRU)"), "=IF(B2>1000.5,\"Yüksek; çok\",TRUE)");
        assert_eq!(to_engine("=DÜŞEYARA(A2;Sayfa2!A:B;2;YANLIŞ)"), "=VLOOKUP(A2,Sayfa2!A:B,2,FALSE)");
        assert_eq!(to_engine("=SUM(A1,A2)"), "=SUM(A1,A2)", "English still works");
        assert_eq!(from_engine("=IF(SUM(A1:A3)>2.5,\"a,b\",FALSE)"), "=EĞER(TOPLA(A1:A3)>2,5;\"a,b\";YANLIŞ)");
        // A cell named like a function is a reference, not a call.
        assert_eq!(to_engine("=AY1+1"), "=AY1+1");
    }

    #[test]
    fn sorts_in_turkish_alphabet_order() {
        let mut names = vec!["Zeynep", "şule", "Çiğdem", "cemil", "Işık", "İrem", "ılgın", "Ahmet", "Ömer", "osman"];
        names.sort_by_key(|n| collation_key(n));
        assert_eq!(names, ["Ahmet", "cemil", "Çiğdem", "ılgın", "Işık", "İrem", "osman", "Ömer", "şule", "Zeynep"]);
    }

    #[test]
    fn offers_function_names_in_alphabet_order() {
        let tr = function_names(true);
        assert!(tr.iter().position(|n| *n == "ÇARPIM") > tr.iter().position(|n| *n == "BUL"));
        assert!(tr.contains(&"DÜŞEYARA") && !tr.contains(&"VLOOKUP"));
        assert!(function_names(false).contains(&"VLOOKUP"));
    }

    #[test]
    fn shown_numbers_swap_separators() {
        assert_eq!(number_display("1,250.50"), "1.250,50");
        assert_eq!(number_display("12.5%"), "12,5%");
    }
}
