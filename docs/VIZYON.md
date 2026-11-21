# BadoOffice vizyonu: neden özgün

Bu belge, BadoOffice'i "bir ofis uygulaması daha" olmaktan çıkaran şeyleri ve önceliklerini tanımlar.
Yeni bir özellik ya da tasarım kararı bu belgedeki ilkelere uymalıdır. Plan: [RAPOR-VE-PLAN.md](RAPOR-VE-PLAN.md).

## Tek cümle

**Türkiye'de sahada ve masada çalışan insanın, internetsiz, reklamsız, hesapsız, tek uygulamada bütün belgesini
okuyup düzenleyebildiği; verisini asla kaybettirmeyen, kendine ait bir görünümü olan cep ofisi.**

## Beş ilke

1. **Veri kutsaldır.** Açmak hiçbir şey yazmaz; kaydetmek dosyanın tamamını güvenle yazar; motorun tutamadığı her şey
   kişiye söylenir; silinen dosya önce çöp kutusuna gider ve uygulamanın içinden geri gelir. Kurtarma kopyası her
   düzenlemede vardır.
2. **İnternetsiz ve gizli.** İnternet izni yok, analiz yok, hesap yok. Bu bir eksiklik değil, ürünün kendisidir:
   OCR, yazı tipleri, dönüştürme, ölçüm, her şey telefonda çalışır.
3. **Türkiye'ye göre yapılmış.** Türkçe önce; `İ/ı` her yerde doğru; Türk resmî biçimleri birinci sınıf:
   UDF (UYAP) okuma **ve yazma**, e-Fatura/e-Arşiv görüntüleme, Türkçe OCR, eski Türkçe kod sayfalarındaki metinler,
   Türkçe formül adları, dönüm/hektar gibi birimler.
4. **Sahadaki mühendis için de.** DWG/DXF'i telefonda açıp ölçebilmek (taşınabilir ölçüm noktaları, yakalama,
   alan/çevre/açı/yarıçap/koordinat, CSV dışa aktarma), taranmış belgede güvenli karartma, imza ve form doldurma.
   Bunlar "ek" değil, uygulamanın ayırt edici yüzüdür.
5. **Kendine ait bir görünüm.** Hazır Material görünümü değil, "Bado tasarım dili": Türk renk paletleri (İznik, Lale,
   Kapadokya, Ege, Zeytin, Safran, Gece), kendi çizdiğimiz tutarlı simge seti, her ekranda aynı düğme ve menü
   bileşenleri, kişinin seçebileceği köşe biçimi, yoğunluk ve yazı boyutu. Ayrıntılar: [TASARIM-DILI.md](TASARIM-DILI.md).

## Ayırt edici özellikler (öncelik sırasıyla)

| # | Özellik | Durum |
|---|---|---|
| 1 | UDF düzenleyip yine UDF olarak kaydetme | Yapılıyor |
| 2 | DWG ölçüm araç takımı (taşınabilir noktalar, yakalama, alan, açı, koordinat, CSV) | Yapılıyor |
| 3 | Bado tasarım dili, Türk paletleri, Görünüm ayarları | Yapılıyor |
| 4 | Uygulama içi çöp kutusu; uzun basma ve ⋮ menüsünde aynı dosya eylemleri | Yapılıyor |
| 5 | Taranmış PDF'de resmi koruyan karartma | Yapılıyor |
| 6 | Zengin slayt tabloları (bütün stil aileleri, düzen, hücre biçimi) | Yapılıyor |
| 7 | Belgelerin içinde çevrimdışı arama (PDF/Word/Excel/sunu metninde, telefonda dizinle) | Sırada |
| 8 | PDF çiziminde ölçek ayarlı ölçüm (taranmış pafta/proje üzerinde mesafe ve alan) | Sırada |
| 9 | Türk resmî belge şablonları (dilekçe, tutanak, hakediş, teklif) | Sırada |
| 10 | Ölçüm ve tablolardan doğrudan Excel'e aktarma | Sırada |

## Kararlılık sözü

- Her derlemede emülatörde gerçek APK ile açma, kaydetme, kurtarma ve dosya yönetimi testleri.
- Bozuk ve kötü niyetli dosyalarla fuzz testleri; motor paniği uygulamayı çökertmez.
- Büyük dosyalarda bellek tahmini ve uyarı; büyük PDF'ler Java belleğine kopyalanmadan işlenir.
- Takılma ve süre ölçümleri telefonda tutulur, kişi isterse raporu kendisi paylaşır.
