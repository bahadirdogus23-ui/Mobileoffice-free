# Erişilebilirlik denetimi (Faz 5)

*13 Kasım 2026 · kapsam: `android/app` Compose arayüzü · plan: [RAPOR-VE-PLAN.md §7.9](RAPOR-VE-PLAN.md)*

Amaç: uygulama TalkBack ile ve büyük yazı boyutunda (%200) kullanılabilsin; gören kullanıcı için görünüm değişmesin.
Bu denetim kodu okuyarak yapıldı; cihazda TalkBack ile elle deneme ve Accessibility Scanner taraması hâlâ yapılmalı.

## Neye bakıldı

| Konu | Nasıl |
|---|---|
| Yalnızca simgeli düğmeler (`Icon`, `IconButton`, "✕ ▲ ▼ ↶ ↷ A− A+ ◀ ▶ ‹ ›" gibi işaretli `TextButton`'lar) | Hepsi tarandı; her birinin okunacak bir adı (`contentDescription`, `Modifier.described(...)`) var mı |
| `contentDescription = null` | Her biri: süs mü (yanında yazı var) yoksa anlam taşıyor mu |
| Küçük dokunma alanları | `Modifier.clickable` verilmiş küçük kutular, renk örnekleri, seçim tutamakları |
| Açık/kapalı durumu | Kalın/İtalik araçları, sekme benzeri seçiciler, `Row + Checkbox/Switch/RadioButton` satırları |
| Çizilen belge yüzeyleri | Tablo ızgarası, PDF/Word/slayt sayfaları, sunum, CAD tuvali, tarama fotoğrafı, imza alanı |
| Büyük yazıda taşma | Sabit boyutlu kutulardaki `.sp` yazılar, tek satırlık başlık ve etiketler |

## Ne değişti

**Okunan adlar (TalkBack)**

- Arama çubuklarındaki ▲ / ▼ düğmeleri (PDF/Word/slayt, metin, tablo): "Önceki eşleşme" / "Sonraki eşleşme"; tablo aramasındaki ✕: "Kapat".
- A− / A+ düğmeleri (PDF metni, akan metin, metin düzenleyici, tablo biçimi, slayt yazısı): "Yazıyı küçült" / "Yazıyı büyüt".
- Slayt düzenleyicideki ↶ / ↷: "Geri al" / "Yinele"; sunumdaki ◀ / ▶ ve taramadaki ‹ / ›: "Önceki" / "Sonraki".
- PDF'e yazı eklerken − / +: "Boyutu küçült" / "Boyutu büyüt".
- Ana ekran: ⚙ düğmesi "Ayarlar", arama kutusundaki ✕ "Aramayı temizle", klasör satırındaki ✕ "Kaldır"; resimlerden PDF'teki ✕ "Kaldır".
- Renk örnekleri (PDF yazısı, Word yazı/vurgu, tablo yazı rengi, slayt renkleri): "Renk 1", "Renk 2"…; tablo stilleri: "Tablo stili 1"…
- Kayıtlı imzalar: "Kayıtlı imza 1"…; çizim alanı: "İmza alanı: parmağınızla çizin".
- Süs işaretleri (araç çubuğu simgeleri, hızlı işlem kutucukları, ayar satırı simgeleri, ☕, ›, 🔍, ＋, ▾) okunmuyor
  (`Modifier.decorative()`): TalkBack yalnızca yanındaki yazıyı söyler.

**Çizilen yüzeyler**

- Tablo ızgarası: "Tablo, B3 hücresi: <içerik>" (seçili hücre ve içeriği).
- PDF/Word sayfaları: "Sayfa 3 / 10"; slaytlar: "Slayt 3 / 10" — sayfa daha çizilmeden de okunur.
- Sunum: "Slayt 3 / 10"; tarama küçük resimleri: "Sayfa 1 / 3"; köşe düzenleyici: "Sayfa fotoğrafı: köşelerini kâğıdın köşelerine sürükleyin".
- CAD tuvali: "Çizim: <dosya adı>".

**Belgeyi okuma (yalnızca TalkBack açıkken)**

Aşağıdakiler yalnızca dokunarak keşfetme (TalkBack) açıkken devreye girer (`rememberScreenReaderOn`,
`AccessibilityManager.isTouchExplorationEnabled`, açılıp kapanması anında izlenir). TalkBack kapalıyken hiçbir metin
yüklenmez, ek düğüm oluşturulmaz; görünüm ve hız değişmez.

- **Sayfa metni**: PDF, Word (EPUB, EML, MHT gibi Word görünümünde açılanlar dahil) ve slayt sayfaları ekrana geldiğinde
  metinleri motor iş parçacığında okunur ve sayfanın semantiğine `text` olarak verilir: "Sayfa 3 / 10. <sayfanın metni>".
  `contentDescription` yerine `text` kullanıldığı için TalkBack metni karakter, kelime ve paragraf adımlarıyla da
  gezebilir. Metin yüklenene kadar eskisi gibi "Sayfa 3 / 10" okunur; yazısız sayfada "bu sayfada okunacak yazı yok".
  Kaynaklar: PDF'te sayfanın kendi metni (`page_text`), slaytta metin kutuları yukarıdan aşağı, soldan sağa (`text_shapes`),
  Word'de yalnızca o sayfanın metni (`page_text`; bir paragraf başladığı sayfada okunur, paragrafın sayfası satırları
  ölçülmeden yerleşimin dizininden bulunur — tüm sayfaların metni toplanmaz). Son 24 sayfanın metni bellekte tutulur,
  sayfa başına en çok 12.000 karakter; sayfa taşıma/silme ya da yeni belge önbelleği temizler ve metin hemen yeniden okunur.
  **Yazarken**: her düzenleme önbelleği geçersiz kılar, ama ekrandaki sayfanın metni ancak yazmaya ~700 ms ara
  verildiğinde yeniden okunur (her tuş bekleme süresini baştan başlatır); o arada eski metin okunur, motor iş parçacığı
  yazmaya ayrılır. Yeni ekrana gelen sayfa beklemeden okunur.
  PDF'te metin PdfCraft motorundan gelir: 40 MB'a kadar olan (ve şifreli) dosyalarda motor açılışta zaten açılmış
  olduğundan ek okuma yoktur; 40 MB'tan büyük dosyada ilk sayfa metni (aramada olduğu gibi) dosyanın tamamını belleğe
  okuyup motoru açar. Android'in `PdfRenderer`'ı metin vermiyor (metin API'si yalnızca yeni sürümlerde), bu yüzden
  daha ucuz bir yol yok.
- **Tablo hücreleri**: ekrandaki her hücrenin üstüne görünmez bir erişilebilirlik düğümü konur ("B3: 1.250,00", boşsa
  "B3: boş"); birleştirilmiş aralık tek düğümdür. TalkBack ile hücreden hücreye kaydırarak gezilir; çift dokunma hücreyi
  seçer (seçili hücrede ikinci çift dokunma, parmakla dokunmada olduğu gibi yazmayı açar). Izgara `CollectionInfo`,
  hücreler `CollectionItemInfo` (satır/sütun) taşır; seçili hücre "seçili" okunur. TalkBack'in kaydırma hareketleri
  tabloyu kaydırır (`scrollBy`), yeni görünen hücreler düğüm olur. Uzaklaştırılmış görünümde en çok 300 hücre.
  Görünen satır/sütun aralığı `derivedStateOf` ile izlenir: düğümler yalnızca aralık değişince (yeni bir satır ya da
  sütun ekrana girince) yeniden oluşturulur; konumları ve boyutları yerleşim aşamasında (`Modifier.layout`) kaydırmadan
  okunur, yani her kaydırma karesinde yeniden oluşturma (recomposition) olmaz.

**Durum bildirme**

- Word/tablo araç çubuğu (`Tool`): düğme rolü ve `selected` durumu — Kalın/İtalik/Altı çizili açıkken "seçili" okunur.
- Sayfa küçük resimleri: seçim modunda işaretli sayfalar, normalde ekrandaki sayfa "seçili".
- Görünüm ayarları: açık/koyu ve köşe seçicisi sekme (`selectable`, `Role.Tab`), tema örnekleri radyo düğmesi gibi.
- Onay kutusu / anahtar satırları tek öğe oldu (`toggleable`, iç denetim `onCheckedChange = null`): tablo süzgeci,
  karartma türleri, form onay kutuları, imzayı sakla, CAD katmanları, okuma ayarları, Ayarlar'daki anahtarlar,
  PDF sayfa ayarı, metni kaydır, tema arka plan tonu. Resimlerden PDF'teki A4/kendi boyutu seçimi `selectable` radyo.
  Böylece TalkBack satırı tek seferde "Gece modu, anahtar, açık" diye okur; eskiden anahtar etiketsiz ayrı odaklanıyordu.
- Renk örnekleri `selectable` / düğme rolüyle işaretlendi (PDF yazı renginde seçili olan "seçili" okunur).

**Dokunma alanı ve büyük yazı**

- Ana ekrandaki 40 dp ⚙ düğmesine `minimumInteractiveComponentSize()` (görünen daire aynı, dokunma alanı 48 dp).
- Sabit kutulardaki simgeler (araç çubuğu, hızlı işlemler, ayar satırları, ⚙) ve dosya türü rozetleri ("DOCX")
  yazı boyutuyla büyümüyor; eskiden %200'de kutudan taşıp kırpılıyordu. Uzantı satırdaki bilgi yazısında kişinin boyutuyla zaten yazıyor.
- Tek satırlık başlıklar (tablo, PDF/Word/slayt, CAD) ve araç etiketleri, tema adları, süzgeç değerleri sonda "…" ile kısalıyor.

Yeni metinler `values/strings.xml` ve `values-tr/strings.xml` dosyalarına `a11y_*` adlarıyla eklendi.

## Kalanlar

- **Sayfa metni ve tablo hücreleri cihazda denenmedi.** Yukarıdaki okuma kod üzerinden yapıldı; TalkBack ile PDF,
  Word, slayt ve tabloda gezinme cihazda doğrulanmalı. Satır adımıyla (line) okuma yok: sayfa düğümü bir metin
  yerleşimi (`TextLayoutResult`) vermiyor; karakter/kelime/paragraf adımları var. Taranmış PDF sayfası, OCR ile
  aranabilir yapılmadıkça "yazı yok" okunur. Sunum ekranı ve sayfa küçük resimleri hâlâ yalnızca numara söyler.
  Tabloda yalnızca ekrandaki hücreler gezilir; ekran dışına geçmek için TalkBack kaydırma hareketi gerekir.
- **Word düzenleme imleci**: yazma 1 dp'lik görünmez bir metin alanından yapılıyor; TalkBack ile düzenleme denenmedi.
- **Küçük dokunma alanları bırakıldı** (düzeni bozacağı için): renk örnekleri (28–32 dp, sıra halinde), tablo stili
  kutucukları (44×40 dp), arama kutusundaki ✕, Görünüm ayarlarındaki bölümlü seçici (~34 dp yükseklik), PDF metin seçimi
  tutamakları (22 dp; Compose dokunmayı 48 dp'ye kendisi genişletir ama yerleşim alanı ayrılmadı), nesne çerçevesi tutamakları.
- **Sürükleme ile yapılan işler** (sayfa sırası, nesne taşıma/boyutlama, tarama köşeleri, imza çizimi) için TalkBack'e
  özel eylem yok; sayfa sırası için ▲/▼ düğmeleri olan yerler (PDF birleştirme, slaytlar) dışında erişilemez.
- **Ayarlar başlıkları** (`ThemePicker` içindeki `Label`) Türkçe büyük harfe çevriliyor (`Locale.forLanguageTag("tr")`);
  İngilizce arayüzde "i" harfi "İ" olur. Erişilebilirlikten çok yerelleştirme hatası; ayrıca düzeltilmeli.
- Cihazda doğrulama: TalkBack ile ana akışlar (aç, ara, düzenle, kaydet, PDF imzala, tara) ve %200 yazı + "Ekran
  boyutu: en büyük" ile her ekranın bir kez gezilmesi; Accessibility Scanner raporu.
