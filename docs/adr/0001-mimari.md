# ADR-0001: Mimari — ArtCraft Rust motorları + Kotlin/Compose arayüzü

- **Durum:** Kabul edildi (Faz 0, 1. hafta)
- **Tarih:** 28 Ekim 2026
- **Bağlam:** Rapor §9.1 seçenekleri karşılaştırdı (A–E).

## Karar

- Belge motorları ArtCraft'ın MIT/Apache-2.0 Rust motorlarıdır; her biri `rust/mo-<biçim>` köprü crate'iyle sarılır.
- Tek yerel kütüphane `libmo_ffi.so`'dur; Kotlin API'si UniFFI ile üretilir (JNA üzerinden).
- Arayüz Kotlin + Jetpack Compose'dur. Düzenlemeler motorun komut kaydı üzerinden yapılır (`execute(id, json)`).
- Her belge için motor çağrıları tek bir iş parçacığında yapılır.

## İlk doğrulama (GridCraft, 28.10.2026)

- `gridcraft-engine` (commit `11bb673`) Android arm64 hedefi için değişikliksiz derleniyor (`cargo check --target aarch64-linux-android`).
- Rust testleri: XLSX aç/kaydet, formül hesaplama, Windows-1254 Türkçe CSV, bozuk girdide hata (çökme yok).
- Üretilen Kotlin bağlamaları masaüstü JVM'de gerçek kütüphaneyle test edildi (3/3).

## Sonuçlar

- Android SDK/NDK bu geliştirme ortamında indirilemediği için APK'yı CI derler (`.github/workflows/ci.yml`).
- Açık konular: süreç yalıtımı (ADR-0003), PDF görüntüleme motoru (ADR-0002), kopyasız render JNI'si (Word/Sunum/PDF ile gelecek).
- Geçici kimlikler: `applicationId = org.mobileofficefree.app`, Kotlin paketi `org.mobileofficefree.engine`;
  ilk mağaza sürümünden önce isim kararıyla (rapor §16) değişebilir.
