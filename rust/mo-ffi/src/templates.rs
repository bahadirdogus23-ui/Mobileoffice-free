//! Turkish starting documents for "New from template": petitions, a CV, an official letter,
//! minutes and a lesson plan as Word files; a budget, a proforma invoice and a price quote as
//! spreadsheets. Each is built with the engines' own import and commands, so what the person
//! gets is an ordinary DOCX or XLSX they can change freely.

use mo_grid::GridDoc;
use mo_word::WordDoc;

use crate::MoError;

/// One template offered under New.
#[derive(uniffi::Record, Clone, Debug)]
pub struct DocTemplate {
    pub id: String,
    /// The file type it makes: "docx" or "xlsx".
    pub ext: String,
    pub title_tr: String,
    pub title_en: String,
    /// A file name to suggest when saving (Turkish, without the extension).
    pub file_name: String,
}

const LIST: &[(&str, &str, &str, &str, &str)] = &[
    ("dilekce", "docx", "Dilekçe", "Petition", "Dilekçe"),
    ("ozgecmis", "docx", "Özgeçmiş (CV)", "CV", "Özgeçmiş"),
    ("resmi_yazi", "docx", "Resmî yazı", "Official letter", "Resmi yazı"),
    ("tutanak", "docx", "Tutanak", "Minutes", "Tutanak"),
    ("ders_plani", "docx", "Ders planı", "Lesson plan", "Ders planı"),
    ("butce", "xlsx", "Aylık bütçe", "Monthly budget", "Aylık bütçe"),
    ("proforma", "xlsx", "Proforma fatura", "Proforma invoice", "Proforma fatura"),
    ("teklif", "xlsx", "Fiyat teklifi", "Price quote", "Fiyat teklifi"),
];

/// The templates offered under New, in the order shown.
#[uniffi::export]
pub fn document_templates() -> Vec<DocTemplate> {
    LIST.iter()
        .map(|&(id, ext, tr, en, file)| DocTemplate {
            id: id.into(),
            ext: ext.into(),
            title_tr: tr.into(),
            title_en: en.into(),
            file_name: file.into(),
        })
        .collect()
}

/// The file for template `id` (see [`document_templates`]).
#[uniffi::export]
pub fn template_document(id: String) -> Result<Vec<u8>, MoError> {
    match id.as_str() {
        "dilekce" => word(DILEKCE),
        "ozgecmis" => word(OZGECMIS),
        "resmi_yazi" => word(RESMI_YAZI),
        "tutanak" => word(TUTANAK),
        "ders_plani" => word(DERS_PLANI),
        "butce" => sheet(&butce()),
        "proforma" => sheet(&proforma()),
        "teklif" => sheet(&teklif()),
        _ => Err(MoError::NotFound(format!("no template {id}"))),
    }
}

