# MobileOffice Free

Android için **ücretsiz, reklamsız, hesapsız ve internetsiz** çalışan ofis ve belge uygulaması (çalışma adı).

- Word (DOCX, DOC, ODT, RTF), Excel (XLSX, XLS, CSV) ve PowerPoint (PPTX) dosyalarını açma ve düzenleme
- PDF görüntüleme, açıklama ekleme, form doldurma, imzalama, birleştirme, bölme, tarama ve OCR
- DWG ve DXF çizimlerini görüntüleme
- Görsel, metin, arşiv ve e-kitap dosyalarını açma; Türkiye'ye özgü UYAP UDF ve e-Fatura XML desteği

**Durum:** Faz 2–3 (düzenleme). Çalışan parçalar:
- **Word** (WordCraft): görüntüleme; yazma, kalın/italik/altı çizili, yazı tipi ve boyut, hizalama, madde/numara, başlık stilleri; resim, tablo, sayfa sonu ekleme; sürükleyerek seçme, kes/kopyala/yapıştır (biçimli), bul-değiştir; geri al/yinele; DOCX/ODT/RTF/MD/TXT/HTML olarak kaydetme, PDF'e dönüştürme. UYAP UDF görüntüleme ve dönüştürme.
- **Excel** (GridCraft): hücre düzenleme, **Türkçe sayı ve formül girişi** (`1.250,5`, `=TOPLA(A1;A2)`, EĞER, DÜŞEYARA…), formül önerileri ve formül tuşları; hücre biçimleri (kalın, renk, dolgu, hizalama, ₺, %), satır/sütun ekleme-silme, Türk alfabesine göre sıralama, sayfa ekleme/yeniden adlandırma/silme; XLSX kaydetme, sayfa yapısıyla PDF.
- **PowerPoint** (DeckCraft): görüntüleme, tam ekran sunum (lazer noktası, süre, notlar), slayt metnini düzenleme, slayt ekleme/çoğaltma/taşıma/silme, PPTX kaydetme, PDF.
- **PDF** (PdfRenderer + PdfCraft): sayfa araçları (döndürme, silme, ayıklama, birleştirme), elle imza ve kayıtlı imzalar, kalem/fosforlu kalem/not, form doldurma (Türkçe harflerle), parola koyma/kaldırma ve parolalı PDF açma, küçültme, **kişisel veri karartma** (T.C. kimlik no, IBAN, telefon, e-posta), Türkçe duyarlı arama ve sesli okuma (tüm Android sürümlerinde), TXT ve JPEG'e dönüştürme.
- **Tarama ve oluşturma:** kamerayla belge tarama (köşe düzeltme, belge/siyah-beyaz görünüm), resimlerden PDF.
- **Diğer:** DWG/DXF görüntüleme (CADCraft), GİB e-Fatura/e-Arşiv/e-İrsaliye görüntüleme, görsel, düz metin (kodlama seçimi ve onarımı), ZIP; gece modu, sesli okuma, ses tuşlarıyla sayfa çevirme, yazdırma; ana ekranda Türkçe duyarsız dosya araması ve Türkiye'ye özel dosya filtreleri.

Kapsamlı rapor ve proje planı: [docs/RAPOR-VE-PLAN.md](docs/RAPOR-VE-PLAN.md)

**Test sürümünü indir:** [MobileOffice-Free.apk](https://github.com/bahadirdogus23-ui/Mobileoffice-free/releases/download/gelistirme/MobileOffice-Free.apk). CI her başarılı derlemede bu dosyayı yeniler ([sürüm sayfası](https://github.com/bahadirdogus23-ui/Mobileoffice-free/releases/tag/gelistirme)). Bu bir geliştirme sürümüdür; kurmak için telefonda bilinmeyen kaynaklardan yüklemeye izin verin. Sonraki test sürümleri bunun üzerine kurulur. Bu dosya 64 bit ARM telefonlar içindir (son yılların hemen tüm telefonları, Android 8.0+); kurulmazsa [sürüm sayfasındaki](https://github.com/bahadirdogus23-ui/Mobileoffice-free/releases/tag/gelistirme) 32 bit ya da "hepsi bir arada" dosyasını deneyin.

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

## Derleme

Gerekenler: Rust (1.95+), JDK 17+, Android SDK, Android NDK (28.2.13676358 önerilir) ve `cargo-ndk`.

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
scripts/build-native.sh            # libmo_ffi.so + Kotlin bağlamaları
cd android && ./gradlew assembleDebug
```

Her push'ta CI APK'yı derler; Actions sayfasındaki `mobileoffice-debug-apk` dosyası telefona kurulabilir.
Katkı kuralları: [AGENTS.md](AGENTS.md).

## Lisans

Karar aşamasında (raporun 16. bölümü).
