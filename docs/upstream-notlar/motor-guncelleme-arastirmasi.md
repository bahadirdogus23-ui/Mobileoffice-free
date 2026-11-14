# Motor güncelleme araştırması — 2026-11-14

Yalnızca araştırma: `rust/Cargo.toml`, `Cargo.lock` ve kod **değişmedi** (AGENTS.md değişmez kural 5). Her motorun
`main` dalı 2026-11-14'te alındı; denemeler depo dışında, `rust/`'un geçici kopyalarında, her motor için ayrı ayrı
yapıldı (yalnızca o motorun `rev`'i değiştirildi). Yerel `rustc 1.97.0`.

İlgili notlar: [gridcraft-satir-siniri.md](gridcraft-satir-siniri.md), [cadcraft-performans.md](cadcraft-performans.md),
[../KAYIPSIZ-KAYIT-KORPUSU.md](../KAYIPSIZ-KAYIT-KORPUSU.md), [../KAYIP-DENETIMI.md](../KAYIP-DENETIMI.md).

## Özet

| Motor | Sabit | `main` | Arada (tarih aralığı) | Bilinen sorunlarımızdan upstream'de düzelen | Öneri |
|---|---|---|---|---|---|
| GridCraft | `11bb673` (10-27) | `c0ee120` (11-12) | 1126 işleme (10-12 … 11-12) | **XLSX 1.000.000 öğe sınırı** — `0b7deaa3` (PR #2527, birleştirme `e8bdd0ef`) | **Şimdi, ayrı bir değişiklikle güncelle** |
| CadCraft | `5654240` (10-27) | `fa7a977` (11-14) | 652 (10-13 … 11-14) | Yok (DWG→DXF→etiket yolu, raster ayıklama, `Header::get` aynı) | Bekle; upstream'e bildir. **Ayrıca:** `acadrust`'ı 0.6.6'ya sabitle (aşağıda) |
| WordCraft | `c4fafc2` (10-27) | `a4b898f` (11-13) | 957 (10-12 … 11-13) | Yok (4 DOCX kaybının dördü de sürüyor) | Bekle (ağ istemcisi bağımlılığı, aşağıda); upstream'e bildir |
| DeckCraft | `55e5ee5` (10-27) | `69379cf` (11-14) | 634 (10-22 … 11-14) | Yok (yazıcı ayarları, küçük resim sürüyor) | İsteğe bağlı; acil değil. Upstream'e bildir |
| PdfCraft | `20eebc1` (10-27) | `0f22627` (11-13) | 3094 (10-12 … 11-13) | (listede PdfCraft sorunu yok) | Ayrı bir değişiklikle güncellenebilir (sağlamlaştırma); deneme: 36/36 |
| craft-libs (`krilla` yaması) | `92d392c` (10-27) | `948b5c2` (11-14) | 1179 (10-05 … 11-14) | — | Yalnızca WordCraft/DeckCraft güncellemesiyle birlikte |

Hiçbir upstream'de lisans değişikliği yok: altısı da `MIT OR Apache-2.0` (çalışma alanı `license`), `LICENSE-MIT`,
`LICENSE-APACHE`, `NOTICE` duruyor. MSRV hepsinde `rust-version = "1.95"` (değişmedi; bizim çalışma alanımızla aynı);
hiçbirinde `rust-toolchain.toml` yok. Upstream'de güncel kararlı Rust ile derleniyor, 1.97 ile sorun görülmedi.

Korpus (`scripts/office_corpus.py`, KAYIPSIZ-KAYIT-KORPUSU.md): GridCraft, WordCraft ve DeckCraft'ın `main`'iyle ayrı ayrı
`corpus_round_trip` + `check` çalıştırıldı → **"nothing lost beyond the 6 known losses"**: altı bilinen kaybın altısı
da hâlâ oluyor, yeni kayıp yok.

---

## GridCraft — `11bb67386c9bb19b0717e21e98adc608162ae2e5` → `c0ee120ac2d9b59630f5abf1151dd9b471fe2926`

### Düzelen sorun: büyük sayfaların kesilmesi

- `0b7deaa3` (2026-10-24, RedBearAK) **"XLSX: the XML element cap counts only elements kept in the tree"**;
  `main`'e PR #2527 ile `e8bdd0ef` (2026-11-05) olarak girdi. Bizim önerdiğimiz düzeltmenin aynısı
  ([gridcraft-satir-siniri.md](gridcraft-satir-siniri.md) §1): `parse_streaming` yığında her öğe için açıldığı andaki
  `kept` değerini (`kept_at_start`) tutuyor; `split` öğesi (`row`, `si`) geri çağırmaya verilince `kept` o değere
  dönüyor. Ayrıca yalnızca kökün çocuk/torunu olan `split` öğeleri akışla veriliyor (`shallow`). `MAX_XML_NODES`
  hâlâ `1_000_000` ve `const`; artık yalnızca ağaçta tutulan öğeleri sayıyor. Paylaşılan metinler (`sst/si`) de aynı
  yoldan düzeliyor. Upstream testi: `a_streamed_part_may_hold_more_elements_than_the_cap`.
- `ff35c6b8` (#2197): bildirilen boyutlardan bellek şişmesine karşı sınırlar (köprü hedefleri 8 KB, koşullu biçim
  aralıkları sayfa başına 1 M, paylaşılan formül açılımı için çalışma kitabı başına 2 GiB bütçe, sayı biçimlerinin
  paylaşılması). Mobil için olumlu.
- Önerdiğimiz §2 (`ReadLimits`/yapılandırılabilir sınır) ve §3 (yapılandırılmış `truncated_sheets` raporu) **yok**;
  kesme uyarısı hâlâ metin. `mo-grid`'deki uyarı tanıma kodu yerinde kalmalı.

### Köprüyü etkileyen API değişiklikleri (`mo-grid`)

- `gridcraft_engine::cmd::data::list_items(wb, sheet, v)` → `list_items(wb, sheet, at, v)` (hücre konumu eklendi).
- `core::Cell`'e yeni alan `py` (Python hücreleri): `Cell { value, formula, style, py: None }` (`xls_styles.rs`).
- `gridcraft-pdf` upstream'de `Point`/`Rect`, Symbol yazı tipi ve yeni `image_rgba(Rect, &rgba, [w, h])` imzası aldı;
  Türkçe harf sorunu upstream'de **düzelmedi**. `vendor/gridcraft-pdf` (11bb673 tabanlı) yeni motorla değiştirilmeden
  derleniyor ve testler geçiyor, yani yamalı kopya ilk adımda olduğu gibi kalabilir; yeni tabana taşımak (Symbol yazı
  tipi için) ayrı bir iş. `PATCHES.md`'deki taban işlemesi o zaman güncellenmeli.
  **Düzeltme (güncelleme sırasında):** bu doğru çıkmadı. `[patch]` ile yamalı kopya yerine geçtiğinde
  `gridcraft-engine` `c0ee120` yeni API'yi (`Point`, `Rect`, `Font::Symbol`, `can_encode`, yeni `image_rgba`)
  çağırdığı için derlenmiyor (denemede yama büyük olasılıkla devrede değildi). Güncellemede `vendor/gridcraft-pdf`
  `c0ee120` tabanına taşındı: upstream dosyaları olduğu gibi alındı, üç Türkçe yaması yeniden uygulandı
  (`PATCHES.md`); `mo-pdf` testinde `image_rgba` çağrısı yeni imzaya uyarlandı.

### Deneme (yalnızca GridCraft `rev`'i değişti)

- `cargo test -p mo-grid`: mevcut 43 testten 41'i (ve eklenen deneme testi) geçti; yukarıdaki iki küçük uyarlamayla derleniyor.
  - `a_sheet_cut_by_the_engine_is_reported_as_truncated` **beklendiği gibi** düştü: 60.000 × 10 sayfa artık kesilmiyor.
    Not dosyasındaki plana göre bu test "tüm satırlar açılır, uyarı yok" olarak değiştirilmeli.
  - Ek deneme testi: CSV'den üretilen **120.000 × 10** XLSX tamamen açılıyor, `truncated()` boş, `strings_kept()` `None`,
    `opened_incomplete()` `false`, 120.000 satır.
  - `turkish::tests::documents_every_function_in_turkish`: upstream'e yeni gelen **`ECMA.CEILING`** işlevinin Türkçe
    açıklaması eksik (`mo-grid/src/turkish.rs`'e bir satır).
- `cargo clippy -p mo-grid --all-targets -D warnings`: temiz. Korpus: XLSX'te yeni kayıp yok.
- Yeni üçüncü taraf paketler (hepsi izin verici): `ratex-*` (MIT; `gridcraft-math`), `zip 9.5.1` (MIT), `fancy-regex`
  (MIT), `md-5`, `sha1`, `typed-path`, `rand 0.8`… `nix` (MIT) Android hedefinde ağaçta yok.

**Öneri: şimdi, kendi değişikliğinde güncelle** — kural 4 açısından en önemli upstream düzeltmesi bu. Aynı değişiklikte:
iki API uyarlaması, `ECMA.CEILING` çevirisi, kesme testinin "bütün satırlar açılır" (120.000 × 10) olarak yeniden
yazılması, `gridcraft-satir-siniri.md`'ye "upstream'de düzeldi (`0b7deaa3`)" notu, sonra Android derlemesi ve gerçek
telefonda büyük bir dosyanın bellek ölçümü (artık sayfanın tamamı belleğe alınıyor).

## CadCraft — `56542407292b911ad30523cfcfa987923fe2f8e2` → `fa7a97728b08fb206a3f95ab6d7b189ef6239660`

### Bilinen sorunların durumu ([cadcraft-performans.md](cadcraft-performans.md))

| Sorun | `main`'de |
|---|---|
| 1. DWG önce bütün DXF metnine çevriliyor | **Sürüyor.** `crates/io/src/lib.rs` `read_source`: `cadcraft_dwg::decode` → `dxf_read::read(&dxf)`; `decode`, `dwg_to_dxf_noted`'u çağırıyor (acadrust → `CappedWriter`). craft-libs'e taşınan yeni ortak köprü (`crates/dwg`, `888bffcb`) de bayt→bayt. |
| 2. Bütün etiketler tek `Vec<Tag>` | **Sürüyor.** `craft_dxf::parse` (DXF ayrıştırıcı craft-libs'e taşındı, `c736e48`) hâlâ `Vec<Tag>` döndürüyor. Kısmi iyileşme `e24c455`: sayılmış ASCII dosyanın etiket dizisi bir kez, başarısız olabilen ayırmayla ayrılıyor (büyürken 1,5 kat geçici tepe yok; sistem reddederse uygulama ölmek yerine hata). Android'in aşırı taahhüdü yüzünden reddin pratikte çalışacağı şüpheli. Ölçülmedi. |
| 3. `Header::get` her çağrıda `String` | **Sürüyor** (`crates/doc/src/header.rs:58-60`). |
| 4. Raster çizici ayıklama yapmıyor | **Sürüyor.** `raster.rs` yalnızca yeni resim (`Kind::Image`) öğelerini görünüm dışında atlıyor. Bizim `mo-cad` ayıklamamız gerekli kalıyor. |
| 5. Ekran ayrıntı düzeyi varsayılan kapalı | Değişmedi (bizde çözüldü). |

Yeni: `f6d84e2` DWG okuyucu/yazıcıyı **alt süreçte** çalıştırabiliyor (`dwg_worker::use_own_worker`), çökme/ayırma
hatası yalnızca o süreci öldürüyor. İsteğe bağlı; çağırılmazsa davranış eskisi gibi. Android'de `current_exe()` ile
kendini başlatma uygulamaya uymuyor (ayrı `android:process` hizmeti gerekir), bu yüzden şimdilik bize yararı yok.

### Önemli yan bulgu: `acadrust` sürümü

Bizim `Cargo.lock`'umuz `acadrust 0.6.8` çözüyor (sabit `rev`'de bile; CadCraft `"0.6.5"` istiyor). CadCraft'ın kendi
kilidi **0.6.6**, `main`'deki `crates/dwg/Cargo.toml` da şunu söylüyor: *"0.6.7 and 0.6.8 … read some damaged DWGs as
empty with no note (tests.rs, #800)"*. Bu, kısmen okunan bir DWG'nin hiçbir not olmadan boş/eksik görünmesi demek
(kural 4'ün "gösterilir" kısmı; CAD'de üzerine kaydetme olmadığından veri silinmez). **Öneri:** ayrı, küçük bir
değişiklikte `cargo update -p acadrust --precise 0.6.6` (motor `rev`'i değişmeden) ve `damaged_dwg_opens_partially…`
benzeri bir testi `mo-cad`'e eklemek.

### Deneme (yalnızca CadCraft `rev`'i değişti)

- `mo-cad` **hiç değişmeden** derleniyor; `cargo test -p mo-cad` 10/10, `clippy -D warnings` temiz.
- Tek uyarı: `rust/Cargo.toml`'daki `[profile.release.package.cadcraft-dxf]` artık eşleşmiyor (paket craft-libs'te
  `craft-dxf` oldu); güncellemede `craft-dxf` (ve isteğe bağlı `cadcraft-dwg`) için değiştirilmeli.
- Yeni üçüncü taraf paketler: ICU4X (`icu_segmenter`, `zerovec`… **Unicode-3.0** lisansı; izin verici, `NOTICE`'e
  atıf gerekir).

**Öneri: bekle.** Açılış belleğini (DWG'de dosyanın ≈ 200 katı) düşürecek değişiklik upstream'de yok; güncellemek
riskli (652 işleme) ama kazancı küçük. Upstream'e bildir: (a) acadrust belgesinden `Drawing`'e doğrudan dönüşüm ya da
akışla DXF ayrıştırma, (b) DXF metnini ayrıştırırken etiketleri bölüm bölüm işleyip bırakma, (c) `Header::get` için
ayırmasız arama, (d) raster çizicide sınır kutusu önbelleği ve üçgen ayıklama. Her biri için ölçüm ve dosya:satır
`cadcraft-performans.md`'de hazır.

## WordCraft — `c4fafc215490a0186ee750454b1ce73a9823daed` → `a4b898f820eb28626865292fd037a155e4a07aef`

### Bilinen kayıpların durumu

| Kimlik | `main`'de |
|---|---|
| `docx-tema-baglari` | **Sürüyor.** Tema bağları yalnızca okunuyor (`read/props.rs`), yazıcı sabit değer yazıyor. |
| `docx-tema-parcasi` | **Sürüyor.** `write/settings.rs` `theme_xml` hâlâ küçük tema üretiyor. |
| `docx-tablo-stili-paragraf` | **Sürüyor.** `write/styles.rs`: `tblStylePr/pPr` yalnızca hizalama varsa ve yalnızca hizalamayla yazılıyor. |
| `docx-resim-oran-kilidi` | **Sürüyor.** `graphicFrameLocks` hiç yazılmıyor (yalnızca `pic:cNvPicPr/a:picLocks`). |
| Sessizce atlanan parçalar | **Sürüyor.** `docx/src/package.rs` okunamayan girdileri hâlâ yalnızca `log::warn` ile atlıyor; `mo-convert` paket denetimimiz gerekli. |

Korpus `main` ile çalıştırıldı: dört kaybın dördü de aynı, yeni kayıp yok. İlgili sayılabilecek tek işleme
`5dfb3acf` (`picture.change` yeni resmin en-boy oranını alıyor; oran kilidini düzeltmiyor).

### Köprüyü etkileyen API değişiklikleri (`mo-word`)

- `Document::styles` artık `Arc<StyleSheet>`: yazmak için `Arc::make_mut(&mut doc.styles)` (`lib.rs` iki yer, `udf.rs`
  bir yer).
- `krilla` yaması: WordCraft `main` kendi çalışma alanında `craft-libs@6e03f65` istiyor (bizde `92d392c`).

### Deneme (WordCraft `rev`'i + `krilla` → `6e03f65`)

- Üç satırlık uyarlamayla `cargo test -p mo-word` 23/23 (korpus dahil), `clippy -D warnings` temiz.
- **Engel sayılabilecek bulgu — ağ kodu:** `e5d46bb2` (dil paketleri) ile `wordcraft-engine`, craft-libs'in
  `craft-font-store`'una bağımlı; o da wasm dışındaki her hedefte koşulsuz `ureq` + `rustls` + `ring` +
  `webpki-roots` getiriyor (`cargo tree --target aarch64-linux-android` ile doğrulandı). Ağ yalnızca masaüstü
  uygulamasının çağırdığı `packs::use_font_store` ile devreye giriyor, biz çağırmıyoruz; uygulamada INTERNET izni de
  yok. Yine de: (1) değişmez kural 1'in ruhuna aykırı olarak `.so`'ya bir HTTPS istemcisi girer, (2) `ring` NDK ile C/asm
  derlemesi ister (Android derlemesi denenmedi), (3) yeni lisanslar: `ring` (Apache-2.0 AND ISC), `webpki-roots`
  (CDLA-Permissive-2.0), `subtle` (BSD-3-Clause), `libloading` (ISC) — uyumlu ama `NOTICE` işi.
  Linux'ta ayrıca `fontique` → `yeslogic-fontconfig-sys` geliyor (Android'de yok).

**Öneri: bekle; upstream'e bildir.** (a) `craft-font-store`'un ağ kısmını (`ureq`) bir Cargo özelliğinin arkasına alma
(ya da `wordcraft-engine`'de dil paketlerini isteğe bağlı özellik yapma), (b) DOCX yazıcıda tema bağlarını ve tema
parçasını koruma, `tblStylePr/pPr` aralıklarını yazma, `wp:cNvGraphicFramePr/a:graphicFrameLocks` yazma, (c) atlanan
paket girdilerini yapılandırılmış uyarı olarak döndürme. Ağ bağımlılığı özelliğe alınmadan güncellenirse en azından
APK'daki `.so` boyutu ve Android derlemesi ayrı denetlenmeli.

## DeckCraft — `55e5ee56b4b2c64973b78fda4182a78f9766fa62` → `69379cf689f908d23cab9313d59fb9e1fd4b7a4f`

- `pptx-yazici-ayarlari`, `pptx-kucuk-resim`: **sürüyor** (`crates/pptx` içinde `printerSettings`/`thumbnail` hiç
  geçmiyor). Sessiz parça atlama (`opc.rs`, yalnızca `log::warn`) da sürüyor.
- Çok sayıda "PPTX'te koru" düzeltmesi var (ör. `aefd54d` 3B sahne, `6a3cbd1` büyük ayar kılavuzları, `e0b0817` boş
  asıl stiller, `6901716` SmartArt metni, `b61c5f1` yansıma); korpusumuz bunları kapsamıyor, ama kayıpsız kayıt için olumlu.
- API: `mo-deck` **değişmeden** derleniyor. `krilla` → `craft-libs@63196e6` (WordCraft'ın `6e03f65`'inin torunu; ikisi
  birlikte güncellenirse `63196e6` ikisine de uyar).
- Deneme: `cargo test -p mo-deck` 22/22 (korpus dahil; iki bilinen kayıp aynı, yeni kayıp yok), `clippy` temiz, yeni
  üçüncü taraf paket yok.

**Öneri: isteğe bağlı, acil değil.** Düzelen bilinen sorun yok; korunma iyileştirmeleri için ayrı bir değişiklikte
güncellenebilir. Upstream'e bildir: bilinmeyen parçaları (yazıcı ayarları; küçük resim için en azından yeniden üretme ya
da koruma) olduğu gibi geçirme.

## PdfCraft — `20eebc1c33ff7e6f42ac1c56bf95192f628a9d97` → `0f22627bff6cde9930817e891290f653d882bbda`

- Listemizde PdfCraft sorunu yok. Arada 3094 işleme; ilgili olanlar: bozuk girdiye karşı sağlamlaştırma (`b06bedef`
  `hayro-syntax` paniği, `344ba9f8` 128 bitten uzun eski şifre anahtarı paniği), çizim hızı (`558ffd7f` çözülemeyen
  resim bir kez çözülüyor, `9b231f24`/`f84290b2` TIFF/PNG öngörücüleri), karartmanın hâlâ çizilen içeriği bozmaması
  (`2d672d95`, `7b1301b3`, `1184738b`, `0cb06e45`).
- API: `annot::Style`'a yeni alanlar geldi → `mo-pdf/src/lib.rs`'te `..annot::Style::default()` eklenmeli.
- `[patch.crates-io]`'ya PdfCraft'ın yamalı `hayro-jbig2` ve `vello_cpu`/`vello_common` kopyaları da eklenmeli;
  `hayro-jbig2` için `cargo update -p hayro-jbig2` gerekiyor, yoksa crates.io sürümü seçilip `pdfcraft-filters`
  derlenmiyor (`charge_reading`, `decode_within` yok).
- Deneme (PdfCraft `rev`'i + `hayro-*`/`lopdf`/`vello_*` yamaları; `vendor/gridcraft-pdf` değişmedi): bir satırlık
  uyarlamayla `cargo test -p mo-pdf` 36/36, `clippy -D warnings` temiz. Yeni üçüncü taraf paketler: ICU4X (Unicode-3.0),
  `url`/`idna` (yalnızca URL ayrıştırma, ağ istemcisi değil), `thread-priority` (MIT), `roxmltree 0.21`.

**Öneri: ayrı bir değişiklikle güncellenebilir** (öncelik GridCraft'tan sonra). Kazanç bozuk/kötü niyetli PDF'lere
karşı sağlamlık ve çizim hızı; köprü değişikliği tek satır. 3094 işleme büyük bir sıçrama olduğundan güncelleme
değişikliğinde imza, form, karartma ve onarım (`open_notes`) yollarının jvm-smoke ve telefonda denenmesi gerekir.

**Bu değişiklikte güncellendi** (`0f22627bff6cde9930817e891290f653d882bbda`). Araştırmadaki bulgular gerçek `[patch]`
bölümüyle doğrulandı: köprüde tek satır (`..annot::Style::default()`; yeni alanların varsayılanları eski davranış:
ayrı kenarlık rengi/dolgu saydamlığı yok, dikdörtgen kenarlık). `[patch.crates-io]`'ya `hayro-jbig2`, `vello_common`,
`vello_cpu` eklendi; PdfCraft'ın kopyaları 0.0.8 sürümünde, WordCraft/DeckCraft `vello_*` 0.2 kullandığı için onları
etkilemiyor. `Cargo.lock`: yalnızca PdfCraft paketleri, `hayro-jbig2` 0.3.1 → yamalı 0.3.0 ve yeni bağımlılıklar
(ICU4X `icu_*`/`zerovec`/`yoke`… Unicode-3.0; `url`/`idna`/`form_urlencoded` MIT/Apache; `thread-priority` MIT, yalnızca
sıkıştırma iş parçacıklarının önceliğini düşürüyor, sonucu yok sayılıyor; `roxmltree 0.21`; `windows*` yalnızca
Windows'ta). Android hedefinde C derleyicisi gerekmiyor (`cargo check --target aarch64-linux-android -p mo-ffi`, ağaçta
`cc` yok). `repair_log` duruyor (`cos/src/document/objects.rs:90`). `cargo test --workspace` (mo-pdf 36/36),
`scripts/check-pdf-redaction.sh` (Poppler) ve jvm-smoke (19/19) geçti; UniFFI arayüzü değişmedi.

Ölçüm (release, x86_64, 5 denemenin en iyisi, ilk sayfa ×2 PdfCraft çizimi / ilk sayfa metni; önce → sonra):
300 sayfalık LibreOffice PDF'i çizim 15,0 → 13,4 ms, metin 54,5 → 23,0 ms; 200 sayfalık duman örneği 19,8 → 15,7 ms,
metin 6,9 → 5,0 ms; 60 sayfalık sunu PDF'i metin 22,4 → 8,8 ms; taranmış görünümlü bir form 34 → 28-30 ms, metin
16,9 → 11,4 ms; küçük formlar değişmedi (3-6 ms). Açılış (yalnızca `Document::open`) büyük dosyalarda biraz yavaşladı
ama ms'nin altında kalıyor (300 sayfa 0,3 → 1,0 ms). Bozuk (rastgele bayt değiştirilmiş) örneklerde açılış 80-100 →
36 ms; iki sürüm de aynı dosyaları açıp aynı dosyaları reddediyor, çökme yok. Çizilen piksel sayıları sağlam
dosyalarda birebir aynı.

## craft-libs — `92d392cff132eaf53e13afff77f38b5c11b73aa0` → `948b5c23c12dc8c87c8b1828892b1462113dd5a7`

Bizde yalnızca `krilla` yaması olarak sabit. `krilla` için arada üç işleme: `fb08be5f`/`b373482f` (CMYK, gri ve spot
renkler tam mürekkebini koruyor), `9b7b748f` (TrueType alt kümeleri OS/2 tablosunu, yani gömme izinlerini koruyor).
Kendi başına güncellemeye değmez; WordCraft/DeckCraft güncellemesinde onların istediği sürüme (`63196e6` ikisini de
karşılar) çekilmeli. Not: craft-libs deposunda `HEAD` adlı bir **dal** var (`refs/heads/HEAD` → `a994987`);
`git ls-remote … HEAD` iki satır döndürüyor, varsayılan dal `main`.

## Upstream'e bildirilecekler (öncelik sırasıyla)

1. WordCraft/craft-libs: `craft-font-store`'un ağ istemcisini isteğe bağlı özellik yapma (gömülü/mobil istemciler için).
2. CadCraft: DWG açılış belleği (doğrudan dönüşüm / akışla ayrıştırma), `Header::get`, raster ayıklama.
3. WordCraft DOCX yazıcı: tema bağları ve tema parçası, tablo stili paragraf aralıkları, `graphicFrameLocks`.
4. DeckCraft PPTX yazıcı: `printerSettings`, `docProps/thumbnail.jpeg`.
5. GridCraft: (düzeldi) — isteğe bağlı `ReadLimits` ve yapılandırılmış kesme raporu; `gridcraft-pdf`'te Türkçe harfler
   (`vendor/gridcraft-pdf/PATCHES.md`).
6. WordCraft/DeckCraft: atlanan paket girdilerini yapılandırılmış uyarı olarak döndürme.

## Yöntem

- `git ls-remote` + `git clone --bare --filter=blob:none` (proxy engellemedi); karşılaştırma için Cargo'nun indirdiği
  iki ağaç (`~/.cargo/git/checkouts/<motor>-*/<kısa rev>`).
- Her deneme: `rust/`'un kopyası, yalnızca ilgili `rev` (gerekirse `krilla`/`hayro-*` yamaları), `CARGO_INCREMENTAL=0`,
  ayrı hedef klasörü, ardından silindi. Korpus: `office_corpus.py generate` → `MO_CORPUS_DIR=… cargo test -p mo-<x>
  tests::corpus_round_trip` → `office_corpus.py check`.
- Android (NDK) derlemesi ve jvm-smoke bu araştırmada çalıştırılmadı; güncelleme değişikliklerinde CI'da doğrulanmalı.
