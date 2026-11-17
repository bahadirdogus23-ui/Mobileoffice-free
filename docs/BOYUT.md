# Motor kitaplığının boyutu (`libmo_ffi.so`)

15 Kasım 2026. Telefonda arm64 sürüm APK'sı ~60 MB, bunun 48,0 MB'ı `libmo_ffi.so`. Google Play'de
indirme boyutu kurulum oranını etkilediği için bu kitaplığın neden bu kadar büyük olduğu ölçüldü ve
özellik kaybetmeden küçültüldü.

Ölçümler ana makinede (x86_64, `cargo build -p mo-ffi --release`, aynı sürüm profili, rust-lld)
yapıldı: yerelde Android NDK yok, arm64 bağlanamadı. x86_64 kodu arm64'ten daha büyük çıkıyor (28
Ekim APK'sında x86_64 39,5 MB, arm64 33,6 MB), yani mutlak sayılar değil **oranlar** Android'e
taşınır. Gerçek etkiyi CI'nin paket boyutu raporu (`package-size-reports`) gösterecek.

## Neden 48 MB? (ana makine, önceki durum: 66,8 MB)

| Bölüm | Boyut | Ne |
|---|---:|---|
| `.text` | 42,6 MB (%64) | Makine kodu: altı motor ve bağımlılıkları |
| `.rodata` | 9,5 MB (%14) | Sabit veriler; bunun 5,8 MiB'ı gömülü dosyalar (aşağıda), kalanı tablolar ve metinler |
| `.eh_frame` + `.eh_frame_hdr` | 5,7 MB (%9) | Panik yakalamak için çözme (unwind) tabloları |
| `.rela.dyn` | 3,6 MB (%5) | 148 669 göreli yer değiştirme kaydı, her biri 24 bayt |
| `.gcc_except_table` | 3,1 MB (%5) | Panik yakalama iniş blokları (`panic = "unwind"`) |
| `.data.rel.ro` | 1,8 MB (%3) | İşaretçi içeren sabitler (vtable'lar) |

Kod, simgelerin ait olduğu crate'e göre (sembollü derlemeden, `nm`; genel `core`/`alloc` kodunun
çoğu aslında motorların tek biçimlendirilmiş kodu):

