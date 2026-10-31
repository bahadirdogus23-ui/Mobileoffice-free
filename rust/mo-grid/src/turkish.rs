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
    ("ÇAPRAZARA", "XLOOKUP"),
    ("ÇOKEĞER", "IFS"),
    ("İLKEŞLEŞEN", "SWITCH"),
    ("EĞERYOKSA", "IFNA"),
    ("ÇOKEĞERORTALAMA", "AVERAGEIFS"),
    ("ÇOKEĞERMAK", "MAXIFS"),
    ("ÇOKEĞERMİN", "MINIFS"),
    ("METİNBİRLEŞTİR", "TEXTJOIN"),
    ("ARALIKBİRLEŞTİR", "CONCAT"),
    ("BENZERSİZ", "UNIQUE"),
    ("FİLTRE", "FILTER"),
    ("SIRALA", "SORT"),
    ("SERİAY", "EOMONTH"),
    ("SERİTARİH", "EDATE"),
    ("TAMİŞGÜNÜ", "NETWORKDAYS"),
    ("İŞGÜNÜ", "WORKDAY"),
    ("ETARİHLİ", "DATEDIF"),
    ("HAFTANINGÜNÜ", "WEEKDAY"),
    ("HAFTASAY", "WEEKNUM"),
    ("GÜNSAY", "DAYS"),
    ("SAAT", "HOUR"),
    ("DAKİKA", "MINUTE"),
    ("SANİYE", "SECOND"),
    ("ZAMAN", "TIME"),
    ("BÜYÜK", "LARGE"),
    ("KÜÇÜK", "SMALL"),
    ("STDSAPMA", "STDEV"),
    ("SATIR", "ROW"),
    ("SÜTUN", "COLUMN"),
    ("SATIRSAY", "ROWS"),
    ("SÜTUNSAY", "COLUMNS"),
    ("DOLAYLI", "INDIRECT"),
    ("KAYDIR", "OFFSET"),
    ("ARA", "LOOKUP"),
    ("EBOŞSA", "ISBLANK"),
    ("ESAYIYSA", "ISNUMBER"),
    ("EMETİNSE", "ISTEXT"),
    ("EHATALIYSA", "ISERROR"),
    ("YAZIM.DÜZENİ", "PROPER"),
    ("YİNELE", "REPT"),
    ("TEMİZ", "CLEAN"),
    ("DEĞİŞTİR", "REPLACE"),
    ("MBUL", "SEARCH"),
    ("ÖZDEŞ", "EXACT"),
    ("TAVANAYUVARLA", "CEILING"),
    ("TABANAYUVARLA", "FLOOR"),
    ("RASTGELEARADA", "RANDBETWEEN"),
    ("S_SAYI_ÜRET", "RAND"),
    ("NBD", "NPV"),
    ("GD", "FV"),
    ("BD", "PV"),
    ("TAKSİT_SAYISI", "NPER"),
    ("FAİZTUTARI", "IPMT"),
];

