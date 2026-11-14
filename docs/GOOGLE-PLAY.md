# Google Play'de yayın: adım adım

Bu belge uygulamayı Google Play'e çıkarmak için yapılacakları sırasıyla listeler. **Sizin**
yapmanız gerekenler (hesap, ödeme, kimlik, test kullanıcıları) ve **depoda hazır olanlar** ayrı
gösterilmiştir. Kurallar Google'ın yardım sayfalarına göre (Kasım 2026); başvurmadan önce Play
Console'daki güncel metne bakın.

## 1. Geliştirici hesabı (siz)

1. <https://play.google.com/console> adresinden **kişisel** geliştirici hesabı açın; bir kerelik
   kayıt ücreti ödenir (25 ABD doları).
2. Kimlik doğrulaması: ad, adres, telefon ve kimlik belgesi istenir. Mağazada geliştirici adı ve
   e-posta görünür (iletişim e-postası olarak bahadirdogus23@gmail.com kullanılabilir).
3. **Kapalı test şartı:** 13 Kasım 2023'ten sonra açılan kişisel hesaplar, yayına (üretim)
   geçmeden önce **en az 12 test kullanıcısıyla 14 gün kesintisiz** kapalı test yapmak zorunda.
   Şimdiden 12+ kişilik bir liste hazırlayın (Gmail adresleri); test kullanıcıları uygulamayı
   indirip birkaç gün kullanmalı.

## 2. Yükleme anahtarı (siz, bir kez)

Play, uygulamayı kendi anahtarıyla imzalar (Play App Signing); sizden yalnızca **yükleme
anahtarı** ister. Bu anahtar **asla** depoya, sohbete ya da e-postaya konmaz.

Bilgisayarda (Java kurulu) ya da telefonda Termux'ta (`pkg install openjdk-21`):

```sh
keytool -genkeypair -v -keystore upload.jks -alias upload -keyalg RSA -keysize 4096 -validity 10000
base64 -w0 upload.jks > upload.jks.b64
```

Sonra GitHub'da depo → **Settings → Secrets and variables → Actions → New repository secret**
ile dört gizli değer ekleyin:

| Ad | Değer |
|---|---|
| `UPLOAD_KEYSTORE_BASE64` | `upload.jks.b64` dosyasının içeriği |
| `UPLOAD_STORE_PASSWORD` | anahtar deposu parolası |
| `UPLOAD_KEY_ALIAS` | `upload` |
| `UPLOAD_KEY_PASSWORD` | anahtar parolası |

`upload.jks` dosyasını ve parolaları güvenli bir yerde yedekleyin (kaybolursa Play'den yeni
yükleme anahtarı istenebilir ama zahmetlidir).

## 3. Derleme ve doğrulama

- CI, Play'in istediği **AAB** paketini (`app-release.aab`) üretmek üzere yapılandırılmıştır ve
  derlemenin “Artifacts” bölümüne `mobileoffice-release` olarak koyar. Gizli değerler
  eklendiyse yükleme anahtarıyla imzalıdır; eklenmediyse yalnızca deneme içindir.
- APK ve AAB ayrı Gradle çağrılarıyla oluşturulur: önce `./gradlew assembleRelease`,
  ardından `./gradlew clean bundleRelease -Pmo.abiSplits=false`. CI temizleme öncesinde
  debug/release APK'larını ve lint raporlarını geçici dizine alır, sonrasında geri yükler.
  Yerelde `clean` önceki derleme çıktılarını siler; gerekli dosyaları önceden kopyalayın.
- **Doğrulama durumu (9 Kasım 2026):** önceki CI'da AAB/ABI split çakışması bildirilmiştir.
  Ayrılmış akışın başarılı CI çalışması, gerçek APK/AAB çıktıları ve Play imzası ayrıca
  doğrulanmalıdır. Yerel bulut ortamında OCR bağımlılığının `jitpack.io` erişimi 403 ile
  engellidir; burada tam Android derlemesi henüz doğrulanmamıştır.
- **Güncelleme (10 Kasım):** ağ engeli kalktı. Yerelde debug derlemesi, 48 Android
  birim testi, lint, release APK ve ayrı AAB derlemesi başarılı. Ancak yalnız ARM64
  yerel motoru hazır; test anahtarı kullanılıyor. Tüm ABI'ler, optimize yerel motor,
  Play yükleme sertifikası ve cihaz testi doğrulanmadan mağaza yayınına hazır değildir.
- Aynı küçültülmüş (R8) sürüm, test sayfasında `MobileOffice-Free-yayin-denemesi.apk` adıyla
  yayımlanır: **Play'e yüklemeden önce bunu telefonda deneyin** (geliştirme sürümünden farklı
  olarak kod küçültülmüştür).
- Hedef API 36 (Android 16), en düşük Android 8.0. 16 KB sayfa hizalaması denetlenir.

## 4. Mağaza kaydı