/// A Word file from an HTML outline, on A4 with 2.5 cm margins.
fn word(html: &str) -> Result<Vec<u8>, MoError> {
    let mut doc = WordDoc::open("sablon.html", html.as_bytes(), None)?;
    doc.execute("layout.size", r#"{"name":"A4"}"#)?;
    doc.execute("layout.margins", r#"{"preset":"normal"}"#)?;
    Ok(doc.export("sablon.docx")?)
}

const DILEKCE: &str = r#"<html><body>
<p style="text-align:right">…/…/20…</p>
<p>&nbsp;</p>
<p style="text-align:center"><b>……………………………… MÜDÜRLÜĞÜNE / BAŞKANLIĞINA</b></p>
<p style="text-align:center"><b>……………………………</b></p>
<p>&nbsp;</p>
<p style="text-align:justify">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;Kurumunuzda/okulunuzda ……………………………… olarak bulunmaktayım. ……………………………………………………………… nedeniyle ……………………………………………………………… hususunda gereğinin yapılmasını saygılarımla arz ederim.</p>
<p>&nbsp;</p>
<p style="text-align:right">Adı Soyadı</p>
<p style="text-align:right">İmza</p>
<p>&nbsp;</p>
<p><b>Adres:</b> ……………………………………………………………</p>
<p><b>Telefon:</b> 05.. ... .. ..</p>
<p><b>T.C. Kimlik No:</b> ………………………</p>
<p><b>E-posta:</b> ……………………………</p>
<p>&nbsp;</p>
<p><b>Ek:</b></p>
<p>1- ……………………………</p>
</body></html>"#;

const OZGECMIS: &str = r#"<html><body>
<h1>Adınız Soyadınız</h1>
<p>Meslek / unvan · Şehir · 05.. ... .. .. · eposta@ornek.com</p>
<h2>Hakkımda</h2>
<p>Deneyiminizi, güçlü yönlerinizi ve hedefinizi iki üç cümleyle anlatın.</p>
<h2>İş deneyimi</h2>
<p><b>Pozisyon</b> — Şirket adı, Şehir (Ay Yıl – Ay Yıl)</p>
<ul><li>Sorumluluk ya da başarı (rakamla: “satışları %20 artırdım”)</li><li>Sorumluluk ya da başarı</li></ul>
<p><b>Pozisyon</b> — Şirket adı, Şehir (Ay Yıl – Ay Yıl)</p>
<ul><li>Sorumluluk ya da başarı</li></ul>
<h2>Eğitim</h2>
<p><b>Bölüm</b> — Üniversite / Lise (Yıl – Yıl)</p>
<h2>Yetenekler</h2>
<ul><li>Bilgisayar: Word, Excel, …</li><li>Ehliyet: B sınıfı</li></ul>
<h2>Yabancı dil</h2>
<table border="1"><tr><td><b>Dil</b></td><td><b>Okuma</b></td><td><b>Yazma</b></td><td><b>Konuşma</b></td></tr>
<tr><td>İngilizce</td><td>İyi</td><td>Orta</td><td>Orta</td></tr></table>
<h2>Referanslar</h2>
<p>Adı Soyadı — Unvan, Kurum · 05.. ... .. ..</p>
</body></html>"#;

const RESMI_YAZI: &str = r#"<html><body>
<p style="text-align:center"><b>T.C.</b></p>
<p style="text-align:center"><b>……………………………… VALİLİĞİ / KAYMAKAMLIĞI</b></p>
<p style="text-align:center"><b>……………………………… Müdürlüğü</b></p>
<p>&nbsp;</p>
<table><tr><td>Sayı&nbsp;&nbsp;&nbsp;: ………………………</td><td style="text-align:right">…/…/20…</td></tr>
<tr><td>Konu&nbsp;&nbsp;: ………………………</td><td></td></tr></table>
<p>&nbsp;</p>
<p style="text-align:center"><b>……………………………… MÜDÜRLÜĞÜNE</b></p>
<p>&nbsp;</p>
<p style="text-align:justify"><b>İlgi:</b> a) …/…/20… tarihli ve ……… sayılı yazı.</p>
<p style="text-align:justify">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;İlgi (a) yazı ile ……………………………………………………………………………………………………………… .</p>
<p style="text-align:justify">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;Bilgilerinizi ve gereğini rica ederim.</p>
<p>&nbsp;</p>
<p style="text-align:right">Adı SOYADI</p>
<p style="text-align:right">Unvanı</p>
<p>&nbsp;</p>
<p><b>Ek:</b> ……………………… (… sayfa)</p>
<p><b>Dağıtım:</b></p>
<p>Gereği: ……………………… &nbsp;&nbsp;&nbsp;&nbsp; Bilgi: ………………………</p>
</body></html>"#;

const TUTANAK: &str = r#"<html><body>
<h1 style="text-align:center">TUTANAK</h1>
<table border="1"><tr><td><b>Tarih / Saat</b></td><td>…/…/20… · ..:..</td></tr>
<tr><td><b>Yer</b></td><td>……………………………</td></tr>
<tr><td><b>Konu</b></td><td>……………………………</td></tr></table>
<p>&nbsp;</p>
<p style="text-align:justify">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;Yukarıda belirtilen tarih ve saatte, ……………………………… adresinde yapılan incelemede ……………………………………………………………………………………… olduğu görülmüştür.</p>
<p style="text-align:justify">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;İşbu tutanak, aşağıda adları yazılı kişilerce birlikte düzenlenip okunarak imza altına alınmıştır.</p>
<p>&nbsp;</p>
<table border="1"><tr><td style="text-align:center"><b>Tutanağı düzenleyen</b></td><td style="text-align:center"><b>Üye</b></td><td style="text-align:center"><b>Üye</b></td></tr>
<tr><td style="text-align:center">Adı Soyadı<br/>Unvanı<br/>İmza</td><td style="text-align:center">Adı Soyadı<br/>Unvanı<br/>İmza</td><td style="text-align:center">Adı Soyadı<br/>Unvanı<br/>İmza</td></tr></table>
</body></html>"#;

const DERS_PLANI: &str = r#"<html><body>
<h1 style="text-align:center">GÜNLÜK DERS PLANI</h1>
<h2>Bölüm I</h2>
<table border="1"><tr><td><b>Ders</b></td><td>……………</td><td><b>Sınıf</b></td><td>……</td></tr>
<tr><td><b>Ünite / Tema</b></td><td>……………</td><td><b>Süre</b></td><td>40 + 40 dk</td></tr>
<tr><td><b>Konu</b></td><td>……………</td><td><b>Tarih</b></td><td>…/…/20…</td></tr></table>
<h2>Bölüm II</h2>
<table border="1"><tr><td><b>Kazanımlar</b></td><td>……………………………</td></tr>
<tr><td><b>Yöntem ve teknikler</b></td><td>Anlatım, soru-cevap, gösterip yaptırma</td></tr>
<tr><td><b>Araç-gereç</b></td><td>Ders kitabı, akıllı tahta, çalışma kâğıdı</td></tr></table>
<h2>Öğrenme-öğretme süreci</h2>
<p><b>Giriş (dikkat çekme, güdüleme):</b> ……………………………</p>
<p><b>Geliştirme:</b> ……………………………</p>
<p><b>Sonuç (özet, pekiştirme):</b> ……………………………</p>
<h2>Ölçme ve değerlendirme</h2>
<p>……………………………</p>
<h2>Bölüm IV · Açıklamalar</h2>
<p>……………………………</p>
<p>&nbsp;</p>
<table><tr><td style="text-align:center">Ders öğretmeni<br/>Adı Soyadı</td><td style="text-align:center">Okul müdürü<br/>Adı Soyadı</td></tr></table>
</body></html>"#;

/// A spreadsheet template: cell inputs, then formats over ranges, then column widths.
struct Sheet {
    cells: Vec<(u32, u32, &'static str)>,
    /// (command, JSON parameters without the range, range "A1:C3").
    formats: Vec<(&'static str, &'static str, &'static str)>,
    /// (columns "A:A", width in Excel pixels).
    widths: Vec<(&'static str, f64)>,
}

fn sheet(s: &Sheet) -> Result<Vec<u8>, MoError> {
    let mut g = GridDoc::blank();
    // Formulas are written with Turkish function names, as the app takes them.
    g.set_turkish(true);
    for &(r, c, v) in &s.cells {
        g.set_input(r, c, v)?;
    }
    for &(id, params, range) in &s.formats {
        let mut p: serde_json::Value = serde_json::from_str(params).map_err(|e| MoError::Save(e.to_string()))?;
        p["range"] = range.into();
        g.execute(id, &p.to_string())?;
    }
    for &(cols, width) in &s.widths {
        g.execute("home.columnWidth", &serde_json::json!({ "cols": cols, "width": width }).to_string())?;
    }
    Ok(g.save("sablon.xlsx")?)
}

/// Title, header and money formats every spreadsheet template shares.
fn common(title: &'static str, header: &'static str) -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("home.mergeCenter", "{}", title),
        ("home.bold", r#"{"on":true}"#, title),
        ("home.fontSize", r#"{"size":16}"#, title),
        ("home.bold", r#"{"on":true}"#, header),
        ("home.fillColor", r##"{"color":"#DDEBF7"}"##, header),
    ]
}

fn butce() -> Sheet {
    let mut cells = vec![
        (0, 0, "AYLIK BÜTÇE"),
        (1, 0, "Ay:"),
        (1, 1, "Ocak 2027"),
        (3, 0, "GELİRLER"),
        (3, 1, "Planlanan"),
        (3, 2, "Gerçekleşen"),
        (3, 3, "Fark"),
        (4, 0, "Maaş"),
        (5, 0, "Ek gelir"),
        (6, 0, "Diğer"),
        (7, 0, "Toplam gelir"),
        (9, 0, "GİDERLER"),
        (9, 1, "Planlanan"),
        (9, 2, "Gerçekleşen"),
        (9, 3, "Fark"),
        (10, 0, "Kira / aidat"),
        (11, 0, "Market"),
        (12, 0, "Faturalar (elektrik, su, doğalgaz)"),
        (13, 0, "İnternet / telefon"),
        (14, 0, "Ulaşım"),
        (15, 0, "Kredi kartı / taksit"),
        (16, 0, "Eğitim"),
        (17, 0, "Sağlık"),
        (18, 0, "Eğlence"),
        (19, 0, "Diğer"),
        (20, 0, "Toplam gider"),
        (22, 0, "KALAN (gelir − gider)"),
        (7, 1, "=TOPLA(B5:B7)"),
        (7, 2, "=TOPLA(C5:C7)"),
        (20, 1, "=TOPLA(B11:B20)"),
        (20, 2, "=TOPLA(C11:C20)"),
        (22, 1, "=B8-B21"),
        (22, 2, "=C8-C21"),
    ];
    // Difference = actual − planned on every line.
    const DIFFS: [&str; 15] = [
        "=C5-B5", "=C6-B6", "=C7-B7", "=C8-B8", "=C11-B11", "=C12-B12", "=C13-B13", "=C14-B14", "=C15-B15", "=C16-B16", "=C17-B17", "=C18-B18",
        "=C19-B19", "=C20-B20", "=C21-B21",
    ];
    let rows = [4u32, 5, 6, 7, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
    for (r, f) in rows.iter().zip(DIFFS) {
        cells.push((*r, 3, f));
    }
    let mut formats = common("A1:D1", "A4:D4");
    formats.extend([
        ("home.bold", r#"{"on":true}"#, "A10:D10"),
        ("home.fillColor", r##"{"color":"#FCE4D6"}"##, "A10:D10"),
        ("home.bold", r#"{"on":true}"#, "A8:D8"),
        ("home.bold", r#"{"on":true}"#, "A21:D21"),
        ("home.bold", r#"{"on":true}"#, "A23:C23"),
        ("home.fillColor", r##"{"color":"#E2EFDA"}"##, "A23:C23"),
        ("home.accounting", r#"{"symbol":"₺"}"#, "B5:D23"),
        ("home.borders", r#"{"preset":"all"}"#, "A4:D8"),
        ("home.borders", r#"{"preset":"all"}"#, "A10:D21"),
        ("home.borders", r#"{"preset":"thickOutside"}"#, "A23:C23"),
    ]);
    Sheet { cells, formats, widths: vec![("A:A", 260.0), ("B:D", 120.0)] }
}

fn proforma() -> Sheet {
    let mut cells = vec![
        (0, 0, "PROFORMA FATURA"),
        (2, 0, "Satıcı:"),
        (2, 1, "Firma adı · Adres · Vergi dairesi / No"),
        (3, 0, "Alıcı:"),
        (3, 1, "Firma adı · Adres · Vergi dairesi / No"),
        (4, 0, "Tarih:"),
        (4, 1, "…/…/20…"),
        (5, 0, "Proforma no:"),
        (5, 1, "PF-2027-001"),
        (7, 0, "Sıra"),
        (7, 1, "Mal / hizmet"),
        (7, 2, "Miktar"),
        (7, 3, "Birim"),
        (7, 4, "Birim fiyat"),
        (7, 5, "Tutar"),
        (8, 0, "1"),
        (8, 1, "Ürün ya da hizmet"),
        (8, 2, "1"),
        (8, 3, "Adet"),
        (8, 4, "0"),
        (18, 4, "Ara toplam"),
        (19, 3, "KDV %"),
        (19, 4, "20"),
        (20, 4, "Genel toplam"),
        (22, 0, "Ödeme: … gün vadeli, banka havalesi (IBAN: TR.. .... .... .... .... .... ..)"),
        (23, 0, "Bu belge proforma faturadır; mali değeri yoktur. Teklif geçerlilik süresi: 15 gün."),
    ];
    const AMOUNTS: [&str; 10] =
        ["=C9*E9", "=C10*E10", "=C11*E11", "=C12*E12", "=C13*E13", "=C14*E14", "=C15*E15", "=C16*E16", "=C17*E17", "=C18*E18"];
    for (i, f) in AMOUNTS.into_iter().enumerate() {
        cells.push((8 + i as u32, 5, f));
    }
    cells.extend([(18, 5, "=TOPLA(F9:F18)"), (19, 5, "=F19*E20/100"), (20, 5, "=F19+F20")]);
    let mut formats = common("A1:F1", "A8:F8");
    formats.extend([
        ("home.bold", r#"{"on":true}"#, "A3:A6"),
        ("home.borders", r#"{"preset":"all"}"#, "A8:F18"),
        ("home.accounting", r#"{"symbol":"₺"}"#, "E9:F18"),
        ("home.accounting", r#"{"symbol":"₺"}"#, "F19:F21"),
        ("home.bold", r#"{"on":true}"#, "E19:F21"),
        ("home.borders", r#"{"preset":"thickOutside"}"#, "E21:F21"),
        ("home.fillColor", r##"{"color":"#E2EFDA"}"##, "E21:F21"),
    ]);
    Sheet { cells, formats, widths: vec![("A:A", 90.0), ("B:B", 260.0), ("C:D", 70.0), ("E:F", 120.0)] }
}

fn teklif() -> Sheet {
    let mut cells = vec![
        (0, 0, "FİYAT TEKLİFİ"),
        (2, 0, "Sayın:"),
        (2, 1, "Müşteri / firma adı"),
        (3, 0, "Tarih:"),
        (3, 1, "…/…/20…"),
        (4, 0, "Geçerlilik:"),
        (4, 1, "30 gün"),
        (6, 0, "Açıklama"),
        (6, 1, "Miktar"),
        (6, 2, "Birim fiyat"),
        (6, 3, "İskonto %"),
        (6, 4, "Tutar"),
        (7, 0, "Kalem 1"),
        (7, 1, "1"),
        (7, 2, "0"),
        (7, 3, "0"),
        (16, 3, "Toplam (KDV hariç)"),
        (17, 3, "KDV (%20)"),
        (18, 3, "Genel toplam"),
        (20, 0, "Fiyatlarımıza montaj ve nakliye dahildir / hariçtir. Teslim süresi: … iş günü."),
        (21, 0, "Saygılarımızla, Firma adı · Yetkili · 05.. ... .. .."),
    ];
    const AMOUNTS: [&str; 9] = [
        "=B8*C8*(1-D8/100)",
        "=B9*C9*(1-D9/100)",
        "=B10*C10*(1-D10/100)",
        "=B11*C11*(1-D11/100)",
        "=B12*C12*(1-D12/100)",
        "=B13*C13*(1-D13/100)",
        "=B14*C14*(1-D14/100)",
        "=B15*C15*(1-D15/100)",
        "=B16*C16*(1-D16/100)",
    ];
    for (i, f) in AMOUNTS.into_iter().enumerate() {
        cells.push((7 + i as u32, 4, f));
    }
    cells.extend([(16, 4, "=TOPLA(E8:E16)"), (17, 4, "=E17*0,2"), (18, 4, "=E17+E18")]);
    let mut formats = common("A1:E1", "A7:E7");
    formats.extend([
        ("home.bold", r#"{"on":true}"#, "A3:A5"),
        ("home.borders", r#"{"preset":"all"}"#, "A7:E16"),
        ("home.accounting", r#"{"symbol":"₺"}"#, "C8:C16"),
        ("home.accounting", r#"{"symbol":"₺"}"#, "E8:E19"),
        ("home.bold", r#"{"on":true}"#, "D17:E19"),
        ("home.borders", r#"{"preset":"thickOutside"}"#, "D19:E19"),
        ("home.fillColor", r##"{"color":"#E2EFDA"}"##, "D19:E19"),
    ]);
    Sheet { cells, formats, widths: vec![("A:A", 260.0), ("B:B", 70.0), ("C:C", 110.0), ("D:D", 120.0), ("E:E", 120.0)] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_builds_and_reopens() {
        for t in document_templates() {
            let bytes = template_document(t.id.clone()).unwrap_or_else(|e| panic!("{}: {e:?}", t.id));
            let name = format!("a.{}", t.ext);
            if t.ext == "docx" {
                let doc = WordDoc::open(&name, &bytes, None).unwrap();
                assert!(doc.page_count() >= 1, "{}", t.id);
            } else {
                let g = GridDoc::open(&name, &bytes, None).unwrap();
                assert!(g.used_size(0).unwrap().rows > 10, "{}", t.id);
            }
        }
        assert!(template_document("yok".into()).is_err());
    }

    #[test]
    fn spreadsheet_templates_calculate() {
        let bytes = template_document("teklif".into()).unwrap();
        let mut g = GridDoc::open("a.xlsx", &bytes, None).unwrap();
        g.set_input(7, 1, "2").unwrap();
        g.set_input(7, 2, "100").unwrap();
        g.set_input(7, 3, "10").unwrap();
        let total = g.cell_texts(0, 18, 4, 1, 1).unwrap().remove(0);
        // 2 × 100 × 0,9 = 180; with 20 % VAT 216.
        assert!(total.contains("216"), "{total}");
        assert_eq!(g.merges(0).unwrap().first(), Some(&[0, 0, 0, 4]));
        let looks = g.cell_looks(0, 6, 0, 1, 1).unwrap();
        assert!(looks[0].bold && looks[0].fill != 0);
        let b = template_document("butce".into()).unwrap();
        let mut g = GridDoc::open("b.xlsx", &b, None).unwrap();
        g.set_input(4, 2, "30000").unwrap();
        g.set_input(10, 2, "12000").unwrap();
        let left = g.cell_texts(0, 22, 2, 1, 1).unwrap().remove(0);
        assert!(left.contains("18"), "{left}");
    }

    #[test]
    fn word_templates_are_a4_and_turkish() {
        let bytes = template_document("dilekce".into()).unwrap();
        let doc = WordDoc::open("a.docx", &bytes, None).unwrap();
        assert_eq!(doc.find("arz ederim").len(), 1);
        let xml = {
            use std::io::Read;
            let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
            let mut s = String::new();
            z.by_name("word/document.xml").unwrap().read_to_string(&mut s).unwrap();
            s
        };
        assert!(xml.contains(r#"w:w="11906""#) || xml.contains(r#"w:w="11905""#), "A4 width");
        assert!(xml.contains("MÜDÜRLÜĞÜNE"));
    }

    #[test]
    fn document_text_reads_every_kind_for_searching() {
        let docx = template_document("dilekce".into()).unwrap();
        assert!(crate::document_text("a.docx".into(), docx, 100_000).unwrap().contains("arz ederim"));
        let xlsx = template_document("butce".into()).unwrap();
        let t = crate::document_text("b.xlsx".into(), xlsx, 100_000).unwrap();
        assert!(t.contains("Market"), "{t}");
        let vcf = b"BEGIN:VCARD\nFN:Zeynep Kaya\nEND:VCARD\n".to_vec();
        assert!(crate::document_text("k.vcf".into(), vcf, 100_000).unwrap().contains("Zeynep Kaya"));
        // Cut at the limit, on a character boundary.
        let long = "ğ".repeat(100).into_bytes();
        assert!(crate::document_text("x.txt".into(), long, 11).unwrap().len() <= 11);
    }

    /// Saving must not lose what was opened: every template's words survive open → save → open,
    /// twice over (the plan's "lossless save" check, on the files the app itself makes).
    #[test]
    fn saving_twice_keeps_every_word() {
        for t in document_templates() {
            let first = template_document(t.id.clone()).unwrap();
            let name = format!("a.{}", t.ext);
            let words = |bytes: &[u8]| crate::document_text(name.clone(), bytes.to_vec(), 1_000_000).unwrap();
            let before = words(&first);
            let mut bytes = first;
            for _ in 0..2 {
                bytes = if t.ext == "docx" {
                    WordDoc::open(&name, &bytes, None).unwrap().export(&name).unwrap()
                } else {
                    GridDoc::open(&name, &bytes, None).unwrap().save(&name).unwrap()
                };
            }
            assert_eq!(words(&bytes), before, "{}", t.id);
        }
    }
}
