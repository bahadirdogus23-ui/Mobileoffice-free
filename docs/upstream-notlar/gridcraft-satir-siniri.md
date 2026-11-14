# GridCraft: büyük XLSX sayfalarının kesilmesi

> **Durum: upstream'de düzeldi, bu değişiklikle güncellendi.** GridCraft
> `c0ee120ac2d9b59630f5abf1151dd9b471fe2926` (`rust/Cargo.toml`) aşağıdaki §1'deki düzeltmeyi içeriyor:
> `0b7deaa3` "XLSX: the XML element cap counts only elements kept in the tree" (PR #2527, birleştirme
> `e8bdd0ef`). Sabit sürüm `11bb673`'ten `c0ee120`'ye bu değişiklikte yükseltildi. Aşağıdaki metin
> `11bb673`'teki durumu anlatır; "Güncellemeden sonra" bölümü ölçümleri verir.

Eski sabit sürüm: `gridcraft` @ `11bb67386c9bb19b0717e21e98adc608162ae2e5`.

## Güncellemeden sonra (`c0ee120`)

- 120.001 satır × 10 sütunluk sade sayfa (CSV'den üretilmiş, 3,4 MB XLSX) **bütün satırlarıyla** açılıyor:
  `truncated()` boş, `strings_kept()` `None`, `opened_incomplete()` `false`. Kaydedip yeniden açınca
  1.200.010 hücrenin hepsi aynı.
- `MAX_XML_NODES` hâlâ `1_000_000` ama artık yalnızca ağaçta o an tutulan öğeleri sayıyor; akışla verilen
  satırların (`<row>`) ve paylaşılan metinlerin (`<si>`) öğeleri satır bırakılınca düşülüyor. Tek bir devasa
  satır ya da çok iyi sıkışan "zip bombası" parça yine sınırlanıyor.
- `mo-grid`'deki kesme tanıma kodu (`Truncation`, `opened_incomplete`, atlanan parçalar) **yerinde kaldı**:
  geçersiz XML'li sayfa ya da sınırı aşan tek satır gibi kesmeler hâlâ uyarı olarak geliyor ve kötü niyetli
  dosyalarda asıl dosyanın üzerine kaydı engelliyor. Test: `a_sheet_beyond_a_million_xml_elements_opens_whole`
  (120.000 × 10, bütün satırlar, kesme yok); uyarı metni ayrıştırıcısının testi
  (`reads_the_engines_damaged_sheet_warning`) duruyor.

### Bellek (masaüstü, `release`, açılıştaki tepe `VmHWM`)

Ölçüm: `GridDoc::open` ile aynı dosyayı açan ayrı bir süreç (depoya alınmayan geçici bir örnek program),
x86_64 Linux, üç çalıştırmanın ortası. Değerler dosya okunduktan sonraki süreç tabanını (≈ 5–7 MB) içerir.

| Dosya | `11bb673` (önce) | `c0ee120` (sonra) |
|---|---|---|
| 120.001 × 10 (3,4 MB XLSX) | 75 MB tepe, 37 MB kalıcı; **49.999 satır** (kesik), 0,52 s | 126 MB tepe, 87 MB kalıcı; **120.001 satır**, 1,3 s |
| 49.999 × 10 (her iki sürümde de tam) | 50 MB tepe, 37 MB kalıcı; 0,48 s | 56 MB tepe, 43 MB kalıcı; 0,49 s |

Yani aynı miktarda veri için bellek aşağı yukarı aynı (+%12); artış, artık sayfanın tamamının belleğe
alınmasından geliyor (satır başına ≈ 0,7 KB kalıcı, 10 sayısal hücre). Bir milyon satırlık 10 sütunluk bir
sayfa bu oranla ≈ 0,7–1 GB tutar; telefonda gerçek bir büyük dosyayla ölçüm ayrıca yapılmalı.

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

Test (`11bb673`'te): `a_sheet_cut_by_the_engine_is_reported_as_truncated` (60.000 × 10); `c0ee120` güncellemesiyle
`a_sheet_beyond_a_million_xml_elements_opens_whole` (120.000 × 10, kesme yok) oldu.

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

`c0ee120` güncellemesinde yapıldı: `mo-grid`'deki test "tüm satırlar açılır, uyarı yok" olarak değiştirildi ve
120.000 × 10 bir sayfayla doğrulandı; uyarı tanıma kodu başka kesmeler için yerinde kaldı. §1 upstream'de; §2 ve §3 hâlâ yok.
