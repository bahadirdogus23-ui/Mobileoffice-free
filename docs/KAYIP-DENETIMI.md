# "Kaydet" (üzerine kaydetme) kayıp denetimi — 2026-11-13

Plan maddesi: **Belge kaybı (P1)**, AGENTS.md değişmez kuralı 4 ("kullanıcı verisini asla kaybetme").

Çıkış noktası: GridCraft'ın büyük sayfaları açarken sessizce kesmesi
([upstream-notlar/gridcraft-satir-siniri.md](upstream-notlar/gridcraft-satir-siniri.md)). Aynı türden hata şudur:
motor dosyayı **kısmen** açar (bozuk bir parçayı atlar, sınırı aşan XML'i yarıda bırakır, okumadığı öğeleri hiç
modele almaz), belge yine de açılır ve uygulama "Kaydet" ile **asıl dosyanın üzerine** yazar. Okunmayan her şey o an
kalıcı olarak silinir.

Bu belge uygulamanın asıl dosyanın üzerine yazabildiği her yolu listeler. Satır numaraları sabitlenmiş üst kaynak
işlemelerine göredir (`rust/Cargo.toml`): WordCraft `c4fafc2`, DeckCraft `55e5ee5`, GridCraft `11bb673`,
PdfCraft `20eebc1` (PDF bölümündeki PdfCraft satırları güncellemeden sonra `0f22627`e göre). Motorlara dokunulmadı;
algılama `mo-*` köprülerinde.

## Ortak düzeltme

Her köprü açarken "bu dosya tam okundu mu?" sorusunu yanıtlar ve UniFFI ile uygulamaya verir:

