# Türkçe metin denetimi (bütün biçimler)

Tarih: 2026-11-14. DWG/DXF'te bulunan Türkçe harf hatalarının ([upstream-notlar/cadcraft-turkce.md](upstream-notlar/cadcraft-turkce.md))
benzerleri öteki biçimlerde arandı: yanlış kod sayfası (1252 ↔ 1254, CP437 ↔ CP857), bildirilmemiş kodlamada
yanlış tahmin, Android'de yazı tipi bulunamaması (boş kutu) ve İ/ı'yı hesaba katmayan büyük/küçük harf işlemleri.

Yöntem: her biçim için testte bayt bayt bir dosya kuruldu (internetten dosya indirilmedi, ikili dosya eklenmedi),
köprümüzün "İıŞşĞğÇçÖöÜü" için verdiği sonuç denetlendi; yanlışsa düzeltme köprüde (`rust/mo-*`) ya da Kotlin'de yapıldı.
Motorlar sabit sürümlerinde bırakıldı (AGENTS.md madde 5).

## Özet tablo

| Biçim / yer | Denenen | Sonuç (önce) | Durum |
|---|---|---|---|
| **ZIP dosya adları** (Kotlin `ArchiveReader`) | Windows'un yerleşik ZIP'i ve eski WinZip: CP857, UTF-8 bayrağı yok | Adlar cihazdaki `IBM857` karakter kümesine bağlıydı; yoksa CP437/Latin-1'e düşüyordu ("Çağrı" → "Ça§r?") | **Düzeltildi**: CP857 çözücü uygulamada yerleşik (JVM'in IBM857 tablosuyla bayt bayt aynı olduğu test ediliyor) |
| ZIP Unicode Path alanı (0x7075) | 7-Zip/Info-ZIP'in DOS adının yanına yazdığı UTF-8 ad | Okunmuyordu ("Rapor ?ubat.txt") | **Düzeltildi**: CRC'si saklanan adla eşleşirse bu ad kullanılıyor (listede ve çıkarılan dosyanın adında); eşleşmezse (dosya sonradan yeniden adlandırılmış) yok sayılıyor |
| ZIP, Batı adları (CP850) | "Café Menü Señor Ação" | — | Doğru (é ü ñ ç ã aynı baytlar). CP857'de yalnızca ì ÿ × ƒ ª º gibi seyrek işaretler ve kutu çizgileri Türkçe harfle yer değiştiriyor |
| 7z adları (`mo-archive`) | — | Doğru: 7z adları her zaman UTF-16 | Değişiklik yok. RAR desteklenmiyor |
| **RTF** `\ansicpg1254`, `\'dd`, ham 8 bit, `\fcharset162` (`\f` ile/`\f`siz, `\ansicpg1252` içinde), `\uN` (`\uc0/1/2`, negatif N) | | Hepsi doğru (WordCraft) | Test eklendi |
| RTF, `\ansicpg` yok ama `\deflang1055`/`\lang1055` | | İ→Ý, ı→ý, Ş→Þ… (RTF'nin varsayılanı 1252) | **Düzeltildi** (`mo-word/src/charset.rs`): Türkçe dilli ve kod sayfası belirtmeyen RTF'ye `\ansicpg1254` ekleniyor. İzlandaca (`\deflang1039`) ve `\deflang10550` gibi benzerler etkilenmiyor |
| **Eski .doc** (Word 97–2003) | UTF-16 parça + 8 bitlik (sıkıştırılmış) parça, en küçük Word 97 dosyası testte kuruluyor | Doğru: Word 8 bitlik parçayı yalnızca bütün harfler 1252'ye sığınca kullanır; ğ ı ş İ Ş Ğ her zaman UTF-16 parçada | Test eklendi. Word 6/95 dosyalarını motor açmıyor (kod sayfası sorunu yok). Özet bilgi (başlık, yazar) kod sayfasına göre çözülüyor |
| **Eski .xls** (BIFF8) | En küçük BIFF8 çalışma kitabı testte kuruluyor: CODEPAGE 1254, Türkçe sayfa adı, SST'de sıkıştırılmış ("Çiçek Ölçü") ve UTF-16 ("İıŞş…") dizgiler | Doğru (craft-xls belirtime uygun: sıkıştırılmış = Latin-1) | Test eklendi. BIFF5 (Excel 95) motor tarafından açılmıyor |
| **Eski .ppt** (`mo-deck/src/ppt.rs`) | TextBytesAtom ("Çiçek Ölçüsü", Latin-1) + TextCharsAtom ("İğne Şişesi") | Doğru ([MS-PPT]: TextBytesAtom UTF-16'nın düşük baytı) | Test eklendi |
| **Düz metin** (.txt, .md, .log) WordCraft ile | Windows-1254, BOM'suz | Kısa Türkçe metin **Çince (GBK)** çıkıyordu ("İıŞş…" → "蔟撖叙…"), uzun metin 1252 ("Şişli" → "Þiþli") | **Düzeltildi**: `mo_convert::text` chardetng'yi Türkiye ipucuyla (`.tr`) çalıştırıyor; Türkçe harfler Batı ya da birkaç bayta dayanan çok baytlı tahmini yeniyor. Kiril/Çince metin kendi kodlamasında kalıyor. `WordDoc::from_text` de aynı yoldan |
| İçerikte arama (`document_text`, .txt) | Windows-1254 | "�" (UTF-8 kayıplı çözülüyordu), aranınca bulunmuyordu | **Düzeltildi** (`mo_word::plain_text`) |
| **HTML** `<meta charset=windows-1254>`, `http-equiv … iso-8859-9`, UTF-16 BOM | | Doğru | Test eklendi |
| HTML, charset yok, 8 bit | | Çince (GBK) | **Düzeltildi** (yukarıdaki gibi; BOM'lu UTF-8 olarak veriliyor) |
| HTML `&scedil; &Scedil; &gbreve; &Gbreve; &Idot; &imath; &inodot;` | | Çözülmüyordu (olduğu gibi görünüyordu) | **Düzeltildi**: sayısal karşılıklarına çevriliyor (UTF-16'da dokunulmuyor) |
| **CSV/TSV** (`mo-grid`) Windows-1254 | | Doğru (önceki çalışma) | Test genişletildi (tırnak ve tire dahil) |
| CSV, yalnız Ç Ö Ü (1252 ile ortak) | | Doğru | Test eklendi |
| CSV/TXT, BOM'suz UTF-16 | | Her harfin yanında NUL, bozuk hücreler | **Düzeltildi** (`encoding::decode_unmarked_utf16`) |
| **EML** `charset=windows-1254` | | Doğru | Test eklendi |
| EML `charset=iso-8859-9` | | Harfler doğru ama Windows-1254'ün “ ” – işaretleri siliniyordu | **Düzeltildi**: tarayıcılar gibi ISO-8859-9 = Windows-1254 |
| EML, charset yok / Content-Type yok | | Bütün Türkçe harfler "�" | **Düzeltildi**: parça baytları aktarım kodlaması çözülüp Türkçe öncelikli okunuyor (UTF-8 ise UTF-8) |
| EML HTML parçası, charset yalnız `<meta>`'da | | "�" | **Düzeltildi** |
| EML, ham 8 bit konu/gönderen (RFC 2047'siz) | | "�� toplant�s�" | **Düzeltildi**: üst başlık bloğu UTF-8'e çevrilip veriliyor |
| EML ek adları (RFC 2047 / RFC 2231, iso-8859-9, windows-1254) | | Doğru | — |
| EML `charset=ibm857` | | "����" | Açık (çok nadir; ayrıştırıcı bu adı tanımıyor, bizim çözücümüz de karıştırmıyor) |
| **MSG** 001E dizgileri | PR_MESSAGE_CODEPAGE 1254 / yok / yanlışlıkla 1252 | Kod sayfası okunmuyordu, hep 1254 sayılıyordu; 0x80–0x9F (“ ” – €) kontrol karakteri oluyordu | **Düzeltildi**: PR_MESSAGE_CODEPAGE'e göre; 1252 etiketli ama Türkçe harfli metin 1254 |
| MSG HTML gövdesi | PR_INTERNET_CPID 1254 / 28599 / 65001 / yok | Hep UTF-8 kayıplı: Türkçe harfler "�" | **Düzeltildi**: PR_INTERNET_CPID → `<meta>` → Türkçe öncelikli tahmin |
| MSG 8 bitlik ek adı (3707001E) | | Okunmuyordu ("ek-1") | **Düzeltildi** |
| **VCF** 2.1 QP `CHARSET=ISO-8859-9`, 8 bit NOTE | | Harfler doğru, “ ” – kontrol karakteri | **Düzeltildi** (`decode_charset` artık encoding_rs ile) |
| **ICS** 8 bit SUMMARY | | Aynı | **Düzeltildi** |
| **UDF** (`mo-word/src/udf.rs`) UTF-8 / BOM'lu UTF-8 | | Doğru | Test eklendi |
| UDF, `encoding="windows-1254"`/`"ISO-8859-9"` ya da bildirimsiz 1254 | | Dosya hiç açılmıyordu ("UTF-8 değil") | **Düzeltildi**: XML bildirimindeki kodlamayla, yoksa Türkçe öncelikli |
| **Yazı tipleri — WordCraft** (Word, metin, e-posta…) | Yazı tipi klasörü olmadan "Calibri", "Times New Roman", "Arial", "Cambria", "Tahoma", "Comic Sans MS" × normal/kalın/italik | Hepsi uygulamaya gömülü Source Sans 3 / Source Serif 4'e düşüyor; 12 harfin hepsi var | Test eklendi (glif numarası ≠ 0). Android klasörleri taranmıyor ama Türkçe için gerekmiyor |
| **Yazı tipleri — DeckCraft** (slaytlar ve PDF'leri) | Aynı aileler + "Calibri Light", "Aptos", "Courier New" | Hepsi gömülü tek yüze, Ubuntu Light'a düşüyordu (kalın ve tırnaklı yazı da ince, düz); 12 harf vardı | **Düzeltildi** (`mo-deck/src/fonts.rs`): WordCraft'ın uygulamada zaten bulunan yüzleri DeckCraft'a veriliyor; Calibri → Source Sans 3, Times New Roman/Cambria → Source Serif 4, Arial/Aptos → Inter, Courier New → JetBrains Mono, kalın/italik gerçek yüzleriyle. APK'ya ek bayt yok |
| Yazı tipleri — GridCraft PDF | | Önceki çalışmada düzeltildi (`vendor/gridcraft-pdf`, gömülü yüzler) | — |
| Yazı tipleri — PDF görüntüleme | | Android PdfRenderer (PDFium) çiziyor; uygulamanın yazdığı PDF'ler yazı tiplerini gömüyor | Başkasının gömmediği yazı tipli PDF'lerde İ eksik kalabilir (bkz. açık konular) |
| Yazı tipleri — CadCraft | | Önceki çalışmada düzeltildi (`mo-cad/src/fonts.rs`) | — |
| **Kotlin**: metin dosyası açma (`TextEncoding.detect`) | BOM, katı UTF-8, sonra Windows-1254 | Türkçe için doğru | Değişiklik yok (bkz. açık konular) |
| Kotlin: pano | Compose/Android dizgileri UTF-16 | Doğru | — |
| Kotlin: `uppercase()`/`lowercase()` yerel ayarsız | Bütün `app/src/main` | Yok; görünen metinde arayüz dili, tanımlayıcılarda `Locale.ROOT` | — |
| Kotlin: arama katlaması (`searchFold`, `ContentSearch`, belge/PDF araması) | "istanbul" | "İSTANBUL", "Istanbul", "ISTANBUL", "ıstanbul" bulunuyor; uzunluk korunuyor | Test genişletildi |
| **Tablo araması** (`GridDoc::find_cells`) | "istanbul" | "İSTANBUL" bulunuyor, "Istanbul" bulunmuyordu (Türkçe kesin küçük harf: I → ı); "cagri" → "Çağrı" bulunmuyordu | **Düzeltildi**: öteki aramalar gibi i biçimleri ve aksanlar birleşiyor. **Değiştirme** kesin kalıyor ("ılık" ≠ "ilik") |
| **Formül önerileri** (Kotlin `FormulaHelp`) | İngilizce arayüzde "=if", "=min" | Türkçe büyük harf "İF"/"MİN" yapıyordu, hiçbir ad eşleşmiyordu; Türkçe adlar "IND" ile bulunmuyordu | **Düzeltildi**: adlar ve yardım araması `searchFold` biçiminde karşılaştırılıyor |

## Türkçe öncelikli kodlama tahmini (`rust/mo-convert/src/text.rs`)

Bildirilmemiş metin için tek bir karar yeri var; Word, e-posta, kişi/takvim, MSG, UDF ve içerik araması bunu kullanıyor:

1. BOM varsa ona göre; BOM'suz UTF-16 sıfır baytlarından (UTF-8'den **önce**: Latin harfli UTF-16 geçerli UTF-8 de olabilir);
   geçerli UTF-8 ise UTF-8.
2. Değilse chardetng, Türkiye ipucuyla (`guess(Some(b"tr"))`).
   - Windows-1254 derse 1254.
   - Windows-1252 derse: metinde 1252'nin İzlandaca okuduğu altı bayttan (D0 DD DE F0 FD FE) biri varsa 1254, yoksa 1252
     (iki kod sayfası öteki harflerde aynı).
   - Çok baytlı (GBK, Big5, Shift_JIS, EUC-JP/KR) derse ve 0x80 üstü baytların en az yarısı on iki Türkçe harf baytıysa 1254
     (gerçek Çince/Japonca/Korece metin baytlarını yüz kadar değere yayar).
   - Başka bir şey (Kiril 1251, Yunanca…) derse onun dediği.
3. Etiketli metinde (`decode_declared`): etiket Batı (1252) ama içerik Türkçe ise 1254 — CAD'deki yanlış etiket sorununun aynısı.
   ISO-8859-9 her yerde Windows-1254 olarak okunuyor (WHATWG; Türk Windows programları böyle etiketleyip 1254'ün tırnaklarını kullanıyor).

## Testler

- `rust/mo-convert/src/text.rs` (tahmin, etiket, `<meta>`), `rust/mo-convert/src/tests.rs`
  (`e_mails_in_turkish_code_pages_keep_their_letters`, `contacts_and_calendars_in_windows_1254_keep_quotes_and_dashes`,
  `outlook_messages_read_their_code_pages` — MSG cfb ile testte kuruluyor).
- `rust/mo-word/src/turkish_tests.rs`: RTF (11 biçim), düz metin/Markdown, HTML, en küçük Word 97 dosyası, UDF (5 kodlama),
  eksik yazı tiplerinin gömülü yüze düşmesi.
- `rust/mo-grid/src/turkish_text_tests.rs`: CSV/TSV/TXT kodlamaları, en küçük BIFF8 dosyası, arama/değiştirme.
- `rust/mo-deck/src/tests.rs`: `old_powerpoint_text_atoms_keep_turkish_letters`,
  `slide_fonts_the_phone_lacks_fall_back_to_a_face_with_turkish_letters`,
  `slide_fonts_resolve_to_the_app_s_bundled_faces_without_system_fonts` (yazı tipi klasörü olmadan aile/kalınlık/italik
  ve glif numarası ≠ 0), `slide_text_lays_out_in_bold_italic_and_serif_faces` (DeckCraft'ın satır dizgisiyle).
- `rust/mo-ffi/src/templates.rs`: içerik aramasında 1254 metin.
- Kotlin: `ArchiveReaderTest` (CP857, IBM850, Unicode Path), `FormulaHelpTest`, `TextLogicTest`. Kotlin yalnızca CI'da
  derleniyor; bu üç dosyanın kodu ve testleri yerelde düz bir JVM projesinde (Compose sınıfları için iki küçük taslakla)
  derlenip çalıştırıldı.

## Açık konular (bu çalışmada düzeltilmedi)

1. ~~**Slaytlar telefonda hep Ubuntu Light ile çiziliyor**~~ — **düzeltildi** (`rust/mo-deck/src/fonts.rs`, motor
   sürümü değişmedi). DeckCraft yazı tiplerini yalnız masaüstü klasörlerinde arıyor ve uygulama isteğe bağlı craft-fonts
   yüzleri olmadan derleniyor; bu yüzden bütün aileler gömülü Ubuntu Light'a düşüyordu. DeckCraft yalnız kendi varsayılan
   ailesini (Inter) ve Office yazı tiplerinin açık karşılıklarını (Calibri için Carlito, Times New Roman ve Cambria için
   Liberation Serif, Courier New için Liberation Mono…) arıyor. Çözüm: WordCraft'ın uygulamaya zaten derlenmiş OFL
   yüzleri (Source Sans 3, Source Serif 4, Inter, JetBrains Mono) ilk sunum açılırken `FontDb::add_font` ile DeckCraft'a
   veriliyor. Inter kendi adıyla; ötekilerin `name` tablosunun bellekteki kopyasında Windows aile adı (ID 1) aranan
   karşılığın adı yapılıyor, asıl aile adı (ID 16) korunuyor. Sonuç:

   | İstenen | Çizilen | Kalın / italik |
   |---|---|---|
   | Calibri, Calibri Light ve telefonda olmayan her aile (PowerPoint bunları Calibri ile çizer: Tahoma, Georgia, Comic Sans…) | Source Sans 3 | gerçek Bold, gerçek Italic (Bold Italic: Italic yüzü kalınlaştırılarak) |
   | Times New Roman, Cambria | Source Serif 4 | dört biçim de gerçek |
   | Arial, Helvetica, Aptos | Inter | Bold → SemiBold yüzü, italik çizicide eğiltiliyor |
   | Courier New | JetBrains Mono | yalnız Regular (kalın çizicide kalınlaştırılıyor) |

   Cihazda gerçek Carlito/Liberation varsa (masaüstü) yine o kullanılıyor. **APK etkisi: 0 bayt yazı tipi** (aynı
   `include_bytes!` verisi; `libmo_ffi.so`'ya yalnız birkaç KB kod). Bellek: ilk sunumda ~3,6 MB yazı tipi kopyası.
   Karşılaştırılan öteki yollar: (a) craft-fonts bir Cargo özelliği değil, `CRAFT_FONTS_DIR` ile verilen ayrı bir depo;
   Latin sunum yüzlerinin (Inter, Carlito, Caladea, Liberation ×3, Montserrat, Lato, Open Sans, Roboto, Merriweather,
   Playfair, Poppins, Nunito Sans; OFL/Apache) yanında ana listedeki Japonca/sembol/RTL yüzlerini de gömüyor (yalnız
   BIZ UDPGothic 4,7 MB) ve çevrimdışı derlemede elimizde yok.
   (b) `/system/fonts` (Roboto, Noto Serif, Droid Sans Mono): ücretsiz ama DeckCraft bu adları hiç aramıyor; yine ad
   değiştirmek gerekir, cihazdan cihaza değişir. (c) Metrik uyumlu Carlito + Caladea + Liberation Sans/Serif/Mono'yu
   (4'er biçim) varlık olarak eklemek en doğru satır kırılımını verir ama tahminen ~5 MB ham, APK'da ~2,5 MB sıkıştırılmış
   (Carlito tek başına ~2,6 MB ham; Latin alt kümesine indirilirse yarısından az).
   Kalan farklar: Source Sans 3 Calibri'den biraz geniş, Source Serif 4 Times'tan geniş; satırlar PowerPoint'ten farklı
   yerde kırılabilir. İleride: Android'in CJK/emoji yüzleri için `/system/fonts` taraması (DeckCraft yaması gerekir).
2. **Yazı tipini gömmeyen başkasının PDF'i**: görüntüleme Android PdfRenderer'da; PDFium'un yerleşik yedek yüzlerinde
   `/Idotaccent` olmayabiliyor (gridcraft-pdf yamasındaki gözlem). Uygulamanın yazdığı PDF'ler yazı tiplerini gömüyor;
   başkasının dosyası için yapacak bir şey yok (görüntüleyiciyi değiştirmeden).
3. **DOS (CP857) düz metin dosyaları** (eski muhasebe programı dökümleri): CP857 ile Windows-1254'ü kısa metinde ayırmak
   güvenilir değil; bu dosyalar 1254 sayılıyor (Ç ü ö gibi harfler bozuk çıkar). Kotlin'deki "Türkçe karakter onarımı"
   (`repairTurkish`) da yalnız UTF-8↔1252 ve 1254↔1252 karışıklıklarını onarıyor. Gerekirse metin görüntüleyicide elle
   "Kodlama: CP857" seçeneği eklenebilir.
4. **Üçüncü taraf .xls üreticileri**: bazıları (eski PHP/Java kitaplıkları) BIFF8'in sıkıştırılmış dizgilerine 1254 baytları
   yazıyor; Excel bunları da Latin-1 okur (Ý ý Þ). Belirtime uyduğumuz için bilerek dokunulmadı.
5. **Kotlin metin görüntüleyici** (`TextEncoding.detect`) BOM'suz UTF-16'yı tanımıyor ve 8 bitlik her metni 1254 sayıyor
   (Kiril/Çince eski kodlamalar yanlış görünür). Türkçe kullanıcı için doğru; Rust tarafıyla birleştirmek ayrı bir iş.
6. E-postada `charset=ibm857` (çok nadir) çözülmüyor.

## Önerilen upstream yamaları

- **WordCraft** `crates/formats/src/txt.rs` (`legacy`) ve HTML içe aktarma: chardetng'ye köken ipucu (`tld`) verilebilmeli
  ya da yerel ayardan alınmalı; kısa metinde çok baytlı tahmin, harflerin çoğu tek bir tek baytlı kod sayfasının
  harfleriyse reddedilmeli. RTF: `\ansicpg` yoksa `\deflang`'dan kod sayfası (1055 → 1254, 1049 → 1251…), Word'ün yaptığı
  gibi. HTML5 adlandırılmış karakter başvurularının tamamı (`&scedil;` vb.).
- **mail-parser**: ISO-8859-9 için Windows-1254 tablosu (WHATWG); charset'siz 8 bit metin parçası ve ham 8 bit başlık
  için yedek kodlama seçeneği.
- **DeckCraft** `deckcraft-fonts`: `system_font_dirs`'e `cfg!(target_os = "android")` için `/system/fonts` ve
  `/product/fonts` (CJK, emoji ve sembol yedekleri `SYSTEM_FALLBACKS`'teki Noto adlarıyla zaten eşleşir); karşılık
  listesine "Roboto", "Noto Serif", "Droid Sans Mono". O zaman 1. maddedeki ad değiştirme gereksizleşir.