| Alan | Durum |
|---|---|
| Uygulama adı | **Karar bekliyor** (ayırt edici yeni ad; aşağıya bakın) |
| Kısa ve uzun açıklama (tr, en) | Hazır: `fastlane/metadata/android/` |
| Simge 512×512, öne çıkan görsel 1024×500 (tr, en) | Hazır: `fastlane/metadata/android/<dil>/images/` |
| Telefon ekran görüntüleri (en az 2, önerilen 4–8) | Telefonunuzdan çekilmeli (aşağıya bakın) |
| Kategori | Verimlilik (Productivity) |
| Gizlilik politikası adresi | `docs/GIZLILIK.md` (herkese açık bağlantısı verilecek) |
| İletişim e-postası | bahadirdogus23@gmail.com |

### “Yenilikler” metni

Play Console'daki “Bu sürümdeki yenilikler” alanı için hazır metinler
`fastlane/metadata/android/<dil>/changelogs/default.txt` dosyalarındadır (en çok 500 karakter).
Her yayından önce güncelleyin.

### Mağaza görselleri

- `fastlane/metadata/android/tr/images/icon.png` ve `en-US/images/icon.png`: 512×512, 32 bit PNG.
  Kare ve kenarlara taşan biçimdedir; köşe yuvarlatmayı ve gölgeyi Play kendisi ekler.
- `fastlane/metadata/android/tr/images/featureGraphic.png` (Türkçe slogan) ve
  `en-US/images/featureGraphic.png` (İngilizce slogan): 1024×500, saydamlıksız PNG.
- İkisi de uygulamanın kendi başlatıcı simgesinden (`android/app/src/main/res/drawable/ic_launcher_*.xml`)
  üretilir. Simge, ad ya da slogan değişirse yeniden üretin:

  ```sh
  python3 scripts/store-graphics.py
  ```

  Betik Node'daki Playwright'ın Chromium'unu ve yerel DejaVu Sans yazı tipini kullanır; internetten
  hiçbir şey indirmez. Slogan metinleri betiğin başındaki `LOCALES` tablosundadır.

**Telefon ekran görüntüleri (siz):** Play en az 2 tane ister (önerilen 4–8); 9:16 (dikey) ya da
16:9 oran önerilir, kenarlar 320–3840 piksel arası olmalı ve uzun kenar kısa kenarın en fazla 2
katı olabilir; JPEG ya da saydamlıksız PNG. Yeni telefonların dikey görüntüleri (ör. 1080×2400,
20:9) bu sınırı aşar: 1080×1920'ye (9:16) kırpın.

1. Uygulamayı telefona kurun, örnek belgeler açın (kişisel bilgi içermeyen: ör. bir bütçe tablosu,
   bir sunum, bir Word belgesi ve belge listesi). Bildirim çubuğunu sade tutun (saat, tam pil).
2. Her ekranda **Güç + Ses kısma** tuşlarına birlikte basın; görüntüler Galeri → Ekran görüntüleri
   klasörüne kaydedilir.
3. Gerekirse Galeri'deki düzenleyiciyle 9:16 orana kırpın.
4. Dosyaları `fastlane/metadata/android/tr/images/phoneScreenshots/` klasörüne `1.png`, `2.png`…
   adlarıyla koyun (İngilizce arayüzle çekilenler `en-US/images/phoneScreenshots/` klasörüne) ya da
   doğrudan Play Console'a yükleyin.

## 5. Play Console formları (yanıtlar hazır)

- **Veri güvenliği:** “Uygulama kullanıcı verisi topluyor ya da paylaşıyor mu?” → **Hayır.**
  İnternet izni yok; hiçbir veri cihazdan çıkmaz.
- **Reklam içeriyor mu?** → Hayır.
- **İçerik derecelendirmesi:** Yardımcı/verimlilik uygulaması; şiddet, cinsellik, kumar yok →
  “Herkes / 3+”.
- **Hedef kitle:** 13 yaş ve üstü (çocuklara yönelik değil).
- **Tüm dosyalara erişim (MANAGE_EXTERNAL_STORAGE) beyanı:** İzin “belge yönetimi” amacıyla
  istenir: uygulama telefondaki Word, Excel, PowerPoint, PDF, UDF ve diğer belgeleri bulup
  listeler, açar ve düzenler. Storage Access Framework tek tek dosya açmak için yeterlidir ve
  uygulama izinsiz de onunla çalışır, ancak telefondaki **tüm belgeleri tek listede gösterme ve
  içlerinde arama** (uygulamanın ana işlevi) için yetmez. Bu işlev mağaza açıklamasında öne
  çıkarılmıştır. (Bkz. ADR-0002.) Reddedilirse izin Play sürümünden çıkarılır; uygulama klasör
  seçerek çalışmaya devam eder.
- **Bağış bağlantısı (“Bana bir kahve al”):** sahibinin kararıyla Play sürümünde de kalır.
  İncelemede ödeme politikasıyla ilgili uyarı gelirse bu bağlantı Play sürümünden kaldırılır.

## 6. Yayın sırası

1. Dahili test (hemen, birkaç kişi) → 2. **Kapalı test (12+ kişi, 14 gün)** → 3. Üretim
   erişimi başvurusu → 4. Üretim (önce %10–20 aşamalı yayın).