| Kaynak | Kod + veri |
|---|---:|
| WordCraft (`wordcraft_*`) | 5,0 MiB |
| `core` + `alloc` + `hashbrown` (motorlar için üretilen genel kod) | 9,1 MiB |
| GridCraft (`gridcraft_*`) | 4,0 MiB |
| acadrust (DWG/DXF okuyucu) | 2,7 MiB |
| fearless_simd (vello'nun SIMD yolları) | 2,4 MiB |
| PdfCraft (`pdfcraft_*`) + hayro | 2,9 MiB |
| Yazı tipi okuma/şekillendirme (read-fonts, skrifa, harfrust ×3 sürüm, rustybuzz, ttf-parser) | 3,8 MiB |
| DeckCraft (`deckcraft_*`) | 1,3 MiB |
| vello | 1,3 MiB |
| CadCraft (`cadcraft_*`) | 1,3 MiB |
| serde_json + serde | 2,0 MiB |
| image, zune, tiff, png, moxcms | 1,3 MiB |

### Gömülü dosyalar (`include_bytes!`), toplam 5,8 MiB

Pinlenmiş motor kaynaklarındaki her veri dosyası `.so` içinde birebir arandı:

| Dosya | Boyut | Kim gömüyor |
|---|---:|---|
| Source Sans 3 (4 kesim) | 1,53 MiB | WordCraft (`wordcraft-fonts`) |
| Inter (3 kesim) | 1,19 MiB | WordCraft ve PdfCraft; **zaten tek kopya** |
| Source Serif 4 (5 kesim) | 1,13 MiB | WordCraft |
| Heceleme sözlükleri (en-us.dic, de/en/pl/it/pt .pat) | 0,84 MiB | WordCraft |
| Ubuntu Light | 0,34 MiB | DeckCraft'ın son çare yazı tipi (`epaint_default_fonts`) |
| Foxit standart PDF yazı tipleri (14 adet .pfb) | 0,25 MiB | hayro (PdfCraft), `embed-fonts` özelliği |
| JetBrains Mono | 0,26 MiB | WordCraft ve PdfCraft; tek kopya |
| CMap tabloları (cmaps.brotli) | 0,24 MiB | hayro, `embed-cmaps` (CJK PDF'ler) |

- Aynı yazı tipini iki motorun ayrı ayrı gömmesi **yok**: Inter ve JetBrains Mono `.so` içinde birer
  kez geçiyor. DeckCraft'ın derleme betiği `CRAFT_FONTS_DIR` verilmezse yazı tipi gömmüyor (CI'de
  verilmiyor). PdfCraft'ın DancingScript'i ve `epaint_default_fonts`'un Hack/NotoEmoji/simge
  yazı tipleri kullanılmadıkları için bağlayıcı tarafından atılıyor.
- ICU verisi (`icu_segmenter_data` vb., pdfcraft-edit ve `url` üzerinden ağaçta) ölçülebilir yer
  kaplamıyor: kullanılmayan kısmı atılıyor.
- Gömülü dosyalar `.so`'nun yalnızca ~%10'u. Bunlar motorların içinden çağrıldığı ve motorlar yazı
  tiplerini çalışma anında dışarıdan alacak bir API sunmadığı için (Source Sans/Serif, Inter
  WordCraft'ın `fontdb/face.rs` içinde sabit) motoru değiştirmeden çıkarılamaz. Çıkarılsalar bile
  APK'ya varlık (asset) olarak girecekleri için indirme boyutu değişmezdi.

## Ne değiştirildi

### 1. LTO aslında hiç çalışmıyordu (en büyük kazanç)

`rust/Cargo.toml` sürüm profili `lto = "fat"`, `codegen-units = 1` diyordu, ama `mo-ffi`
`crate-type = ["cdylib", "lib"]` idi. Çıktılar arasında rlib olunca rustc bağlantı zamanı
iyileştirmesini (LTO) yapmıyor: bağlayıcıya 342 ayrı `.rlib` gidiyordu, son bağlama 8 saniye
sürüyordu. `lib` türünü kullanan bir şey yok (uniffi-bindgen `.so`'yu `--library` ile okuyor,
JVM testi de `.so`'yu yüklüyor, `mo-ffi`'nin birim testleri cdylib ile de derleniyor), bu yüzden
`crate-type = ["cdylib"]` yapıldı. Bundan sonra tek bir 108 MB'lık LTO nesnesi bağlanıyor.

### 2. Yer değiştirme kayıtları Android biçiminde paketleniyor

`rust/.cargo/config.toml`: üç Android hedefi için `-C link-arg=-Wl,--pack-dyn-relocs=android`.
148 669 göreli kayıt düz RELA olarak 24'er bayt tutuyordu (28 Ekim arm64 `.so`'sunda da düz RELA
vardı). Android 6.0'dan (API 23) beri her sürüm bu sıkıştırılmış biçimi yüklüyor; uygulamanın
minSdk'sı 26. cargo-ndk kendi bayraklarını bağlayıcı sarmalayıcısıyla verdiği için
`config.toml`'daki rustflags korunuyor (cargo-ndk 4.1.2 kaynağında doğrulandı).

### Ölçülen etki (ana makine, x86_64, sembolsüz)

| Durum | `.so` | Fark | gzip -9 | xz -6 |
|---|---:|---:|---:|---:|
| Önce | 66,80 MB | | 27,57 MB | 19,05 MB |
| + LTO gerçekten açık (cdylib) | 53,52 MB | **−%19,9** | 24,68 MB | 17,66 MB |
| + `--pack-dyn-relocs=android` | 52,06 MB | **−%22,1** toplam | 24,34 MB | 17,48 MB |

LTO'dan sonra: `.text` 42,6 → 37,0 MB, `.eh_frame` 4,9 → 2,7 MB, `.gcc_except_table` 3,1 → 1,2 MB,
`.rela.dyn` 3,6 → 1,7 MB (paketlemeyle ~0). Aynı oran arm64'e uyarsa 48 MB'lık kitaplık ~37 MB'a
iner.

**CI ölçümü (15 Kasım, ilk arm64 derlemesi):** yayın APK'sındaki `lib/arm64-v8a/libmo_ffi.so` **45,2 MB** (önceki raporda 48,0 MB; arada başka özellikler de eklendi). Kazanç arm64'te x86_64 tahmininden çok küçük çıktı (~%6): Android derlemesinde bağlama zamanı iyileştirmesinin bir kısmı zaten etkiliydi ya da arm64 kodu bu değişikliklerden daha az etkileniyor. APK boyutu ~60,2 MB. Sıradaki seçenekler: `opt-level = "z"` ölçüldü ve açılmadı (aşağıda "Boyut mu hız mı?"); `--icf=all` duruyor.

**İndirme ve kurulum boyutu:**

- `jniLibs.useLegacyPackaging = false`: `.so` APK'da **sıkıştırılmadan** ve sayfa hizalı duruyor,
  telefonda doğrudan APK'dan yükleniyor (ayrıca açılıp kopyalanmıyor). Doğrudan dağıtılan APK'da
  (GitHub sürümleri, yan yükleme) indirilen dosya bu yüzden ham boyuttadır: kazancın tamamı (−%22)
  indirmeye de yansır.
- Google Play AAB'den cihaza özel bölünmüş APK üretir ve indirmeyi kendisi sıkıştırır. Play'deki
  indirme boyutu sıkıştırılmış boyuta yakındır: burada kazanç gzip ölçüsüyle ~−%12 (LTO'nun payı
  büyük; yer değiştirme kayıtları zaten iyi sıkışıyordu). Cihazdaki kurulum boyutu ise ham boyuttur
  ve −%22 küçülür.
- `.so`'yu sıkıştırılmış saklamak (`useLegacyPackaging = true`) yan yüklenen APK'yı küçültür ama
  kurulumda kitaplık açılıp ikinci kez diske yazılır (kurulum boyutu büyür, açılış yavaşlar); Play
  indirmesine etkisi yoktur. Değiştirilmedi.

### Bedeli

- Sürüm derlemesi uzuyor: bu makinede `mo-ffi` ~6 dakikadan ~11 dakikaya (4 çekirdek). CI üç ABI
  derlediği için Android işi ~15 dakika uzayabilir. Hata ayıklama derlemeleri ve `cargo test`
  etkilenmiyor.
- Davranış değişmiyor: aynı kod, aynı `opt-level`, `panic = "unwind"` korunuyor
  (`docs/upstream-notlar/motor-panikleri.md`). LTO genelde hızı da artırır.

## Denenen ama açılmayanlar

| Seçenek | Ölçülen ek kazanç | Neden açılmadı |
|---|---:|---|
| `opt-level = "z"` (CAD crate'leri 3'te kalarak) | 53,44 → 50,12 MB (−%6,2) | Ölçüldü: Word dizgisi, slayt ve PDF çizimi, tablo açma 1,3–1,8 kat yavaşlıyor. Ayrıntı: aşağıda "Boyut mu hız mı?" |
| `-Wl,--icf=all` (özdeş fonksiyon/sabit birleştirme) | −0,59 MB (−%1,1) | Salt okunur `static`'leri de birleştirebilir; adres kimliğine dayanan bir kod varsa sessizce bozulur. Kazanç küçük. `--icf=safe` hiçbir şey kazandırmıyor (rustc adres-önem tablosu üretmiyor). |
| `--pack-dyn-relocs=relr` | `android`'den biraz daha iyi | Android'de API 28+ (standart RELR API 30+) gerekiyor; minSdk 26. minSdk 28'e çıkarsa `android+relr` kullanılabilir. |
| `panic = "abort"` | `.eh_frame` + `.gcc_except_table` (LTO sonrası ~4 MB) | Motor panikleri Kotlin'e istisna olarak gelmek yerine uygulamayı düşürür. Açılmayacak. |
| `strip`, `codegen-units = 1`, `--gc-sections` | — | Zaten açık (rustc `--gc-sections`'ı kendisi veriyor). |
| Motor özelliklerini kapatmak | — | Cargo özellikleri toplanır: motorların açtığı özellikler (`hayro/embed-fonts`, `embed-cmaps`, `lopdf/rayon`, `image` biçimleri) dışarıdan kapatılamaz. Köprüler zaten yalnızca gereken crate'lere bağımlı (ör. `mo-deck` düzenleme motorunu ve ses/görüntü çözücülerini almıyor). |

## Kalan seçenekler (motor değişikliği isteyenler)

- Aynı kitaplığın birden çok sürümü: harfrust 0.12/0.13/0.14, read-fonts/skrifa ve font-types
  iki sürüm, vello_cpu iki kopya. Motor pinleri birlikte yükseltildiğinde birleşebilir (AGENTS.md
  kural 5: ayrı PR, testlerle).
- WordCraft'ın Source Sans 3/Source Serif 4 kesimleri (2,7 MiB): yukarı akışa yazı tiplerini
  çalışma anında verme API'si önerilebilir; o zaman bile boyut APK varlıklarına taşınır, indirme
  azalmaz.
- `opt-level = "z"`: ölçüldü, açılmadı (aşağıda). Yalnızca motorlar bölünüp çizimde/açmada kullanılmayan
  kod ayrı crate'lere taşınırsa yeniden denenebilir.

## Boyut mu hız mı? `opt-level` ölçümü (15 Kasım 2026)

Ana makinede (x86_64, 4 çekirdek) gerçek sürüm profiliyle (fat LTO, `codegen-units = 1`, `strip`,
`panic = "unwind"`) üç profil derlendi. Her biri için `libmo_ffi.so` boyutu ve aynı profille, depo
crate'lerine bağlı geçici bir ölçüm programının (depoya girmedi) süreleri ölçüldü; her işlem 5 kez,
en iyisi. Örnekler duman testinin büyük dosyaları: 300 sayfalık DOCX, 45 002 × 10 hücrelik XLSX,
60 slaytlık PPTX, 200 sayfalık PDF, 16 MB'lık üretilmiş DXF (kat planı ızgarası).

- **A (şimdiki):** her yerde `"s"`, CAD crate'leri + kurbo + tiny-skia `3`.
- **B:** her yerde `"z"`, CAD crate'leri + kurbo + tiny-skia `3`'te kalıyor.
- **C:** A gibi, ama yalnızca soğuk crate'ler `"z"`: PDF yazıcıları (krilla, pdf-writer, subsetter,
  write-fonts, wordcraft-pdf, deckcraft-pdf, gridcraft-pdf, craft-pdf, pdfcraft-create/optimize/redact/
  edit/annot/forms/organize/xfa), eski biçim okuyucuları (wordcraft-docbin, craft-xls, cfb,
  craft-offcrypto), WMF/EMF (wordcraft-/deckcraft-metafile), formül dizgisi (ratex-*), seyrek görüntü
  çözücüleri (hayro-jpeg2000/jbig2/ccitt, image-webp, tiff).

| | A (şimdiki) | B (`"z"` her yerde) | C (soğuk crate'ler `"z"`) |
|---|---:|---:|---:|
| `.so` (x86_64, `size -A`) | 53,45 MB | 50,12 MB (**−%6,2**) | 53,03 MB (**−%0,8**) |
| `.text` | 36,98 MB | 32,00 MB | 36,28 MB |
| gzip -9 | 24,64 MB | 22,58 MB (−%8,4) | 24,43 MB (−%0,9) |
| Word: açma + 300 sayfa dizgi | 617 ms | 1056 ms (**+%71**) | 660 ms (+%4) |
| Word: yeniden dizgi | 598 ms | 1035 ms (**+%73**) | 652 ms (+%9) |
| Word: 20 sayfa çizim (2×) | 96 ms | 126 ms (**+%31**) | 96 ms (±0) |
| Word: DOCX kaydet | 38 ms | — | 37 ms (±0) |
| Word: PDF'ye aktar (300 sayfa) | 372 ms | — | 386 ms (+%4) |
| XLSX: 45k satır açma | 464 ms | 664 ms (**+%43**) | 480 ms (+%3) |
| XLSX: 40 blok × 60×20 hücre metni+görünüşü | 17,9 ms | 23,6 ms (**+%32**) | 17,9 ms (±0) |
| XLSX: kaydet | 323 ms | — | 327 ms (+%1) |
| XLSX: 2000 satırı PDF'ye aktar | 119 ms | — | 131 ms (+%10) |
| PPTX: açma | 9,3 ms | 13,7 ms (**+%47**) | 9,7 ms (+%4) |
| PPTX: 60 slayt çizim (1600 px) | 150 ms | 276 ms (**+%83**) | 156 ms (+%4) |
| PPTX: PDF'ye aktar | 1421 ms | — | 1699 ms (**+%20**) |
| PDF: 40 sayfa çizim (2,5×) | 666 ms | 1222 ms (**+%83**) | 675 ms (+%1) |
| CAD: DXF açma | 555 ms | 552 ms (±0) | 562 ms (+%1) |
| CAD: ilk çizim (kapsam + görüntü listesi) | 958 ms | 956 ms (±0) | 997 ms (+%4) |
| CAD: 4 yakınlıkta çizim (1080×2340) | 450 ms | 435 ms (−%3) | 434–472 ms (±%5) |

"—": B ölçülürken aktarma işlemleri programda yoktu; B zaten çizimde elendiği için yeniden derlenmedi.
C'nin aktarma süreleri, aynı programla ayrıca derlenen A ile karşılaştırıldı. Ölçüm gürültüsü
±%5 civarında (A ve C sırayla dönüşümlü çalıştırıldığında çizim süreleri birbirinin içinde kaldı).

**Karar: değişiklik yok, `opt-level = "s"` kalıyor.**

- B, %6,2 küçültme için Word, slayt, PDF ve tablo işlemlerinin hepsini %31–83 yavaşlatıyor. Yavaşlama
  birkaç sıcak crate'te değil her yerde (zip/XML okuma, dizgi, yazı tipi şekillendirme, tarama): tüm
  bu crate'ler yeniden `"s"`'ye alınırsa geriye kazanılacak boyut kalmıyor; C bunun üst sınırını
  gösteriyor.
- C, açma ve çizimi etkilemiyor ama yalnızca %0,8 (~0,4 MB) küçültüyor ve PPTX'i PDF'ye aktarmayı
  %20 yavaşlatıyor (%10 eşiğinin üstünde). deckcraft-pdf ve krilla listeden çıkarılırsa kazanç
  %0,5'in altına düşer; bu kadar küçük bir kazanç için profile 30 satırlık bir liste eklenmedi.
- CAD crate'lerinin `3` olması B'de CAD sürelerinin hiç değişmemesiyle de doğrulandı.
- Ölçüm x86_64'te; arm64'te mutlak süreler farklıdır ama `"z"`'nin kıstığı satır içi açma ve
  döngü iyileştirmeleri iki mimaride de aynı, oranların benzer kalması beklenir.

## Office uyumlu yazı tipleri: APK'ya +3,0 MB (17 Kasım 2026)

Sahibinin onayıyla ("Office uyumlu yazı tipleri ekle") slaytlar ve Word için Calibri, Cambria, Arial ve
Times New Roman'ın aynı genişlikli açık karşılıkları eklendi (ayrıntı ve lisans:
[TURKCE-METIN-DENETIMI.md](TURKCE-METIN-DENETIMI.md), açık konu 1). `libmo_ffi.so` değişmedi: dosyalar
`.so`'ya gömülmüyor, `android/app/src/main/assets/fonts/` varlıkları olarak geliyor ve çalışma anında
motorlara veriliyor. `.ttf` varlıkları APK'da sıkıştırılarak (deflate) saklanıyor.

| Aile (Regular/Bold/Italic/BoldItalic) | Ham | Sıkıştırılmış (gzip -9) |
|---|---:|---:|
| Carlito (Calibri) | 2 734 244 B | 1 152 843 B |
| Liberation Sans (Arial) | 1 650 416 B | 856 180 B |
| Liberation Serif (Times New Roman) | 1 516 540 B | 835 699 B |
| Caladea (Cambria) | 333 228 B | 166 961 B |
| **Toplam (16 dosya)** | **6 234 428 B (5,95 MiB)** | **3 011 683 B** |
| Eklenmeyen: Liberation Mono (Courier New) | 1 193 484 B | 684 931 B |

- **APK farkı: +3 014 298 bayt (~3,01 MB, 2,87 MiB)**: `assets/fonts/` klasörünün (16 `.ttf` + lisans
  metinleri, NOTICE hariç) `zip -9` ile ölçülen boyutu; AGP'nin deflate düzeyinde `zip -6` ile 3 017 545 bayt. Kesin
  değeri CI'nin paket boyutu raporu gösterecek.
- Öncelik sırası Türkçe sunumlarda en sık görülenlere göre: Carlito, Liberation Sans, Liberation Serif,
  sonra küçük olduğu için Caladea. ~3 MB hedefi için Liberation Mono dışarıda kaldı (eşaralıklı metni
  Word JetBrains Mono ile, slaytlar Inter ile çiziyor).
- Bellek: dosyalar yalnızca ilgili motor ilk kez belge açınca okunuyor; her motor kendi kopyasını tutuyor
  (~6,2 MB; ikisi birden ~12,5 MB). Okuma ve kayıt süresi Ölçümler'de "Sunu/Word: yazı tiplerini yükleme".
