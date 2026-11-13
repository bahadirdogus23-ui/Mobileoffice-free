# Test Paneli 3: 11 Kasım 2026 test paketi

Kaynak: kullanıcının paylaştığı `MobileOffice Test Paneli 3.html` (32 madde).
Kullanıcı bu paneli henüz uygulamadığını belirtti. Önceki paneldeki sonuçlar
bu panelin farklı kodlarına taşınmaz. Aşağıdaki maddelerin tamamı cihazda bekliyor:

## 12 Kasım: biriktirilen değişiklikler

Bu bölümdeki değişiklikler 12 Kasım güncel ARM64 test APK'sına dahil edildi.
Yerel motor yeniden derlendi; `clean assembleDebug testDebugUnitTest lintDebug`
başarılı. 74 Android testi, imza, paket ve 16 KB hizalama kontrolleri geçti.
`MobileOffice-20261112.apk`: 157.063.706 bayt, SHA-256
`5e78a54fa2d4f07f07d279b5d91a1b4f6be4131a6a27768aec76578cdb6fa8c0`.
İndirme alternatifi `MobileOffice-20261112.zip`: 60.236.859 bayt;
içindeki APK'nın aynı özete sahip olduğu doğrulandı. Dosyalar yerelde
`/workspace/shared/downloads/` altında. Telefon uygulamasından indirme ve
cihaz kurulumu doğrulanmadı. GitHub'a veya mağazaya yayın yapılmadı.

- Excel panosu: biçim/formül içeren yerel kopya yalnız ait olduğu çalışma
  kitabında kullanılır. Başka kitapta aynı telefon panosu metni açıkça metin
  olarak gönderilir. Temizlenmiş telefon panosu eski yerel kopyayı yapıştırmaz.
  Hatalı kopyalama boş sonuç gibi işlenmez; gecikmiş eski kitap yanıtı yeni
  kitabın pano durumunu değiştirmez. Motorun boş hücre kopyası satır sonu
  üretir; bu örnekte önceki panonun kalması yeniden üretilemedi.
- Excel kaydı: kayıt isteğinden sonra yeni düzenleme istenmişse çalışma kitabı
  temiz sayılmaz, kurtarma kopyası silinmez ve kayıt sonrası kapatılmaz.
  Önceki kitaba ait düzenleme/kayıt sonuçları yeni kitaba uygulanmaz.
- Excel sayfa/hücre değişimi: gecikmiş sonuç başka kitabı veya sayfayı
  yeniden seçmez; hücrenin düzenleme metni yalnız aynı kitap, sayfa ve hücreye
  uygulanır. Hızlı sayfa seçiminde yalnız son sayfa isteği gösterilir.
- Ek motor testleri: aynı kitapta göreli formül ve kalın biçim korunarak
  yapıştırma; dolu hücreyi kopyalanan boş hücreyle temizleme; başka kitaba
  Türkçe metin ve sayısal değer aktarma. Kitaplar arası biçim/formül aktarımı
  bu değişiklikle eklenmedi; bu yol metin aktarımıdır.

- PDF metin güncellemesi önce belge kopyasında tamamlanır. Boş metin gibi
  reddedilen bir güncelleme özgün metni ve geri alma kaydını artık silmez;
  hata sonrası kaydet/aç ve geri alma testi eklendi. Bu motor değişikliği
  sonraki APK'dan önce Android yerel kütüphanesinin yeniden derlenmesini gerektirir.
- PDF kaydı aynı belge/revizyon için başarılı olursa kurtarma kopyası temizlenir.
  Önizleme oluşturma başarısız olsa da motorda uygulanmış değişiklik kaydedilmemiş
  sayılır. Gecikmiş metin seçimi sonuçları ve eski açılış sonuçları uygulanmaz.
  Aynı URI yeniden açılsa bile yeni oturum kimliği kullanılır. Belge kapanınca
  seçili metin, form, sıkıştırma kopyası ve seçim bilgileri sıfırlanır.

