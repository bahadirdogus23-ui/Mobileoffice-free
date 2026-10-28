# ADR-0002: Cihazdaki tüm belgeleri listeleme

- **Durum:** Kabul edildi (proje sahibinin kararı, 28 Ekim 2026)
- **Değiştirdiği öneri:** Rapor §16, karar 5 ("v1'de tüm dosyalara erişim istememek")

## Karar

Ev ekranı, referans uygulamalarda olduğu gibi telefondaki **tüm desteklenen belgeleri** listeler:

- Android 11+ (API 30+): `MANAGE_EXTERNAL_STORAGE` ("Tüm dosyalara erişim"). Kullanıcı Ayarlar'dan açar.
- Android 8–10: `READ_EXTERNAL_STORAGE` (`maxSdkVersion=32`), Android 10 için `requestLegacyExternalStorage`.
- Tarama MediaStore üzerinden yapılır (`DeviceFiles.scan`); dosya içeriği okunmaz, yalnız ad/boyut/tarih.

İzin verilmezse uygulama eksiksiz çalışır: "Dosya aç", "Birlikte aç" ve "Son açılanlar" izinsizdir.

## Sonuçlar

- **Google Play:** Bu izin için Play Console'da *Permissions Declaration Form* doldurulmalı, kullanım alanı "belge yönetimi"
  (Play politikasında izin verilen kullanımlardan biri) seçilmeli; onay garanti değildir. Reddedilirse Play sürümü izni
  kaldırır ("Son açılanlar" + klasör izni), F-Droid/GitHub sürümleri izni korur.
- **İnternet izni yine yok**; geniş depolama erişimiyle birlikte bu, gizlilik vaadinin dayanağıdır.
- Açıklama metni ve izin kartı bunu kullanıcıya açıkça söyler.
