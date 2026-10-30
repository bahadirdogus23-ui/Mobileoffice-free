# Cihaz ölçümleri (Faz 0)

Uygulamadaki **Ölçümler** ekranından alınır (ana ekran, sağ üst). Her satır gerçek bir telefondan gelir.

| Tarih | Cihaz | Android | Sürüm | APK | libmo_ffi.so | Toplam bellek (PSS) | Dosya açma | İlk sayfa |
|---|---|---|---|---|---|---|---|---|
| 30.10.2026 | Samsung Galaxy S25 Ultra (SM-S938B), 11 GB RAM | 17 (API 37) | 0.1.0-dev.23 | 47,6 MB (arm64) | 35,8 MB | 281,0 MB | PDF 8–153 ms, HTML (0,7 MB) 25–44 ms, XLSX 29 ms; Word → PDF 34 ms | PDF 48–67 ms, Word 86–136 ms |
| 28.10.2026 | Samsung Galaxy S25 Ultra (SM-S938B), 11 GB RAM | 17 (API 37) | 0.1.0-dev.18 | 43,8 MB (arm64) | 32,0 MB | 471,6 MB (sayfa önbelleği 384 MB'a kadar; dev.19'da 256 MB'a indirildi) | PDF 364 sayfa / 6,5 MB: 444 ms, MD 94 ms, XLSX 20–42 ms | PDF 111 ms, Word 137 ms |
| 28.10.2026 | Samsung Galaxy S25 Ultra (SM-S938B), 11 GB RAM | 17 (API 37) | 0.1.0-dev.15 | 106,4 MB (tek APK, üç ABI) | 32,0 MB (arm64) | 139,9 MB | PDF 12–183 ms, DOCX 16 ms, MD 39 ms, XLSX 19–24 ms | PDF 33–72 ms, Word 70–106 ms (1080 px) |

## Notlar

- **dev.15 ile görülen ve düzeltilen sorunlar:**
  - Yakınlaştırınca sayfalar yanıp sönüyordu. Sayfa önbelleği Java yığınına göre ayarlanmıştı ve büyütülmüş bir sayfa önbelleğin tamamını dolduruyordu. Önbellek artık RAM'e göre ayarlanıyor.
  - Ekranın üstünde ve altında boşluk kalıyordu, çünkü kenar payları iki kez ekleniyordu.
- dev.18'de kullanıcı tarafından doğrulandı: yanıp sönme ve kenar boşlukları düzeldi.
- Sonraki sürümlerden itibaren işlemci başına ayrı APK var: arm64 için 45,9 MB.
- Bekleyen ölçümler: eski/yavaş bir tablet ve 3–4 GB RAM'li bir telefon.