- Önceki panel Y17/H2: e-fatura ekranının A4 kâğıt/masa stilleri yalnız
  `screen` ortamına uygulanır. Yazdırmada satıcının genişlik, kenar boşluğu ve
  sayfa sonu kuralları korunur. Satıcı tasarımı olmayan faturalar da ekranda
  aynı A4 genişliğini kullanır. Chromium kontrolünde önce 640 px genişliğe
  düşme hatası üretildi; düzeltmeden sonra satıcı, HTML parçası ve sade tasarım
  mobil/masaüstünde 794 px olarak ölçüldü. `print` ortamında satıcının özgün
  640 px genişliği ve 7 px kenar boşluğu korundu. Gerçek Android WebView ve
  yazıcı testi hâlâ cihazda yapılmalı.
- Fatura yükleme işlemi yeni açılışta veya kapatmada iptal edilir; eski sonuç
  yeniden açılmaz, eski hata ve yükleniyor durumu temizlenir.
- Tarama: çaprazlaşan, çöken, içbükey veya görüntü dışındaki köşeler kabul
  edilmez. Perspektif dönüşümü başarısızsa yanlış görüntüyle PDF yazılmaz.
  Döndürme/küçültme sonrası kullanılmayan büyük bitmap kopyaları serbest bırakılır.
- Tarama PDF'si yazılırken kapatma ve sayfa düzenleme engellenir; kaynak
  fotoğraflar işlem sürerken silinmez. Hatalı kayıttan sonra sayfalar korunur.
- Kontrast ve parlaklık zaten mevcut; tekrar özellik eklenmedi.

Bağımsız tarayıcı kontrolü için `scripts/check-invoice-layout.py` kullanılır;
Python Playwright ve Chromium gerekir. HTML çıktıları InvoiceTest içindeki
`MO_INVOICE_HTML_DIR` ortam değişkeniyle alınır. Masaüstü Chromium kontrolü
Android WebView ile aynı ortam değildir.

| Grup | Maddeler | Kapsam |
| --- | --- | --- |
| Excel | X1–X9 | Biçim, formül başvuruları, sütun/aralık seçimi, otomatik kaydırma, grafikler, not yazarı, tablo, açılır liste, yinelenenler |
| Sunu | S1–S8 | Nesne taşıma/boyutlandırma, tablo, yazı tipi, slaytlar arası taşıma, hücre düzenleme, konuşmacı notu, ODP |
| Word | W1 | DOCX içine eklenen tablonun çizgileri |
| PDF | P1–P5 | Seçim tutamaçları, metin eklerken konum, birleştirme listesi, çoklu seçim, araç adları |
| CAD | C1–C2 | Orta noktaya yakalama, görünen alanın PDF çıktısı |
| Genel | Z1–Z7 | Kurtarma, bildirim seçenekleri, ölçümler ve kalıcılığı, kayıt temizleme, ana ekran, şablonlar |

Bu pakette son eklenen düzeltmeler:

- Çoklu satır/sütun seçimi (12 Kasım): ekle/sil işlemleri yalnız seçimin
  başlangıcını değil tamamını kullanır. Ters yönde seçim ve başlık seçimi
  testleri eklendi. Motor testinde iki satır/sütun ekleyip silerken çevredeki
  verilerin doğru taşındığı ve korunduğu doğrulandı.
- D4/D6 (önceki panel, 12 Kasım): genel hizalama ekrandaki metnin ilk
  karakterinden tahmin edilmez. Hücrenin gerçek değer türüne göre sayılar
  sağa, mantıksal değerler/hatalar ortaya hizalanır. Sayıyla veya eksiyle
  başlayan metinler metin olarak kalır; boş komşulara taşma hesabına katılır.
  Kullanıcının açıkça seçtiği hizalama korunur. XLSX kaydet/aç testi eklendi.
