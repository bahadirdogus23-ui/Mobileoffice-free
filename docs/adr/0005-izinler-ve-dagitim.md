# ADR-0005: İzinler, Android sürümü ve dağıtım

- **Durum:** Kabul edildi (Faz 0 kapanışı, 7 Kasım 2026)
- **Bağlam:** Rapor §16 kararlar 4, 6, 7, 8

## Karar

**İzinler.** Uygulama yalnız depolama izinlerini ister; bunların amacı kullanıcının belgelerini
listelemektir (ADR-0002). **İnternet izni yoktur**: belgeler cihazdan hiç çıkmaz, analitik ve
Google Play hizmetleri kullanılmaz. Kamera izni de yoktur; tarama için telefonun kendi kamera
uygulaması açılır. İzin verilmese bile dosya seçici üzerinden her şey çalışır.

**Android sürümü.** `minSdk 26` (Android 8.0), `targetSdk 36` (Android 16). Play'in yıllık hedef
API şartı Faz 4 takvimindedir.

**Düzenleme kapsamı.** DOCX, XLSX, PPTX ve PDF yerinde düzenlenir ve kaydedilir. Eski biçimler
(DOC, XLS, PPT), CSV, UDF ve benzerleri görüntülenir; düzenlenince modern biçimde kopya olarak
kaydedilir. Açmak dosyaya yazmaz, kaydetmek dosyanın tamamını yazar.

**Dağıtım.**

1. Şimdi: GitHub. Her yeşil derleme, "gelistirme" ön sürümündeki APK'leri yeniler; dört ayrı APK
   vardır (64-bit, eski 32-bit, x86_64 ve hepsi bir arada).
2. Sonra: Google Play dahili test, ardından kapalı test (12+ kişi, 14 gün). Bunun için hesap türü
   kararı (rapor §16 karar 9) ve imzalama anahtarının güvenli saklanması gerekir.
3. v1.0 ile: F-Droid / IzzyOnDroid. Uygulamada kapalı kaynaklı bağımlılık olmadığı için uygundur.

## Bekleyen

Uygulamanın lisansı (rapor §16 karar 2) proje sahibinin kararıdır. Karar verilince ayrı bir ADR
ve depo kökünde `LICENSE` dosyası eklenecek. F-Droid ve Play yayını bundan sonra yapılabilir.
