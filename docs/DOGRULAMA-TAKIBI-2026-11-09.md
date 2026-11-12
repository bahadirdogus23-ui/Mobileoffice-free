# 9 Kasım 2026 raporu: doğrulama ve açık işler

Bu kayıt, paylaşılan proje durum raporunu yerel kaynak ve testlerle karşılaştırır.
Kod değişikliği, başarılı CI veya cihaz kabul testi yerine geçmez. Yayın yapılmadı.

## 10 Kasım güncellemesi: Android derleme engeli aşıldı

- `jitpack.io` erişimi çalışıyor; önceki 403 engeli kalktı.
- `assembleDebug testDebugUnitTest lintDebug` başarılı; 13 test sınıfında 48 test
  geçti, hata veya atlanan test yok.
- `assembleRelease`, ardından ayrı `clean bundleRelease -Pmo.abiSplits=false`
  çağrısı başarılı. `buildReleasePreBundle` geçti; önceki çoklu küçültülmüş kaynak
  hatası bu yerel derlemede tekrarlanmadı. Uzak CI henüz çalıştırılmadı.
- ARM64 release APK: yaklaşık 145,6 MB; apksigner, zipalign (16 KB) ve paket içi
  ELF/ABI kontrolleri geçti. Boyut engel olarak kullanılmadı. Yerel motor debug
  profilinde olduğundan bu optimize edilmiş son release boyutu değildir.
- AAB: 119.480.188 bayt; ZIP CRC kontrolü ve `jarsigner -verify` başarılı.
  Test/debug sertifikası kendinden imzalıdır; bu Play yükleme imzası doğrulaması değildir.
  `bundletool validate` ve cihaz kurulumu henüz yapılmadı.
- Yerel motor yalnız ARM64 için derlenmiştir. Diğer ABI çıktıları tam değildir;
  AAB ve universal paket tüm cihazlara dağıtıma hazır sayılmaz.
- APK'lar, AAB, test sonuçları ve raporlar yerelde
  `/workspace/mobileoffice-setup/verified-builds/` altında korundu.

Aşağıdaki 9 Kasım ağ engeli kayıtları tarihsel durumdur; güncel sonuçlar yukarıdadır.

## Bu turda yapılanlar

- APK ve AAB için ayrı Gradle çağrıları ve AAB için `-Pmo.abiSplits=false` eklendi.
  Temiz derleme öncesinde APK'lar ve raporlar korunur; çıktı dosyalarının varlığı kontrol edilir.
- Gerçek Gradle yapılandırmasında varsayılan ABI split açık, özellik verildiğinde kapalı
  olduğu doğrulandı. Tam release APK/AAB derlemesi henüz doğrulanmadı.
- Gizlilik politikasında cihaz içi saklama ile sunucuya veri toplama ayrıldı.
- ADR-0004'te Tesseract/C++ ve yerel süreç çökmesi riski açıklandı.
- RAR ve eski PPT/PPS sınırları README'ye eklendi.
- Mevcut PDF karartma testinin kaydettiği PDF, bağımsız Poppler metin çıkarımıyla
  kontrol edildi: testteki kişisel veriler çıkarılamıyor, kalan metin korunuyor.
  Tekrarlamak için: `bash scripts/check-pdf-redaction.sh` (Rust ve `poppler-utils` gerekir).
  Bu sentetik örnek, saha raporundaki 58 kimlik numaralı dosyanın yeniden testi değildir.
- Üç sayfada 58 ayrı, sentetik ve kontrol basamakları geçerli kimlik numarası içeren
  regresyon eklendi. Boşluklu/boşluksuz yazımlar, işaretleri kaydetme/yeniden açma,
  tüm işaretleri uygulama ve yeniden kaydetme sınanır. Poppler özgün dosyada 58
  numaranın her birini görmeli; son dosyada hiçbir rakam kalmamalı ve üç sayfanın
  korunacak metinleri durmalıdır. Bu denetim CI'a eklendi.