| Köprü | UniFFI | Ne zaman `true` / dolu |
|---|---|---|
| `mo-grid` | `SheetDocument.openedIncomplete()`, `truncatedSheets()`, `sharedStringsKept()`, **`unreadSheets()`**, **`skippedParts()`**, **`losses(): List<SheetLoss>`** | Kesilen sayfa / metin listesi (önceki işleme), **hiç okunamayan sayfa, atlanan parça (stiller, açıklamalar, çizimler…), motorun geri yazmadığı her şey (sınırı aşan köprüler, dilimleyiciler, `.xlsx` içinde makro, senaryolar…)** |
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
| `chart sheet "X" was dropped: …` (`passthrough/read.rs:232`): grafik sayfası geçiş bütçesini aşınca silinir | **Bu işlemede düzeltildi**: `unreadSheets()` kapsamına alındı |
| Motorun "kaybettim" uyarıları (GridCraft `c0ee120`): köprü sınırları (`sheet_read/parts.rs:16-41`), koşullu biçim türü/sınırı ve Excel 2010 uzantısı (`rules.rs:114-120`, `sheet_read.rs:865`), okunamayan/düz hücreye dönen özet tablo (`read.rs:356`, `pivot/read.rs`), okunamayan VBA (`read.rs:463`), Python parçası (`python_part.rs`), veri tablosu ve sınırı aşan paylaşılan formül (`sheet_read.rs:698-736`), tarih filtresi (`parts.rs:87`), okunamayan tarih/sayı hücresi (`sheet_read.rs:578-587`), hücre içi resim (`richdata.rs`, `sheet_read.rs:600-606`), bağlı resim, desteklenmeyen grafik, gömülü nesne, gruplanmış şekil (`drawing.rs:161-207`), geçişin düşürdüğü özel özellikler, özel XML, veri bağlantısı, dış bağlantı, sorgu tablosu, yazıcı ayarı, denetim ve OLE nesnesi, üst/alt bilgi resmi (`passthrough/read.rs:190-395`), sesli okunuş çakışması (`read.rs:79`) | **Bu işlemede düzeltildi**: `losses()` (`mo-grid/src/losses.rs` `from_warnings`). Uyarı metniyle eşleşir, her ifade `reads_the_engines_loss_warnings` ile sabitlendi; "dropped/skipped/not kept/not supported…" diyen bilinmeyen yeni bir uyarı da kayıp sayılır (`Other`). Kayıp sayılmayanlar: olmayan parçaya başvuru (`missing part`, `missing picture`, parçası olmayan grafik/sayfa/makro), Excel'in de tutamayacağı içerik (ızgara dışı hücre, bozuk sesli okunuş, ana hücresi olmayan paylaşılan formül, olmayan paylaşılan metne başvuru) ve korunduğunu söyleyenler ("kept for saving", "links … were kept", `.xlsm`'de "macros … were kept") |
| Motorun **uyarısız** atladıkları: dilimleyici ve zaman çizelgesi (`passthrough.rs:146-150` yeniden yazılan türler arasında, okuyucu modellemiyor), Power Pivot veri modeli, sayfa özel özellikleri, paylaşılan çalışma kitabı düzeltmeleri, adlandırılmış sayfa görünümleri, gizlilik etiketleri (bilinmeyen ilişki türleri), grafik üstü şekiller (`chartUserShapes`), mürekkep (`contentPart`, `drawing.rs:204`), sayfada senaryo, birleştirme, özel görünüm, izlenen hücre, akıllı etiket, web yayımlama, DrawingML üst/alt bilgi (`sheet_read.rs:216-341` `_ => {}`), okunmayan sayfa uzantıları (`sheet_read.rs:823`: yalnızca koşullu biçim, doğrulama, mini grafik okunur; `x14:protectedRanges`, web uzantıları…), filtrede tarih gruplaması (`parts.rs:67`), çalışma kitabında `fileSharing` (değiştirme parolası, salt okunur önerisi), özel görünümler, web yayımlama nesneleri ve slicer/zaman çizelgesi/veri modeli uzantıları | **Bu işlemede düzeltildi**: `losses()` (`from_package`): ilişki türleri (bilinen-korunan listesi dışındaki her tür kayıp), çalışma kitabı ve sayfa öğeleri/uzantıları paketten okunur. Sayfa XML'inin yalnızca `</sheetData>` sonrası akışla taranır (hücreler bellekte tutulmaz) |
| `.xlsx` içinde VBA projesi: okuyucu tutar ("kept but not executed") ama yazıcı makrosuz pakete yazmaz (`write.rs:422`) | **Bu işlemede düzeltildi**: `SheetLoss.MACROS` (yalnızca `.xlsx`/`.xltx`; `.xlsm`'de korunduğu sınandı) |
| Korunduğu sınananlar (aç → kaydet → yeniden aç): dış çalışma kitabı bağlantısı, özel XML, veri bağlantısı, özel belge özellikleri, köprüler, veri doğrulama, koşullu biçim, mini grafikler, birleştirilmiş hücreler, sayfa ve çalışma kitabı koruması, tanımlı adlar; korpus dosyası (açıklama, doğrulama, ad, filtre…) | Engellenmez (`what_the_engine_writes_back_does_not_block_saving_over_the_file`, `corpus_round_trip` artık `openedIncomplete` olmadığını da denetler) |
| Uygulama | `SheetViewModel`: her kayıp türü için Türkçe/İngilizce not (`sheet_loss_*`) ve `.xlsx`'te `save_in_place_off`; `canSaveInPlace` = `false`. "Farklı kaydet" ile asıl dosya seçilirse satır kesilmişse `save_over_truncated`, değilse `save_over_incomplete` |
| Kalan | Yazıcının biçim aslına uygunluğu (grafik biçimlendirmesi, grafik stil/tema parçaları, ad öznitelikleri `function`/`vbProcedure`, birden fazla pencere, açıklama kutusu biçimi, sesli okunuş ayarları, son sıralama durumu, yoksayılan hata işaretleri, küçük resim) "kayıp" sayılmıyor ([KAYIPSIZ-KAYIT-KORPUSU.md](KAYIPSIZ-KAYIT-KORPUSU.md)). Gerçekçi olmayan sınırlar algılanmıyor: 65.536 dinamik dizi aralığı, 16.384 özet tablo alanı (`sheet_read.rs:156-164`, `pivot.rs:46`). Grafik sayfasına bağlı tanımlı adlar (`read.rs:382`) okunmuyor. Bilinmeyen zararsız bir uzantı ya da ilişki türü yanlışlıkla "Farklı kaydet"e zorlayabilir (veri kaybı değil) |

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
dosyaya ("-duzenlendi.pdf", "-kucuk.pdf") yazılır. Yazıcı tam yeniden yazar (`mo-pdf/src/lib.rs:104`, `written` →
`write_full`);
PdfCraft sınırla açılmış belgeyi kaydetmeyi kendisi reddeder (PdfCraft `0f22627`: `crates/cos/src/writer.rs:260`, `:403`).
Karartma işaretleri karartılmadan kayıt açılmaz (`PagedScreen` `startSave`).

**Durum**: güvenli. PdfCraft'ın açılışta yaptığı onarımlar (`repair_log`, `cos/src/document/objects.rs:90`: yeniden kurulan
çapraz başvuru tablosu, eksik başlık, atlanan nesne numaraları…) artık köprüden geliyor (`PdfDoc::open_notes`,
UniFFI `PdfDocument.openNotes()`) ve `PdfViewModel.openedNotice` ile Word/Sunu notları gibi gösteriliyor
(`pdf_repaired` + motorun ilk satırları). 40 MB'tan büyük dosyalar açılışta motorla açılmadığından bu bildirim ilk
düzenleme, arama ya da kayıtta gelir. **Kalan**: kişi seçicide asıl dosyayı seçerse onarılmış belge üzerine
yazılabilir (kişi önceden uyarılmış olur; kayıt engellenmiyor).

