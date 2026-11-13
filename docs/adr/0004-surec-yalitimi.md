# ADR-0004: Süreç yalıtımı

- **Durum:** Kabul edildi (Faz 0 kapanışı, 7 Kasım 2026)
- **Bağlam:** Rapor §9.7 ve §16 (motorları ayrı bir süreçte çalıştırma seçeneği)

## Karar

Belge motorları şimdilik **uygulamayla aynı süreçte** çalışır. Ayrı süreç (`android:process` veya
`isolatedProcess` hizmeti) v1.0 öncesinde ancak çökme verisi gerektirirse eklenecek.

## Gerekçe

- Belge motorlarının büyük bölümü Rust ile yazılmıştır; bu, tüm yerel kodun bellek güvenli
  olduğunu kanıtlamaz. OCR, Tesseract4Android üzerinden C/C++ kitaplıklarını kullanır (ADR-0006);
  PDF görüntüleme de Android'in yerel `PdfRenderer` bileşenine dayanır (ADR-0003).
  Yerel bellek hataları ve süreç çökmeleri ayrıca değerlendirilmelidir.
- UniFFI sınırında yakalanabilen Rust `panic` hataları Kotlin'de bir istisnaya çevrilir
  (`mo-ffi/src/lib.rs`). Bu koruma yerel sinyalleri, bellek bozulmasını veya süreç sonlandırılmasını
  kapsamaz. Kilit zehirlenirse (`PoisonError`), belge kullanılmaya devam eder.
- Kotlin tarafında yakalanmayan hatalar `Diagnostics` ile kaydedilir ve kullanıcıya e-postayla
  bildirme imkânı sunulur. Kaydedilmemiş değişiklikler 30 saniyede bir kurtarma kopyasına alınır
  (`RecoveryStore`); çökmeden sonra dosya yeniden açılınca geri yükleme önerilir.
- Ayrı süreç her belge için ek bellek, Binder üzerinden büyük bayt dizisi kopyası (sayfa
  resimleri) ve karmaşık yaşam döngüsü getirir. Bu maliyet şu an kazancından büyüktür.

## Yeniden değerlendirme ölçütü

Beta sürecinde motor kaynaklı yerel çökme oranı %0,5'i geçerse ya da güvenilmeyen kaynaklardan
gelen dosyalarla ilgili bir güvenlik açığı bulunursa, PDF ve CAD motorları ayrı bir süreçte
çalışacak şekilde taşınır. OCR/Tesseract kaynaklı yerel çökmeler de bu incelemeye dahil edilir;
OCR için ayrı süreç gereksinimi ayrıca değerlendirilir. Henüz cihaz matrisiyle doğrulanmış
bir yerel çökme oranı yoktur.
