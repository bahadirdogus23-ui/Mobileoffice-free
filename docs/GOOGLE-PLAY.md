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

## 3. Derleme (depoda hazır)

- Her başarılı derlemede CI, Play'in istediği **AAB** paketini (`app-release.aab`) üretir ve
  derlemenin “Artifacts” bölümüne `mobileoffice-release` olarak koyar. Gizli değerler
  eklendiyse yükleme anahtarıyla imzalıdır; eklenmediyse yalnızca deneme içindir.
- Aynı küçültülmüş (R8) sürüm, test sayfasında `MobileOffice-Free-yayin-denemesi.apk` adıyla
  yayımlanır: **Play'e yüklemeden önce bunu telefonda deneyin** (geliştirme sürümünden farklı
  olarak kod küçültülmüştür).
- Hedef API 36 (Android 16), en düşük Android 8.0. 16 KB sayfa hizalaması denetlenir.

## 4. Mağaza kaydı

| Alan | Durum |
|---|---|
| Uygulama adı | **Karar bekliyor** (ayırt edici yeni ad; aşağıya bakın) |
| Kısa ve uzun açıklama (tr, en) | Hazır: `fastlane/metadata/android/` |
| Simge 512×512, öne çıkan görsel 1024×500 | Hazırlanacak |
| Telefon ekran görüntüleri (en az 2, önerilen 4–8) | Telefonunuzdan çekilmeli |
| Kategori | Verimlilik (Productivity) |
| Gizlilik politikası adresi | `docs/GIZLILIK.md` (herkese açık bağlantısı verilecek) |
| İletişim e-postası | bahadirdogus23@gmail.com |

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