- Kaydedilen DOCX/XLSX/PPTX ve UDF'den dönüştürülen DOCX için bağımsız okuyucu
  kontrolleri eklendi: Türkçe metin, UDF tablosu, XLSX formülü ve önbellek değeri,
  düzenlenen/kopyalanan slayt metni. Bunlar küçük sentetik örneklerdir; karmaşık
  gerçek belge korpusunun görsel sadakat testi yerine geçmez.
  Kurulum: `python3 -m pip install -r scripts/interop-requirements.txt` (sanal ortamda).
  Çalıştırma: `bash scripts/check-office-interop.sh`. CI da bu denetimi çalıştırır.
- Ayarlar'daki hata/öneri bildiriminin mevcut kaynakta tek giriş olduğu görüldü;
  eski Z4 kaydı cihaz yeniden testi olmadan kapatılmadı.
- CI'a APK/AAB içindeki yerel motorların ABI, ELF mimarisi ve 16 KB hizalama
  kontrolü eklendi; kontrol üçüncü taraf `.so` dosyalarını da kapsar. 32 bit ARM
  hizalama istisnası korunur. Paket denetleyicisinin dokuz regresyon testi vardır.
  Bu kontrol imza doğrulaması veya `bundletool validate` yerine geçmez.
- CI, debug/release APK'lar ve AAB için `package-size-reports` artifact'i üretmek
  üzere yapılandırıldı: gerçek arşiv boyutu, yerel kitaplıklar ve en büyük 20 girdi.
  ARM64 APK 60.000.000 baytı aşarsa hedef aşımı raporlanır; hedef değiştirilmedi.
  Ağ engeli nedeniyle bu turda gerçek APK/AAB boyutu ölçülemedi.

## Doğrulanmış temel

Bu turdan önce aynı çalışma alanında 134 Rust testi, 17 JVM testi, biçim kontrolü
ve Clippy geçti. ARM64 yerel motor derlendi ve 16 KB ELF hizalaması doğrulandı.
Android APK/test/lint denemesi OCR bağımlılığı için `jitpack.io` 403 yanıtında durdu.
Ortam izin listesine bu alan adı eklenmeden tam Android doğrulaması yapılamıyor.

## Açık kabul işleri

| Öncelik | İş | Kapanış kanıtı / gereken girdi |
|---|---|---|
| P1 | Release CI | Başarılı çalıştırma; dört release APK ve AAB; debug APK ve lint raporlarının korunması |
| P1 | Play imzası | Sahibi tarafından güvenli secret kurulumu; yükleme sertifikası ve AAB doğrulaması; Play test kanalı sonucu |
| P1 | PDF karartma | Saha dosyası veya anonim eşdeğeri; tüm işaretlerin uygulanması ve bağımsız metin çıkarımı |
| P1 | Belge kaybı | DOCX/XLSX/PPTX/PDF/UDF korpusu; aç–düzenle–kaydet; harici okuyucu karşılaştırması |
| P2 | Panel 2 regresyonu | S1/S2/S4, W4, X5/X12/X16, Y3/Y4/Y6, C2, Z4 için yeni APK/sürüm, cihaz ve geçti/kaldı kanıtı; Z3 adımının açıklanması |
| P2 | Boyut ve performans | Release ARM64 APK boyutu; üç donanım sınıfında bellek, açılış ve düzenleme süreleri |
| P2 | Format kapsamı | Gerçek dosya başına açma/kaydetme sonucu; sınırlı desteğin açık etiketlenmesi |
| P3 | Mağaza ve ürün | Sahip tarafından ad/logo, hesap türü, görseller ve dağıtım kararları |

İmza anahtarları sohbete veya depoya eklenmez. Dal adı değiştirilmedi, GitHub issue/PR
açılmadı, uzak CI tetiklenmedi ve hiçbir paket yayımlanmadı. Bu belge yerel iş takibidir.
