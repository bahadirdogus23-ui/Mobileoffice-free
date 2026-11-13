# CadCraft: büyük DWG/DXF dosyalarında açma süresi ve bellek

Sabitlenmiş sürüm: `cadcraft` @ `56542407292b911ad30523cfcfa987923fe2f8e2` (`rust/Cargo.toml`).
Satır numaraları bu sürüme göredir (`~/.cargo/git/checkouts/cadcraft-*/5654240/`).

## Belirti (sahada)

S25 Ultra sınıfı telefonda 16,9 MB'lık DWG 10,6–10,9 sn'de açılıyor, yerel bellek 1845 MB'a kadar
çıkıyor (toplam PSS 763 MB). 4–6 GB RAM'li telefonlarda uygulamanın sistemce öldürülme riski var.
Bir görünüm çizimi ortalama 295 ms, en çok 1267 ms sürüyordu.

## Ölçüm (masaüstü, 4 çekirdek, release; test sırasında üretilen sentetik çizim)

612.500 nesne (çizgi, LWPOLYLINE, yay, daire, metin, düz ve ANSI31 tarama, iç içe blok
eklemeleri, kesikli çizgi tipi) → motorun kendi yazıcısıyla 18,2 MB DWG ve 118 MB DXF.

| Adım | Süre | Bellek |
|---|---|---|
| `cadcraft_io::read` (DWG) | 8,0–8,6 sn | tepe (VmHWM) **3,36 GB**; çizim açıldıktan sonra kalıcı ≈ 585 MB |
| — bunun DWG→DXF çevirisi (callgrind, 2,5 MB'lık küçük örnek) | işin ≈ %29'u | DXF metni bütünüyle bellekte |
| — DXF ayrıştırma + çizime yerleştirme | işin ≈ %71'i | bütün etiket (tag) dizisi bellekte |
| 118 MB DXF doğrudan | 4,5 sn | tepe 2,6 GB |

Açılıştaki süre ve tepe belleğin tamamı motorun içinde; bizim katmanımız (mo-cad, mo-ffi, Kotlin)
dosyanın baytlarını yalnızca birer kez kopyalıyor (≈ 18 MB) ve onları açılış biter bitmez bırakıyor.

## Motordaki nedenler (dosya:satır)

1. **DWG önce bütün bir DXF metnine çevriliyor, sonra o metin ayrıştırılıyor.**
   `crates/io/src/lib.rs:66-68` — `cadcraft_dwg::dwg_to_dxf_noted(...)` ardından
   `dxf_read::read(&dxf)`. `crates/dwg/src/lib.rs:90-115`: acadrust belgesi `CappedWriter`'a
   (`Vec<u8>`) DXF olarak yazılıyor; bu metin DWG'nin ≈ 6–7 katı. Aynı anda bellekte: acadrust'ın
   belge modeli + DXF metni + (aşağıdaki) etiket dizisi + oluşan `Drawing`.
   *Öneri:* acadrust belgesinden `Drawing`'e doğrudan dönüşüm ya da en azından DXF'i akış hâlinde
   ayrıştırma; acadrust belgesini DXF yazıldıktan hemen sonra bırakma.
2. **DXF'in bütün etiketleri tek bir `Vec<Tag>` olarak tutuluyor.**
   `crates/io/src/dxf_read.rs:80` — `cadcraft_dxf::parse(bytes)` (`crates/dxf/src/lib.rs:173`)
   tüm dosyayı etiketlere çeviriyor; bölümler (`sections`) ve kayıtlar (`records`) bunun üzerinde
   ayrıca dilimleniyor. Ayrıca `crates/io/src/dxf_raw.rs` (`dxf_raw::read`, `head`, `norm`) kayıt
   için ham verileri yeniden biçimlendirip saklıyor (küçük örnekte okuma işinin ≈ %15'i). Salt
   görüntüleyen bir uygulama için bu ham kopyayı atlayan bir seçenek yok.
3. **Başlık değişkenine her bakış bir `String` ayırıyor.**
   `crates/doc/src/header.rs:58-59` — `Header::get` her çağrıda
   `name.trim_start_matches('$').to_ascii_uppercase()` ile yeni bir dize üretip `BTreeMap`'te
   arıyor. Görüntüleme listesi kurulurken nesne başına çağrılıyor
   (`crates/render/src/props.rs:37` `LTSCALE`, `:57` `LWDEFAULT`): küçük örnekte liste kurmanın
   ≈ %29'u (`resolve_common` içinde). *Öneri:* sık okunan değişkenleri `Builder`'da bir kez okuyup
   saklamak ya da anahtarları büyük harfle sabit `&str` olarak aramak.
4. **CPU raster çizici görünüşe göre ayıklama/ayrıntı düzeyi uygulamıyor.**
   `crates/render/src/raster.rs:74-167` — her çizimde listenin bütün öğeleri dolaşılıyor;
   çoklu çizginin sınır kutusu her karede noktalarından yeniden hesaplanıyor (`:95`), üçgenler
   (`Kind::Tris`, metin dolguları, düz taramalar) hiç ayıklanmıyor (`:120-138`), piksel altı öğeler
   tek tek kenar yumuşatmalı çizgi olarak çiziliyor. Tam görünümde 3,5 milyon öğe → 3 sn.
5. **Ekran için ayrıntı düzeyi (`Options::min_dash`, `Options::view`) varsayılan olarak kapalı.**
   `crates/render/src/pipeline.rs:49-65`. Açılmazsa her kesik çizgi parçası ve taramanın her desen
   çizgisi bütün çizim için listede tutuluyor (`crates/render/src/hatch.rs:5-9`). Bu bizim
   tarafımızda düzeltildi (aşağıda); motorun kendi tuvali de böyle kullanıyor
   (`crates/ui-egui/src/canvas/list.rs:125-139`).
6. **Nesne yakalama için uzamsal dizin genel bir önbellekte duruyor** ve belge kapansa da kalıyor
   (`crates/engine/src/spatial.rs:221-254`, `CACHE_SIZE = 4`). Biz belge kapanırken
   `spatial::clear_cache()` çağırıyoruz.

## Bizim tarafta yapılanlar (motoru yamamadan, yalnızca açık API ile)

- `mo-cad`: her alan için bir **genel bakış listesi**, motorun ekran ayrıntı düzeyiyle kuruluyor
  (eğri toleransı ve 2 piksellik kesik/desen eşiği, alanın 6000 piksel genişliğe kadar olan
  görünümlerine göre). Liste: 3,55 M öğe / 10,85 M nokta (≈ 288 MB) → 2,29 M / 8,30 M (≈ 208 MB).
- Daha yakın görünümlerde model alanı, motorun uzamsal dizininin (`SpatialIndex::query`)
  bulduğu, görünümün çevresindeki nesnelerden `build_entities` ile kurulan bir **ayrıntı
  listesinden** çiziliyor (yarım piksel eğri ayrıntısı, tarama desenleri yalnızca pencerede);
  görünüm pencereden çıkınca ya da yakınlaştırma 2 kat değişince yeniden kuruluyor; uzaklaşınca
  bırakılıyor.
- Her kare çizilmeden önce görünmeyen öğeler atılıyor, yarım pikselden yakın noktalar
  düşürülüyor, piksel altı öğeler piksel başına tek nokta oluyor. Kare dizileri ve piksel tamponu
  kareden kareye yeniden kullanılıyor; piksel dizisi kopyalanmadan (`Pixmap::take`) veriliyor.
- Kotlin: aynı anda tek çizim; çizim sürerken istenen görünümler sıraya dizilmiyor, bitince
  yalnızca en yenisi çiziliyor. Bir önceki bitmap sonraki çizimin tuvali olarak yeniden
  kullanılıyor. Dosya, sağlayıcının bildirdiği boyutta tek diziye okunuyor.

| Sentetik DWG (18,2 MB, 612 bin nesne), 1080×2340 | Önce | Sonra |
|---|---|---|
| Açma (`CadDoc::open`) | 8,1 sn | 8,1–8,6 sn (motor) |
| Tam görünüm çizimi | 3070–3200 ms | 530–570 ms |
| ×4 | 700 ms | 280–360 ms |
| ×16 | 191 ms | 58 ms (ilk kez 620 ms: uzamsal dizin + ayrıntı listesi) |
| ×64 | 133 ms | 8 ms |
| ×256 | 125 ms | 3 ms |
| Katman gizle + çiz | 3,7 sn | 1,1 sn |
| Görüntüleme listesi | ≈ 288 MB | ≈ 208 MB |

Açılıştaki 3,36 GB tepe (sahadaki 1845 MB) yukarıdaki 1. ve 2. maddeler düzelmeden düşmez.

## Açmadan önce bellek denetimi (uygulamada)

Motor düzelene kadar tepe bellek düşmediği için `CadViewModel` çizimi motora vermeden önce
tepeyi tahmin ediyor (`CadMemory.kt`) ve `ActivityManager.getMemoryInfo()` ile karşılaştırıyor.
Ölçüm (masaüstü, release, motorun kendi yazdığı sentetik planlar; dosya başına ayrı süreç, VmHWM):

| Biçim | Dosya → tepe | Dosya baytı başına |
|---|---|---|
| DWG | 1,0 MB → 185 MB · 2,5 MB → 465 MB · 4,8 MB → 888 MB · 14,9 MB → 2,7 GB · 18,2 MB → 3,5 GB | 194–201× |
| ASCII DXF | 6 MB → 142 MB · 16 MB → 363 MB · 31 MB → 691 MB · 96 MB → 2,1 GB · 118 MB → 2,6 GB | 23–25× |
| İkili DXF | 4,5 MB → 140 MB · 23 MB → 684 MB · 71 MB → 2,1 GB | 31–33× |

Tepe dosya boyutuyla doğrusal. Tahmin = 32 MB + dosya × (DWG 200, DXF 24, ikili DXF 32);
bütçe = `availMem` − en az 256 MB ya da Android'in `threshold` değeri (hangisi büyükse).
Tahmin bütçeyi aşarsa kişiye sorulur ("Yine de aç" / "Vazgeç"); "Yine de aç"tan önce diğer
açık belgelerin kurtarma kopyası alınır (`RecoveryHub.copyAllAndWait`, en çok 5 sn).
Karar, tahmin ve açılıştan sonraki gerçek VmHWM ölçüm raporuna olay olarak yazılır
("CAD bellek tahmini: DWG 17.4 MB → 3.4 GB, boş 2.9 GB (eşik 216 MB) → soruldu"); gerçek
telefonlardaki oranlar bu olaylardan izlenip katsayılar gerekirse düzeltilir.