/// What the common functions do, in Turkish, by English name; the others keep the engine's
/// English description.
const DESCRIPTIONS: &[(&str, &str)] = &[
    ("SUM", "Sayıları toplar. Örnek: =TOPLA(A1:A10)"),
    ("AVERAGE", "Sayıların ortalamasını verir."),
    ("IF", "Koşul doğruysa bir değer, yanlışsa başka bir değer verir. Örnek: =EĞER(A1>50;\"Geçti\";\"Kaldı\")"),
    ("IFERROR", "Formül hata verirse yerine gösterilecek değeri verir."),
    ("IFNA", "Formül #YOK verirse yerine gösterilecek değeri verir."),
    ("IFS", "Sırayla koşullara bakar; ilk doğru koşulun değerini verir."),
    ("SWITCH", "Bir değeri listeyle karşılaştırır; ilk eşleşenin sonucunu verir."),
    ("COUNTIF", "Bir koşula uyan hücreleri sayar. Örnek: =EĞERSAY(B:B;\">100\")"),
    ("COUNTIFS", "Birden çok koşula uyan hücreleri sayar."),
    ("SUMIF", "Bir koşula uyan hücreleri toplar."),
    ("SUMIFS", "Birden çok koşula uyan hücreleri toplar."),
    ("AVERAGEIF", "Bir koşula uyan hücrelerin ortalamasını verir."),
    ("AVERAGEIFS", "Birden çok koşula uyan hücrelerin ortalamasını verir."),
    ("MAXIFS", "Koşullara uyan hücrelerin en büyüğünü verir."),
    ("MINIFS", "Koşullara uyan hücrelerin en küçüğünü verir."),
    ("VLOOKUP", "Tablonun ilk sütununda arar, aynı satırdaki başka bir sütunun değerini verir. Örnek: =DÜŞEYARA(A2;Fiyatlar!A:C;3;YANLIŞ)"),
    ("HLOOKUP", "Tablonun ilk satırında arar, aynı sütundaki başka bir satırın değerini verir."),
    ("XLOOKUP", "Bir aralıkta arar, karşılık gelen aralıktaki değeri verir; DÜŞEYARA'nın yenisi."),
    ("LOOKUP", "Bir satır ya da sütunda arar, karşılık gelen değeri verir."),
    ("INDEX", "Aralıkta belirtilen satır ve sütundaki değeri verir."),
    ("MATCH", "Bir değerin aralıktaki sırasını verir."),
    ("INDIRECT", "Metin olarak yazılmış bir başvurunun gösterdiği değeri verir."),
    ("OFFSET", "Bir başvurudan belirli satır ve sütun kaydırılmış aralığı verir."),
    ("TODAY", "Bugünün tarihini verir."),
    ("NOW", "Şu anki tarih ve saati verir."),
    ("DATE", "Yıl, ay ve günden tarih oluşturur."),
    ("TIME", "Saat, dakika ve saniyeden zaman oluşturur."),
    ("YEAR", "Tarihin yılını verir."),
    ("MONTH", "Tarihin ayını (1–12) verir."),
    ("DAY", "Tarihin gününü (1–31) verir."),
    ("HOUR", "Zamanın saatini verir."),
    ("MINUTE", "Zamanın dakikasını verir."),
    ("SECOND", "Zamanın saniyesini verir."),
    ("WEEKDAY", "Tarihin haftanın kaçıncı günü olduğunu verir."),
    ("WEEKNUM", "Tarihin yılın kaçıncı haftası olduğunu verir."),
    ("DAYS", "İki tarih arasındaki gün sayısını verir."),
    ("DATEDIF", "İki tarih arasındaki farkı gün, ay ya da yıl olarak verir (\"d\", \"m\", \"y\")."),
    ("EOMONTH", "Belirli sayıda ay sonraki ayın son gününü verir."),
    ("EDATE", "Belirli sayıda ay sonraki aynı günü verir."),
    ("NETWORKDAYS", "İki tarih arasındaki iş günlerini sayar (hafta sonları ve tatiller hariç)."),
    ("WORKDAY", "Belirli sayıda iş günü sonraki tarihi verir."),
    ("MAX", "En büyük sayıyı verir."),
    ("MIN", "En küçük sayıyı verir."),
    ("LARGE", "Aralıktaki k'ıncı en büyük değeri verir."),
    ("SMALL", "Aralıktaki k'ıncı en küçük değeri verir."),
    ("COUNT", "Sayı içeren hücreleri sayar."),
    ("COUNTA", "Boş olmayan hücreleri sayar."),
    ("COUNTBLANK", "Boş hücreleri sayar."),
    ("ROUND", "Sayıyı belirtilen basamağa yuvarlar. Örnek: =YUVARLA(A1;2)"),
    ("ROUNDDOWN", "Sayıyı aşağı (sıfıra doğru) yuvarlar."),
    ("ROUNDUP", "Sayıyı yukarı (sıfırdan uzağa) yuvarlar."),
    ("CEILING", "Sayıyı en yakın katına yukarı yuvarlar."),
    ("FLOOR", "Sayıyı en yakın katına aşağı yuvarlar."),
    ("INT", "Sayıyı aşağı doğru tamsayıya yuvarlar."),
    ("ABS", "Sayının mutlak değerini verir."),
    ("SQRT", "Sayının karekökünü verir."),
    ("POWER", "Sayının kuvvetini verir."),
    ("MOD", "Bölme işleminden kalanı verir."),
    ("PRODUCT", "Sayıları çarpar."),
    ("SUMPRODUCT", "Aralıkların karşılıklı elemanlarını çarpıp toplar."),
    ("MEDIAN", "Sayıların ortancasını verir."),
    ("STDEV", "Örneklemin standart sapmasını verir."),
    ("RAND", "0 ile 1 arasında rastgele sayı verir."),
    ("RANDBETWEEN", "İki sayı arasında rastgele tamsayı verir."),
    ("AND", "Bütün koşullar doğruysa DOĞRU verir."),
    ("OR", "Koşullardan biri doğruysa DOĞRU verir."),
    ("NOT", "Mantıksal değeri tersine çevirir."),
    ("CONCATENATE", "Metinleri birleştirir."),
    ("CONCAT", "Metinleri ve aralıkları birleştirir."),
    ("TEXTJOIN", "Metinleri araya bir ayırıcı koyarak birleştirir."),
    ("LEN", "Metnin karakter sayısını verir."),
    ("LEFT", "Metnin başından belirtilen sayıda karakter verir."),
    ("RIGHT", "Metnin sonundan belirtilen sayıda karakter verir."),
    ("MID", "Metnin ortasından, belirtilen yerden başlayarak karakter verir."),
    ("UPPER", "Metni büyük harfe çevirir."),
    ("LOWER", "Metni küçük harfe çevirir."),
    ("PROPER", "Her kelimenin ilk harfini büyük yapar."),
    ("TRIM", "Metindeki fazla boşlukları siler."),
    ("CLEAN", "Metindeki yazdırılamayan karakterleri siler."),
    ("TEXT", "Sayıyı belirtilen biçimde metne çevirir. Örnek: =METNEÇEVİR(A1;\"0,00\")"),
    ("VALUE", "Sayı gibi görünen metni sayıya çevirir."),
    ("SUBSTITUTE", "Metindeki bir parçayı başkasıyla değiştirir."),
    ("REPLACE", "Metnin belirli konumundaki karakterleri değiştirir."),
    ("FIND", "Bir metnin başka bir metin içindeki yerini verir (büyük-küçük harfe duyarlı)."),
    ("SEARCH", "Bir metnin başka bir metin içindeki yerini verir (büyük-küçük harfe duyarsız)."),
    ("EXACT", "İki metin tam olarak aynıysa DOĞRU verir."),
    ("REPT", "Metni belirtilen sayıda yineler."),
    ("UNIQUE", "Aralıktaki benzersiz değerleri verir."),
    ("FILTER", "Aralığı bir koşula göre süzer."),
    ("SORT", "Aralığı sıralar."),
    ("ROW", "Başvurunun satır numarasını verir."),
    ("COLUMN", "Başvurunun sütun numarasını verir."),
    ("ROWS", "Aralıktaki satır sayısını verir."),
    ("COLUMNS", "Aralıktaki sütun sayısını verir."),
    ("ISBLANK", "Hücre boşsa DOĞRU verir."),
    ("ISNUMBER", "Değer sayıysa DOĞRU verir."),
    ("ISTEXT", "Değer metinse DOĞRU verir."),
    ("ISERROR", "Değer hataysa DOĞRU verir."),
    ("PMT", "Kredinin dönemsel taksitini hesaplar. Örnek: =PMT(%2;12;-10000)"),
    ("RATE", "Kredinin dönemsel faiz oranını hesaplar."),
    ("NPV", "Nakit akışlarının net bugünkü değerini verir."),
    ("FV", "Yatırımın gelecekteki değerini verir."),
    ("PV", "Yatırımın bugünkü değerini verir."),
    ("NPER", "Kredinin kaç dönemde ödeneceğini verir."),
    ("IPMT", "Belirli bir taksitteki faiz tutarını verir."),
];

