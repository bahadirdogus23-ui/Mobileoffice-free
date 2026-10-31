# Mobile Office Free — Telefonda Test Planı

Bu liste, telefonda henüz denenmemiş bütün özellikleri adım adım dener. Her maddede **ne yapılacağı** ve **ne görülmesi gerektiği** yazıyor.
Bir madde beklendiği gibi çalışmazsa, maddenin numarasını, kısa bir açıklamayı ve mümkünse bir ekran görüntüsünü gönderin. Örnek: "E4: sıralamada Şule en sona gitti".
Uygulama çökerse bir sonraki açılışta **çökme raporu** penceresi çıkar. Onu "Paylaş" ile gönderin. Ana ekrandaki **Tanılama** sayfasındaki raporu da ekleyebilirsiniz.

**Sürüm:** <https://github.com/bahadirdogus23-ui/Mobileoffice-free/releases/download/gelistirme/MobileOffice-Free.apk>
(S25 Ultra için bu dosya. Eski 32 bitlik tablet için `MobileOffice-Free-eski-32bit.apk`.)

**Hazırlık:** Telefonda şu örnek dosyalar olsun:

- birkaç sayfalı bir **Word** (DOCX) dosyası;
- formüllü ya da biçimli bir **Excel** dosyası; varsa dondurulmuş başlık satırı ve birleştirilmiş hücreleri olsun;
- bir **PowerPoint** (PPTX) dosyası; konuşmacı notları olanı tercih edin;
- bir **PDF**; içinde IBAN, telefon ya da T.C. kimlik numarası geçsin;
- doldurulabilir bir **PDF form**; e-Devlet ya da dilekçe şablonu olabilir;
- **parolalı** bir PDF, varsa;
- birkaç **fotoğraf**.

---

## A. Kurulum ve ana ekran

| No | Ne yapılacak | Beklenen |
|---|---|---|
| A1 | APK'yı indirip kurun (eski sürümün üzerine) | "Uygulama yüklenmedi" hatası çıkmaz, açılır |
| A2 | Ana ekranın üstüne bakın | Başlıkta **Ayarlar** ve **Tanılama**; altında **Tara** ve **Resimden PDF** düğmeleri |
| A3 | Arama kutusuna `istanbul` yazın (adında İSTANBUL geçen bir dosya varsa) | Büyük/küçük ve İ/ı farkına takılmadan bulur |
| A4 | Alttaki sekmeler ve UDF / e-Fatura / CAD filtreleri | Doğru dosyalar listelenir |

## B. Ayarlar ve gizlilik

| No | Ne yapılacak | Beklenen |
|---|---|---|
| B1 | **Ayarlar → Uygulamayı kilitle**'yi açın, uygulamayı kapatıp yeniden açın | Önce telefonun kilit ekranı (PIN/desen; telefon sunuyorsa parmak izi) gelir, sonra uygulama |
| B2 | Kilit açıkken başka uygulamaya geçip 10 sn içinde dönün | Kilit **sorulmaz** |
| B3 | 30 saniyeden uzun ayrılıp dönün | Kilit sorulur |
| B4 | Telefonu yan çevirin (kilitliyken ve açıkken) | Çevirince yeniden kilitlenmez |
| B5 | **Ekran görüntüsünü engelle**'yi açıp ekran görüntüsü almayı deneyin | Engellenir; son uygulamalar listesinde önizleme kararır |
| B6 | **Gizli mod**'u açıp bir dosya açın, ana ekrana dönün | Dosya "Son açılanlar"a eklenmez |
| B7 | **Son açılanları temizle** | Liste boşalır; dosyalar silinmez |
| B8 | Telefonda ekran kilidi yoksa | "Önce telefona bir ekran kilidi koyun" yazar, kilit açılmaz |

## C. Word (DOCX)

