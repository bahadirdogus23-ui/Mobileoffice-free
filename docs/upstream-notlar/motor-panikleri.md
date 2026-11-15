# Motor panikleri, takılmalar ve bellek patlamaları (sağlamlık testleri)

Plan 5. aşama (kararlılık, çökme oranı < %0,5) için bozuk ve kötü niyetli dosyalarla yapılan
"mutasyon" testlerinin bulguları. Testler her köprünün `src/robustness.rs` dosyasında; yardımcısı
`rust/mo-fuzz` (bağımlılıksız, belirlenimci xorshift, bekçi iş parçacığı, sayaçlı ayırıcı).

## Panik stratejisi: neyi yakalıyoruz, neyi yakalayamıyoruz

- `rust/Cargo.toml` sürüm profilinde `panic = "unwind"` (açıklamasıyla) zaten var; `abort` değil.
  Değiştirilmedi.
- UniFFI 0.32 dışa açılan her işlevi `uniffi_core::ffi::rustcalls::rust_call` içinde
  `std::panic::catch_unwind` ile sarar: `libmo_ffi.so` içindeki bir panik Kotlin'e
  `InternalException` (bir `kotlin.Exception`) olarak gelir, uygulamayı düşürmez. Uygulama açma
  yollarını `runCatching` ile çağırdığından bu istisna da hata olarak gösterilir.
- Bunun için `unwind` şart: `panic = "abort"` olsaydı her panik süreci sonlandırırdı. Bedeli biraz
  daha büyük `.so` (unwind tabloları ve iniş blokları; fark bu çalışmada ölçülmedi). Öneri:
  **`unwind` kalmalı**; boyut kazancı bir çökmeye değmez.
- Panik sırasında tutulan `Mutex` zehirlenir; `mo-ffi` belgeleri `unwrap_or_else(PoisonError::into_inner)`
  ile almaya devam ediyor (motor kendi durumunu geri alıyor).
- **Yakalanamayanlar** (bu yüzden köprülerde önlendi): yığın taşması (derin özyineleme; Android'de
  `Dispatchers.IO` iş parçacığının yığını ~1 MB), ayırma hatası (`handle_alloc_error` → abort;
  ör. dosyanın bildirdiği 4 GB'lık sözlük), sonsuz döngü/ikinci dereceden süre (istisna yok, ANR).

Köprülerde ek `catch_unwind` eklenmedi: UniFFI zaten her çağrıyı sarıyor ve aşağıdaki testlerde
motorlardan hiç panik çıkmadı. Bir motor paniği bulunursa, açma çağrısını `catch_unwind` ile
sarıp `MoError::Open` döndürmek (Kotlin'de tür ayrımı için) yeterli olur.

## Taranan motor yolları ve sonuç

Varsayılan çalıştırma ve ayrıca `MO_FUZZ_ITERS=2000 MO_FUZZ_SEED=777` ile:

| Hedef | Motor | Panik | Takılma / bellek |
|---|---|---|---|
| `word-open` (bozuk DOCX/ODT/RTF/HTML/MD) | WordCraft | yok | 256 KB Markdown → 2416 sayfa: dizgi 1,6 sn (release); kusur değil |
| `udf` (UDF → HTML içe aktarma) | WordCraft | yok | yok |
| `xlsx-open`, `xls-open`, `csv` | GridCraft | yok | yok |
| `pptx`, `ppt-cfb` | DeckCraft | yok | yok |
| `dxf-open` (açma, katmanlar, kapsam, çizim) | CADCraft | yok | yok |
| `ttf` (bozuk TrueType) | cadcraft-fonts | yok | yok |
| `pdf` (açma, notlar, metin, arama, karartma, OCR, kaydetme) | PdfCraft | yok | **var, aşağıda** |

## PdfCraft: tekrarlanan `trailer` ile çapraz başvuru onarımı ikinci dereceden

- Sabit sürüm: `pdfcraft` @ `0f22627bff6cde9930817e891290f653d882bbda`,
  `crates/cos/src/document/reader.rs` ~736. satır.
- Çapraz başvuru tablosu okunamayınca dosya nesne başlıklarından yeniden kuruluyor; ardından
  dosyadaki **her** `trailer` sözcüğünden sonra `Lexer::object()` çağrılıyor. Ayraçsız
  "trailertrailertrailer…" dizisinde her çağrı dizinin sonuna kadar tek bir sözcük okuyor: süre
  tekrar sayısının karesiyle büyüyor.
- Yeniden üretme: geçerli 2 sayfalık bir PDF'in sonundaki `xref` girdilerinden birini bozun ve
  `startxref`'ten önce boşluksuz N kez `trailer` yazın. x86 release ölçümü (yalnız
  `Document::open_with_password`): 170 KB → 3,4 sn, 206 KB → 5,1 sn, 276 KB → 9,5 sn,
  416 KB → 21 sn. Telefonda birkaç kat yavaş: 400 KB'lık bir dosya açılırken uygulama dakikalarca
  yanıt vermez. Bulan girdi: `pdf` hedefi, tohum 777, 4. durum (146 798 bayt).
- Panik değil, köprüden kesilemez (motorun içinde). Öneri upstream'e: `trailer` sözcüğünden sonra
  boşluk/ayraç ya da `<<` beklemek, ya da en fazla bir sözcük uzunluğu okumak.

## Köprülerde ve bağımlılıklarda bulunup düzeltilenler (özet)

- `roxmltree` 0.20 belirteçleyicisi öğe başına özyinelemeli: 18 000 iç içe öğeli EPUB OPF'si yığını
  taşırdı (abort). 0.21 (ağaçta zaten vardı) iç içeliği döngüyle okuyor; `mo-convert` ve `mo-deck`
  ona geçti.
- `sevenz-rust2` `read_file`, katı bloktaki önceki girdileri bellekte tam tutuyor: küçük bir
  arşivdeki dev sıfır girdi belleği dolduruyordu; ayrıca bellek sınırı verilmeyince bildirilen
  sözlük boyutu kadar ayırma deneniyordu. `mo-archive` artık yalnız ilgili bloğu çözüp öncekileri
  atıyor ve 256 MB sınır veriyor.
- Kendi kodumuzdaki ikinci dereceden taramalar, sınırsız ayırmalar ve özyineleme: commit
  mesajlarında (`mo-convert`, `mo-archive`, `mo-word`, `mo-grid`, `mo-deck`, `mo-pdf`).
