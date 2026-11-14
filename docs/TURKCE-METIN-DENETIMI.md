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
| EML `charset=ibm857` / `cp857` / `857` / `csIBM857` | 8 bit ve quoted-printable | "����" | **Düzeltildi**: ayrıştırıcı da encoding_rs de CP857'yi tanımadığı için parça baytları `mo_convert::text::decode_cp857` ile (128 girişlik tablo; Python'un ve uygulamadaki ZIP çözücüsünün tablosuyla aynı, 0xD5/0xE7/0xF2 boş) okunuyor. `decode_label` de bu adları tanıyor (VCF/UDF). RFC 2047 başlığında (`=?ibm857?…?=`) hâlâ çözülmüyor |
| **MSG** 001E dizgileri | PR_MESSAGE_CODEPAGE 1254 / yok / yanlışlıkla 1252 | Kod sayfası okunmuyordu, hep 1254 sayılıyordu; 0x80–0x9F (“ ” – €) kontrol karakteri oluyordu | **Düzeltildi**: PR_MESSAGE_CODEPAGE'e göre; 1252 etiketli ama Türkçe harfli metin 1254 |
| MSG HTML gövdesi | PR_INTERNET_CPID 1254 / 28599 / 65001 / yok | Hep UTF-8 kayıplı: Türkçe harfler "�" | **Düzeltildi**: PR_INTERNET_CPID → `<meta>` → Türkçe öncelikli tahmin |
| MSG 8 bitlik ek adı (3707001E) | | Okunmuyordu ("ek-1") | **Düzeltildi** |
| **VCF** 2.1 QP `CHARSET=ISO-8859-9`, 8 bit NOTE | | Harfler doğru, “ ” – kontrol karakteri | **Düzeltildi** (`decode_charset` artık encoding_rs ile) |
| **ICS** 8 bit SUMMARY | | Aynı | **Düzeltildi** |
| **UDF** (`mo-word/src/udf.rs`) UTF-8 / BOM'lu UTF-8 | | Doğru | Test eklendi |
| UDF, `encoding="windows-1254"`/`"ISO-8859-9"` ya da bildirimsiz 1254 | | Dosya hiç açılmıyordu ("UTF-8 değil") | **Düzeltildi**: XML bildirimindeki kodlamayla, yoksa Türkçe öncelikli |
| **Yazı tipleri — WordCraft** (Word, metin, e-posta…) | Yazı tipi klasörü olmadan "Calibri", "Times New Roman", "Arial", "Cambria", "Tahoma", "Comic Sans MS" × normal/kalın/italik | Hepsi uygulamaya gömülü Source Sans 3 / Source Serif 4'e düşüyor; 12 harfin hepsi var | Test eklendi (glif numarası ≠ 0). Android klasörleri taranmıyor ama Türkçe için gerekmiyor |
| **Yazı tipleri — DeckCraft** (slaytlar ve PDF'leri) | Aynı aileler + "Calibri Light", "Aptos", "Courier New" | Hepsi gömülü tek yüze, Ubuntu Light'a düşüyordu (kalın yazı da ince); 12 harf vardı | **Kısmen düzeltildi** (`mo-deck/src/fonts.rs`): uygulamada zaten bulunan Inter (Regular/Medium/SemiBold) değiştirilmeden DeckCraft'a veriliyor; bütün aileler Inter ile, kalın yazı SemiBold ile çiziliyor. APK'ya ek bayt yok. Tırnaklı/eşaralıklı yüz yok (bkz. açık konular 1) |
| Yazı tipleri — GridCraft PDF | | Önceki çalışmada düzeltildi (`vendor/gridcraft-pdf`, gömülü yüzler) | — |
| Yazı tipleri — PDF görüntüleme | | Android PdfRenderer (PDFium) çiziyor; uygulamanın yazdığı PDF'ler yazı tiplerini gömüyor | Başkasının gömmediği yazı tipli PDF'lerde İ eksik kalabilir (bkz. açık konular) |
| Yazı tipleri — CadCraft | | Önceki çalışmada düzeltildi (`mo-cad/src/fonts.rs`) | — |
| **Kotlin**: metin dosyası açma (`TextViewModel`) | BOM, BOM'suz UTF-16, katı UTF-8, sonra Türkçe öncelikli tahmin | BOM'suz UTF-16 her harfin yanında NUL; 8 bitlik her metin 1254 (Kiril/Çince bozuk) | **Düzeltildi**: Rust'taki ortak çözücü (`mo_convert::text::decode_text`, FFI `decodeTextFile`) arka planda (IO iş parçacığı) çağrılıyor; 16 MB sınırı korunuyor, kesilen son harf atılıyor. Bulunan kodlama başlıkta görünüyor ("windows-1251 · 12 satır") |
| Kotlin: metin dosyasını **yerinde kaydetme** | 1254, UTF-16LE (BOM'lu/BOM'suz), UTF-8 (BOM'lu/BOM'suz), CRLF | Yalnız uygulamanın altı kodlaması vardı | **Düzeltildi**: dosya açıldığı kodlamayla, BOM'u ve satır sonlarıyla geri yazılıyor (`encodeTextFile`, encoding_rs; UTF-16 elle). O kodlamada olmayan bir harf (1254'te emoji) kaydı durduruyor ve "Farklı kodlamayla kaydet › UTF-8" öneriliyor; hiçbir zaman "?" yazılmıyor. Çözülürken "�" konmuş (geri yazılınca baytları değişecek) dosya yerinde hiç kaydedilmiyor |
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
  `slide_fonts_resolve_to_the_app_s_bundled_faces_without_system_fonts` (yazı tipi klasörü olmadan aile/kalınlık,
  yüzün paketteki dosyayla bayt bayt aynı olması ve glif numarası ≠ 0), `slide_text_lays_out_in_bold_and_italic_faces`
  (DeckCraft'ın satır dizgisiyle).
- `rust/mo-ffi/src/templates.rs`: içerik aramasında 1254 metin.
- `rust/mo-convert/src/text.rs`: `decoded_files_save_back_byte_for_byte` (1254, UTF-8 ±BOM, UTF-16LE/BE ±BOM, CRLF,
  1251), `letters_an_encoding_lacks_are_refused_not_replaced`, `files_that_would_not_come_back_the_same_say_so`,
  `the_start_of_a_large_file_drops_a_cut_letter`, `dos_code_page_857`; `rust/mo-convert/src/tests.rs`:
  `e_mails_in_the_turkish_dos_code_page_keep_their_letters`.
- `tools/jvm-smoke/src/test/kotlin/TextFileTest.kt`: `decodeTextFile`/`encodeTextFile` Kotlin bağlamalarıyla.
- Kotlin: `ArchiveReaderTest` (CP857, IBM850, Unicode Path), `FormulaHelpTest`, `TextLogicTest`. Kotlin yalnızca CI'da
  derleniyor; bu üç dosyanın kodu ve testleri yerelde düz bir JVM projesinde (Compose sınıfları için iki küçük taslakla)
  derlenip çalıştırıldı.

## Açık konular (bu çalışmada düzeltilmedi)

1. **Slaytlarda tek aile: Inter** (önceden her şey Ubuntu Light'tı; kısmen düzeltildi, `rust/mo-deck/src/fonts.rs`,
   motor sürümü değişmedi). DeckCraft yazı tiplerini yalnız masaüstü klasörlerinde arıyor ve uygulama isteğe bağlı
   craft-fonts yüzleri olmadan derleniyor. DeckCraft yalnız kendi varsayılan ailesini (Inter) ve Office yazı tiplerinin
   açık karşılıklarını (Calibri için Carlito, Times New Roman/Cambria için Caladea/Liberation Serif, Courier New için
   Liberation Mono, Arial…) adıyla arıyor; aile takma adı veren bir API'si yok. Bulamadığı her aile Inter'e düşüyor.
   Şimdi WordCraft'ın uygulamaya zaten derlenmiş Inter Regular/Medium/SemiBold dosyaları ilk sunum açılırken
   **değiştirilmeden** `FontDb::add_font` ile veriliyor: Calibri, Arial, Times New Roman, Courier New… hepsi Inter;
   kalın SemiBold yüzüyle, italik çizicide eğiltilerek. **APK etkisi: 0 bayt yazı tipi**; bellek ~1,25 MB.

   **Lisans gerekçesi.** Uygulamadaki öteki yüzler (Source Sans 3, Source Serif 4, JetBrains Mono) DeckCraft'ın
   aradığı bir adla bulunabilsin diye `name` tablosu değiştirilerek denendi, sonra bırakıldı: ad tablosunu değiştirmek
   OFL'de "Değiştirilmiş Sürüm" demek ve dışa aktarılan PDF'ler yüzü (alt kümesini) gömerek dağıtıyor.
   - Source Sans 3 ve Source Serif 4: OFL 1.1, **Ayrılmış Yazı Tipi Adı "Source"** (Adobe; `OFL-SourceSans3.txt`,
     `OFL-SourceSerif4.txt` ilk satır). Değiştirilmiş sürüm bu adı birincil ad olarak taşıyamaz.
   - Aranan adlar da başkalarının: Carlito ve Liberation (ve büyük olasılıkla Caladea) kendi OFL'lerinde Ayrılmış Yazı
     Tipi Adı (çevrimdışı doğrulanamadı, hafızadan); Arial, Times New Roman, Courier New, Calibri ticari marka. Bir yüzü
     bu adlarla göstermek yanıltıcı olur.
   - Inter ("Copyright 2016 The Inter Project Authors") ve JetBrains Mono ("Copyright 2020 The JetBrains Mono Project
     Authors"): OFL 1.1, Ayrılmış Ad **yok**. Inter zaten aranan ad olduğu için değiştirilmeden kullanılabiliyor
     (Özgün Sürüm, yalnız lisans metni ve telif satırı korunmalı: README'de). JetBrains Mono'yu DeckCraft aramıyor.

   Kalan yollar (APK büyüklüğü ↔ benzerlik):
   - (a) craft-fonts: Cargo özelliği değil, `CRAFT_FONTS_DIR` ile verilen ayrı depo; Latin sunum yüzleri (Inter, Carlito,
     Caladea, Liberation ×3, Montserrat, Lato, Open Sans, Roboto, Merriweather, Playfair, Poppins, Nunito Sans;
     OFL/Apache) yanında ana listedeki Japonca/sembol/RTL yüzlerini de gömüyor (yalnız BIZ UDPGothic 4,7 MB).
   - (b) `/system/fonts` (Roboto, Noto Serif, Droid Sans Mono): DeckCraft bu adları aramıyor; ad değiştirmeden
     kullanılamaz (aynı lisans/yanıltma sorunu, üstelik cihaza göre değişir).
   - (c) Carlito + Caladea + Liberation Sans/Serif/Mono'yu **değiştirmeden** varlık olarak eklemek: DeckCraft tam bu
     adları arıyor, metrikler Office'le aynı, lisans temiz (Özgün Sürüm). Tahminen ~5 MB ham, APK'da ~2,5 MB
     (Carlito tek başına ~2,6 MB; Latin alt kümesi bir değişiklik sayılır, aynı ad kullanılamaz). En doğru sonuç bu.
   - (d) Upstream: DeckCraft'a aile takma adı API'si (ör. `FontDb::add_alias("Liberation Serif", "Source Serif 4")`)
     ya da Android klasörleri; o zaman Source Serif 4/JetBrains Mono değiştirilmeden, sıfır baytla kullanılır.
2. **Yazı tipini gömmeyen başkasının PDF'i**: görüntüleme Android PdfRenderer'da; PDFium'un yerleşik yedek yüzlerinde
   `/Idotaccent` olmayabiliyor (gridcraft-pdf yamasındaki gözlem). Uygulamanın yazdığı PDF'ler yazı tiplerini gömüyor;
   başkasının dosyası için yapacak bir şey yok (görüntüleyiciyi değiştirmeden).
3. **DOS (CP857) düz metin dosyaları** (eski muhasebe programı dökümleri): CP857 ile Windows-1254'ü kısa metinde ayırmak
   güvenilir değil; bu dosyalar 1254 sayılıyor (Ç ü ö gibi harfler bozuk çıkar). Kotlin'deki "Türkçe karakter onarımı"
   (`repairTurkish`) da yalnız UTF-8↔1252 ve 1254↔1252 karışıklıklarını onarıyor. Gerekirse metin görüntüleyicide elle
   "Kodlama: CP857" seçeneği eklenebilir.
4. **Üçüncü taraf .xls üreticileri**: bazıları (eski PHP/Java kitaplıkları) BIFF8'in sıkıştırılmış dizgilerine 1254 baytları
   yazıyor; Excel bunları da Latin-1 okur (Ý ý Þ). Belirtime uyduğumuz için bilerek dokunulmadı.
5. ~~Kotlin metin görüntüleyici BOM'suz UTF-16'yı tanımıyor ve 8 bitlik her metni 1254 sayıyor.~~ **Düzeltildi**: görüntüleyici
   Rust'taki ortak çözücüyü kullanıyor ve kaydederken dosyanın kodlamasını sessizce değiştirmiyor (yukarıdaki tablo).
   Kalan: karışık satır sonlu dosya (bazı satırlar CRLF, bazıları LF) kaydedilince hepsi CRLF oluyor (önceden de böyleydi);
   "Farklı kodlamayla kaydet" listesi hâlâ uygulamanın altı kodlaması (açılan dosyanın 1251 gibi kodlaması listede yok,
   yerinde kaydetme onu koruyor).
6. ~~E-postada `charset=ibm857` çözülmüyor.~~ **Düzeltildi** (gövde parçaları). Kalan: RFC 2047 başlıklarında `=?ibm857?…?=`
   (konu, gönderen adı) ayrıştırıcıda çözülmüyor; DOS (CP857) düz metin dosyaları yine 1254 sayılıyor (madde 3).

## Önerilen upstream yamaları

- **WordCraft** `crates/formats/src/txt.rs` (`legacy`) ve HTML içe aktarma: chardetng'ye köken ipucu (`tld`) verilebilmeli
  ya da yerel ayardan alınmalı; kısa metinde çok baytlı tahmin, harflerin çoğu tek bir tek baytlı kod sayfasının
  harfleriyse reddedilmeli. RTF: `\ansicpg` yoksa `\deflang`'dan kod sayfası (1055 → 1254, 1049 → 1251…), Word'ün yaptığı
  gibi. HTML5 adlandırılmış karakter başvurularının tamamı (`&scedil;` vb.).
- **mail-parser**: ISO-8859-9 için Windows-1254 tablosu (WHATWG); charset'siz 8 bit metin parçası ve ham 8 bit başlık
  için yedek kodlama seçeneği.
- **DeckCraft** `deckcraft-fonts`: aile takma adı API'si (`add_alias(istenen, var_olan)`), böylece uygulamanın
  değiştirilmemiş Source Serif 4 / JetBrains Mono yüzleri tırnaklı/eşaralıklı karşılık olur; `system_font_dirs`'e
  `cfg!(target_os = "android")` için `/system/fonts` ve `/product/fonts` (CJK, emoji ve sembol yedekleri
  `SYSTEM_FALLBACKS`'teki Noto adlarıyla zaten eşleşir); karşılık listesine "Roboto", "Noto Serif", "Droid Sans Mono".
