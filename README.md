# MobileOffice Free

Android için **ücretsiz, reklamsız, hesapsız ve internetsiz** çalışan ofis ve belge uygulaması (çalışma adı).

- Word (DOCX, DOC, ODT, RTF), Excel (XLSX, XLS, CSV) ve PowerPoint (PPTX) dosyalarını açma ve düzenleme
- PDF görüntüleme, açıklama ekleme, form doldurma, imzalama, birleştirme, bölme, tarama ve OCR
- DWG ve DXF çizimlerini görüntüleme
- Görsel, metin, arşiv ve e-kitap dosyalarını açma; Türkiye'ye özgü UYAP UDF ve e-Fatura XML desteği

**Durum:** Faz 0 (teknik doğrulama). Çalışan parçalar: Excel/CSV görüntüleme ve basit hücre düzenleme (GridCraft), Word görüntüleme (WordCraft), PDF görüntüleme (Android PdfRenderer), PowerPoint görüntüleme (DeckCraft), DWG/DXF görüntüleme (CADCraft: yakınlaştırma, katmanlar, düzen sekmeleri). Ayrıca görsel ve düz metin görüntüleyicileri; tüm sayfalı görüntüleyicilerde yakınlaştırma, sayfaya git, gece modu, ekranı açık tut, ses tuşlarıyla sayfa çevirme ve yazdırma; ana ekranda Türkçe karakter duyarsız dosya adı araması. Kapsamlı rapor ve proje planı: [docs/RAPOR-VE-PLAN.md](docs/RAPOR-VE-PLAN.md)

**Test sürümünü indir:** [MobileOffice-Free.apk](https://github.com/bahadirdogus23-ui/Mobileoffice-free/releases/download/gelistirme/MobileOffice-Free.apk). CI her başarılı derlemede bu dosyayı yeniler ([sürüm sayfası](https://github.com/bahadirdogus23-ui/Mobileoffice-free/releases/tag/gelistirme)). Bu bir geliştirme sürümüdür; kurmak için telefonda bilinmeyen kaynaklardan yüklemeye izin verin. Sonraki test sürümleri bunun üzerine kurulur.

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
