# MobileOffice Free

Android için **ücretsiz, reklamsız, hesapsız ve internetsiz** çalışan ofis ve belge uygulaması (çalışma adı).

- Word (DOCX, DOC, ODT, RTF), Excel (XLSX, XLS, CSV) ve PowerPoint (PPTX) dosyalarını açma ve düzenleme
- PDF görüntüleme, açıklama ekleme, form doldurma, imzalama, birleştirme, bölme, tarama ve OCR
- DWG ve DXF çizimlerini görüntüleme
- Görsel, metin, arşiv ve e-kitap dosyalarını açma; Türkiye'ye özgü UYAP UDF ve e-Fatura XML desteği

**Durum:** Planlama aşaması. Kapsamlı rapor ve proje planı: [docs/RAPOR-VE-PLAN.md](docs/RAPOR-VE-PLAN.md)

## İlkeler

1. Ücretsiz ve reklamsız, her zaman
2. Hesap yok, internet izni yok; belgeler cihazdan çıkmaz
3. Önce görüntüle, sonra düzenle; veri kaybı yok
4. Mobil için baştan tasarlanmış, herkes için erişilebilir
5. Türkçe birinci sınıf, açık kaynak ve şeffaf

## Teknik yaklaşım (özet)

- Arayüz: Kotlin ve Jetpack Compose
- Belge motorları: ArtCraft ekibinin MIT/Apache-2.0 lisanslı açık kaynak Rust motorları (WordCraft, GridCraft, DeckCraft, PdfCraft, CADCraft), UniFFI/JNI köprüsüyle
- Hızlı PDF görüntüleme: PDFium

Ayrıntılar, yol haritası, riskler ve karar bekleyen konular rapordadır.

## Lisans

Karar aşamasında (raporun 16. bölümü).
