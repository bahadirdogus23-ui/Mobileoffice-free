# ADR-0006: Cihazda Türkçe metin tanıma (OCR)

- **Durum:** Kabul edildi (7 Kasım 2026)
- **Bağlam:** Rapor Faz 2, “Türkçe OCR”; ADR-0005 (internet izni yok, Google hizmetleri yok)

## Karar

Metin tanıma **Tesseract** ile, tamamen telefonda yapılır:

- Kitaplık: [Tesseract4Android](https://github.com/adaptech-cz/Tesseract4Android) `4.9.0`
  (Apache 2.0; Tesseract ve Leptonica'yı Android için derler). Yalnızca JitPack'te yayımlandığı
  için JitPack depo olarak eklendi; Gradle'ın `exclusiveContent` süzgeci bu depodan **yalnızca**
  `cz.adaptech.tesseract4android` grubunun indirilmesine izin verir, başka hiçbir bağımlılık
  oradan gelemez. Sürüm sabittir; yükseltme ayrı bir değişiklikle yapılır.
- Model: `tessdata_fast` deposundaki `tur.traineddata` (Apache 2.0), commit
  `87416418657359cb625c412a48b6e1d6d41c29bd`, SHA-256
  `7393381111e1152420fc4092cb44eef4237580d21b92bf30d7d221aad192c6b7`, 4,5 MB. Uygulamanın
  içinde (`assets/tessdata`) gelir; ilk kullanımda uygulamanın kendi klasörüne kopyalanır.
  İndirme yoktur.

## Gerekçe

- Google ML Kit Play hizmetlerine bağlıdır ve modelini indirebilir: ADR-0005 ile çelişir.
- Saf Rust seçenekleri (ör. `ocrs`) Türkçe harfleri (ş, ğ, ı, İ) tanıyan bir modelle gelmez.
- Tesseract'ın Türkçe modeli olgundur ve çevrimdışıdır; “fast” model boyut ile doğruluk
  arasında telefon için uygun dengedir.

## Sonuçlar

- APK, her işlemci mimarisi için birkaç MB yerel kitaplık ve 4,5 MB model kadar büyür.
- İlk aşamada resim görüntüleyicide ve PDF sayfalarında “Metni tanı” vardır: tanınan metin
  seçilir, kopyalanır, paylaşılır ya da TXT olarak kaydedilir. Taranmış PDF'e görünmez metin
  katmanı eklemek (aranabilir PDF) sonraki adımdır.
- Tanıma süreleri Ölçümler'de “OCR: tanıma” olarak görünür.