/// What a function is for: its name as typed, its arguments and what it does.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDoc {
    /// The name as typed (Turkish when Turkish is on and it has one).
    pub name: String,
    /// How it is called, with the argument separator that is typed.
    pub signature: String,
    pub description: String,
    pub category: &'static str,
}

/// Every function the engine knows, the common ones first in their category's order, with
/// Turkish names, separators and descriptions when `turkish`.
pub fn function_docs(turkish: bool) -> Vec<FunctionDoc> {
    use gridcraft_functions::Category;
    let mut out: Vec<FunctionDoc> = gridcraft_functions::all()
        .iter()
        .map(|f| {
            let name = if turkish { turkish_name(f.name).unwrap_or(f.name) } else { f.name };
            let mut signature = f.signature.to_string();
            if turkish && let Some(rest) = signature.strip_prefix(f.name) {
                signature = format!("{name}{}", rest.replace(", ", "; "));
            }
            let description = turkish
                .then(|| DESCRIPTIONS.iter().chain(crate::turkish_docs::MORE_DESCRIPTIONS).find(|(e, _)| *e == f.name).map(|(_, d)| d.to_string()))
                .flatten()
                .unwrap_or_else(|| f.description.to_string());
            let category = match (f.category, turkish) {
                (Category::Financial, true) => "Finansal",
                (Category::Logical, true) => "Mantıksal",
                (Category::Text, true) => "Metin",
                (Category::DateTime, true) => "Tarih ve saat",
                (Category::Lookup, true) => "Arama ve başvuru",
                (Category::MathTrig, true) => "Matematik",
                (Category::Statistical, true) => "İstatistik",
                (Category::Engineering, true) => "Mühendislik",
                (Category::Information, true) => "Bilgi",
                (Category::Database, true) => "Veritabanı",
                (Category::Compatibility, true) => "Uyumluluk",
                (Category::Web, true) => "Web",
                (Category::Cube, true) => "Küp",
                (Category::AddIn, true) => "Eklenti",
                (Category::Financial, false) => "Financial",
                (Category::Logical, false) => "Logical",
                (Category::Text, false) => "Text",
                (Category::DateTime, false) => "Date and time",
                (Category::Lookup, false) => "Lookup and reference",
                (Category::MathTrig, false) => "Math and trigonometry",
                (Category::Statistical, false) => "Statistical",
                (Category::Engineering, false) => "Engineering",
                (Category::Information, false) => "Information",
                (Category::Database, false) => "Database",
                (Category::Compatibility, false) => "Compatibility",
                (Category::Web, false) => "Web",
                (Category::Cube, false) => "Cube",
                (Category::AddIn, false) => "Add-in",
            };
            FunctionDoc { name: name.to_string(), signature, description, category }
        })
        .collect();
    // Functions the evaluator implements itself and the function list leaves out (TEXT, LET…).
    for &en in gridcraft_engine::calc::SPECIAL_FUNCTIONS {
        if gridcraft_functions::lookup(en).is_none() {
            let name = if turkish { turkish_name(en).unwrap_or(en) } else { en };
            let description = turkish.then(|| DESCRIPTIONS.iter().find(|(e, _)| *e == en).map(|(_, d)| d.to_string())).flatten().unwrap_or_default();
            out.push(FunctionDoc { name: name.to_string(), signature: format!("{name}(…)"), description, category: "" });
        }
    }
    // The common ones (those with a Turkish name) first, then the rest; each part alphabetical.
    let common = |d: &FunctionDoc| NAMES.iter().any(|(t, e)| d.name == *t || d.name == *e);
    out.sort_by_key(|d| (!common(d), collation_key(&d.name)));
    out.dedup_by(|a, b| a.name == b.name);
    out
}

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
    fn documents_every_function_in_turkish() {
        let docs = function_docs(true);
        assert!(docs.len() > 400, "{}", docs.len());
        let sum = docs.iter().find(|d| d.name == "TOPLA").unwrap();
        assert_eq!(sum.signature, "TOPLA(number1; [number2]; ...)");
        assert!(sum.description.starts_with("Sayıları toplar"));
        // Every function the engine lists has a Turkish description.
        let en = function_docs(false);
        let untranslated: Vec<&str> = gridcraft_functions::all()
            .iter()
            .filter(|f| !DESCRIPTIONS.iter().chain(crate::turkish_docs::MORE_DESCRIPTIONS).any(|(e, _)| *e == f.name))
            .map(|f| f.name)
            .collect();
        assert!(untranslated.is_empty(), "{untranslated:?}");
        assert!(en.iter().any(|d| d.name == "BESSELJ"));
        assert_eq!(sum.category, "Matematik");
        // The common ones come first.
        assert!(docs.iter().position(|d| d.name == "DÜŞEYARA").unwrap() < docs.iter().position(|d| d.name == "BESSELJ").unwrap());
        // Every Turkish name is a function the engine knows, and is typed back to it.
        for (tr, en) in NAMES {
            assert!(gridcraft_engine::calc::is_known_function(en), "{en}");
            assert_eq!(to_engine(&format!("={tr}(1)")), format!("={en}(1)"));
        }
        let en = function_docs(false);
        assert!(en.iter().any(|d| d.name == "VLOOKUP" && d.signature.contains(", ")));
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
