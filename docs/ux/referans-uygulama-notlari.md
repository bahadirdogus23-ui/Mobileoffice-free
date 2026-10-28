# Referans uygulama notları ("Ofis Okuyucu Plus", 28.10.2026 ekran görüntüleri)

Proje sahibinin paylaştığı ekran görüntülerinden alınacaklar ve bizim farkımız.

## Alacaklarımız

| Görülen | Bizde |
|---|---|
| Altta **Ev / Word / Excel / PPoint / PDF** sekmeleri | Var (Faz 0): `HomeScreen`, sekme başına filtre |
| Satırda küçük resim, ad, **tür · boyut · tarih** | Tür rozeti + tür · boyut · tarih var; sayfa küçük resmi Faz 1 |
| **"Görüntülenen %50"** okuma ilerlemesi | Var: Word görüntüleyici kaydırma konumunu kaydediyor |
| Satır menüsü (⋮) | Var: Paylaş, Listeden kaldır; Yıldızla, Yeniden adlandır Faz 1 |
| Görüntüleyicide başlık + **"1/2"** sayfa sayacı | Var (Word, PDF, sunum); sayaca dokununca "Sayfaya git" |
| Alt çubuk: **Okuma · Sayfalar · Paylaş · Dönüştür · Diğer** | Okuma, Sayfaya git, Yazdır, Paylaş var; Dönüştür Faz 2 |
| Okuma ayarları: gece modu, sayfa numarası, ekranı açık tut, döndürme kilidi, dikey/yatay kaydırma, ses tuşlarıyla sayfa çevirme | Gece modu, ekranı açık tut, ses tuşları var; iki parmakla ve çift dokunarak yakınlaştırma var; döndürme kilidi ve yatay kaydırma Faz 1 |
| Sayfalar: sayfaya git, küçük resimler, kelime/karakter sayısı | Faz 1 (WordCraft kelime sayımı hazır) |
| Diğer: yazdır, yıldızla, kısayol, Drive'a kaydet, sil, özellikler | Faz 1; "Drive'a kaydet" yerine sistem paylaşımı (internet izni yok) |

## Bizim farkımız

- **Reklam yok.** Referansta "Dönüştür" özelliği *"Bu özelliği kullanmak için bir reklam izleyin"* ile kilitli. Bizde tüm dönüştürmeler ücretsiz ve reklamsız.
- **Depolama izni yok.** Referans cihazdaki bütün belgeleri listeliyor (geniş depolama izni). Biz yalnız kullanıcının açtığı dosyaları ve izin verdiği klasörleri gösteriyoruz (rapor §16, karar 5).
- **Düzenleme.** Referans yalnız görüntülüyor; biz Office dosyalarını düzenleyeceğiz (Faz 3).
- **Gece modu.** Referansta sayfa yalnız griye boyanıyor; bizde motor sayfayı gerçek koyu renklerle çizebiliyor (WordCraft `RenderOptions.dark`).