| Risk (üst kaynak) | Durum |
|---|---|
| Sayfa ağacı bulunamayan ya da boş, ağır bozuk dosya PdfCraft'ta hatasız ve **0 sayfayla** açılıyordu (`organize::page_count` → `NoPageTree` ya da 0); uygulama boş görüntüleyici gösteriyor, kayıt ~430 baytlık boş bir PDF yazıyordu. Asıl dosya yerinde kalsa da kişi yeni dosyayı belgesi sanıp asıl dosyayı silebilirdi | **Düzeltildi**: `PdfDoc::open` böyle dosyayı `PdfError::NoPages` ile reddeder (UniFFI `MoException.NoPages`); uygulama "Dosya açılamadı: … hiçbir sayfası okunamadı, dosya çok bozuk" (`pdf_no_pages`) sorununu "Bildir" ile gösterir. Android'in görüntüleyicisi 0 sayfa verirse de aynı hata. Birleştirme (her iki "PDF ekle" yolu) böyle dosyayı adıyla reddeder (`merge_no_pages`); kaydetme, sayfa çıkarma, sıkıştırma ve görüntü kopyası sayfasız belge yazmaz (`written`) |

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
- `mo-pdf`: `tells_what_opening_a_damaged_file_repaired`, `a_missing_header_is_reported_and_later_edits_do_not_add_notes`,
  `a_file_whose_pages_cannot_be_found_does_not_open`, `combining_refuses_a_file_without_pages`,
  `nothing_is_written_from_a_document_without_pages`.
- `mo-grid`: `reads_the_engines_unread_sheet_and_part_warnings`, `a_sheet_or_part_the_engine_could_not_read_blocks_saving_over_the_file`,
  `reads_the_engines_loss_warnings`, `what_the_engine_writes_back_does_not_block_saving_over_the_file`,
  `content_the_engine_skips_silently_blocks_saving_over_the_file`, `macros_block_saving_over_an_xlsx_but_not_an_xlsm`,
  `losses_the_engine_warns_about_block_saving_over_the_file`, `losses::tests`.
- `tools/jvm-smoke`: `WordDocumentTest.tellsWhatOpeningLeftOut`, `PdfDocumentTest.tellsWhatOpeningRepaired`, `PdfDocumentTest.refusesAFileWithoutReadablePages`, `BlankDocumentTest`, `SheetDocumentTest` (`tellsWhatSavingBackWouldNotKeep`).
