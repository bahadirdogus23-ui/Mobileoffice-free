# ADR-0003: PDF görüntüleme motoru

- **Durum:** Kabul edildi (Faz 0, 28 Ekim 2026)
- **Bağlam:** Rapor §16 karar 10 (PDFium / Android PdfRenderer / PdfCraft)

## Karar

Faz 0–1'de PDF'ler **Android'in kendi `PdfRenderer`'ıyla** görüntülenir (`PdfViewModel`).

- PdfRenderer içinde PDFium çalışır; her Android sürümünde (API 21+) hazırdır, uygulamaya **sıfır bayt** ekler.
- Android 11+ cihazlarda Play sistem güncellemeleriyle yenilenir (Android 15'teki iyileştirmeler geriye taşındı).
- Word ile aynı sayfa ekranını ve önbelleği kullanır (`PagedViewModel` / `PagedScreen`).

## Neden şimdilik PdfCraft değil

PdfCraft'ın ortak kütüphaneleri (`craft-libs`) WordCraft/GridCraft'ınkinden farklı bir commit'e sabitli; aynı derlemede
birleştirmek ayrı bir iş. PdfCraft'ın asıl değeri düzenleme araçlarında (açıklama, form, imza, sayfa işlemleri,
karartma): bunlar Faz 2'de `mo-pdf` köprüsüyle gelir. Görüntüleme ve düzenleme aynı PDF baytlarını okuduğu için
tutarlılık sorunu beklenmez.

## Bilinen sınırlar ve yeniden değerlendirme

- Android 14 ve öncesinde PdfRenderer parolalı PDF açamaz → Faz 2'de PdfCraft ile çözülecek.
- Metin seçme/arama PdfRenderer'da yalnız yeni sürümlerde var → Faz 2'de PdfCraft metin çıkarımı.
- Eski cihazlarda gömülü PDFium sürümü eski kalabilir; sorun görülürse gömülü PDFium (bblanchon derlemeleri) değerlendirilir.
