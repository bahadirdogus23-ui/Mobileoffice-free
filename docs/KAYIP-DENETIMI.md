# "Kaydet" (üzerine kaydetme) kayıp denetimi — 2026-11-13

Plan maddesi: **Belge kaybı (P1)**, AGENTS.md değişmez kuralı 4 ("kullanıcı verisini asla kaybetme").

Çıkış noktası: GridCraft'ın büyük sayfaları açarken sessizce kesmesi
([upstream-notlar/gridcraft-satir-siniri.md](upstream-notlar/gridcraft-satir-siniri.md)). Aynı türden hata şudur:
motor dosyayı **kısmen** açar (bozuk bir parçayı atlar, sınırı aşan XML'i yarıda bırakır, okumadığı öğeleri hiç
modele almaz), belge yine de açılır ve uygulama "Kaydet" ile **asıl dosyanın üzerine** yazar. Okunmayan her şey o an
kalıcı olarak silinir.

Bu belge uygulamanın asıl dosyanın üzerine yazabildiği her yolu listeler. Satır numaraları sabitlenmiş üst kaynak
işlemelerine göredir (`rust/Cargo.toml`): WordCraft `c4fafc2`, DeckCraft `55e5ee5`, GridCraft `11bb673`,
PdfCraft `20eebc1`. Motorlara dokunulmadı; algılama `mo-*` köprülerinde.

## Ortak düzeltme

Her köprü açarken "bu dosya tam okundu mu?" sorusunu yanıtlar ve UniFFI ile uygulamaya verir:

| Köprü | UniFFI | Ne zaman `true` / dolu |
|---|---|---|
| `mo-grid` | `SheetDocument.openedIncomplete()`, `truncatedSheets()`, `sharedStringsKept()`, **`unreadSheets()`**, **`skippedParts()`** | Kesilen sayfa / metin listesi (önceki işleme), **hiç okunamayan sayfa, atlanan parça (stiller, açıklamalar, çizimler…)** |
| `mo-word` | **`WordDocument.openedIncomplete()`, `losses(): List<WordLoss>`** | Bozuk/sınır aşan parça, adıyla uyuşmayan biçim, motorun okumadığı dipnot, açıklama, şekil, nesne, form, resim, web içeriği… |
| `mo-deck` | **`SlidesDocument.openedIncomplete()`** | Okunamayan slayt (motor uyarısı), **sessizce atlanan asıl slayt/düzen/not/resim parçası, yarıda kesilmiş XML**, `.ppt`/`.odp`'den dönüştürme |

Paket (zip) denetimi `mo-convert/src/package.rs` içindedir: her girdi açılır (CRC denetimi), motorun ayrıştırdığı her
XML parçası motorun kendi sınırlarıyla (öğe sayısı, derinlik, öznitelik, bayt) ve kapanmamış öğe aranarak okunur.

Uygulama (`WordViewModel`, `SlidesViewModel`, `SheetViewModel`) böyle bir dosyada:
- "Kaydet"i kapatır, yalnızca "Farklı kaydet" sunar (`savesInPlace` / `canSaveInPlace` = `false`);
- "Farklı kaydet" ile asıl dosya seçilirse yazmaz, `save_over_incomplete` / `save_over_truncated` iletisini gösterir;
- açılışta neyin okunamadığını Türkçe/İngilizce olarak (`values/`, `values-tr/`) notların **başında** gösterir.

