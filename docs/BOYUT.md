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

**CI ölçümü (15 Kasım, ilk arm64 derlemesi):** yayın APK'sındaki `lib/arm64-v8a/libmo_ffi.so` **45,2 MB** (önceki raporda 48,0 MB; arada başka özellikler de eklendi). Kazanç arm64'te x86_64 tahmininden çok küçük çıktı (~%6): Android derlemesinde bağlama zamanı iyileştirmesinin bir kısmı zaten etkiliydi ya da arm64 kodu bu değişikliklerden daha az etkileniyor. APK boyutu ~60,2 MB. Sıradaki seçenekler (ölçerek): `opt-level = "z"` (hız ölçümüyle), `--icf=all`.

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
| `opt-level = "z"` (CAD crate'leri 3'te kalarak) | 53,52 → 50,16 MB (−%6,3); hepsiyle birlikte 48,16 MB | Döngü vektörleştirmesini ve satır içi açmayı kısıyor; PDF/Word/slayt çizimi yavaşlayabilir. Depoda çizim hızı ölçen bench yok; ölçmeden hızdan feragat edilmedi. Önce cihazda PDF ve CAD çizim süresi ölçülmeli (`docs/olcumler.md`). |
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
- `opt-level = "z"`: cihazda çizim süreleri ölçülüp kabul edilebilir bulunursa, yalnızca çizimde
  sıcak olmayan crate'ler (biçim okuyucular/yazıcılar, formüller) için paket bazında açılabilir.
