# CadCraft: DWG/DXF çizimlerinde Türkçe harfler (İ ı Ş ş Ğ ğ)

Sabitlenmiş sürüm: `cadcraft` @ `56542407292b911ad30523cfcfa987923fe2f8e2` (`rust/Cargo.toml`), onun
kullandığı `acadrust` 0.6.6. Satır numaraları bu sürümlere göredir
(`~/.cargo/git/checkouts/cadcraft-*/5654240/`, `~/.cargo/registry/src/*/acadrust-0.6.6/`).

## Belirti (sahada)

"DWG'lerde Türkçe i sorunu var": Türk mühendis ve mimarların gönderdiği çizimlerde İ/ı (ve Ş/ş,
Ğ/ğ) yanlış görünüyor. Örnek dosya yok; nedenler kodu okuyup her biri için çoğaltan bir test
yazarak bulundu (`rust/mo-cad/src/tests.rs`, `fonts.rs`, `text.rs`).

## Ne doğru çalışıyor (denendi, sorun değil)

- **Kod sayfası doğru yazılmış çizimler.** `$DWGCODEPAGE` = `ANSI_1254` olan R12/R2000 DXF ve
  bunlardan yapılan 2000/2004/2018 DWG'ler doğru okunuyor. acadrust'ın DWG kod sayfası tablosu
  (`src/io/dxf/code_page.rs:229-271`, `dwg_code_page_name`) ODA/libredwg tablosuyla aynı: 33 =
  `ANSI_1254`, 10 = `ISO8859-9`, 15 = `DOS857`; dosya başlığındaki indeks (0x13) bütün akışlarda
  kullanılıyor (`src/io/dwg/dwg_reader.rs:1277-1332`). CadCraft'ın DXF okuyucusu `ANSI_1254`'ü
  biliyor (`crates/dxf/src/codepage.rs:43`).