| No | Ne yapılacak | Beklenen |
|---|---|---|
| C1 | DOCX açın, **Düzenle**'ye basın | Klavye açılır, altta araç çubuğu çıkar |
| C2 | Metnin ortasına dokunun | Mavi imleç dokunduğunuz yerde |
| C3 | Türkçe yazın: `Şükrü Ağaoğlu İzmir'e ığdır` (Samsung klavye, öneriler açık) | Harfler eksiksiz ve **tek kez** yazılır; önerilen kelimeyi seçince doğru girer |
| C4 | Enter, sonra Sil tuşu | Yeni paragraf açılır; sil tek karakter siler |
| C5 | Bir kelimeye **basılı tutun**, parmağı kaldırmadan sağa kaydırın | Seçim kaydırdıkça büyür (mavi alan) |
| C6 | Seçiliyken **K**, **İ**, **A** | Kalın, italik, altı çizili olur; düğme seçili görünür |
| C7 | Yazı tipi (Aa) ve boyut (11) menüleri | Seçilen yazı tipi/boyut uygulanır |
| C8 | Hizalama, madde (•) ve numara (1.) | Doğru uygulanır |
| C9 | **Stil → Başlık 1** | Paragraf başlık olur |
| C10 | **Ekle → Resim** (galeriden bir fotoğraf) | Fotoğraf imleç yerine eklenir |
| C11 | **Ekle → Tablo** (3×3) ve **Sayfa sonu** | Tablo ve yeni sayfa oluşur |
| C12 | **Kes / Kopyala / Yapıştır**; başka uygulamadan kalın bir yazı kopyalayıp yapıştırın | Biçim korunarak yapıştırılır |
| C13 | **Değiştir**: bir kelimeyi başka kelimeyle değiştir | "N yer değiştirildi" yazar |
| C14 | Klavyenin kapattığı alt kısımda yazın | Sayfa imleci takip ederek kayar |
| C15 | **Geri al** / **Yinele** | Adım adım geri gelir |
| C16 | **Kaydet**, dosyayı kapatıp yeniden açın; bilgisayarda/Word'de açın | Değişiklikler duruyor; dosya bozulmamış |
| C17 | Kaydetmeden geri tuşuna basın | "Kaydedilmemiş değişiklikler" sorulur |
| C18 | DOC veya UDF dosyasında Düzenle → Kaydet | Yeni bir DOCX olarak kaydeder (asıl dosya değişmez) |
| C19 | **Okuma → Metin görünümü** | Metin ekran genişliğinde akar; A−/A+ yazıyı büyütür; metin seçilip kopyalanabilir |

## D. Excel

