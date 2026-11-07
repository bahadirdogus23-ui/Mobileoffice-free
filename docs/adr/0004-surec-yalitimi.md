# ADR-0004: Süreç yalıtımı

- **Durum:** Kabul edildi (Faz 0 kapanışı, 7 Kasım 2026)
- **Bağlam:** Rapor §9.7 ve §16 (motorları ayrı bir süreçte çalıştırma seçeneği)

## Karar

Belge motorları şimdilik **uygulamayla aynı süreçte** çalışır. Ayrı süreç (`android:process` veya
`isolatedProcess` hizmeti) v1.0 öncesinde ancak çökme verisi gerektirirse eklenecek.

## Gerekçe

- Motorların tamamı güvenli Rust'tır; içlerinde C/C++ kitaplık yoktur. PDF görüntüleme Android'in
  kendi `PdfRenderer`'ıyla yapılır (ADR-0003). Bu yüzden bellek bozulmasıyla süreci çökertecek yerel
  kod yolu yok denecek kadar azdır.
- Bir motorda oluşan `panic` uygulamayı çökertmez. UniFFI onu Kotlin'de bir istisnaya çevirir
  (`mo-ffi/src/lib.rs`). Kilit zehirlenirse (`PoisonError`), belge kullanılmaya devam eder.
- Kotlin tarafında yakalanmayan hatalar `Diagnostics` ile kaydedilir ve kullanıcıya e-postayla
  bildirme imkânı sunulur. Kaydedilmemiş değişiklikler 30 saniyede bir kurtarma kopyasına alınır
  (`RecoveryStore`); çökmeden sonra dosya yeniden açılınca geri yükleme önerilir.
- Ayrı süreç her belge için ek bellek, Binder üzerinden büyük bayt dizisi kopyası (sayfa
  resimleri) ve karmaşık yaşam döngüsü getirir. Bu maliyet şu an kazancından büyüktür.

## Yeniden değerlendirme ölçütü

Beta sürecinde motor kaynaklı yerel çökme oranı %0,5'i geçerse ya da güvenilmeyen kaynaklardan
gelen dosyalarla ilgili bir güvenlik açığı bulunursa, PDF ve CAD motorları ayrı bir süreçte
çalışacak şekilde taşınır.