- Z4/Z5 (12 Kasım): bozuk ölçüm satırları diğer kayıtların yüklenmesini
  durdurmaz; eski süre kaydı biçimi okunmaya devam eder. Dosya yenilenemezse
  hata algılanır ve yazma yeniden zamanlanır. Temizleme, anlık kayıt oluşturma
  ve yazıcıya gönderme aynı kilitle sıralanır. Raporun süre sayaçları kopyalanır.
- S6: seçili tablonun/metin kutusunun metnini düzenleme yalnız o nesnenin
  kimliği içinde hedef arar; üstüne denk gelen başka kutuya geçmez.
- P1: seçim tutamacı karşı ucu geçtiğinde sabit uç sürükleme başındaki yerde
  korunur. Yeni seçim istekleri kuyruktaki eski koordinatları geçersiz kılar;
  başka belgeye geçilince eski sonuç arayüze uygulanmaz.
- P3: birleştirmede kaydetme ekranından vazgeçmek dosyaları ve sıralamayı
  silmez; yazma hatasında da yeniden denemek için liste korunur.
- X4: seçim veri alanından öteye uzadıkça kaydırılabilir boş alan genişler.
  Otomatik kaydırma döngüsü başlangıçtaki satır/sütun sınırlarında kalmaz.
  Çok dar veya henüz ölçülmemiş görünümde geçersiz koordinat aralığı oluşturulmaz.
- Formülde tırnak içindeki metin ve sayfa adları yazılırken işlev önerileri
  çıkmaz; hücreye dokunmak metnin içine adres eklemez veya yarım formülü
  kaydetmeye çalışmaz. Çift tırnak/tek tırnak kaçışları da tanınır.
- X2: başka sayfaya ait formül aralıklarının son hücresi mevcut sayfada
  işaretlenmez. Excel'in satır/sütun sınırı dışındaki adresler elenir;
  Unicode adların içindeki adres benzeri parçalar başvuru sayılmaz.
- UDF dahil dizine eklenmemiş dosyaların yalnız ilk 300 taneyle sınırlanması
  giderildi. MediaStore'a gönderilen parti sınırlı kalır; kalan dosyalar doğrudan
  listelenir. SD kart kökleri de taranır ve ana ekrana dönüşte klasörler yenilenir.
- PDF formunun açıklaması `text` gibi genel bir değer olduğunda anlamlı alan adı
  veya sayfada basılı etiket kullanılır. Kaynak formun adı ve açıklaması değiştirilmez.
- PDF kimlik/telefon/IBAN aramalarında tipografik boşluklar tanınır; satır ve
  tablo ayırıcıları üzerinden farklı sayılar birleştirilmez.
- Kurtarma kopyası yazılamadığında başarı zamanı güncellenmez. Kuyruktaki
  kurtarma işlemi belge değişmişse atlanır; başarısız dosya değiştirme bildirilir.

Z1 özellikle cihazda yeniden denenmeli: otomatik kurtarma anlık değildir ve
Android işlemi sonlandırmadan önce bekleyen işi tamamlamayabilir. Kaydedilmemiş
tek kopyası önemli olan bir belge yerine deneme kopyası kullanın.

Doğrulama: 74 Android birim testi ve Android lint başarılı; 34 mo-pdf testi ve
mo-pdf Clippy başarılı. ARM64 motor bu kaynaklarla yeniden derlendi, yerel
kütüphanelerin 16 KB hizalama kontrolü geçti. Bunlar cihaz testlerinin yerine geçmez.
12 Kasım hizalama, çoklu satır/sütun ve pano değişiklikleri için ayrıca 38 mo-grid testi ve mo-grid Clippy
başarılı; yeni ARM64 motorlu APK'nın imza, paket ve hizalama kontrolleri geçti.

Paket yalnız ARM64 Android test kurulumu içindir; debug anahtarıyla imzalanır.
Önceki kurulum başka anahtarla imzalıysa Android üzerine kurulumu reddedebilir.
Bu durumda veri kaybetmemek için mevcut uygulamayı kaldırmadan önce imza ve
dosya/kurtarma durumunu değerlendirin. Mağazaya veya GitHub'a yayın yapılmadı.