| No | Ne yapılacak | Beklenen |
|---|---|---|
| D1 | Gerçek bir XLSX açın | Sütun genişlikleri ve satır yükseklikleri Excel'deki gibi; gizli sütunlar görünmez |
| D2 | Birleştirilmiş başlık hücreleri | Tek kutu olarak, ortada görünür; içinde çizgi yok |
| D3 | Dondurulmuş başlık satırı/sütunu olan dosyada kaydırın | Başlık yerinde kalır; sınırda kalın çizgi |
| D4 | Uzun bir yazının sağı boşsa | Yazı boş hücrelerin üzerine taşar (kesilmez) |
| D5 | Kalın, renkli, dolgulu, kenarlıklı hücreler | Dosyadaki gibi görünür |
| D6 | Bir hücreye `1.250,5` yazıp Tamam | Sayı olarak kaydedilir, `1250,5` görünür, sağa yaslanır |
| D7 | `=TOPLA(A1;A2)`, `=EĞER(A1>100;"Yüksek";"Düşük")`, `=DÜŞEYARA(...)` | Doğru hesaplanır; formül çubuğunda Türkçe görünür |
| D8 | Formül çubuğuna `=top` yazın | Altta **TOPLA**, **TOPLAÇARPIM** önerileri çıkar; dokununca `TOPLA(` yazılır |
| D9 | Öneri yokken alttaki = + - * / ( ) ; : $ tuşları | İmlecin olduğu yere eklenir |
| D10 | `31.10.2026` yazın | Tarih olarak kalır (sayıya dönüşmez) |
| D11 | Araç çubuğu: K, İ, A, 🎨 dolgu, ▦ kenarlık, hizalama, ₺, %, ondalık | Seçili hücreye uygulanır, hemen görünür |
| D12 | **+ Satır / − Satır / + Sütun / − Sütun** | Satır/sütun eklenir/silinir; formüller kayar |
| D13 | İsim sütunu seçip **A→Z** (ör. Şule, Ahmet, Çiğdem, Zeynep) | **Ahmet, Çiğdem, Şule, Zeynep** (Türk alfabesi); satırlar bütün olarak taşınır |
| D14 | **Sayfa ▾**: Yeni sayfa, yeniden adlandır (Türkçe ad), sil → **Geri al** | Hepsi çalışır; silinen sayfa Geri al ile döner |
| D15 | **Sayfa ▾ → Üst satırı dondur** | Kaydırınca ilk satır sabit kalır |
| D16 | **Kaydet** (XLSX'te) ve bilgisayardaki Excel'de açın | Formüller İngilizce standart (`SUM`), Excel'de düzgün açılır |
| D17 | **PDF** (sayfa yapısı, yatay, yazdırma alanı) | Türkçe harfler doğru, ayarlar uygulanmış |

## E. PowerPoint

| No | Ne yapılacak | Beklenen |
|---|---|---|
| E1 | PPTX açıp **Sun** | Tam ekran; durum/gezinme çubukları gizli; ekran kararmaz |
| E2 | Sağa dokun / sola kaydır / sola dokun | Sonraki / önceki slayt; geçiş anında |
| E3 | Ekrana **basılı tutup** parmağı gezdirin | Kırmızı lazer noktası parmağı izler |
| E4 | Ortaya dokunun → **Notlar** | Süre sayacı ve konuşmacı notları görünür |
| E5 | **Düzenle** → bir başlığa dokunun → metni değiştirin → **Uygula** | Başlık değişir, biçimi (renk, boyut) korunur |
| E6 | Boş bir yere dokunun | "Burada yazı yok" uyarısı |
| E7 | **Yeni slayt**, **Çoğalt**, ▲ ▼, **Sil** | Slaytlar eklenir/kopyalanır/taşınır/silinir; son slayt silinemez |
| E8 | **Kaydet** ve PowerPoint'te/başka uygulamada açın | Değişiklikler var, dosya bozuk değil |

## F. PDF

| No | Ne yapılacak | Beklenen |
|---|---|---|
| F1 | **Ara** (Android sürümü ne olursa olsun) `istanbul`, `cicek` | İSTANBUL ve Çiçek bulunur, sarı işaretlenir |
| F2 | **Okuma → Sesli oku** ve **Metin görünümü** | Türkçe okur; metin ekrana akar |
| F3 | **Sayfalar → Seç**: döndür, sil, ayıkla, **PDF ekle** | İşlemler doğru; **Kaydet** yeni dosya (`-duzenlendi.pdf`) |
| F4 | **İmzala** → çiz → **Bitti** → sayfaya dokun | İmza dokunulan yerde; döndürülmüş sayfada da düz |
| F5 | İmzayı **sakla** işaretli bırakın, tekrar **İmzala** | Kayıtlı imza üstte; dokununca kullanılır; silinebilir |
| F6 | **Çiz** → Kalem/Kırmızı/Mavi/Fosforlu → çizin → **Bitti** | Çizgiler yerinde kalır; fosforlu yazıyı okunur bırakır |
| F7 | **Çiz → Not** → sayfaya dokun → Türkçe not | Not simgesi eklenir |
| F8 | **Geri al** (çizim modunda) | Bu oturumda eklenen son çizim/not kalkar; dosyada önceden olanlara dokunmaz |
| F9 | **Formu doldur** (gerçek bir form) → Türkçe ad soyad, onay kutusu → **Uygula** → **Kaydet** | Alanlarda Ş, Ğ, İ, ı doğru; başka PDF uygulamasında da doğru görünür |
| F10 | **Parola → Parolayla koru** → Kaydet → kaydedilen dosyayı açın | Parola sorulur; yanlış parolada "Parola yanlış" |
| F11 | Parolalı bir PDF açın | Parola sorulur; doğru parolayla açılır, arama çalışır |
| F12 | **Küçült → Standart / En küçük** (fotoğraflı PDF) | "12 MB → 3 MB" gibi sonuç; küçük dosyada "zaten küçük" |
| F13 | **Karart**: T.C. kimlik, IBAN, telefon, e-posta + bir isim → **Bul ve işaretle** | Bulunan yerler kırmızı çerçeveli; "N yer işaretlendi" |
| F14 | Karartmadan **Kaydet**'e basın | Uyarı çıkar, karartma paneli açılır (bilgi yanlışlıkla kaydedilmez) |
| F15 | **Kalıcı olarak karart** → **Kaydet** → yeni dosyada aynı numarayı arayın | Siyah kutular var; **arama bulamaz** (metin gerçekten silinmiş) |
| F16 | **İşaretleri kaldır** | Çerçeveler gider, metin yerinde |
| F17 | **Dönüştür → TXT** ve **JPEG (ZIP)** | Metin dosyası ve her sayfa için bir JPEG içeren ZIP |
| F18 | Yazdır | Yazdırma ekranı açılır (parolalı/düzenlenmiş PDF'te de) |

## G. Tarama ve resimden PDF

| No | Ne yapılacak | Beklenen |
|---|---|---|
| G1 | **Tara → Fotoğraf çek** (masada bir kağıt, hafif gölgeli) | Telefonun kamerası açılır; çekince sayfa gelir |
| G2 | Dört mavi köşeyi kağıdın köşelerine sürükleyin | Köşeler parmağı izler |
| G3 | **Belge** / **Siyah-beyaz** / **Renkli** | Belge: arka plan beyaz, gölge kalkar, yazı net |
| G4 | **Döndür**, **Sil**, **+ Fotoğraf çek**, **Galeriden** | Sayfalar eklenir; alttaki küçük resimlerden geçilir |
| G5 | **PDF yap** → kaydet | Eğik çekilen sayfa düzelmiş A4 PDF açılır |
| G6 | **Resimden PDF** → 3 fotoğraf (biri yatay) → **A4 sayfalara** | Yatay foto yatay sayfada; telefon fotoğrafları dik/doğru yönde |
| G7 | Aynısını **Her sayfa resim boyutunda** | Sayfalar fotoğraf boyutunda |

## H. Diğer (önceki sürümlerden, kısa kontrol)

| No | Ne yapılacak | Beklenen |
|---|---|---|
| H1 | UDF belgesi aç, PDF/DOCX'e dönüştür | Biçim UYAP'takiyle aynı |
| H2 | e-Fatura XML aç | Özet kartı ve faturanın kendi tasarımı |
| H3 | TXT aç, düzenle, kaydet; **Dönüştür → PDF** (sayfa yapısı sorar) | Türkçe karakterler korunur |
| H4 | DWG/DXF aç | Çizim görünür, yakınlaştırma çalışır |
| H5 | Yakınlaştırma, gece modu, ses tuşlarıyla sayfa çevirme | Titreme yok; ayarlar hatırlanır |

---

## Geri bildirim için

1. **Çalışmayan maddeler:** numara + ne oldu + ekran görüntüsü.
2. **Yavaş olanlar:** hangi dosya, kaç sayfa / MB, yaklaşık süre.
3. **Görünüşü yanlış olanlar:** Excel/Word/PowerPoint'te bilgisayardakinden farklı görünen dosyanın ekran görüntüsü.
4. **Kullanımı zor gelenler:** düğme yeri, isim, eksik bir adım.
5. **Eski tablet** (32 bit sürüm): açılış hızı ve yakınlaştırma.

Test sonucuna göre bir sonraki aşamanın planı çıkarılacak. Aşağıdakiler, testlerden önce bilinen eksiklerdir:

- Türkçe OCR (taranmış belgede metin tanıma) henüz yok.
- Word'de yorum ve değişiklik izleme yok.
- Excel'de grafik, filtre ve otomatik doldurma yok.
- PowerPoint'te şekil ya da resim taşıma yok.
- Word'den alınan PDF'lerde emoji yazı tipi yok.
