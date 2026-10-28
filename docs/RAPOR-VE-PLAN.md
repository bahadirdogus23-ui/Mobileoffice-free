# MobileOffice Free: Kapsamlı Rapor ve Proje Planı

**Android için ücretsiz, reklamsız, hesapsız ve internetsiz çalışan ofis ve belge uygulaması**

| | |
|---|---|
| Tarih | 28 Ekim 2026 |
| Sürüm | 1.0 (ilk plan) |
| Durum | Planlama. Karar bekleyen konular: [§16](#16-karar-bekleyen-konular) |
| Çalışma adı | *MobileOffice Free* (depo adı; nihai isim önerileri §16'da) |
| Platform | Android telefon, tablet, katlanabilir cihazlar ve ChromeOS |

---

## İçindekiler

1. [Yönetici özeti](#1-yönetici-özeti)
2. [Bu rapor nasıl hazırlandı](#2-bu-rapor-nasıl-hazırlandı)
3. [Referans analizi: ArtCraft "Crafting Apps"](#3-referans-analizi-artcraft-crafting-apps)
4. [Pazar, kullanıcılar ve konumlandırma](#4-pazar-kullanıcılar-ve-konumlandırma)
5. [Ürün vizyonu, ilkeler ve hedefler](#5-ürün-vizyonu-ilkeler-ve-hedefler)
6. [Kapsam: dosya türü destek matrisi](#6-kapsam-dosya-türü-destek-matrisi)
7. [Özellik planı (modül modül)](#7-özellik-planı-modül-modül)
8. [Kullanıcı deneyimi tasarımı](#8-kullanıcı-deneyimi-tasarımı)
9. [Teknik mimari](#9-teknik-mimari)
10. [Güvenlik ve gizlilik](#10-güvenlik-ve-gizlilik)
11. [Lisans, yasal uyum ve marka](#11-lisans-yasal-uyum-ve-marka)
12. [Kalite güvencesi ve test](#12-kalite-güvencesi-ve-test)
13. [Yol haritası ve takvim](#13-yol-haritası-ve-takvim)
14. [Ekip, maliyet ve sürdürülebilirlik](#14-ekip-maliyet-ve-sürdürülebilirlik)
15. [Riskler ve azaltma planı](#15-riskler-ve-azaltma-planı)
16. [Karar bekleyen konular](#16-karar-bekleyen-konular)
17. [Sonuç ve hemen atılacak adımlar](#17-sonuç-ve-hemen-atılacak-adımlar)

Ekler: [A: Android MIME türleri](#ek-a-android-mime-türleri) · [B: Köprü API taslağı](#ek-b-köprü-api-taslağı) · [C: Sözlük](#ek-c-sözlük) · [D: Kaynaklar](#ek-d-kaynaklar)

---

## 1. Yönetici özeti

**Ne yapıyoruz?** Android telefon ve tabletler için tek bir uygulama:

- Word, Excel, PowerPoint ve PDF dosyalarını **açar ve düzenler**,
- DWG/DXF çizimlerini **görüntüler** (istendiği gibi yalnızca görüntüleme),
- Günlük hayatta en sık karşılaşılan diğer dosyaları (görsel, metin, e-kitap, arşiv, UYAP UDF, e-Fatura XML…) tek yerden açar,
- **Tamamen ücretsizdir, reklam göstermez, hesap istemez ve internetsiz çalışır.**

**En önemli bulgu.** Örnek verilen beş uygulama (WordCraft, GridCraft, DeckCraft, PdfCraft, CADCraft) ArtCraft ekibinin **açık kaynaklı (MIT veya Apache-2.0 lisanslı)** projeleridir. Hepsi saf Rust ile yazılmıştır. Belge motorları arayüzden bilinçli olarak ayrılmıştır ("motor önce" mimarisi). **Hiçbirinin mobil sürümü yoktur.** GridCraft kendi değerlendirmesinde mobil/dokunmatik arayüzü açıkça "eksik" olarak listeler.

Bu, bizim için büyük bir fırsattır. Yıllar sürecek dosya okuma, sayfa düzeni ve hesaplama motorlarını sıfırdan yazmak yerine bu motorları, lisansın izin verdiği şekilde, Android'e taşıyabiliriz. Üzerine de **mobil için baştan tasarlanmış bir arayüz** kurarız.

**Önerilen yaklaşım**

| Katman | Seçim |
|---|---|
| Arayüz | Kotlin + Jetpack Compose (Material 3): dokunmatik, erişilebilir, Türkçe öncelikli |
| Belge motorları | ArtCraft Rust motorları, tek paylaşılan kütüphane (`.so`); Kotlin köprüsü UniFFI/JNI |
| PDF görüntüleme | Hız için PDFium (Chrome'un ve Android'in kendi PDF motoru); düzenleme, form, imza ve sayfa işlemleri PdfCraft motoruyla |
| DWG/DXF | CADCraft motoru, yalnız görüntüleme |

**Neden onlardan daha iyi olacak?**

1. Beş ayrı masaüstü uygulamasının işini **tek bir mobil uygulamada** ve bir dosya merkezinde topluyor.
2. Masaüstü şeridi (ribbon) yerine **dokunmatik ve tek elle kullanılabilir** bir tasarım sunuyor.
3. **Türkiye'ye özel özellikler** içeriyor: UYAP UDF, e-Fatura XML, Türkçe Excel fonksiyon adları (TOPLA, EĞER, DÜŞEYARA), Türkçe OCR, TC Kimlik No/IBAN karartma ve bozuk Türkçe karakterleri (Windows-1254) düzeltme.
4. **İnternet izni bile istemiyor**, bu yüzden gizlilik iddiası doğrulanabilir.
5. **Android ile bütünleşiyor**: "Birlikte aç", paylaş, yazdır, kamerayla belge tarama, sesli okuma ve bulut uygulamalarındaki dosyalar.
6. **Herkese hizmet ediyor**: düşük donanımlı telefonlar, büyük yazı, TalkBack ve sesli okuma.

**Takvim (tahmini).** Varsayım: 1–2 geliştirici + yapay zekâ destekli geliştirme.

| Adım | Ne zaman |
|---|---|
| Faz 0: teknik doğrulama | 3 hafta |
| MVP: evrensel görüntüleyici | ~3. ay |
| PDF araçları + Türkiye paketiyle açık beta | ~5. ay |
| Office düzenleme | ~8. ay |
| **v1.0** | **~11. ay** |

Faz 0 sonunda gerçek ölçümlerle yeniden tahmin yapılacak.

**Maliyet.** Sunucu maliyeti yoktur; uygulama tamamen cihazda çalışır. Zorunlu tek ödeme Google Play geliştirici hesabıdır (25 USD, bir kez). Asıl maliyet geliştirme emeğidir.

**En büyük riskler:**

- Upstream motorlar henüz **alfa** aşamasında (%50–60 "gerçek işe hazır"); CADCraft ise ön-alfa.
- Mobil işlemcide çizim (render) hızı.
- Uygulama boyutu.
- Çok hızlı değişen upstream kodu.

Azaltma planı [§15](#15-riskler-ve-azaltma-planı)'te.

**Hemen sonraki adım:** Faz 0. Motorları Android'e derleyip her formattan bir dosyayı gerçek bir telefonda göstermek, boyut, hız ve belleği ölçmek ve §16'daki kararları vermek.

---

## 2. Bu rapor nasıl hazırlandı

- Verilen getartcraft.com sayfalarına bu çalışma ortamından erişilemedi. Bu yüzden sitenin bağlantı verdiği **resmi GitHub depoları** (storytold/wordcraft, gridcraft, deckcraft, pdfcraft, cadcraft) indirildi ve incelendi:
  - `README.md`, `ROADMAP.md`, `docs/architecture.md`, `docs/gaps.md`, `docs/file-format-parity.md` ve `Cargo.toml` dosyaları okundu.
  - Son commit'ler 27 Ekim 2026 tarihlidir (Ek D).
- Bilgiler basın haberleriyle çapraz kontrol edildi.
- Google Play ve Android gereksinimleri Google'ın resmi sayfalarından doğrulandı.
- "Tahmin" ve "doğrulanmalı" etiketli bilgiler ölçülmemiş varsayımlardır; Faz 0'da kontrol edilecek.
- Upstream projeler çok hızlı ilerliyor (günde onlarca birleştirme). Buradaki yüzde ve sayılar 27 Ekim 2026 itibarıyladır.

---

## 3. Referans analizi: ArtCraft "Crafting Apps"

### 3.1 Genel tablo

- **Geliştirici:** ArtCraft ekibi (kurucusu Brandon Thomas) ve Discord topluluğu. Aynı ailede PhotoCraft, VectorCraft, FilmCraft gibi yaratıcı araçlar da var.
- **Teknoloji:** Saf Rust (edition 2024, Rust ≥ 1.95). Arayüz egui ile GPU üzerinde (wgpu) çiziliyor. Masaüstü (macOS, Windows, Linux, FreeBSD) ve web (WebAssembly) sürümleri var. **Android/iOS sürümü yok.**
- **Lisans:** Kod MIT **veya** Apache-2.0. "ArtCraft" adı ve logoları tescilli marka; fork'lar bunları kaldırmak zorunda.
- **Geliştirme yöntemi:** Yol haritaları kalan işi "yapay zekâ ajanı saati" cinsinden ölçüyor; geliştirmenin büyük kısmı yapay zekâ kodlama ajanlarıyla yapılıyor. Buna rağmen ciddi kalite kapıları var: biçim denetimi, clippy, yüzlerce test, katman kontrolü, WebAssembly derleme kontrolü, varlık lisans denetimi, fuzzing ve gerçek dosya korpusu taramaları.
- **Ortak mimari ilkeleri:**
  1. **Motor önce:** Arayüz değiştirilebilir bir katmandır. Arayüz katmanının altındaki hiçbir crate egui, winit veya wgpu'ya bağımlı değildir. Motor crate'lerinin WebAssembly'ye derlenebildiği CI'da denetlenir (yalnız CADCraft'ın DWG köprüsü masaüstüne özeldir). Bu, motorların Android'e taşınabilir olduğuna dair güçlü bir işarettir.
  2. **Her eylem bir komuttur:** Her işlemin bir kimliği ve JSON parametreleri vardır (ör. `home.bold`, `format.bold`, `slide.new`). Şerit, kısayollar, komut satırı aracı ve MCP sunucusu aynı komutları çalıştırır.
  3. **Asla çökme:** Komutlar panik yakalayıcı ile korunur; bir hata olursa belge önceki hâline döner.
  4. **Temiz oda (clean-room):** Kod başkalarının kodundan değil, açık standartlardan ve gözlemden yazılır.
  5. **Gizlilik:** Hesap yok, telemetri yok, tamamen çevrimdışı.

### 3.2 Uygulama bazında özet

| Uygulama | Hedef aldığı | Aşama ve hazırlık (upstream tahmini) | Dosya desteği | Öne çıkan özellikler |
|---|---|---|---|---|
| **WordCraft** | Microsoft Word | Alfa, v0.4.0 (10.10.2026). Gerçek işe hazır ~%60, temel kullanıcı için ~%63 | DOCX/DOCM/DOTX/DOTM okuma-yazma; DOC (97–2003) okuma (kısmi); ODT, RTF, HTML, Markdown, LaTeX, TXT içe/dışa aktarma; seçilebilir metinli PDF dışa aktarma | Stiller, listeler, tablolar, üstbilgi/altbilgi, resim/şekil ve metin kaydırma, içindekiler, dipnot, kaynakça (APA/MLA/Chicago/IEEE), değişiklik izleme, yorumlar, belge karşılaştırma, posta birleştirme, denklemler, mürekkep, yazım denetimi (Türkçe Hunspell sözlüğüyle), okuma modu. 400+ komut. **Tam Türkçe arayüz kataloğu (2.325 girdi).** 188 sayfalık belgede yazarken yeniden düzen ~1,4 ms |
| **GridCraft** | Microsoft Excel | Alfa. ~%50 (temel kullanıcı ~%63) | XLSX/XLSM/XLTX okuma-yazma (makrolar saklanır, çalıştırılmaz); CSV/TSV; XLS (değer, formül, sayı biçimi; hücre biçimi yok); XLSB ve ODS (yalnız değerler); PDF | 513 fonksiyon (dinamik diziler, LET, LAMBDA…), bağımlılık grafiğiyle hesaplama, tablolar, koşullu biçimlendirme, veri doğrulama, 17 grafik türü, mini grafikler, pivot tablolar, sıralama/filtre, Hızlı Doldurma, yazdırma |
| **DeckCraft** | PowerPoint | Alfa. ~%55 (temel kullanıcı ~%62) | PPTX/POTX/PPSX okuma-yazma; **PPT ve ODP yok**; PDF, PNG/JPEG, animasyonlu GIF; anahat (TXT/RTF/MD) | Asıl slaytlar ve düzenler, 8 tema, 150+ şekil, bağlayıcılar, efektler, tablo/grafik/SmartArt (temel), ses ve video (saf Rust kodekler), Morph dahil geçişler, animasyonlar, sunucu görünümü, kalem, otomatik kurtarma. 320 komut |
| **PdfCraft** | Adobe Acrobat Pro | Alfa. ~%50 | PDF (tüm sürümler, hasarlı dosya onarımı); FDF/XFDF form verisi; PNG/JPEG/TIFF, metin, RTF, Word, HTML dışa aktarma; görselden PDF | Döşemeli render, arama, okuma sırasına uygun metin seçimi, yer imleri, sayfa düzenleme, birleştir/ayır/çıkar, tam şifreleme desteği (40-bit RC4'ten AES-256'ya), formlar (AcroForm + XFA, korumalı JavaScript), yorumlar, katmanlar, ekler, doldur ve imzala (fotoğraftan imza tarama dahil), karartma, dijital imza, OCR, sıkıştırma, karşılaştırma, ölçüm. Kayıtlar artımlı ve atomik. pdf.js test korpusundaki 983 dosyada 0 çökme |
| **CADCraft** | AutoCAD | **Ön-alfa.** ~%38 (temel kullanıcı ~%46) | DXF R12–2018 okuma (ASCII/ikili), R2000 yazma; DWG R13–2018 okuma (MPL-2.0 lisanslı `acadrust` kütüphanesiyle); PDF çizim çıktısı, SVG, PNG | Komut satırı, nesne yakalama, katmanlar, ölçülendirme, tarama (hatch), bloklar, düzenler ve görünüm pencereleri, parametrik kısıtlar. Türkçe dahil 15 arayüz dili |

### 3.3 Onlardan alacaklarımız (güçlü yanlar)

- **Hazır, test edilmiş motorlar:** DOCX/XLSX/PPTX/PDF/DXF/DWG okuma-yazma, sayfa düzeni, formül hesaplama ve render.
- **"Her eylem bir komut" mimarisi:** Mobil arayüzün motorla tek bir dar kapıdan konuşmasını sağlar (`execute(komut, json)`). Menüler ve komut arama, komut kataloğundan otomatik üretilebilir.
- **Asla çökme ve veri kaybetmeme kültürü:** Panik yakalama, artımlı ve atomik kaydetme, otomatik kurtarma.
- **Türkçe hazırlığı:** WordCraft ve CADCraft'ta Türkçe arayüz var; WordCraft Türkçe Hunspell sözlüklerini (ISO 8859-9 dahil) okuyabiliyor.
- **Gizlilik ve çevrimdışılık:** Bizim ilkelerimizle birebir örtüşüyor.
- **Kalite süreçleri:** Korpus taramaları, fuzzing ve lisans denetimi bizim CI'ımıza örnek olabilir.

### 3.4 Onları geçeceğimiz noktalar (boşluklar ve fırsatlar)

| Boşluk (upstream'in kendi belgelerinden) | Bizim çözümümüz |
|---|---|
| Mobil/dokunmatik arayüz yok | Mobil için baştan tasarlanmış Compose arayüzü |
| Beş ayrı masaüstü uygulaması | Tek uygulama, ortak dosya merkezi ve araçlar sekmesi |
| DeckCraft `.ppt`/`.odp` açamıyor | Rust ile PPT ve ODP içe aktarıcı (upstream'e katkı), Faz 4 |
| GridCraft `.xls` biçimlendirmesini ve ODS formüllerini okumuyor | İçe aktarıcıları geliştirme (katkı), Faz 4 |
| GridCraft CSV kodlamada Windows-1252'ye düşüyor; Türkçe "ğ, ı, ş, İ" harfleri "ð, ý, þ, Ý" olarak bozuluyor | Windows-1254/ISO-8859-9 algılama (katkı), Faz 1 |
| PdfCraft OCR yalnız **aksansız Latin** okuyor (ç, ğ, ı, ö, ş, ü yok) | Türkçe OCR (Tesseract `tur` modeli veya model eğitimi), Faz 2 |
| PdfCraft render'ı ödünç alınmış (hayro); hız ve doğruluk ölçülmemiş | Görüntülemede PDFium; PdfCraft düzenleme için |
| GridCraft kaydederken modellemediği içeriği düşürebiliyor; büyük çalışma kitapları açılmıyor | "Kayıp uyarısı", varsayılan "kopya olarak kaydet", büyük dosyada salt okunur hızlı görünüm |
| DeckCraft'ta SVG resimler çizilmiyor, yazdırma yok | Android tarafında SVG çizimi ve Android yazdırma servisi |
| Kamera ile tarama, "Birlikte aç", paylaşım, sistem yazdırma yok | Android entegrasyonları |
| Türkiye'ye özgü formatlar yok | UDF ve e-Fatura görüntüleyiciler |
| CADCraft DWG kaydederken modellemediği her şeyi düşürüyor | Biz yalnız görüntülüyoruz; bu risk bize yansımıyor |

### 3.5 Lisans ve marka notu

- **Kod kullanılabilir.** MIT/Apache-2.0 kodu telif ve lisans bildirimlerini koruyarak (Apache NOTICE dahil) kullanabilir, değiştirebilir ve dağıtabiliriz.
- **Marka kullanılamaz.** ArtCraft adı ve logoları kullanılamaz. Karışıklık olmaması için "...Craft" biçiminde bir isim de seçmemeliyiz.
- **`acadrust` (MPL-2.0)** dosya düzeyinde copyleft'tir. Değiştirmeden kullanmak serbesttir; değiştirirsek değişen dosyaların kaynağını yayımlarız.
- **Öneri:** ArtCraft topluluğuyla (Discord) erkenden iletişime geçmek ve mobil çalışmayı duyurmak. Motorlarda gereken değişiklikleri upstream'e PR olarak göndermek iki tarafa da yarar.

---

## 4. Pazar, kullanıcılar ve konumlandırma

### 4.1 Hedef kullanıcılar

| Kişi (örnek) | İhtiyaç | Bizim için kritik özellik |
|---|---|---|
| Öğrenci (Ayşe, 20) | Ödev (DOCX/PPTX), PDF ders notu, sunum | Hafif uygulama, şablonlar, PDF'e aktarma, sunum modu, notlara vurgu |
| Öğretmen / kamu çalışanı (Mehmet, 45) | Resmî yazı, dilekçe, çizelge, e-Devlet PDF'leri, ODT dosyaları | DOCX/ODT düzenleme, PDF imzalama, yazdırma |
| Avukat / hukuk bürosu (Zeynep, 35) | UYAP UDF dosyaları, dilekçe, kişisel veri karartma | UDF görüntüleme/PDF, TC Kimlik No karartma, PDF birleştirme |
| Esnaf / muhasebe (Hasan, 50) | e-Fatura XML, Excel tabloları, Türkçe CSV | e-Fatura görüntüleyici, kodlama düzeltme, TL biçimleri |
| Mimar / mühendis / şantiye şefi (Can, 30) | Sahada DWG/DXF görüntüleme, ölçüm | Hızlı DWG açma, katmanlar, ölçüm, PDF paylaşma |
| Yaşlı veya az gören kullanıcı (Fatma, 68) | Gelen belgeyi okumak | Büyük yazı, sesli okuma, sade arayüz |
| Düşük donanımlı telefon kullanıcısı | 2–3 GB RAM, az depolama, kotalı internet | Çevrimdışı çalışma, küçük boyut, hafif mod |

### 4.2 Kullanıcıların yaşadığı başlıca sorunlar

- **Ücret ve reklam:** Reklam bombardımanı, "premium" duvarları, abonelik.
- **Gizlilik:** Zorunlu hesap ve bulut, belgelerin internete gönderilmesi kaygısı.
- **Boyut ve hız:** Büyük ve yavaş uygulamalar.
- **CAD:** DWG açmak için ücretli veya reklamlı uygulamalar.
- **Türkiye'ye özgü sorunlar:** UYAP UDF dosyalarının telefonda açılamaması ve bozuk Türkçe karakterler ("ÅŸ", "ð" gibi).
- **Biçim bozulması:** Düzenleyip kaydedince belgenin bozulması.

### 4.3 Rakip görünümü

Bu genel bir değerlendirmedir; sürümler ve fiyatlar değişebilir.

| Tür | Örnekler | Güçlü yan | Zayıf yan (bizim fırsatımız) |
|---|---|---|---|
| Büyük ofis paketleri | Microsoft 365, Google Dokümanlar/E-Tablolar/Slaytlar | Yüksek uyumluluk, bulutta ortak çalışma | Hesap ve bulut odaklı; bazı özellikler abonelikli; büyük boyut |
| Reklamlı ücretsiz paketler | WPS Office ve benzerleri | Geniş format desteği | Reklam, premium kilitleri, gizlilik soru işaretleri |
| PDF uygulamaları | Adobe Acrobat Reader, Xodo vb. | Güçlü PDF görüntüleme | Düzenleme ve dönüştürme çoğunlukla ücretli |
| Açık kaynak paketler | Collabora Office (LibreOffice motoru) | Ücretsiz, açık kaynak, çok geniş format | Büyük boyut (bir kullanıcı ~100 MB bildirmiş; eski ve doğrulanmamış), masaüstü benzeri arayüz |
| "Tüm belge okuyucu" türü uygulamalar | Play'de çok sayıda | Basit görüntüleme | Yoğun reklam, düzenleme yok, geniş izin talepleri |
| CAD görüntüleyiciler | AutoCAD mobil, DWG FastView vb. | DWG uyumluluğu | Hesap, abonelik veya reklam |

### 4.4 Konumlandırma

> **"Telefonundaki her belgeyi aç, düzenle, imzala, paylaş. Ücretsiz, reklamsız, hesapsız, internetsiz."**

---

## 5. Ürün vizyonu, ilkeler ve hedefler

### 5.1 On ilke

1. **Ücretsiz ve reklamsız, her zaman.** "Premium" ya da kilitli özellik yok.
2. **Hesap yok, internet izni yok.** Manifestte `INTERNET` izni bulunmaz; belgeler cihazdan çıkmaz. Bu iddia herkes tarafından doğrulanabilir.
3. **Çevrimdışı öncelikli.** Her özellik internetsiz çalışır.
4. **Önce görüntüle, sonra düzenle.** Dosya anında görüntüleme modunda açılır; kalem düğmesiyle düzenlemeye geçilir.
5. **Veri kaybı yok.** Otomatik kayıt, çökme kurtarma, orijinali koruma ve "kaydederken kaybolacaklar" uyarısı.
6. **Mobil öncelikli.** Tek elle kullanım; dokunma hedefleri en az 48 dp.
7. **Herkes için.** TalkBack, büyük yazı, yüksek kontrast, sesli okuma ve düşük donanım desteği.
8. **Türkçe birinci sınıf.** Çeviriyle yetinmeyip Türkiye'ye özgü ihtiyaçları karşılar.
9. **Açık kaynak ve şeffaf.** Kaynak kod, yol haritası ve karar kayıtları (ADR) açıktır.
10. **Hafif ve hızlı.** Boyut ve hız bütçeleri CI'da ölçülür.

### 5.2 "Daha iyi" olmanın ölçülebilir hedefleri (v1.0)

| Ölçüt | Hedef | Nasıl ölçülür |
|---|---|---|
| Açılabilen dosya uzantısı | ≥ 40 | Format test matrisi |
| Soğuk başlatma (orta segment telefon) | < 1,5 sn | Macrobenchmark |
| PDF ilk sayfa (50 sayfa, 5 MB) | < 0,5 sn | Benchmark |
| DOCX ilk sayfa (20 sayfa) | < 1,5 sn | Benchmark |
| İndirme boyutu (arm64) | ≤ 60 MB (hedef ≤ 45 MB) | APK Analyzer, CI boyut bütçesi |
| Çökme oranı | < %0,5 (Play'in "kötü davranış" eşiği %1,09) | Android vitals |
| ANR oranı | < %0,2 (Play eşiği %0,47) | Android vitals |
| Erişilebilirlik | Tüm ana akışlar TalkBack ile tamamlanabilir | Elle test + Accessibility Scanner |
| Mağaza puanı | ≥ 4,5 | Play Console |

---

## 6. Kapsam: dosya türü destek matrisi

Gösterim: **F1–F5** = planlanan faz ([§13](#13-yol-haritası-ve-takvim)) · **–** = bilinçli olarak yok · **🔬** = araştırılacak

| Kategori | Uzantılar | Görüntüleme | Düzenleme | Motor / kütüphane | Not |
|---|---|---|---|---|---|
| Kelime işlem | .docx .docm .dotx .dotm | F1 | F3 | WordCraft | Makrolar saklanır, asla çalıştırılmaz |
| | .doc (Word 97–2003) | F1 | F3 (DOCX olarak kaydet) | WordCraft (docbin) | Okuma kısmi (upstream tahmini ~%60) |
| | .odt .rtf | F1 | F3 | WordCraft (formats) | Upstream doğruluğu ~%50; iyileştirme katkısı |
| | .txt .md .html .tex, Word XML | F1 | F1 (metin), F3 (zengin) | WordCraft + metin düzenleyici | |
| Hesap tablosu | .xlsx .xlsm .xltx .xltm | F1 | F3 | GridCraft | VBA saklanır, çalıştırılmaz |
| | .xls (Excel 97–2003) | F1 | F3 (XLSX olarak kaydet) | GridCraft | Değer, formül, sayı biçimi var; hücre biçimi F4'te (katkı) |
| | .xlsb .ods | F1 (yalnız veri) | XLSX olarak kaydet | GridCraft | Formül ve biçim F4'te (katkı) |
| | .csv .tsv | F1 | F1 | GridCraft + kodlama algılama | Türkçe: `;` ayırıcı ve Windows-1254 algılama |
| Sunum | .pptx .ppsx .potx | F1 | F3 | DeckCraft | .ppsx doğrudan sunum modunda açılır |
| | .ppt .pps .pot | F4 | PPTX olarak kaydet | Yeni Rust içe aktarıcı (katkı) | Upstream'de yok |
| | .odp | F4 | PPTX olarak kaydet | Yeni içe aktarıcı (katkı) | Upstream'de yok |
| PDF | .pdf | F1 | F2 | PDFium (görüntüleme) + PdfCraft (düzenleme) | Şifreli PDF, formlar, imza, sayfa işlemleri |
| | .fdf .xfdf | F2 | F2 | PdfCraft | Form ve yorum verisi alışverişi |
| CAD (yalnız görüntüleme) | .dwg (R13–2018) | F1 | – | CADCraft + acadrust | Yazma bilinçli olarak yok |
| | .dxf (R12–2018) | F1 | – | CADCraft | |
| | .stl .obj .gltf/.glb .3mf | 🔬 F5 | – | ör. Filament (Apache-2.0) | İsteğe bağlı 3B önizleme |
| Görsel | .jpg .png .webp .gif .bmp .heic/.heif .avif .tiff .svg .ico | F1 | F2 (kırp, döndür, PDF'e çevir) | Android ImageDecoder + ek kodekler | HEIF Android 9+, AVIF Android 12+ yerleşik |
| Metin ve kod | .txt .log .json .xml .yaml .ini, kod dosyaları | F1 | F1 | Yerleşik düzenleyici | Sözdizimi renklendirme, kodlama seçimi |
| E-kitap | .epub | F4 | – | Readium Kotlin Toolkit (BSD-3) | |
| Arşiv | .zip | F1 | – | Yerleşik | İçindeki belgeyi doğrudan açma |
| | .7z .rar .tar .gz .bz2 .xz | F2 | – | libarchive (BSD) | RAR yalnız okuma |
| E-posta | .eml .msg | F4 | – | Ayrıştırıcı | Ekleri açma |
| Kişi ve takvim | .vcf .ics | F4 | – | Yerleşik | Rehbere/takvime ekleme |
| Türkiye | .udf (UYAP) | F2 | PDF/DOCX'e dönüştürme | Yeni Rust okuyucu → WordCraft belge modeli | Format resmî olarak belgelenmemiş |
| | e-Fatura / e-Arşiv / e-İrsaliye .xml (UBL-TR) | F2 | – | XSLT dönüşümü | PDF'e aktarma |
| Şimdilik kapsam dışı | .pages .numbers .key .vsdx .pub .wpd, medya | – | – | 🔬 F5: isteğe bağlı "Uyumluluk paketi" (LibreOffice çekirdeği) | Medya dosyaları sistem oynatıcıya yönlendirilir |

Toplam: v1.0'da **45'ten fazla uzantı**.

---

## 7. Özellik planı (modül modül)

Öncelikler: **P0** = ilgili fazın olmazsa olmazı · **P1** = v1.0'da olmalı · **P2** = v1.0 sonrası

### 7.1 Ana ekran, dosya yönetimi ve ortak özellikler

| Özellik | Öncelik | Faz |
|---|---|---|
| Ana sayfa: son dosyalar, favoriler, kategori kartları (Belgeler, Tablolar, Sunumlar, PDF, Çizimler, Görseller, Diğer) | P0 | 1 |
| Dosya açma: sistem dosya seçicisi (SAF), "Birlikte aç" ve "Paylaş" ile gelen dosyalar (WhatsApp, e-posta ekleri), Drive/OneDrive/Dropbox/Nextcloud uygulamalarındaki dosyalar | P0 | 1 |
| Seçilen klasörlere kalıcı erişim ve bu klasörleri tarayıp listeleme | P0 | 1 |
| İçerik tanıma: uzantı yanlış olsa bile dosyayı imzasından (magic bytes) tanıma | P0 | 1 |
| Arama: dosya adı (P0); belge içi tam metin araması için yerel dizin (P1) | P0/P1 | 1/4 |
| Yeni oluştur: boş veya şablondan Belge, Tablo, Sunum; ayrıca Tara ve PDF birleştir | P0 | 2–3 |
| Kaydetme: yerinde kaydet (yazma izni varsa), "kopya olarak kaydet", "farklı biçimde kaydet" | P0 | 1–3 |
| Otomatik kayıt, çökme kurtarma, ilk düzenlemede orijinalin yedeği | P0 | 2–3 |
| **Kayıp uyarısı:** motorun desteklemediği öğeler kaydederken kaybolacaksa önceden uyarı | P0 | 3 |
| Dönüştürme merkezi: Office→PDF, PDF→görsel/metin/DOCX, görseller→PDF, DWG/DXF→PDF/PNG, UDF→PDF/DOCX, CSV↔XLSX | P0/P1 | 2–4 |
| Yazdırma (Android yazdırma servisi) ve paylaşma (orijinal ya da PDF olarak) | P0 | 1 |
| Bul / değiştir | P0 | 1 (bul), 3 (değiştir) |
| Karanlık mod, okuma modu, sepya, tam ekran | P0 | 1 |
| Sesli okuma (TTS, Türkçe ses) | P1 | 2 |
| Çoklu belge: son belgeler arasında hızlı geçiş, tablette yan yana | P1 | 4 |
| Uygulama kilidi (biyometrik), gizli mod (geçmiş tutmaz), ekran görüntüsü engeli seçeneği | P1 | 4 |
| Uygulama kısayolları ("Tara", "Son PDF"), ana ekran widget'ı | P2 | 4–5 |

### 7.2 PDF modülü (PDFium ile görüntüleme, PdfCraft ile düzenleme)

| Grup | Özellikler | Öncelik | Faz |
|---|---|---|---|
| Görüntüleme | Döşemeli hızlı render, sürekli/tek/çift sayfa, küçük resimler, yer imleri, sayfa etiketleri, bağlantılar, arama, metin seçme ve kopyalama, gece modu, şifreli PDF | P0 | 1 |
| Okuma kolaylığı | Yeniden akış ("mobil okuma") modu, sesli okuma, kalınan sayfayı hatırlama | P1 | 2 |
| Açıklama | Vurgula, altını çiz, üstünü çiz, kalem, silgi, yapışkan not, metin kutusu, şekiller (dikdörtgen, elips, çizgi, ok), damga, yorumlara yanıt | P0 | 2 |
| Formlar | AcroForm doldurma (metin, onay kutusu, radyo, liste), formu temizleme, form verisini FDF/XFDF olarak alma-verme | P0 | 2 |
| İmza | Parmakla imza çizme, görselden veya kâğıttaki imzanın fotoğrafından imza, baş harf, tarih; kayıtlı imzalar | P0 | 2 |
| Sayfa işlemleri | Sürükle-bırak sıralama, döndürme, silme, çoğaltma, boş sayfa ya da başka PDF/görselden sayfa ekleme, sayfa çıkarma, bölme, birleştirme | P0 | 2 |
| Güvenlik | Parola ekleme/kaldırma (AES-256), izinler, kalıcı karartma (redaksiyon) ve Türkçe kişisel veri kalıpları, meta veri temizleme | P1 | 2 |
| Dönüştürme ve küçültme | PDF→PNG/JPEG/metin/DOCX (temel), görseller→PDF, Office→PDF; "e-posta için küçült" ön ayarı | P1 | 2–3 |
| Tarama ve OCR | Kamerayla çok sayfalı tarama, otomatik kenar bulma, perspektif düzeltme, filtreler, **Türkçe OCR** ile aranabilir PDF | P1 | 2 |
| Gelişmiş | Dijital imza (PKCS#12) ve doğrulama, belge karşılaştırma, ölçüm, PDF/A kontrolü, katmanlar, ekler | P2 | 4+ |

### 7.3 Belge modülü (Word, WordCraft motoru)

| Grup | Özellikler | Öncelik | Faz |
|---|---|---|---|
| Görüntüleme | Sayfa görünümü + **mobil görünüm** (metin ekran genişliğine akar), başlıklarla gezinme, yorumları ve değişiklikleri görme, sözcük sayısı | P0 | 1 |
| Yazma | Türkçe Q/F klavye ve IME uyumu, otomatik düzeltmeyle uyumlu giriş, seçim tutamaçları ve büyüteç, biçimli kes/kopyala/yapıştır, geri al/yinele | P0 | 3 |
| Biçim | Yazı tipi, boyut, kalın/italik/altı çizili/üstü çizili, renk, vurgu, hizalama, girinti, satır aralığı, madde işaretli/numaralı listeler, stiller (Başlık 1–3, Normal…), biçim boyacısı | P0 | 3 |
| Ekleme | Resim (galeri/kamera), tablo (satır/sütun ekleme-silme, birleştirme), köprü, sayfa sonu, özel karakter; temel şekiller | P0/P1 | 3 |
| Sayfa | Kenar boşlukları, yönlendirme, kâğıt boyutu, üstbilgi/altbilgi, sayfa numarası, sütunlar | P1 | 3 |
| Başvurular | İçindekiler oluşturma/güncelleme, dipnot/sonnot | P1 | 3–4 |
| İnceleme | Yorum ekleme/yanıtlama, değişiklik izleme, kabul/ret, Türkçe yazım denetimi | P1 | 3–4 |
| Şablonlar | Dilekçe, özgeçmiş, resmî yazı, toplantı tutanağı, rapor, mektup | P1 | 3 |
| Gelişmiş | Posta birleştirme, kaynakça, belge karşılaştırma, denklem düzenleme, mürekkep | P2 | 5+ |

### 7.4 Tablo modülü (Excel, GridCraft motoru)

| Grup | Özellikler | Öncelik | Faz |
|---|---|---|---|
| Görüntüleme | Sanal ızgara (yalnız görünen hücreler çizilir), sayfa sekmeleri, donmuş bölmeler, grafikler, koşullu biçim, birleştirilmiş hücreler, notlar, yakınlaştırma | P0 | 1 |
| Veri girişi | Hücre düzenleme, sayısal ve **formül klavyesi** (= + − * / ( ) ; : $ ve fonksiyon önerileri), formül çubuğu, otomatik doldurma tutamacı | P0 | 3 |
| Formüller | 500+ fonksiyon; **Türkçe fonksiyon adları ve `;` ayırıcı** (TOPLA, EĞER, DÜŞEYARA…); formül hatalarının açıklaması | P0/P1 | 3 |
| Biçim | Yazı tipi, dolgu, kenarlık, hizalama, metni kaydırma, sayı biçimleri (₺, tarih, yüzde), hücre stilleri | P0 | 3 |
| Yapı | Satır/sütun ekleme-silme-gizleme, sayfa ekleme/silme/yeniden adlandırma, satır yüksekliği ve sütun genişliği | P0 | 3 |
| Veri | Sıralama, filtre, bul/değiştir, veri doğrulama, koşullu biçimlendirme | P1 | 3 |
| Grafikler | Grafik ekleme ve düzenleme (sütun, çubuk, çizgi, pasta…), mini grafikler | P1 | 3–4 |
| Pivot | Görüntüleme ve yenileme (P1); oluşturma (P2) | P1/P2 | 4 |
| Yazdırma | Yazdırma alanı, sayfaya sığdırma, PDF'e aktarma | P1 | 3 |

### 7.5 Sunum modülü (PowerPoint, DeckCraft motoru)

| Grup | Özellikler | Öncelik | Faz |
|---|---|---|---|
| Görüntüleme | Slayt görünümü, slayt sıralayıcı (ızgara), konuşmacı notları, geçiş ve animasyonlarla tam ekran sunum | P0 | 1 |
| Sunum modu | Dokunarak veya kaydırarak ilerleme, lazer işaretçi, kalemle çizim, süre sayacı; harici veya kablosuz ekranda sunucu görünümü | P0/P1 | 1/3 |
| Düzenleme | Metin kutularını düzenleme, slayt ekleme/çoğaltma/silme/sıralama, düzen ve tema seçimi, resim ve şekil ekleme, taşıma/boyutlandırma/döndürme tutamaçları, not düzenleme | P0 | 3 |
| Zengin içerik | Tablolar, temel grafikler, ses/video oynatma (Android Media3), geçiş ve animasyon seçme | P1 | 3–4 |
| Dışa aktarma | PDF, PNG/JPEG, animasyonlu GIF | P1 | 3 |
| Gelişmiş | Asıl slayt düzenleme, SmartArt, Morph | P2 | 5+ |

### 7.6 CAD görüntüleyici (CADCraft motoru, yalnız görüntüleme)

| Grup | Özellikler | Öncelik | Faz |
|---|---|---|---|
| Açma | DWG (R13–2018), DXF (R12–2018) | P0 | 1 |
| Gezinme | Parmakla kaydırma ve yakınlaştırma, "tümünü göster", çift dokunuşla yakınlaştırma | P0 | 1 |
| Katmanlar | Katman listesi, açma/kapama, renkler | P0 | 1 |
| Düzenler | Model alanı ve kâğıt alanı (layout) sekmeleri | P0 | 1 |
| İnceleme | Nesneye dokununca tür, katman ve ölçü bilgisi; çizimde metin arama | P1 | 2 |
| Ölçüm | Uzunluk, alan, açı, koordinat (nesne yakalamalı) | P1 | 2 |
| Görünüm | Siyah/beyaz arka plan, çizgi kalınlığını açma/kapama, tek renkli baskı önizlemesi | P1 | 2 |
| Dışa aktarma | PDF ve PNG; paylaşma | P1 | 2 |
| Bilinen sınırlar | 3B katı modeller (ACIS), SHX yazı tipleri (yerine çizgi yazı tipiyle çizilir), harici referanslar (xref) ve modellenmemiş nesneler | – | – |

### 7.7 Diğer görüntüleyiciler

| Tür | Özellikler | Öncelik | Faz |
|---|---|---|---|
| Görsel | Yakınlaştırma, döndürme, kırpma, PDF'e çevirme, EXIF bilgisi | P0/P1 | 1–2 |
| Metin ve kod | Sözdizimi renklendirme, satır numarası, kodlama seçimi (UTF-8, Windows-1254, ISO-8859-9), büyük dosyada akıcı kaydırma | P0 | 1 |
| Markdown | Önizleme ve bölünmüş ekranda düzenleme | P1 | 2 |
| JSON / XML | Ağaç görünümü, düzenli biçimlendirme | P1 | 2 |
| CSV | Tablo olarak açma; ayırıcı ve kodlamayı otomatik algılama | P0 | 1 |
| Arşiv | İçeriği listeleme, seçili dosyaları çıkarma, içindeki belgeyi doğrudan açma, parolalı ZIP | P0/P1 | 1–2 |
| E-kitap | Sayfalama, yazı boyutu, tema, yer imi, sesli okuma | P1 | 4 |
| E-posta | Başlıklar, gövde, ekleri açma | P2 | 4 |
| Kişi / takvim | Önizleme, rehbere ve takvime ekleme | P2 | 4 |

### 7.8 Türkiye paketi

| Özellik | Açıklama | Öncelik | Faz |
|---|---|---|---|
| **UYAP UDF** | Görüntüleme, PDF/DOCX'e dönüştürme, yazdırma, paylaşma. UDF bir ZIP arşividir; içinde `content.xml`, `documentproperties.xml` ve `sign.sgn` bulunur. Okuyucu, belgeyi WordCraft modeline çevirir; böylece aynı düzen ve PDF/DOCX çıktısı bedavaya gelir | P0 | 2 |
| **e-Fatura / e-Arşiv / e-İrsaliye** | Faturanın içinde gömülü XSLT şablonu varsa onunla görüntüleme, yoksa varsayılan şablon; özet kartı (satıcı, alıcı, tarih, matrah, KDV, toplam); PDF'e aktarma; ZIP içinden açma. Gömülü XSLT'nin hangi UBL alanında taşındığı GİB kılavuzundan doğrulanmalı | P0 | 2 |
| Türkçe karakter kurtarma | Windows-1254/ISO-8859-9 otomatik algılama; bozuk görünen metni ("ð, ý, þ" gibi) tek dokunuşla düzeltme | P0 | 1 |
| Türkçe Excel | Ondalık virgül, `;` argüman ayırıcı, Türkçe fonksiyon adları, TL biçimleri. Dosyaya her zaman standart İngilizce adlarla yazılır | P1 | 3 |
| Türkçe yazım denetimi | Hunspell `tr_TR` sözlük paketi (lisansı ayrıca kontrol edilecek) + Android'in sistem yazım denetleyicisi | P1 | 3 |
| Türkçe OCR | ç, ğ, ı, İ, ö, ş, ü harflerini tanıyan OCR (Tesseract `tur` modeli; Apache-2.0) | P1 | 2 |
| KVKK dostu karartma | TC Kimlik No (algoritmayla doğrulamalı), VKN, IBAN (TR, mod-97), cep telefonu (+90 5xx), e-posta | P1 | 2 |
| Türkçe şablonlar | Dilekçe, özgeçmiş, resmî yazı, tutanak, teklif, proforma fatura, bütçe, ders planı | P1 | 3 |
| e-İmza bilgisi | İmzalı PDF'lerde imza bilgisini gösterme; nitelikli e-imza ile imzalama araştırması | P2 | 5+ |

### 7.9 Erişilebilirlik ve kapsayıcılık

- **TalkBack:** Bütün düğmeler etiketli; belge metni ekran okuyucuya açık; mantıklı odak sırası.
- **Görme:** Sistem yazı ölçeği %200'e kadar desteklenir; yüksek kontrast ve renk körlüğüne uygun renkler kullanılır (WCAG 2.2 AA).
- **Sesli okuma:** PDF, Word ve EPUB için hız ve ses seçimi; okunan cümle vurgulanır.
- **Okunabilirlik:** Kolay okunur yazı tipi seçeneği (ör. Atkinson Hyperlegible, OFL lisanslı).
- **Düşük donanım:** 2 GB RAM'li cihazlarda çalışma hedefi. "Hafif mod" animasyonları kapatır ve düşük çözünürlüklü önizleme kullanır.
- **Tek elle kullanım:** Sık kullanılan kontroller ekranın alt kısmındadır.

### 7.10 Gelecek (v1.0 sonrası, araştırma)

- **Cihaz üstü yapay zekâ:** Özetleme, çeviri, yazım önerisi. Varsayılan kapalı, internetsiz; boyut ve pil etkisi ölçülerek eklenir.
- **Asistan entegrasyonu:** "Her eylem bir komut" mimarisi sayesinde komutlar Android'in sistem asistanlarına açılabilir (Android AppFunctions araştırılacak).
- **iOS sürümü:** Rust motorları ortaktır; SwiftUI + UniFFI Swift bağlamalarıyla yapılabilir.
- **Kapsam dışı:** Gerçek zamanlı ortak düzenleme sunucu gerektirir.

---

## 8. Kullanıcı deneyimi tasarımı

### 8.1 Bilgi mimarisi

```text
Alt gezinme:  [Ana Sayfa]   [Dosyalar]   [Araçlar]   [Ayarlar]        (+) Yeni
                  │              │            │
   Son / Favori / Kategoriler    │     PDF birleştir, böl, sıkıştır, tara,
                                 │     imzala, dönüştür, UDF→PDF, …
              Klasörler (SAF), bulut sağlayıcıları, arama
```

### 8.2 Temel akış

**Aç → (anında) Görüntüle → ✏️ Düzenle → otomatik kayıt → Paylaş / Dışa aktar**

- Dosya her zaman önce hızlı görüntüleme modunda açılır. Düzenleme motoru gerektiğinde arka planda hazırlanır.
- Yazma izni olmayan kaynaklardan (e-posta eki, WhatsApp) gelen dosyalarda "Kopya olarak kaydet" otomatik önerilir.

### 8.3 Düzenleyici kalıpları (tüm modüllerde ortak)

- **Üst çubuk:** geri, dosya adı, geri al/yinele, kayıt durumu ("Kaydedildi"), menü.
- **Bağlama duyarlı alt araç çubuğu:** Metin seçiliyken biçim, hücre seçiliyken hücre, şekil seçiliyken şekil araçları gösterilir.
- **Alt sayfa (bottom sheet) panelleri:** Yazı tipi, paragraf, tablo ve grafik ayarları.
- **Komut arama:** "Ne yapmak istersiniz?" kutusu. Liste motorların komut kataloğundan otomatik üretilir, böylece her özelliğe menüde yer bulunmasa da ulaşılabilir.
- **Uzun basma bağlam menüsü,** seçim tutamaçları ve büyüteç.

### 8.4 Biçime özel tasarım notları

- **Word:** "Mobil görünüm" ile "sayfa görünümü" arasında tek dokunuşla geçiş. Varsayılan mobil görünümdür.
- **Excel:** Formül klavyesi, seçimi genişletme tutamaçları, donmuş başlıklar, altta sayfa sekmeleri, hücre ayrıntı paneli.
- **PowerPoint:** Altta (telefon) ya da yanda (tablet) slayt şeridi; ızgara görünümü; tek dokunuşla sunum.
- **PDF:** Araç seçici (vurgula, kalem, not, imza) ve sayfa düzenleme ızgarası.
- **CAD:** Katman paneli, ölçüm aracı, arka plan rengi düğmesi.

### 8.5 Büyük ekran ve giriş aygıtları

- **Uyarlanabilir düzen:** API 36'yı hedefleyen uygulamalarda Android 16, büyük ekranlarda (en kısa kenar ≥ 600 dp) yönlendirme ve yeniden boyutlandırma kısıtlarını yok sayar. Bu nedenle tüm ekran boyutlarına ve yönlere uyan düzen zorunludur.
- **Tablet:** Liste-ayrıntı (iki bölmeli) düzen.
- **Klavye ve fare:** Klavye kısayolları (Ctrl+S/B/I/Z…), sağ tık menüsü.
- **Kalem:** Kalemle (stylus) PDF'e el yazısı.
- **Harici ekran:** Sunucu görünümü ile harici ekrana sunum.
- **Kenardan kenara ekran ve öngörülü geri hareketi:** Hedef API 36 ile zorunludur; geri işlemleri `OnBackPressedDispatcher` ile yönetilmelidir.

### 8.6 Görsel dil ve dil desteği

- **Tema:** Material 3, dinamik renk (Android 12+); açık, koyu ve AMOLED siyah temalar.
- **Format renkleri:** Belge mavi, Tablo yeşil, Sunum turuncu, PDF kırmızı, Çizim camgöbeği. Microsoft, Adobe veya Autodesk simgeleri taklit edilmez.
- **Diller:** Türkçe ve İngilizce ile başlanır; topluluk çevirileri Weblate ile yapılır. Uygulamaya özel dil seçimi desteklenir (Android 13+).

### 8.7 Hata ve boş durumlar

- **Bozuk dosya:** "Dosya onarılarak açıldı."
- **Şifreli dosya:** Parola sorulur.
- **Desteklenmeyen öğe:** Gri yer tutucu ve kısa bir açıklama gösterilir.
- **Çok büyük dosya:** "Hızlı görüntüleme modu" önerilir.
- **İlk açılış:** Hiçbir izin istenmez. Atlanabilir üç kartlık bir tanıtım gösterilir.

---

## 9. Teknik mimari

### 9.1 Seçeneklerin karşılaştırılması

| Kriter | **A) ArtCraft motorları + Compose (önerilen)** | B) LibreOffice çekirdeği (Collabora tabanlı) | C) Kotlin + Apache POI vb. | D) WebView + JS kütüphaneleri | E) egui uygulamalarını doğrudan Android'e derlemek |
|---|---|---|---|---|---|
| Format kapsamı | Geniş (PPT/ODP sonra) | Çok geniş | Orta | Dar–orta | Geniş |
| Düzenleme derinliği | Yüksek (alfa) | Çok yüksek (olgun) | Düşük (render yok) | Düşük | Yüksek |
| Mobil kullanım deneyimi | Çok iyi (yerel arayüz) | Orta (masaüstü benzeri web arayüzü) | İyi | Orta | Zayıf (masaüstü şerit) |
| Uygulama boyutu | Orta (tahmin 30–60 MB) | Büyük | Küçük–orta | Küçük | Orta |
| Performans | İyi (Faz 0'da ölçülecek) | Orta | Orta | Düşük–orta | İyi |
| Geliştirme süresi | Orta | Yüksek (ağır C++ derlemesi) | Çok yüksek | Düşük (yalnız prototip) | Düşük |
| Lisans | MIT/Apache (+ MPL acadrust) | MPL-2.0 | Apache-2.0 | Karışık | MIT/Apache |
| Bakım riski | Upstream çok hızlı değişiyor | Dev kod tabanı | Tüm yük bizde | Kütüphane kalitesi | Upstream'e tam bağımlı |

**Karar önerisi: A.** B ise ileride nadir formatlar için isteğe bağlı bir "Uyumluluk paketi" olarak araştırılabilir (Faz 5).

### 9.2 Önerilen mimari

```text
┌──────────────────────── Android uygulaması (Kotlin) ────────────────────────┐
│ Arayüz: Jetpack Compose + Material 3                                         │
│   Ana Sayfa · Dosyalar · Araçlar · Ayarlar                                   │
│   Görüntüleyiciler: PDF · Belge · Tablo · Sunum · CAD · Görsel · Metin · …   │
│   Düzenleyici kabukları: araç çubukları, paneller, seçim/tutamaç katmanı     │
│ Uygulama katmanı: ViewModel + Coroutines/Flow, bağımlılık enjeksiyonu (Hilt) │
│ Altyapı: SAF/DocumentFile, Room (son dosyalar, dizin), DataStore,            │
│          WorkManager, Yazdırma, Paylaşım, TTS, CameraX                       │
├──────────────────────── Köprü: UniFFI + ince JNI (render) ──────────────────┤
│ libmobileoffice.so (Rust, tek paylaşılan kütüphane)                          │
│   mo-word  → wordcraft: doc, docx, docbin, formats, layout, render, pdf      │
│   mo-grid  → gridcraft: core, formula, functions, calc, xlsx, chart, engine  │
│   mo-deck  → deckcraft: model, pptx, text, render, anim, pdf, engine         │
│   mo-pdf   → pdfcraft: cos, organize, annot, forms, redact, ocr, engine      │
│   mo-cad   → cadcraft: dxf, dwg (acadrust), doc, render, io, engine          │
│   mo-tr    → UDF okuyucu, e-Fatura, kodlama algılama (Windows-1254)          │
├──────────────────────── Yerel C/C++ ─────────────────────────────────────────┤
│ PDFium (hızlı PDF görüntüleme)   ·   libarchive (7z/rar/tar)                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 9.3 Köprü tasarımı: "her şey komut"

Mobil arayüz motorlarla dar ve kararlı bir API üzerinden konuşur. Taslak: [Ek B](#ek-b-köprü-api-taslağı).

| İşlev | Ne yapar |
|---|---|
| `open(bytes/fd, seçenekler)` | Belgeyi açar, motor oturumu oluşturur |
| `execute(komutKimliği, parametreJson)` | Tüm düzenlemeler komut kimliğiyle yapılır; upstream komut kayıtları doğrudan kullanılır |
| `commands()` | Komut kataloğunu verir; menüler ve komut arama buradan üretilir |
| `inspect(sorgu)` | Seçim, biçim, hücre değeri gibi durumları sorgular |
| `render(...)` | Kopyasız çizim: UniFFI dışında ince bir JNI fonksiyonu Android `Bitmap` belleğine doğrudan çizer |
| `save(biçim)` | Dosya baytlarını ve **kayıp uyarılarını** döndürür |
| `undo()` / `redo()` | Geri al / yinele |

Bu tasarımın avantajı şudur: Upstream yeni bir özellik eklediğinde çoğu zaman köprüye dokunmadan arayüzden çağrılabilir. Kırılma riski azalır.

### 9.4 Çizim (render) hattı, modül bazında

| Modül | Strateji |
|---|---|
| PDF | PDFium ile 512×512 piksel döşemeler; önce düşük çözünürlük, sonra keskin; görünmeyen sayfalar için iptal |
| Word | Motorun CPU rasterleştiricisi (vello_cpu) ile döşemeler → `Bitmap` |
| Sunum | Slayt başına raster: küçük resim + aktif slayt; geçişler Compose katmanlarıyla |
| Excel | Izgara **Compose Canvas ile yerel çizilir** (motordan yalnız görünen hücrelerin değer ve stili alınır); grafikler motor rasteriyle |
| CAD | Motorun ürettiği çizim listesi (çizgiler, üçgenler) GPU'da çizilir; akıcı yakınlaştırma için gerekirse raster döşeme yedeği |

Ortak kurallar:

- Bellek önbelleği cihazın bellek sınıfına göre boyutlanır (LRU). `onTrimMemory` geldiğinde önbellek boşaltılır.
- Komşu sayfalar önceden hazırlanır.
- Her belgeye bir motor iş parçacığı ayrılır; çizim, motorların ucuz "kopya-yazınca" (copy-on-write) anlık görüntüleri üzerinden yapılır.

### 9.5 Metin girişi (IME) köprüsü

- Özel bir `InputConnection` katmanı, `commitText`, `setComposingText`, `deleteSurroundingText`, `setSelection` ve `getTextBeforeCursor` çağrılarını motor komutlarına çevirir.
- WordCraft telefon klavyelerinin "çevreyi sil ve yeniden yaz" davranışını (web sürümü için) zaten destekliyor; bu iyi bir başlangıç noktası.
- Test matrisi: Gboard, Samsung Klavye, SwiftKey; Türkçe Q ve F düzenleri; otomatik düzeltme; sesle yazma; donanım klavyesi.

### 9.6 Dosya erişimi ve güvenli kaydetme

- **Erişim:** Storage Access Framework (SAF) kullanılır. "Son dosyalar" için kalıcı URI izinleri alınır. "Birlikte aç" ve "Paylaş" için intent filtreleri tanımlanır ([Ek A](#ek-a-android-mime-türleri)).
- **Güvenli kaydetme:**
  1. Önce uygulama içi geçici dosyaya yazılır.
  2. Dosya yeniden açılarak doğrulanır.
  3. Hedefe kopyalanır.
  4. Başarı onaylanana kadar yedek tutulur.
- **İzin politikası:** "Tüm dosyalara erişim" (MANAGE_EXTERNAL_STORAGE) izni **varsayılan olarak istenmez** (bkz. §16, karar 5).

### 9.7 Süreç yalıtımı ve kararlılık

- **Ayrı süreç:** Belge motorları ayrı bir süreçte (`:engine`) çalışır. Yerel bir çökme arayüzü öldürmez; arayüz yeniden bağlanır ve otomatik kayıttan geri yükler.
- **Veri aktarımı:** Görüntüler süreçler arası `SharedMemory`/`HardwareBuffer` ile kopyasız aktarılır.
- **Sertleştirme (P2):** `isolatedProcess` korumalı alanı kullanılır. Motorun dosya sistemine hiç erişimi olmaz; yalnız kendisine verilen dosya tanıtıcılarını okur.
- Ek maliyet Faz 0'da ölçülür (ADR-0003).

### 9.8 Yazı tipleri

- **Sistem yazı tipleri:** Roboto, Noto ve Noto CJK kullanılır.
- **Metrik uyumlu paket yazı tipleri:** Carlito ↔ Calibri, Caladea ↔ Cambria, Liberation ↔ Arial/Times/Courier. Upstream de bunları kullanıyor ve hepsi Türkçe karakterleri içeriyor.
- Kullanıcı kendi `.ttf`/`.otf` dosyalarını ekleyebilir.

### 9.9 Boyut stratejisi

- **Dağıtım:** Android App Bundle ile ABI'ye göre bölünmüş indirme (arm64-v8a, armeabi-v7a, x86_64).
- **Rust derleme profili:** `opt-level = "s"`, `lto = "fat"`, `codegen-units = 1`, `strip = true`. Upstream'in panik yakalayıcıları için `panic = "unwind"` korunmalı.
- **Hariç tutulacaklar (özellik bayraklarıyla):** egui arayüz crate'leri, MCP sunucuları, güncelleme denetleyicisi ve DeckCraft'ın video kodekleri. Video oynatma için Android'in donanım kod çözücüsü (Media3) kullanılır.
- **İsteğe bağlı indirme:** OCR modelleri ve büyük yazı tipleri Play sürümünde isteğe bağlı modüllerle indirilir; F-Droid sürümünde paketin içinde gelir.
- **CI boyut bütçesi:** Bütçeyi aşan PR uyarı alır.

### 9.10 Upstream (ArtCraft) entegrasyonu

- **Sabitleme:** Bağımlılıklar Cargo git bağımlılığı olarak **commit'e sabitlenir** (upstream kendi ortak kütüphanelerini de böyle sabitliyor).
- **Ayda bir kontrollü güncelleme:** Önce korpus regresyon testleri, sonra birleştirme.
- **Yerel yamalar:** `PATCHES.md` dosyasında listelenir; her yama küçük ve upstream'e gönderilebilir tutulur (ör. Android yazı tipi keşfi, Windows-1254 algılama, Türkçe OCR).
- **Ana motor crate'leri** egui/winit/wgpu'dan bağımsızdır ve WebAssembly derlemesi CI'da denetlenir (CADCraft'ın DWG köprüsü hariç; o da yerel kod olduğundan Android'e uygundur). Faz 0'da her biri `aarch64-linux-android` için derlenerek doğrulanır.
- **Yan bağımlılıklar:** Motorlar storytold/craft-libs (craft-pdf, krilla, hayro çatalları) gibi git bağımlılıkları çeker. Bunların da Android'de derlendiği Faz 0'da kontrol edilir.
- **Terk riski:** Upstream yavaşlar ya da terk edilirse lisans fork'a izin verir; ArtCraft markası kullanılmaz.

### 9.11 Teknoloji yığını

| Alan | Seçim | Lisans |
|---|---|---|
| Dil ve arayüz | Kotlin, Jetpack Compose, Material 3, Navigation Compose | Apache-2.0 |
| Mimari | ViewModel, Coroutines/Flow, Hilt | Apache-2.0 |
| Veri | Room (son dosyalar, tam metin dizini), DataStore (ayarlar) | Apache-2.0 |
| Arka plan işleri | WorkManager (dönüştürme, OCR, dizinleme) | Apache-2.0 |
| Kamera ve medya | CameraX (tarama), Media3 (ses/video) | Apache-2.0 |
| Görsel | Android ImageDecoder, Coil; SVG için AndroidSVG veya bir Rust kütüphanesi | Apache-2.0 vb. |
| E-kitap | Readium Kotlin Toolkit | BSD-3 |
| Arşiv | libarchive (NDK) | BSD-2 |
| OCR | Tesseract (`tur` modeli) veya upstream `ocrs` | Apache-2.0 / MIT-Apache |
| Rust köprüsü | UniFFI, cargo-ndk, NDK r28+ (16 KB hizalama) | MPL-2.0 / MIT-Apache |
| PDF görüntüleme | PDFium (hazır Android derlemeleri mevcut) | BSD-3 (doğrulanmalı) |
| Kod kalitesi | ktlint, detekt, Android Lint, clippy, rustfmt, cargo-deny | MIT / Apache-2.0 |
| Test | JUnit, Compose UI Test, Roborazzi, Macrobenchmark | Apache-2.0 |
| Lisans ekranı | AboutLibraries | Apache-2.0 |
| Yayın | Gradle Play Publisher veya fastlane; F-Droid meta verisi | MIT |

Kesin sürümler Faz 0'da sabitlenir. Hedef SDK 36 (Android 16) olacak; minimum SDK kararı §16'da.

### 9.12 Depo yapısı (monorepo önerisi)

```text
Mobileoffice-free/
├─ android/                  Gradle projesi
│  ├─ app/                   uygulama modülü (play ve fdroid çeşitleri)
│  ├─ core/                  ui, tasarım sistemi, dosya erişimi, ortak araçlar
│  ├─ feature/               home, files, tools, pdf, word, sheet, slide, cad,
│  │                         image, text, archive, scanner, tr-pack, settings
│  └─ engine/                UniFFI'nin ürettiği Kotlin + jniLibs (.so)
├─ rust/                     Cargo çalışma alanı
│  ├─ mo-ffi/                tek .so; UniFFI arayüzü ve render JNI
│  ├─ mo-word/ mo-grid/ mo-deck/ mo-pdf/ mo-cad/   motor köprüleri
│  └─ mo-tr/ mo-text/ mo-archive/                  UDF, e-Fatura, kodlama, arşiv
├─ corpus/                   test belgeleri (yalnız lisanslı veya sentetik)
├─ docs/                     bu rapor, ADR'ler, tasarım notları
├─ fastlane/metadata/        mağaza metinleri ve ekran görüntüleri (F-Droid de okur)
├─ AGENTS.md / CLAUDE.md     yapay zekâ ajanları ve katkıcılar için kurallar
└─ .github/workflows/        CI/CD
```

### 9.13 Derleme ve CI/CD

**Her PR'da çalışanlar:**

- `cargo fmt`, `clippy -D warnings` ve Rust testleri.
- Üç ABI için `cargo ndk` derlemesi ve UniFFI bağlama üretimi.
- Gradle derlemesi, ktlint, detekt, Lint, birim testleri ve ekran görüntüsü testleri.
- **Kontroller:** lisans denetimi (cargo-deny + Gradle lisans raporu), **16 KB hizalama** (yerel kodlu uygulamalar için Play zorunluluğu) ve APK boyut bütçesi.

**Gecelik çalışanlar:** Belge korpusu taraması, fuzzing ve performans karşılaştırması.

**Yayın:**

- İmzalı AAB, Play dahili test kanalına otomatik yüklenir.
- GitHub Releases'a APK yüklenir.
- F-Droid kaynaktan derler; hedef yeniden üretilebilir derlemedir.

---

## 10. Güvenlik ve gizlilik

- **İnternet izni yok, analiz/reklam SDK'sı yok, Google Play Hizmetleri bağımlılığı yok.** Böylece uygulama F-Droid'e de uygun olur.
- **Belgeler güvenilmeyen girdi kabul edilir:**
  - Motor ayrı süreçte çalışır ve boyut sınırları uygulanır; fuzzing yapılır (upstream de yapıyor).
  - **Makrolar asla çalıştırılmaz.**
  - PDF JavaScript'i varsayılan olarak kapalıdır.
  - Belgedeki bağlantılar onay sorularak tarayıcıda açılır.
- **Yerel depolama:**
  - Geçici dosyalar uygulamaya özel alanda tutulur ve temizlenir.
  - İsteğe bağlı uygulama kilidi (biyometrik) sunulur.
  - Hassas belgelerde ekran görüntüsü engeli seçilebilir.
  - Gizli modda son dosyalar kaydedilmez.
- **Çökme raporları:** Otomatik gönderim yoktur. Play Console'un Android vitals verileri kullanılır. Ek olarak kullanıcı isterse belge içeriği içermeyen bir hata günlüğünü e-posta veya paylaşım ile gönderebilir.
- **Play "Veri güvenliği" formu:** "Veri toplanmaz, paylaşılmaz." Kısa bir gizlilik politikası sayfası hazırlanır.

---

## 11. Lisans, yasal uyum ve marka

### 11.1 Başlıca bağımlılıkların lisansları

| Bileşen | Lisans | Yükümlülük |
|---|---|---|
| ArtCraft motorları (WordCraft, GridCraft, DeckCraft, PdfCraft, CADCraft) | MIT veya Apache-2.0 | Telif ve lisans metni; Apache NOTICE içeriği |
| hayro, krilla, vello_cpu, skrifa ve benzeri Rust crate'leri | MIT ve/veya Apache-2.0 | Aynı |
| acadrust (DWG) | MPL-2.0 | Değiştirilirse değişen dosyaların kaynağı |
| UniFFI | MPL-2.0 | Aynı |
| PDFium | BSD-3 (doğrulanmalı) | Telif bildirimi |
| AndroidX, Compose, CameraX, Media3 | Apache-2.0 | NOTICE |
| libarchive | BSD-2 | Telif bildirimi |
| Readium | BSD-3 | Telif bildirimi |
| Tesseract ve `tur` modeli | Apache-2.0 | NOTICE |
| Yazı tipleri (Carlito, Caladea, Liberation, Noto) | OFL ve benzeri | Yazı tipi lisans metni |
| Türkçe Hunspell sözlüğü | Kontrol edilecek | — |

Bu bildirimler uygulama içinde bir "Açık kaynak lisansları" ekranında toplanır.

### 11.2 Uygulamanın kendi lisansı

Karar bekliyor; seçenekler §16'da.

- **Rust köprüleri ve motor katkıları:** MIT veya Apache-2.0 (upstream'e geri gönderilebilmesi için).
- **Android uygulaması:**
  - GPL-3.0-or-later: Türevlerin de açık kaynak kalmasını güvenceye alır.
  - MIT/Apache-2.0: Upstream ile aynıdır; ArtCraft'ın mobil arayüzümüzü benimseyebilmesini sağlar.

Her iki durumda da Google Play'e ve F-Droid'e uygundur.

### 11.3 Marka ve ifade kuralları

- **ArtCraft:** Adı ve logoları kullanılmaz. "Hakkında" ekranında logosuz bir metin atfı yeterlidir.
- **Microsoft, Adobe, Autodesk:** Word, Excel, PowerPoint, Acrobat, AutoCAD ve DWG bu şirketlerin markalarıdır. Mağaza metninde yalnız "… dosyalarıyla uyumlu" gibi açıklayıcı ifadeler kullanılır. Logolar ve simgeler taklit edilmez; gerektiğinde dipnotla marka sahibi belirtilir.
- **İsim:** İsim seçilmeden önce TÜRKPATENT ve Play Store taraması yapılır. Uygulama adında "Office" kelimesinden kaçınmak daha güvenli olabilir.

### 11.4 Formatlar ve mevzuat

- **UDF:** Resmî olarak belgelenmemiş bir formattır. Okuyucu birlikte çalışabilirlik amacıyla, kullanıcının kendi belgelerini açması için yazılır. Yayın öncesi kısa bir hukuki görüş alınması önerilir.
- **KVKK:** Uygulama kişisel veri işlemez ve toplamaz. Basit bir bilgilendirme metni yeterlidir.
- **Karartma özelliği:** Kullanıcıların belgeleri KVKK'ya uygun paylaşmasına yardım eder.

---

## 12. Kalite güvencesi ve test

### 12.1 Test katmanları

| Katman | Araç | Kapsam |
|---|---|---|
| Rust birim ve özellik testleri | cargo test, proptest | Upstream testleri + köprü testleri |
| Fuzzing | cargo-fuzz | Köprü giriş noktaları, UDF ve e-Fatura ayrıştırıcıları |
| Kotlin birim testleri | JUnit, Turbine | ViewModel ve veri katmanı |
| Arayüz testleri | Compose UI Test, Roborazzi | Açık/koyu tema, TR/EN, büyük yazı, tablet |
| Cihaz matrisi | Gradle Managed Devices / emülatör | API 26, 30, 34, 36; telefon, tablet, katlanabilir |
| Performans | Macrobenchmark, Baseline Profiles | Başlatma, ilk sayfa, kaydırma akıcılığı |
| Erişilebilirlik | Accessibility Test Framework + elle TalkBack | Ana akışlar |
| Belge korpusu | Gecelik tarama | Çökme, render karşılaştırma, aç-kaydet-aç |

### 12.2 Belge korpusu

**Kaynaklar:**

- Kendi ürettiğimiz Türkçe örnekler: dilekçe, bordro, fatura, sunum, DWG örnekleri.
- Kamuya açık korpuslar (lisansları kontrol edilerek): pdf.js test dosyaları, Apache POI test dosyaları, GovDocs1.
- GİB'in örnek e-Fatura dosyaları.
- Gönüllülerin paylaştığı, kişisel veriden arındırılmış belgeler.

**Kural:** Kişisel veri içeren belge korpusa asla eklenmez.

**Doğruluk ölçümü:**

- Referans PDF çıktılarıyla görsel benzerlik (SSIM). Referanslar ör. LibreOffice'in sunucu modundan alınır ve yaklaşık değer verir.
- Metin çıkarma için F1 puanı.
- Aç-kaydet-aç döngüsünde XML farkları.

### 12.3 Türkçeye özgü hata sınıfları

- **"Türkçe I sorunu":**
  - Varsayılan yerel ayarla `uppercase()` çağrılırsa "file" → "FİLE" olur; uzantı ve anahtar karşılaştırmaları bozulur. Programatik büyük/küçük harf dönüşümlerinde `Locale.ROOT` zorunlu tutulmalı ve bunu denetleyen bir lint kuralı eklenmelidir.
  - Kullanıcıya dönük aramada ise İ/i ve I/ı doğru eşleştirilmelidir.
- **Sıralama:** Türkçe harf sırasına uygun (Collator `tr`).
- **Dosya adları:** Türkçe karakterli adlar ve yolları sorunsuz işlenmelidir.
- **Kodlama:** Windows-1254 ve ISO-8859-9.
- **Biçimler:** Ondalık virgül ve TL biçimleri.

### 12.4 Beta programı

- **Kapalı test (Play):** Kasım 2023 sonrası açılan **kişisel** geliştirici hesaplarında üretime çıkmadan önce zorunludur: en az **12 test kullanıcısı**, kesintisiz **14 gün**.
- **Ardından:** Açık test (açık beta), IzzyOnDroid/F-Droid ve GitHub sürümleri.
- **Geri bildirim:** GitHub Issues ve uygulama içi "Sorun bildir" (belge içeriği gönderilmez).

### 12.5 Tamamlanma tanımı (her özellik için)

- Testler yazılmış ve geçiyor.
- TalkBack ile kullanılabiliyor.
- Karanlık temada doğru görünüyor.
- TR/EN metinleri hazır.
- Büyük ekranda doğru görünüyor.
- Performans bütçesi içinde.
- Kullanıcı belgesi ve gerekiyorsa ADR güncellendi.

---

## 13. Yol haritası ve takvim

Varsayım: Başlangıç Kasım 2026; 1–2 geliştirici + yapay zekâ destekli geliştirme. Yarı zamanlı tek kişiyle süreler yaklaşık iki katına çıkar. Takvim Faz 0 sonunda gerçek ölçümlerle güncellenecek.

| Faz | Takvim (tahmini) | Hedef | Ana çıktılar | Çıkış kriteri |
|---|---|---|---|---|
| **0** | Hafta 1–3 (Kasım 2026) | Teknik doğrulama | Rust motorları Android'de; 5 formatta demo; ölçümler; ADR-0001…0005 | Devam/dur kararı: boyut ≤ 60 MB, ilk sayfa süreleri bütçede, demo dosyalarda çökme yok |
| **1** | Hafta 4–11 (Kasım sonu 2026–Ocak 2027) | **MVP: evrensel görüntüleyici** | Ana ekran, dosya erişimi; PDF, Word, Excel, PowerPoint, DWG/DXF, görsel, metin, CSV ve ZIP görüntüleme; arama, paylaşma, yazdırma; TR/EN | 40+ uzantı açılıyor; dahili test kanalında yayın |
| **2** | Hafta 12–19 (Ocak–Mart 2027) | **PDF araç kutusu + Türkiye paketi** | Açıklama, form, imza, sayfa işlemleri, birleştirme/bölme, parola, sıkıştırma, tarama + Türkçe OCR, UDF, e-Fatura, 7z/RAR | Play kapalı testi (12+ kişi, 14 gün) → **açık beta** |
| **3** | Hafta 20–33 (Mart–Haziran 2027) | **Office düzenleme** | Word/Excel/PowerPoint mobil düzenleyicileri, şablonlar, kayıp uyarısı, otomatik kayıt, Türkçe Excel | Temel düzenleme akışları korpusta kayıpsız; beta kullanıcılarından olumlu geri bildirim |
| **4** | Hafta 34–41 (Haziran–Ağustos 2027) | Genişleme | PPT/ODP içe aktarma, XLS biçimleri, EPUB, EML/MSG, VCF/ICS, tablet/klavye/kalem, uygulama kilidi, tam metin arama; yıllık hedef API güncellemesi | Özellik dondurma |
| **5** | Hafta 42–47 (Ağustos–Eylül 2027) | **v1.0** | Performans, kararlılık, erişilebilirlik denetimi, mağaza materyalleri, F-Droid | Çökme < %0,5, ANR < %0,2 → v1.0 yayını |

> **Yıllık hedef API uyarısı:** Google Play, yeni uygulama ve güncellemelerin 31 Ağustos 2026'dan itibaren Android 16'yı (API 36) hedeflemesini istiyor. Bu eşik her yıl yükseliyor; 2027 için API 37 beklenmeli (doğrulanmalı). Faz 4 takvimine bu güncelleme eklendi.

### 13.1 Faz 0: ilk üç haftanın iş listesi

**1. hafta: Altyapı**

- [x] Depo iskeleti (`android/`, `rust/`, `docs/`), temel CI ve `AGENTS.md`/`CLAUDE.md` kuralları
- [x] Rust Android hedefleri (aarch64, armv7, x86_64), cargo-ndk, NDK r28+
- [x] ArtCraft depolarını belirli commit'lere sabitleme (Cargo git rev)
- [x] En az bağımlılıklı motorla başlama: GridCraft (XLSX → hücre değerleri) + UniFFI "merhaba dünya"

**2. hafta: Her formatta ilk görüntü**

- [x] WordCraft: DOCX sayfasını `Bitmap`'e çizme (şimdilik bir kopya ile; kopyasız yol ölçümden sonra)
- [x] DeckCraft: PPTX slaytını çizme (video kodekleri kapalı)
- [ ] PDF: PDFium, Android PdfRenderer ve PdfCraft (hayro) hız/doğruluk karşılaştırması (görüntüleme için şimdilik PdfRenderer seçildi, ADR-0003; ölçüm cihazda yapılacak)
- [x] CADCraft: DWG ve DXF çizim listesini ekranda çizme

**3. hafta: Ölçüm ve karar**

- [ ] Düşük, orta ve üst segment birer telefonda ölçüm: `.so` boyutu (ABI başına), bellek, ilk sayfa süreleri (üst segment yapıldı: [olcumler.md](olcumler.md))
- [x] 16 KB hizalama doğrulaması (CI her APK'da denetliyor; PDFium kullanılmadığı için ayrıca gerekmedi)
- [ ] Türkçe karakter testi: "ğüşıöçİĞÜŞÖÇ" render, arama ve IME (render ve dosya adı/metin araması testli; IME düzenleme fazında)
- [ ] Karar toplantısı ve ADR'ler:
  - [x] 0001: mimari
  - [ ] 0002: PDF motoru
  - [ ] 0003: süreç yalıtımı
  - [ ] 0004: lisans
  - [ ] 0005: izinler ve dağıtım
- [ ] Faz 1 iş listesi ve tahminlerin güncellenmesi

### 13.2 Faz kabul testlerine örnekler

| Faz | Örnek kabul testleri |
|---|---|
| **1** | WhatsApp'tan gelen 20 sayfalık bir DOCX "Birlikte aç" ile 2 saniyede açılır, Türkçe karakterler doğru görünür, PDF olarak paylaşılır. |
| **2** | 3 PDF birleştirilir, 2 sayfası döndürülür, imza eklenir, parola konur. Dosya Acrobat ve Chrome'da sorunsuz açılır. |
| **2** | Avukatın UDF dosyası açılır, PDF'e dönüştürülür, yazdırılır. |
| **2** | Windows-1254 kodlu bir CSV'deki "ŞİŞLİ" doğru görünür. |
| **3** | Bir XLSX'e formül eklenip kaydedilir; dosya Excel ve LibreOffice'te açıldığında formül ve biçimler korunmuştur. Desteklenmeyen bir öğe varsa kayıttan önce uyarı çıkar. |

---

## 14. Ekip, maliyet ve sürdürülebilirlik

### 14.1 Roller

| Rol | Sorumluluk |
|---|---|
| Proje sahibi / ürün yöneticisi | Öncelikler, kararlar, topluluk, mağaza |
| Android/Kotlin geliştirici | Arayüz, dosya erişimi, Android entegrasyonları |
| Rust geliştirici | Köprüler, motor uyarlamaları, upstream katkıları |
| Tasarımcı (yarı zamanlı veya gönüllü) | Kullanıcı deneyimi, simge, mağaza görselleri |
| Gönüllüler | Test, çeviri, belge korpusu |
| Yapay zekâ kodlama ajanları (ör. Claude Code) | ArtCraft'ın yaptığı gibi küçük ve testli PR'lar; kurallar `AGENTS.md`/`CLAUDE.md`'de, CI kalite kapıları zorunlu |

### 14.2 Maliyetler

| Kalem | Tutar | Not |
|---|---|---|
| Google Play geliştirici hesabı | 25 USD (bir kez) | Kişisel hesapta kapalı test şartı var; kuruluş hesabı için D-U-N-S numarası gerekir |
| Sunucu / bulut | 0 | Uygulama tamamen cihazda çalışır |
| CI/CD | 0 | Herkese açık depoda GitHub Actions |
| F-Droid / IzzyOnDroid | 0 | |
| Çeviri platformu | 0 | Açık kaynak projelere ücretsiz Weblate barındırması (koşullar doğrulanmalı) |
| Web sitesi | ~0 | GitHub Pages; alan adı isteğe bağlı (yıllık düşük ücret) |
| Test cihazları | İsteğe bağlı | Düşük, orta ve üst segment birer telefon + bir tablet önerilir |
| Marka tescili | İsteğe bağlı | İsim netleşince TÜRKPATENT başvurusu |
| Geliştirme emeği | Asıl maliyet | Gönüllü çalışma veya zaman yatırımı |

### 14.3 Reklamsız sürdürülebilirlik

- **Bağış:** GitHub Sponsors, Open Collective, Liberapay. Uygulamada küçük bir "Destek ol" bağlantısı olur; özellik kilidi olmaz.
- **Hibe programları:** Açık kaynak hibeleri (ör. NLnet/NGI); uygunluk koşulları kontrol edilmeli.
- **Kurumsal sponsorluk:** Uygulamada logo gösterilmez; teşekkür sayfasında yer verilir.
- **Üniversite işbirlikleri:** Bitirme projeleri, staj, Türkçe korpus oluşturma.
- **Upstream işbirliği:** Motor bakım yükü ArtCraft topluluğuyla paylaşılır.
- **Kırmızı çizgiler:** Reklam yok, veri satışı yok, ücretli özellik yok.

---

## 15. Riskler ve azaltma planı

| # | Risk | Olasılık | Etki | Azaltma |
|---|---|---|---|---|
| R1 | Upstream motorlar alfa: gerçek dosyalarda biçim kaybı veya hatalar | Yüksek | Yüksek | Önce görüntüleme; düzenlemede kayıp uyarısı ve "kopya olarak kaydet"; Türkçe korpus testleri; hataları upstream'e bildirip düzeltme |
| R2 | Upstream çok hızlı değişiyor; API kırılmaları | Yüksek | Orta | Commit'e sabitleme, ince komut tabanlı köprü, ayda bir kontrollü güncelleme, regresyon testleri |
| R3 | Upstream yavaşlar veya terk edilir | Düşük–orta | Yüksek | Lisans fork'a izin veriyor; kritik crate'lerde bilgi birikimi; katkılarla içeriden tanışıklık |
| R4 | Mobil işlemcide yavaş render (CPU rasterleştirme) | Orta | Yüksek | Döşeme, önbellek, önce düşük çözünürlük; PDF'te PDFium; Excel ve CAD'de yerel/GPU çizim; Faz 0 ölçümü |
| R5 | Uygulama boyutunun büyümesi | Orta | Orta | ABI bölme, özellik bayrakları, isteğe bağlı modüller, CI boyut bütçesi |
| R6 | Büyük dosyalarda bellek yetmemesi (GridCraft akışlı okuma yapmıyor) | Orta | Orta | Boyut sınırı + salt okunur hızlı görünüm; upstream'e akış katkısı; `onTrimMemory` |
| R7 | Türkçe klavye/IME uyumsuzlukları | Orta | Yüksek | Özel InputConnection katmanı; Gboard/Samsung/SwiftKey test matrisi |
| R8 | DWG uyumluluğu (ön-alfa CADCraft, acadrust) | Orta | Orta | Yalnız görüntüleme; DXF yolu; gerçek DWG korpusu; desteklenmeyen nesneler için bilgi notu |
| R9 | Google Play politikaları (izinler, hedef API, 16 KB) | Orta | Yüksek | SAF öncelikli tasarım; internet izni yok; yıllık hedef API takvimi; CI'da 16 KB kontrolü |
| R10 | Kötü niyetli belgelerle saldırı | Orta | Yüksek | Ayrı süreç, makro/JS kapalı, fuzzing, boyut sınırları, hızlı güncelleme |
| R11 | Lisans veya marka hatası | Düşük | Yüksek | cargo-deny + Gradle lisans raporu, lisans ekranı, ArtCraft markasını kullanmama, hukuki gözden geçirme |
| R12 | İnsan kaynağı yetersizliği, motivasyon kaybı | Orta | Yüksek | Küçük ve sık sürümler, net MVP, topluluk, yapay zekâ destekli geliştirme |
| R13 | Reklamlı kötü niyetli klonlar | Orta | Orta | Marka koruması, resmî kanallar, imzalı sürümler, mağaza şikâyet süreçleri |
| R14 | UDF/e-Fatura biçim değişiklikleri | Düşük–orta | Orta | Örnek dosya korpusu, sürüm algılama, esnek ayrıştırıcı |

---

## 16. Karar bekleyen konular

| # | Konu | Seçenekler | Öneri |
|---|---|---|---|
| 1 | **Uygulama adı ve logo** | Ör. *Kâtip*, *Evrak*, *Belgem*, *Dosyam*… | Kısa, Türkçe kökenli ama uluslararası söylenebilir bir isim; TÜRKPATENT ve Play taraması; adda "Office"/"Craft" kullanmamak |
| 2 | **Lisans** | (a) Uygulama GPL-3.0-or-later + Rust tarafı MIT/Apache; (b) her şey MIT/Apache | Ücretsiz ve açık kalmayı güvenceye almak için (a). ArtCraft ile ortak bir mobil arayüz hedeflenirse (b) |
| 3 | **ArtCraft ile işbirliği** | Bağımsız çalışmak / iletişim kurup katkı vermek | Faz 0'da Discord'da tanışma; yamaları upstream'e gönderme |
| 4 | **Minimum Android sürümü** | API 23 (AndroidX varsayılanı, kullanıcıların ~%99'u) / API 26 (Android 8.0) | API 26; Faz 0'da cihaz dağılımına bakıp gerekirse 23'e inme |
| 5 | **"Tüm dosyalara erişim" izni** | İstemek / istememek | **Karar verildi (28.10.2026): istenir**, Play'de "belge yönetimi" beyanıyla. İzinsiz de her şey çalışır. Bkz. [ADR-0002](adr/0002-tum-dosyalara-erisim.md) |
| 6 | **İnternet izni** | Hiç istememek / bulut entegrasyonu için istemek | Hiç istememek. Bulut dosyalarına erişim Drive/OneDrive/Dropbox/Nextcloud uygulamalarının SAF sağlayıcılarıyla |
| 7 | **Dağıtım kanalları** | Play / F-Droid / GitHub | Üçü birden; önce Play dahili test + GitHub, sonra F-Droid |
| 8 | **v1 düzenleme kapsamı** | Tüm formatlar / modern formatlar | DOCX, XLSX, PPTX, PDF düzenlenir; eski formatlar (DOC, XLS, PPT) görüntülenir ve modern biçimde kopya olarak kaydedilir |
| 9 | **Geliştirici hesabı türü** | Kişisel (kapalı test şartı) / kuruluş (D-U-N-S gerekir) | Kuruluş/dernek kurulacaksa kuruluş hesabı; değilse kişisel hesap + 12 kişilik test grubu hemen oluşturulmalı |
| 10 | **PDF görüntüleme motoru** | PDFium (gömülü) / Android PdfRenderer / PdfCraft (hayro) | **Karar verildi (28.10.2026): Faz 0–1'de Android PdfRenderer**, düzenleme araçları Faz 2'de PdfCraft. Bkz. [ADR-0003](adr/0003-pdf-goruntuleme.md) |

---

## 17. Sonuç ve hemen atılacak adımlar

Örnek gösterilen uygulamalar güçlü ama masaüstü ve web için yapılmış, alfa aşamasında ve **mobilde yoklar**. Kodları açık kaynak ve motorları arayüzden bağımsız. Bu, Android'de **onlardan daha iyi** bir uygulama yapmak için nadir bir fırsat.

Fark yaratacak şeyler:

- Mobil için baştan tasarlanmış tek bir uygulama,
- Türkiye'ye özel formatlar ve dil desteği,
- İnternet izni bile istemeyen gizlilik,
- Erişilebilirlik,
- Veri kaybetmeme disiplini.

**Önümüzdeki 10 adım:**

1. §16'daki kararları vermek (özellikle 1, 2, 4, 9).
2. Google Play geliştirici hesabını açmak ve 12 kişilik test grubunu şimdiden oluşturmak.
3. Depo iskeletini, CI'ı ve `AGENTS.md`/`CLAUDE.md` kurallarını kurmak.
4. ArtCraft depolarını commit'e sabitleyip Android'e derlemek.
5. GridCraft ile ilk UniFFI demosunu yapmak.
6. DOCX, PPTX, PDF ve DWG için ilk görüntüleri telefonda almak.
7. Boyut, hız ve bellek ölçümlerini üç cihazda yapmak.
8. ADR-0001…0005'i yazmak ve devam/dur kararını vermek.
9. Faz 1 iş listesini GitHub Issues/Projects'te açmak.
10. ArtCraft topluluğuyla iletişime geçmek.

---

## Ek A: Android MIME türleri

"Birlikte aç" ve "Paylaş" için intent filtreleri. Bazı dosya yöneticileri bilinmeyen türleri `application/octet-stream` olarak gönderir. Bu nedenle bu tür de kaydedilir ve dosya içeriğinden tanınır. `pathPattern` eşleşmesi `content://` adreslerinde güvenilir değildir.

| Uzantı | MIME türleri |
|---|---|
| .pdf | `application/pdf` |
| .doc / .docx / .docm / .dotx | `application/msword`, `application/vnd.openxmlformats-officedocument.wordprocessingml.document`, `application/vnd.ms-word.document.macroEnabled.12`, `application/vnd.openxmlformats-officedocument.wordprocessingml.template` |
| .odt / .rtf | `application/vnd.oasis.opendocument.text`, `application/rtf`, `text/rtf` |
| .xls / .xlsx / .xlsm / .xlsb | `application/vnd.ms-excel`, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`, `application/vnd.ms-excel.sheet.macroEnabled.12`, `application/vnd.ms-excel.sheet.binary.macroEnabled.12` |
| .ods / .csv / .tsv | `application/vnd.oasis.opendocument.spreadsheet`, `text/csv`, `text/comma-separated-values`, `text/tab-separated-values` |
| .ppt / .pptx / .ppsx / .odp | `application/vnd.ms-powerpoint`, `application/vnd.openxmlformats-officedocument.presentationml.presentation`, `application/vnd.openxmlformats-officedocument.presentationml.slideshow`, `application/vnd.oasis.opendocument.presentation` |
| .dwg / .dxf | `image/vnd.dwg`, `application/acad`, `application/x-dwg`, `image/vnd.dxf`, `application/dxf` (+ `application/octet-stream`) |
| .udf | Standart türü yok → `application/octet-stream` + içerik tanıma |
| .xml (e-Fatura) | `application/xml`, `text/xml` |
| .epub | `application/epub+zip` |
| .zip / .7z / .rar | `application/zip`, `application/x-7z-compressed`, `application/vnd.rar`, `application/x-rar-compressed` |
| .txt / .md / .json | `text/plain`, `text/markdown`, `application/json` |
| .eml / .msg | `message/rfc822`, `application/vnd.ms-outlook` |
| .vcf / .ics | `text/vcard`, `text/x-vcard`, `text/calendar` |
| Görseller | `image/*` |

## Ek B: Köprü API taslağı

UniFFI'nin üreteceği Kotlin API'sinin hedeflenen biçimi aşağıda. Bu bir taslaktır; Faz 0'da netleşecek.

```kotlin
interface MoDocument : AutoCloseable {
    val kind: DocKind                     // WORD, SHEET, SLIDES, PDF, CAD, UDF, INVOICE, TEXT, …
    fun pageCount(): Int                  // sayfa / çalışma sayfası / slayt / düzen sayısı
    fun commands(): String                // komut kataloğu (JSON): menüler ve komut arama
    fun execute(id: String, paramsJson: String = "{}"): String   // her düzenleme bir komuttur
    fun inspect(queryJson: String): String                       // seçim, biçim, hücre değeri…
    fun undo(): Boolean
    fun redo(): Boolean
    fun save(format: String): SaveReport  // baytlar + "kaydederken kaybolacaklar" uyarıları
}

// Render kopyasız olsun diye UniFFI dışında ince bir JNI fonksiyonuyla yapılır:
external fun renderTile(doc: Long, page: Int, scale: Float,
                        left: Int, top: Int, target: android.graphics.Bitmap): Boolean
```

Komut kimlikleri doğrudan upstream komut kayıtlarından gelir:

```kotlin
sheet.execute("home.bold", """{"range":"B4:F4"}""")                          // GridCraft
word.execute("format.bold")                                                   // WordCraft
deck.execute("slide.new", """{"layout":"titleOnly","title":"Merhaba"}""")    // DeckCraft
```

## Ek C: Sözlük

| Terim | Anlamı |
|---|---|
| AAB | Android App Bundle; Play'in cihaza göre küçük APK ürettiği yayın biçimi |
| ABI | İşlemci mimarisi (arm64-v8a, armeabi-v7a, x86_64) |
| ADR | Mimari karar kaydı; neden o kararın verildiğini yazan kısa belge |
| ANR | "Uygulama yanıt vermiyor" hatası |
| IME | Giriş yöntemi; ekran klavyesi |
| JNI / NDK | Kotlin/Java ile C/C++/Rust kodu arasındaki köprü ve Android'in yerel geliştirme kiti |
| MVP | En küçük kullanılabilir ürün |
| OCR | Görüntüdeki yazıyı metne çevirme |
| OOXML / ODF | Microsoft Office (DOCX/XLSX/PPTX) ve OpenDocument (ODT/ODS/ODP) standart dosya biçimleri |
| Render | Belgenin ekrana çizilmesi |
| SAF | Storage Access Framework; Android'in güvenli dosya erişim sistemi |
| TTS | Metni sese çevirme (sesli okuma) |
| UniFFI | Mozilla'nın Rust kütüphanelerine Kotlin/Swift bağlamaları üreten aracı |
| Upstream | Kodunu kullandığımız asıl proje (burada ArtCraft depoları) |

## Ek D: Kaynaklar

**İncelenen depolar** (27 Ekim 2026 itibarıyla son commit'ler):

- WordCraft: <https://github.com/storytold/wordcraft> (`c4fafc2`)
- GridCraft: <https://github.com/storytold/gridcraft> (`11bb673`)
- DeckCraft: <https://github.com/storytold/deckcraft> (`55e5ee5`)
- PdfCraft: <https://github.com/storytold/pdfcraft> (`20eebc1`)
- CADCraft: <https://github.com/storytold/cadcraft> (`5654240`)
- ArtCraft uygulama sayfaları (bu çalışmada erişilemedi): <https://getartcraft.com/apps>

**Basın:**

- Korben: <https://korben.info/en/i-tested-the-12-free-clones-of-adobe-and-microsoft.html>
- WWWhatsnew: <https://wwwhatsnew.com/2026/10/15/wordcraft-gridcraft-deckcraft-artcraft-clones-office-codigo-abierto/>
- Let's Data Science: <https://letsdatascience.com/news/brandon-thomas-releases-open-source-office-replicas-d4b27e59>
- ProPakistani: <https://propakistani.pk/2026/10/09/free-open-source-copies-of-photoshop-premiere-and-lightroom-are-here/>
- CADCraft rehberi: <https://pasqualepillitteri.it/en/news/22966/cadcraft-free-autocad-alternative-rust>

**Android ve Google Play:**

- Hedef API seviyesi: <https://developer.android.com/google/play/requirements/target-sdk> · <https://support.google.com/googleplay/android-developer/answer/11926878>
- 16 KB sayfa boyutu: <https://developer.android.com/guide/practices/page-sizes> · <https://www.freecodecamp.org/news/google-16-kb-page-size-requirement-what-to-do/>
- Tüm dosyalara erişim politikası: <https://support.google.com/googleplay/android-developer/answer/10467955> · <https://developer.android.com/training/data-storage/manage-all-files>
- Yeni kişisel hesaplar için test şartı: <https://support.google.com/googleplay/android-developer/answer/14151465>
- Android 16 büyük ekran değişiklikleri: <https://android-developers.googleblog.com/2025/01/orientation-and-resizability-changes-in-android-16.html>
- Android 15 PdfRenderer iyileştirmeleri: <https://developer.android.com/about/versions/15/features>
- AndroidX varsayılan minSdk: <https://developer.android.com/jetpack/androidx/versions>

**Araçlar ve kütüphaneler:**

- UniFFI: <https://docs.rs/crate/uniffi/latest>
- acadrust: <https://docs.rs/acadrust>
- hayro: <https://docs.rs/hayro>
- PDFium Android derlemeleri: <https://github.com/bblanchon/pdfium-binaries>
- Collabora Office Android: <https://www.collaboraonline.com/blog/collabora-office-android-beta/> · <https://gitlab.com/fdroid/rfp/-/issues/1399>

**Türkiye'ye özgü:**

- UDF biçimi ve açık kaynak dönüştürücü (udfpdf, Apache-2.0): <https://cdn.jsdelivr.net/npm/udfpdf@2.0.0/README.md>
- e-Fatura Uygulaması Entegrasyon Kılavuzu: <https://www.turmob.org.tr/arsiv/mbs/resmigazete/-e-FaturaUygulamasiEntegrasyonKilavuzu-v1.9.pdf>
