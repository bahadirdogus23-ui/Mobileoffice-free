# GridCraft: büyük XLSX sayfalarının kesilmesi

Sabitlenmiş sürüm: `gridcraft` @ `11bb67386c9bb19b0717e21e98adc608162ae2e5` (`rust/Cargo.toml`).

## Belirti

50.000 satır × 10 sütunluk sade bir sayfa açıldığında motor 49.999. satırda durur ve
`sheet "X" is damaged (the file contains invalid XML: the part holds more than 1000000 elements); kept 49999 rows`
uyarısını verir. Dosya bozuk değildir; geri kalan satırlar sessizce yüklenmez. Kişi bu hâliyle asıl
dosyanın üzerine kaydederse kesilen satırlar kalıcı olarak silinir (AGENTS.md, değişmez kural 4).

## Sınır nerede

- `crates/xlsx/src/xml.rs:20` — `pub const MAX_XML_NODES: usize = 1_000_000;`
  Amacı: tek bir parçanın ağaç olarak tutulan XML'inin bellekte gigabaytlara şişmesini önlemek
  (çok iyi sıkışan, sonsuz boş öğeli "zip bombası" parçalar; boş bir `El` yüz bayttan fazla tutar).
- `crates/xlsx/src/xml.rs:234-252` — `parse_streaming` içindeki `kept` sayacı. `split` listesindeki
  öğeler (sayfada `<row>`, paylaşılan metinlerde `<si>`) geri çağırmaya verilir ve ağaçta tutulmaz;
  ama **onların alt öğeleri** (`<c>`, `<v>`, `<f>`, `<is>`, `<t>`) bitip ebeveynlerine eklenirken
  `kept` artırılır ve satır geri çağırmaya verilip bırakıldığında **hiç azaltılmaz**. Böylece
  sınır, ağaçta o an tutulan öğeleri değil, dosyanın baştan beri gördüğü tüm hücre öğelerini sayar.
- `crates/xlsx/src/sheet_read.rs:105-114` — `read_sheet` hatayı yakalar, okunan satırları tutar
  ve yalnızca bir uyarı yazar (`kept {} rows`).
- `crates/xlsx/src/read.rs:185-197` — paylaşılan metinler için aynı durum:
  `shared strings are damaged (...); kept N strings`. Kesimden sonraki metne başvuran hücreler
  boş açılır (`sheet_read.rs:466-472`).

Pratik sınır: bir satırdaki her değerli hücre `<c>` + `<v>` = 2 öğe (formül varsa +1, satır içi
metin `<is><t>` ile 3). Yani yaklaşık **1.000.000 / (2 × dolu sütun)** satır: 10 sütunda ~50.000,
20 sütunda ~25.000. Muhasebe/ERP dışa aktarımları ve e-Fatura listeleri bunu kolayca aşar.

Yapılandırılamaz: `MAX_XML_NODES` bir `const`; `io::open_bytes*` ve `read_xlsx` hiçbir seçenek,
sınır yapısı ya da Cargo özelliği almaz. (XLSB `xlsb.rs:28` ve ODS `ods.rs:26` için 1.000.000
hücre sınırı da sabit, ama onlar dosyayı hiç açmaz; sessiz kayıp yoktur.)

## Uygulamada şimdilik ne yapıyoruz

Motoru yamamadan (kural 5) `mo-grid` bu iki uyarıyı tanır:

- `GridDoc::truncated()` — kesilen sayfalar (ad, okunan satır, son satır),
  `GridDoc::strings_kept()` — kesilen paylaşılan metin listesi,
  `GridDoc::opened_incomplete()` — ikisinden biri varsa `true`.
- UniFFI: `SheetDocument.truncatedSheets()`, `sharedStringsKept()`, `openedIncomplete()`.
- Android (`SheetViewModel`): bu durumda "Kaydet" (asıl dosyanın üzerine) kapatılır, içe aktarma
  notlarının başında Türkçe/İngilizce açık bir uyarı gösterilir ("… sayfası N. satırda kesildi"),
  "Farklı kaydet" asıl dosyanın kendisine yazmayı reddeder ve başarılı kayıttan sonra kesik
  sayfaların eksik olduğunu hatırlatır.

Test: `mo-grid/src/tests.rs` → `a_sheet_cut_by_the_engine_is_reported_as_truncated` (60.000 × 10).

## Önerilen upstream değişikliği (ayrı, bilinçli bir motor güncellemesi için)

1. **Hata düzeltmesi (asıl çözüm):** `parse_streaming` içinde, `split` öğesi geri çağırmaya
   verildiğinde onun alt ağacı için sayılan öğeleri `kept`'ten düş. Örneğin yığına her öğeyle
   birlikte "açıldığı andaki `kept`" değerini koy; `split` öğesi bitince
   `kept = o_andaki_değer` yap. Böylece sınır gerçekten "ağaçta tutulan" öğeleri sayar ve tek bir
   devasa `<row>` (ya da `<si>`) yine sınırlanır; akışla okunan satır sayısı ise yalnızca
   1.048.576 × 16.384 ızgarasıyla ve paket bayt sınırlarıyla sınırlanır.
2. **İsteğe bağlı:** hücre sayısı için ayrı, açık bir sınır (ör. `ReadLimits { max_cells, max_xml_nodes }`
   ve `open_bytes_with_limits`) ki mobil istemciler bellek bütçesine göre ayarlayabilsin. Telefon
   için öneri: toplam 20–30 milyon hücre (GridCraft hücresi değer + stil kimliği + isteğe bağlı
   formülle kabaca 40–80 bayt; 25 M hücre ≈ 1–2 GB üst sınır, gerçek dosyalar çok daha seyrek).
3. Kesme olursa uyarı yerine yapılandırılmış bir bilgi (`ReadReport::truncated_sheets`) döndür ki
   istemciler metin ayrıştırmak zorunda kalmasın.

Güncelleme yapıldığında: `mo-grid`'deki testi "tüm satırlar açılır, uyarı yok" olarak değiştirip
120.000 × 10 bir sayfayla doğrulayın; uyarı tanıma kodu, başka kesmeler için yerinde kalabilir.