Tam açılan dosyalarda davranış değişmedi (testler: motorun örnek DOCX/PPTX'i, boş belgeler, düz RTF/HTML/MD).

## Kelime işlemci (`WordViewModel`, `IN_PLACE` = docx, docm, odt, rtf, md, txt, html, htm)

"Kaydet" dosyayı **kendi uzantısının yazıcısıyla** yazar (`WordDoc::export(fileName)` → `save_bytes`); DOCX bayt
olarak `.odt`'ye yazılmaz. `.doc`, `.dotx`, `.udf`, e-posta/e-kitap vb. her zaman "Farklı kaydet" (DOCX).

| Biçim | Kısmi açılma riski (üst kaynak) | Durum |
|---|---|---|
| DOCX/DOCM | Açılmayan zip girdisi atlanır (`crates/docx/src/package.rs:225`, `:247`); stiller, numaralandırma, ayarlar, dipnot, son not, açıklama parçası ayrıştırılamazsa atlanır (`read.rs:192-194`, yalnızca `log::warn`); ana belge dışı XML esnek okunur, kapanmamış öğeler kapatılır = yarıda kesilen parça kısmen okunur (`xml.rs:336`, `:461`); 8.000.000 öğe / 256 derinlik / 1024 öznitelik sınırı (`xml.rs:15-19`) | **Bu işlemede düzeltildi**: `Loss::Damaged` |
| DOCX adlı başka biçim | `.docx` adlı RTF/OLE/HTML içeriğine göre açılır (`crates/engine/src/io.rs:275`), kaydet DOCX'e çevirirdi | **Düzeltildi**: `Loss::OtherFormat` |
| ODT | İçe aktarıcı dipnot, açıklama, metin kutusu, çizim şekli, form, alan bildirimlerini hiç okumaz (`crates/formats/src/odt.rs:11`, `odt/body.rs:516-531`); bozuk XML akışı orada bitirir (`odt/xml.rs:51`); 8'den derin tablo, 64'ten derin liste düşer; resim bütçesi 256 MB (`odt/limits.rs:2`) | **Düzeltildi**: `Notes`, `Comments`, `Shapes`, `Objects`, `Forms`, `Other`, `Damaged`, `Pictures` |
| RTF | Dipnot, açıklama, nesne, şekil (`shpinst`, `do`), dizin girdileri atlanır (`crates/formats/src/rtf/word.rs:287-292`); PNG/JPEG dışı resim (WMF/EMF) düşer (`rtf/text.rs:151`); 512'den derin grup | **Düzeltildi** (WMF yalnızca `nonshppict` yedeğiyse sayılmaz) |
| HTML/HTM | Baytlardan açıldığından başka dosyadaki resimler yüklenemez, yalnızca `alt` metni kalır (`crates/formats/src/lib.rs:54`, `html/builder.rs:613`); `script`, `style`, `link`, `head` atılır (`html/builder.rs:511`) | **Düzeltildi**: `Loss::WebContent` (yalnızca satır içi biçimli basit sayfalar üzerine kaydedilir) |
| Markdown | Başka dosyadaki resim alt metni olur, dışa aktarım her resmi yer tutucu yazar (`crates/formats/src/markdown.rs:7-10`); ön bilgi (front matter) ve HTML korunmaz | **Düzeltildi**: `Pictures`, `Other`, `WebContent` |
| TXT | `.txt` Metin ekranında açılır (aşağıda); kelime işlemciye gelmez | Güvenli |
| Kalan | Tam okunan DOCX/ODT/RTF'de bile yazıcı her şeyi aynen geri yazmaz (tema bağları vb., [KAYIPSIZ-KAYIT-KORPUSU.md](KAYIPSIZ-KAYIT-KORPUSU.md)); ODT/RTF yazıcıları alt kümedir (`odt.rs:3-8`, `rtf.rs:3-8`), stiller sadeleşir. 50.000'den fazla üst bilgi/not/açıklama parçası (`docx/src/read.rs:54`) algılanmıyor (gerçekçi değil). Büyük harf duyarsız yinelenen zip adları (`package.rs:286`) algılanmıyor | **Kalan**: bilinen yazıcı kayıpları korpus betiğinde izleniyor; algılama yalnızca okuma tarafını kapsar |

## Hesap tablosu (`SheetViewModel`, yalnızca `.xlsx` üzerine kaydedilir)

| Risk (üst kaynak) | Durum |
|---|---|
| 1.000.000 öğe sınırı sayfayı keser, "is damaged; kept N rows" (`crates/xlsx/src/sheet_read.rs:112`); paylaşılan metinler kesilir (`read.rs:197`) | Önceki işlemede düzeltildi (`truncatedSheets`, `sharedStringsKept`). Ayrıştırılamayan sayfa da bu yoldan "0. satırda kesildi" olarak gelir. Sınırın kendisi GridCraft `c0ee120` ile upstream'de düzeldi (artık yalnızca ağaçta tutulan öğeleri sayıyor); algılama başka kesmeler için yerinde |
| Okunamayan sayfa boş sayfa olarak açılır (`read.rs:279`), iletişim/makro sayfası (`read.rs:270`) ve grafik sayfası (`passthrough/read.rs:187`) kaydedilmez | **Bu işlemede düzeltildi**: `unreadSheets()` |
| Ayrıştırılamayan isteğe bağlı parça atlanır: tema, stiller, açıklamalar, çizimler, tablolar (`read.rs:88`) | **Düzeltildi**: `skippedParts()` |
| Köprü/koşullu biçim sınırları, makro, dış bağlantı, denetim düşmeleri (`sheet_read/parts.rs:25`, `rules.rs:117`, `read.rs:393`, `passthrough/read.rs:256-376`) | **Kalan**: motor bunları uyarı olarak bildiriyor ve uygulama içe aktarma notlarında gösteriyor (kural 4'ün "gösterilir" kısmı); üzerine kaydetmeyi engellemiyor. Gerekirse aynı yoldan eklenebilir |

## Sunu (`SlidesViewModel.savesInPlace`: pptx, ppsx, potx)

Yazıcı uzantıya göre PPTX/PPSX/POTX yazar (`deckcraft_pptx::export_as`). `.ppt`/`.odp` her zaman "Farklı kaydet".

| Risk (üst kaynak) | Durum |
|---|---|
| Okunamayan slayt atlanır ve **uyarı olur** (`crates/pptx/src/read.rs:478`) ama uygulama yine de üzerine kaydediyordu | **Bu işlemede düzeltildi** |
| Açılmayan, 512 MB'tan büyük, bütçeyi aşan zip girdisi sessizce atlanır (`opc.rs:60-91`); asıl slayt eksikse atlanır ve varsayılan konur (`read.rs:525`, `:532`); 96 MB'tan büyük ya da ayrıştırılamayan XML parçası yok sayılır; yarıda kesilen XML kısmen okunur (`xml.rs:303`) | **Düzeltildi**: paket denetimi |
| `.pptx` adlı `.ppt`/`.odp` içeriği dönüştürülerek açılır, kaydet PPTX yazardı | **Düzeltildi** |
| 10.000'den fazla slayt (`read.rs:427`) | **Kalan** (gerçekçi değil) |

## PDF (`PdfViewModel`)

Asıl dosyanın üzerine kaydetme **yok**: düzenlenmiş PDF, sıkıştırma, sayfa çıkarma, birleştirme, OCR metni hep yeni
dosyaya ("-duzenlendi.pdf", "-kucuk.pdf") yazılır. Yazıcı tam yeniden yazar (`mo-pdf/src/lib.rs:736`, `write_full`);
PdfCraft sınırla açılmış belgeyi kaydetmeyi kendisi reddeder (`crates/cos/src/writer.rs:201`, `:272`).
Karartma işaretleri karartılmadan kayıt açılmaz (`PagedScreen` `startSave`).

**Durum**: güvenli. PdfCraft'ın açılışta yaptığı onarımlar (`repair_log`, `cos/src/document.rs:48`: yeniden kurulan
çapraz başvuru tablosu, eksik başlık, atlanan nesne numaraları…) artık köprüden geliyor (`PdfDoc::open_notes`,
UniFFI `PdfDocument.openNotes()`) ve `PdfViewModel.openedNotice` ile Word/Sunu notları gibi gösteriliyor
(`pdf_repaired` + motorun ilk satırları). 40 MB'tan büyük dosyalar açılışta motorla açılmadığından bu bildirim ilk
düzenleme, arama ya da kayıtta gelir. **Kalan**: kişi seçicide asıl dosyayı seçerse onarılmış belge üzerine
yazılabilir (kişi önceden uyarılmış olur; kayıt engellenmiyor).

## Metin (`TextViewModel`)

Kendi kodlaması ve satır sonlarıyla, `"wt"` ile üzerine yazar. 16 MB'tan (`MAX_BYTES`) büyük dosya kesilerek açılır
ama o zaman düzenlenemez (`TextUiState.editable`), kayıt yolu yoktur. Windows-1254'te tanımsız baytlar kayıtta
"yazılamaz" hatası verir (sessiz kayıp değil). **Durum**: güvenli. **Kalan (düşük)**: bozuk UTF-16 dizileri açılırken
U+FFFD'ye dönüşür ve kayıtta öyle yazılır; karışık satır sonları tek türe çevrilir.

## Diğerleri

| Tür | Üzerine kaydetme | Durum |
|---|---|---|
| CAD (DWG/DXF) | Yok; yalnızca PDF'e dışa aktarım | Güvenli |
| Resim, arşiv, e-fatura | Yok (salt okunur) | Güvenli |
| Yeni belge / şablon (`HomeScreen`) | Kişinin seçtiği yeni dosyaya yazar | Güvenli |
| Kurtarma kopyası | Uygulamanın önbelleğine yazar; geri alınınca kaynak dosyanın "tam okunmadı" bayrağı korunur | Güvenli |

## Testler

- `mo-convert`: `package::tests` (kesik/bozuk XML, sınırlar, CRC hatası).
- `mo-word`: `a_docx_part_the_engine_skips_is_reported`, `what_the_odt_reader_skips_is_reported`,
  `what_the_rtf_reader_skips_is_reported`, `what_web_pages_and_markdown_lose_is_reported`.
- `mo-deck`: `what_deckcraft_leaves_out_on_opening_is_flagged`.
- `mo-pdf`: `tells_what_opening_a_damaged_file_repaired`, `a_missing_header_is_reported_and_later_edits_do_not_add_notes`.
- `mo-grid`: `reads_the_engines_unread_sheet_and_part_warnings`, `a_sheet_or_part_the_engine_could_not_read_blocks_saving_over_the_file`.
- `tools/jvm-smoke`: `WordDocumentTest.tellsWhatOpeningLeftOut`, `PdfDocumentTest.tellsWhatOpeningRepaired`, `BlankDocumentTest`, `SheetDocumentTest`.