- **`\U+XXXX` ve `\M+nXXXX` kaçışları** (onaltılık rakamlar küçük harfle de) çözülüyor. Küçük `\u`
  bir kaçış değil (MTEXT'te "alt çizgiyi kapat"); öyle kalması doğru.
- **Kotlin tarafı.** Katman listesi ve dosya adlarında `uppercase()`/`lowercase()` yok;
  `ignoreCase = true` karşılaştırmaları yerel ayardan bağımsız.

## Bulunan nedenler ve köprüdeki (mo-cad) çözümler

### 1. Telefonda Türkçe harfler boş kutu olarak çiziliyordu (asıl görünür neden)

- `crates/fonts/src/stroke.rs:11-113` — motorun yerleşik tek çizgili yazı tipi ("CADCraft
  Stroke") yalnızca ASCII ve birkaç simge içeriyor. SHX yazı tipli metinler (txt.shx, romans.shx)
  ve TrueType yazı tipi kurulu olmayan metinler bununla çiziliyor; içinde olmayan her harf
  (İ ı Ş ş Ğ ğ Ç ç Ö ö Ü ü) kurulu bir yazı tipinden alınıyor, bulunamazsa boş kutu çiziliyor
  (`crates/fonts/src/lib.rs:167-180`, `missing_glyph_box`).
- `crates/fonts/src/ttf.rs:224-268` (`scan`) — kurulu yazı tipleri yalnızca macOS, Windows ve
  masaüstü Linux klasörlerinde aranıyor (`:238` `/usr/share/fonts`, `/usr/local/share/fonts`,
  `~/.fonts`). Android'in `/system/fonts` klasörü hiç taranmıyor, dolayısıyla telefonda yedek yazı
  tipi yok ve Türkçe harflerin hepsi kutu oluyordu. PDF'e aktarma da aynı çizimi kullandığı için
  PDF'te de aynıydı. (Masaüstünde yazı tipi bulunduğundan testlerde görünmüyordu.)

**Köprüdeki çözüm** (`rust/mo-cad/src/fonts.rs`): ilk çizim açılırken `/system/fonts` içinden
Türkçe harfleri olan ilk Latin yazı tipi (`RobotoStatic-Regular.ttf`, `Roboto-Regular.ttf`,
`NotoSans-Regular.ttf`, `DroidSans.ttf`, `DejaVuSans.ttf`) `cadcraft_fonts::ttf::register` ile
motora yedek yazı tipi olarak veriliyor (≈ 0,2–1 MB, bir kez).

**Önerilen upstream yaması** (`crates/fonts/src/ttf.rs`, `scan`):

```rust
    } else if cfg!(target_os = "android") {
        dirs.push("/system/fonts".into());
        dirs.push("/product/fonts".into());
    } else {
        dirs.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(Into::into));
```

ve `PREFERRED_FALLBACK` listesine `"roboto"`. Daha iyisi: tek çizgili yazı tipine Latin Genişletilmiş-A
harflerini temel harf + işaret olarak eklemek (ı = noktasız `i`; İ = `I` + nokta; Ş/ş = `S`/`s` +
çengel; Ğ/ğ = `G`/`g` + kısa işareti; Ç/ç, Ö/ö, Ü/ü), böylece metin tek tip yazı tipinde kalır.

### 2. Kod sayfası `ANSI_1252` yazan Türkçe çizimler: İ→Ý, ı→ý, Ş→Þ, ş→þ, Ğ→Ð, ğ→ð

Pek çok Türkçe çizimin başlığında `ANSI_1252` yazıyor (İngilizce şablondan ya da Türkçe olmayan bir
sistemde oluşturulmuş), baytları ise Windows-1254. 1252 olarak okununca iki kod sayfasının farklı
olduğu altı harf bozuluyor (Ç Ö Ü ç ö ü ikisinde de aynı; "i sorunu" diye fark edilen bu). Çizim
sonradan 2007+ olarak kaydedildiyse bozuk harfler artık onun Unicode metni. Motor başlığa uyuyor
(doğru davranış); kendi başına düzeltmesi beklenemez.

**Köprüdeki çözüm** (`rust/mo-cad/src/text.rs`, `prepare`): başlığı Batı Avrupa (`ANSI_1252`, ISO
8859-1, yok ya da bilinmeyen) ya da Türkçe olan bir çizimin metninde bu altı harf (veya aşağıdaki
`%%nnn` kodları) varsa ve İzlandaca görünmüyorsa (á é í ó ú æ sayısının dört katı bu harflerden
azsa), metin Windows-1254 olarak okunuyor. Dönüşüm bütün DXF metnine aynı şekilde uygulandığı için
katman, blok ve metin adları birbirini bulmaya devam ediyor. DWG'ler motorun DWG köprüsünün
ürettiği DXF üzerinden aynı yoldan geçiyor (`cadcraft_io::read` ile aynı adımlar:
`dwg_to_dxf_noted` → düzeltme → `read_dxf`).

### 3. `%%nnn` karakter kodları Unicode sayılıyor

`crates/fonts/src/lib.rs:61-66` (`decode_controls`) — `%%221` Unicode U+00DD (Ý) olarak çiziliyor.
AutoCAD'de `%%nnn` çizimin kod sayfasındaki nnn numaralı karakter: Türkçe çizimde `%%221` İ,
`%%253` ı, `%%222` Ş, `%%254` ş, `%%208` Ğ, `%%240` ğ.

**Köprüdeki çözüm**: Türkçe çizimlerde (başlık `ANSI_1254`/ISO 8859-9 ya da 2. maddedeki tespit)
metin değerlerindeki (grup kodu 1 ve 3) bu altı kod, motorun kodları okuduğu kuralla
(`%%` + en çok üç rakam; `%%%`, `%%d` … üç karakter) harfe çevriliyor.

**Önerilen upstream yaması**: `decode_controls`'a çizimin kod sayfasını geçirip 128–255 arasını
`cadcraft_dxf::decode_code_page(&[n as u8], code_page)` ile çözmek.

### 4. CadCraft'ın DXF okuyucusunun bilmediği kod sayfaları (DOS857 = Türkçe DOS)

- `crates/dxf/src/codepage.rs:29-62` (`decoder`) — `DOS857` (ve DOS855/860/861/863/864/865/869,
  Johab, KOI8) yok; bunlar `:118-125` ile Windows-1252 olarak okunuyor.
- `crates/dwg/src/lib.rs:116` — DWG köprüsü acadrust belgesini `DxfWriter` ile DXF'e yazıyor;
  acadrust 2007 öncesi sürümlerde metni çizimin kod sayfasıyla kodluyor
  (`acadrust src/io/dxf/writer/mod.rs:99-115`). Sonuç: acadrust DOS857 DWG'yi doğru çözüyor ama
  CadCraft'a DOS857 baytlı bir DXF veriyor, CadCraft bunu 1252 sanıyor. 2000/2004 DWG'lerde de
  bozuk görünüyordu (2007+ DWG'ler UTF-8 DXF'e çevrildiği için doğruydu).

**Köprüdeki çözüm**: başlıktaki kod sayfasını CadCraft bilmiyor ama acadrust biliyorsa, DXF metni
acadrust'ın tablolarıyla (`acadrust::io::dxf::code_page::try_encoding_from_code_page`) çözülüp
UTF-8 olarak motora veriliyor. Bunun için `mo-cad` acadrust'a doğrudan bağlı (`=0.6.6`, Cargo.lock'ta
zaten olan sürüm; yükseltme yok).

**Önerilen upstream yaması**: `decoder`'a DOS857 vb. tabloları eklemek ya da DWG köprüsünde DXF'i
her zaman UTF-8 yazmak (acadrust belgesinin `header.code_page` değerini yazmadan önce `UTF-8`
yapmak ya da çıktıyı R2007+ sürümüyle yazmak).

## Kalan riskler

- 2. maddedeki tespit sezgisel: başlığı `ANSI_1252` olan, çok kısa İzlandaca metinli (ör. tek bir
  "það") bir çizim Türkçe sanılır. Türkiye'den gelen çizimlerde bu olasılık çok düşük.
- Katman adları motorda `eq_ignore_ascii_case` ile eşleştiriliyor (`crates/doc/src/lib.rs:210-211`):
  yalnızca büyük/küçük harfle ayrılan Türkçe adlar ("KAPI" / "Kapı") eşleşmez. AutoCAD DWG'de
  nesneler katmanı tanıtıcıyla (handle) bulduğu için DWG'den gelen çizimlerde adlar zaten aynı
  yazılıyor; elle yazılmış DXF'lerde sorun olabilir. Köprüde düzeltilmedi.
- Yedek yazı tipi Latin/Yunan/Kiril harflerini kapsıyor; Çince/Japonca/Arapça metin telefonda hâlâ
  kutu olabilir (o yazı tipleri büyük, belleğe yük; ayrı iş).
- İkili (binary) DXF dosyalarına 2. ve 3. madde uygulanmıyor (nadir).
