# Kayıpsız Kayıt Derlemi (aç → düzenle → kaydet → bağımsız okuyucu)

Plan maddesi: **Belge kaybı (P1)**. Amaç, motorların kaydederken içeriği sessizce düşürmesini yakalamak.

## Nasıl çalışır

`bash scripts/check-office-interop.sh` her çalıştığında (CI de bu betiği çağırır):

1. `scripts/office_corpus.py generate` geçici bir klasöre sentetik bir derlem üretir. Dosyalar bağımsız Python
   kütüphaneleriyle (python-docx, openpyxl, python-pptx; sürümleri `scripts/interop-requirements.txt` içinde) yazılır.
   İndirilen ya da kişisel dosya yoktur, depoya ikili dosya eklenmez; küçük PNG resim kodla üretilir.
2. `mo-word`, `mo-grid` ve `mo-deck` köprülerindeki `tests::corpus_round_trip` testleri her dosyayı uygulamanın kendi
   köprüsüyle açar, motor üzerinden **bir** düzenleme yapar, kaydeder (`*.saved.*`) ve köprünün bildirdiği uyarıları yazar.
   `MO_CORPUS_DIR` tanımlı değilse bu testler hiçbir şey yapmaz.
3. `scripts/office_corpus.py check` kaydedilen dosyaları aynı bağımsız kütüphanelerle okur ve özgün dosyayla karşılaştırır:
   özgün dosyada olan her şey hâlâ orada olmalı, artı düzenleme.

## Derlemin kapsamı

| Tür | İçerik | Düzenleme |
|---|---|---|
| DOCX | Başlık 1/2, kalın/italik/altı çizili/renkli/16 pt Arial sözcükler, ortalanmış paragraf, madde işaretli ve numaralı liste, yatay ve dikey birleştirilmiş hücreli tablo, satır içi resim, sayfa sonu, üst bilgi/alt bilgi, kenar boşlukları, belge başlığı/yazarı/konusu, İ ı Ş ş Ğ ğ Ü ü Ö ö Ç ç | Belgenin başına `Düzenlendi ` yazılır (`caret.docStart` + `text.insert`) |
| XLSX | Türkçe adlı üç sayfa, sayı/tarih/metin/mantıksal değerler, SUM, IF, VLOOKUP ve sayfalar arası formül, birleştirilmiş hücre, sayı biçimleri, kalın/italik/renk/dolgu/kenarlık/hizalama, sütun genişliği, satır yüksekliği, dondurulmuş bölme, otomatik filtre, yatay sayfa, açılır liste doğrulaması, hücre notu (yazarıyla), tanımlı ad, yazar/başlık | `Veri!B3` 75 yapılır (`cell.set`); formüllerin önbellekteki değerleri yeni değere göre denetlenir |
| PPTX | Üç slayt, başlık/gövde yer tutucuları, kalın/renkli/24 pt sözcüklü metin kutusu, birleştirilmiş hücreli tablo, resim (baytlarıyla), konuşmacı notları, slayt düzeni adları, şekil konum ve boyutları, slayt boyutu, başlık/yazar, Türkçe metin | İlk slaytın başlığına ` (düzenlendi)` eklenir |

Ayrıca her dosyada özgün paketteki her parça türünün (yorumlar, resimler, notlar, temalar…) kayıttan sonra da bulunduğu
denetlenir; köprü bir uyarı bildirirse bu da hata sayılır.

## Bilinen kayıplar

Aşağıdaki kayıplar bugün gerçekten oluyor. Betik bunların **tam olarak bugünkü gibi** olduğunu doğrular: biri düzelirse
de (tabloyu güncellemek için) betik hata verir. Burada olmayan her kayıp betiği düşürür.
Hepsi bağlantıyla sabitlenmiş üst kaynak motorlarda; bu depodaki `mo-*` köprüleri kaydı doğrudan motora bırakıyor.

| Kimlik | Tür | Ne kayboluyor | Nerede |
|---|---|---|---|
| `docx-tema-baglari` | DOCX | Stillerdeki tema yazı tipi ve renk bağları (`asciiTheme`, `themeColor`…) sabit değer olarak yazılıyor. Görünüm neredeyse aynı (örneğin Başlık 1 rengi 365F91 yerine 376092), ama Word'de tema değiştirilince bu stiller artık değişmiyor. | WordCraft DOCX yazıcısı |
| `docx-tema-parcasi` | DOCX | Tema parçası küçültülerek yeniden yazılıyor: yazı dizilerine özel yazı tipleri (Japonca, Arapça…) ve degrade/gölge/3B efekt stilleri düşüyor. | WordCraft DOCX yazıcısı |
| `docx-tablo-stili-paragraf` | DOCX | Tablo stillerinin koşullu bölümlerindeki (ilk satır, son satır…) paragraf aralıkları düşüyor. | WordCraft DOCX yazıcısı |
| `docx-resim-oran-kilidi` | DOCX | Resmin en-boy oranı kilidi (`noChangeAspect`) düşüyor; resim Word'de orantısız boyutlandırılabilir hâle geliyor. | WordCraft DOCX yazıcısı |
| `pptx-yazici-ayarlari` | PPTX | Yazıcı ayarları parçası (`ppt/printerSettings/*.bin`) düşüyor. | DeckCraft PPTX yazıcısı |
| `pptx-kucuk-resim` | PPTX | Dosyanın önizleme resmi (`docProps/thumbnail.jpeg`) düşüyor. Düzenlemeden sonra zaten eskimiş olurdu. | DeckCraft PPTX yazıcısı |

## Kayıp sayılmayan farklar

Şunlar kaydedilen dosyada farklı görünür, ama içerik ya da görünüm değişmez; denetlenmez:

- XLSX: satır içi metinler paylaşılan metin tablosuna taşınıyor; yorum parçaları yeniden adlandırılıyor; Excel'in yeniden
  ürettiği gizli `_xlnm._FilterDatabase` adı, boş `workbookProtection`, varsayılan renk paleti ve varsayılan
  `sheetPr` ayarları yazılmıyor; dolgu renginin saydamlık baytı `00` yerine `FF`.
- DOCX: `rsid` izleme kimlikleri, numara kimliği olmayan (etkisiz) `numPr`, sıfır tablo girintisi yazılmıyor;
  `docProps/app.xml` istatistikleri yeniden yazılıyor.
- PPTX: sunum ve görünüm ayarlarındaki uzantı kayıtları (`extLst`), varsayılan `defPPr` düzeyi ve `sysClr` yerine aynı
  rengin `srgbClr` olarak yazılması.

## Yeni bir öğe eklemek

`scripts/office_corpus.py` içinde ilgili `make_*` işlevine öğeyi, `*_facts` işlevine okunmasını ekleyin. Yeni bir kayıp
çıkarsa denetimi gevşetmeyin: `KNOWN_LOSSES` sözlüğüne bir sınama ile ekleyin ve bu tabloya bir satır yazın.
