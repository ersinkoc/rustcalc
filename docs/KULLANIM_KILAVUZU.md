# RustCalc kullanım kılavuzu

Bu kılavuz RustCalc'ın her bölümünü örneklerle anlatır. Aynı rehber uygulamanın
içinde de var: **F1**'e basın veya kenar çubuğunda **? Yardım**'ı seçin. Dili
kenar çubuğundaki **EN / TR** ile değiştirebilirsiniz; seçiminiz kaydedilir.
[English user guide](USER_GUIDE.md)

- [Hesap makinesi](#hesap-makinesi)
- [Yüzde](#yüzde)
- [Tam ve yaklaşık sonuçlar](#tam-ve-yaklaşık-sonuçlar)
- [Fonksiyonlar](#fonksiyonlar)
- [Tarihler](#tarihler)
- [Bellek ve geçmiş](#bellek-ve-geçmiş)
- [Formüller](#formüller)
- [Birim dönüştürücü](#birim-dönüştürücü)
- [Programcı](#programcı)
- [Grafik](#grafik)
- [Ayarlar ve güncellemeler](#ayarlar-ve-güncellemeler)
- [Sık sorulan sorular](#sık-sorulan-sorular)

## Hesap makinesi

Bir ifade yazın; sonuç siz yazarken **CANLI ÖNİZLEME** alanında görünür.
**Enter**'a (veya `=`) basınca sonuç kaydedilir ve geçmişe eklenir. Yazmadan
önce giriş alanına tıklamanız gerekmez; ekrandaki tuşları fareyle de
kullanabilirsiniz. **Standart** temel tuşları, **Bilimsel** fonksiyonları da
gösterir.

Bir sonuçtan sonra:

- bir sayı yazarsanız yeni hesap başlar;
- bir işlem yazarsanız (`+ - * / ^ %`) sonuçtan (`ans`) devam edilir.

Giriş alanı normal bir metin kutusudur: imleci taşıyabilir, seçebilir,
yapıştırabilir, Ctrl+Z ile geri alabilirsiniz. Tuşlar imlecin olduğu yere yazar
veya seçili metnin yerine geçer; **Sil** imleçten önceki karakteri, **AC** (veya
Esc) her şeyi temizler.

### İşlem sırası

| Yazdığınız | Sonuç | Neden |
| --- | --- | --- |
| `2 + 3 * 4` | `14` | Çarpma ve bölme, toplama ve çıkarmadan önce yapılır |
| `(2 + 3) * 4` | `20` | Önce parantezler |
| `2^3^2` | `512` | Üsler sağdan gruplanır: 2^(3^2) |
| `-2^2` | `-4` | Üs, eksi işaretinden önce uygulanır |
| `(-2)^2` | `4` | Negatif sayının karesi için parantez kullanın |
| `3!^2` | `36` | Faktöriyel ve yüzde en önce uygulanır |
| `2(3+4)` | `14` | Paranteze bitişik sayı çarpılır |
| `24 / 4 / 2` | `3` | Aynı düzeydeki işlemler soldan sağa yapılır |

### Ondalık ayırıcı

Virgül veya nokta kullanabilirsiniz: `1,5` ve `1.5` ikisi de bir buçuktur.
**Binlik ayırıcı yazmayın:** `1.000` bin değil, birdir. Bin için `1000` yazın.
`1e3` 1000, `2,5e-3` 0,0025 demektir.

## Yüzde

RustCalc yüzdeyi alışverişte kullandığımız hesap makineleri gibi hesaplar: bir
sayıya yüzde eklemek veya çıkarmak, sayıyı kendisinin o payı kadar değiştirir.

| Yazdığınız | Sonuç | Anlamı |
| --- | --- | --- |
| `100 + 10%` | `110` | 100'e, 100'ün %10'u eklenir |
| `100 - 10%` | `90` | 100'den, 100'ün %10'u düşülür |
| `250 - 15%` | `212.5` | 250'ye %15 indirim |
| `1000 + 20%` | `1200` | KDV dahil fiyat |
| `100 * 10%` | `10` | 100'ün %10'u |
| `100 / 10%` | `1000` | 100'ün 0,1'e bölümü |
| `10%` | `0.1` | Tek başına yüzde bir kesirdir |
| `100 + 10% + 10%` | `121` | Her adım ara toplamı kullanır |
| `200 * (1 + 10%)` | `220` | Parantez içinde 1 + 1'in %10'u = 1,1 |
| `100 + (5 + 5)%` | `110` | Yüzde parantezli bir ifade olabilir |

Kural, `+` veya `-` işaretinden hemen sonra bir yüzde geldiğinde uygulanır.
`100 + 10% * 2` ifadesinde yüzde bir çarpımın parçasıdır (0,1 × 2), bu yüzden
sonuç 100,2 olur. Emin değilseniz payı açıkça yazın: `100 + 100 * 10%`.

## Tam ve yaklaşık sonuçlar

Normal aritmetik tam ondalık hesaplanır:

| Yazdığınız | Sonuç |
| --- | --- |
| `0.1 + 0.2` | `0.3` |
| `19.99 * 3` | `59.97` |
| `1 / 8` | `0.125` |

Sonuçlar en fazla 28 ondalık basamak içerebilir ve yaklaşık ±7,9 × 10²⁸
büyüklüğüne kadar çıkabilir. `1/3` gibi bitmeyen ondalıklar 28. basamakta
yuvarlanır.

Bilimsel fonksiyonlar (kök, logaritma, trigonometri, tam sayı olmayan üsler)
yaklaşık 15 anlamlı basamakla hesaplanır ve **≈** ile işaretlenir. Sonucun tam
olduğu kanıtlanabiliyorsa ≈ görünmez:

| Yazdığınız | Sonuç |
| --- | --- |
| `sqrt(144)` | `12` |
| `log(1000)` | `3` |
| `sin(30)` | `0.5` |
| `asin(0.5)` | `30` |

`sqrt(2)` ise ≈ 1.4142135623731 gösterir. Bu işaret değerle birlikte `ans`,
bellek ve geçmişe taşınır.

Sıfıra bölme, `sqrt(-1)`, `tan(90)` veya desteklenen aralığı aşan sonuçlar
sonucun altında hata olarak gösterilir; hiçbir şey kaydedilmez.

## Fonksiyonlar

Argümanları **`;`** ile ayırın, çünkü virgül ondalık ayırıcıdır.

| Yazdığınız | Sonuç | Fonksiyon |
| --- | --- | --- |
| `sqrt(16)` | `4` | Karekök (küpkök için `cbrt`) |
| `log(100)` | `2` | 10 tabanında logaritma; `ln` doğal logaritma |
| `log(8; 2)` | `3` | İstediğiniz tabanda logaritma |
| `abs(-7)` | `7` | Mutlak değer |
| `floor(2.7)` | `2` | Aşağı yuvarlar (`ceil` yukarı) |
| `round(2.675; 2)` | `2.68` | Basamağa yuvarlar; tam yarımlar çift basamağa gider |
| `round(2.5)` | `2` | Yarım, çift sayıya yuvarlanır |
| `max(4; 9; 2)` | `9` | En büyük (`min`, `sum`, `avg` da var) |
| `avg(2; 4; 9)` | `5` | Ortalama |
| `mod(17; 5)` | `2` | Bölümden kalan; işaret bölenden gelir |
| `gcd(48; 18)` | `6` | En büyük ortak bölen (`lcm` en küçük ortak kat) |
| `ncr(5; 2)` | `10` | Kombinasyon (permütasyon için `npr`) |
| `5!` | `120` | Faktöriyel, 0 ile 27 arası |
| `if(7 > 5; 1; 0)` | `1` | Koşula göre değer seçer |

Karşılaştırmalar (`<`, `>`, `<=`, `>=`, `==`, `!=`) 1 (doğru) veya 0 (yanlış)
verir. `and`, `or` ve `not` bunları birleştirir. `if`, `and` ve `or` yalnızca
gereken kısmı hesaplar; `if(x == 0; 0; 1/x)` asla sıfıra bölmez.

Trigonometri (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`) **DEG/RAD** ayarına
uyar; hiperbolik fonksiyonlar (`sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`)
düz sayı alır. Sabitler: `pi`, `e`, `ans` (son sonuç), `mem` (bellek).

## Tarihler

Tarihler gün sayısı olarak tutulur, bu yüzden normal aritmetik çalışır:

| Yazdığınız | Anlamı |
| --- | --- |
| `date(2026; 12; 31) - today()` | 31 Aralık 2026'ya kalan gün |
| `today() + 90` | 90 gün sonraki tarih (hesap makinesinde gün sayısı olarak görünür) |
| `weekday(today())` | 1 = Pazartesi … 7 = Pazar |
| `years(date(1990; 5; 15); today())` | Bir tarihten bu yana tam yıl (yaş) |
| `workdays(today(); date(2026; 12; 31))` | O tarihe kadar Pazartesi–Cuma günleri |
| `addmonths(date(2024; 1; 31); 1)` | 29 Şubat 2024: ayın 31'i kısa ayda son güne döner |

Hesap makinesi tarihleri gün sayısı olarak gösterir; **Formüller** çalışma alanı
tarih olarak gösterir ve hazır tarih hesapları içerir. Resmî tatiller hesaba
katılmaz. Bütün tarih fonksiyonları [formül rehberinde](FORMULAS.md#dates).

## Bellek ve geçmiş

| Tuş | İşlem |
| --- | --- |
| MS | Geçerli sonucu belleğe kaydeder |
| M+ / M− | Geçerli sonucu belleğe ekler / bellekten çıkarır |
| MR | `mem` değerini ifadeye ekler |
| MC | Belleği temizler |

Geçmiş en fazla 500 işlem saklar; en yenisi en alttadır. Bir sonuca tıklarsanız
o sonuçla yeni hesap başlar; **↗** ifadeyi yeniden düzenlemek, **☆** sabitlemek
(sabitlenenler eskiler silinirken korunur), **Kopyala** kopyalamak, **×** silmek
içindir. Arama kutusu ifadeye veya sonuca göre süzer; **Sabitlenenler** yalnızca
sabitlenmiş kayıtları gösterir.

**Dosyalar…** menüsü JSON yedek kaydeder, okunabilir metin olarak dışa aktarır
veya bir yedeği içe aktarır (birleştirir, tekrarları atlar ve onay ister).
**Silmeyi geri al**, bu oturumda silinen kayıtları veya temizlenen geçmişi geri
getirir.

## Formüller

**05 Formüller** hazır ve size özel hesaplamaları çalıştırır. Soldan bir formül
seçin, değerleri yazın, sonuçları okuyun; sonuçlar siz yazdıkça güncellenir.

- Sayı alanları ifade kabul eder: `1500*12`, `ans` veya önceki bir alanın adı.
- Tarih alanları `2026-10-21`, `21.10.2026`, `21/10/2026` veya `today() + 30`
  gibi ifadeler kabul eder; **Bugün** düğmesi bugünün tarihini yazar.
- Kopyalamak için bir sonuca tıklayın. **Değerleri sıfırla** varsayılanlara döner.

Hazır şablonlar: kredi taksiti, bileşik faiz, birikim planı, KDV ekle, KDV ayır,
kâr marjı, saatlikten yıllık ücrete, iki tarih arası gün, gün ekle veya çıkar,
iş günü ile son tarih, ay veya yıl ekle, yaş, indirim, yüzde değişim, toplamın
yüzdesi, hesabı böl, yolculuk yakıt masrafı, yolculuk süresi, birim fiyat
karşılaştırma, elektrik maliyeti ve vücut kitle indeksi.

### Kendi formülünüzü oluşturmak

1. **+ Yeni formül**'ü seçin (veya bir şablonda **Kopyasını özelleştir**).
2. Formüle bir ad verin; kategori isteğe bağlıdır.
3. **Girdiler**: her girdi için bir değişken adı (ör. `fiyat`), ekranda görünecek
   etiket, tür (sayı veya tarih) ve varsayılan değer girin.
4. **Sonuçlar**: her sonuç için bir ad, etiket ve ifade yazın; ifade girdileri ve
   üstteki sonuçları kullanabilir. Gösterim biçimini seçin: sayı, para (2
   ondalık), yüzde, tarih veya haftanın günü. İsterseniz birim ekleyin (`m²`,
   `TL` gibi).
5. Düzenleyici siz yazarken formülü denetler ve her sonucu varsayılan değerlerle
   önizler. **Formülü kaydet** ile kaydedin.

Örnek — oda boyası:

| Ad | Etiket | İfade |
| --- | --- | --- |
| `en`, `boy`, `yukseklik` | Girdiler (m) | — |
| `alan` | Duvar alanı | `2 * (en + boy) * yukseklik` |
| `boya` | İki kat için boya | `ceil(alan * 2 / 10)` |

Değişken adları harfle başlar; harf, rakam ve `_` içerebilir (Türkçe karakter
kullanmayın: `yukseklik`). Fonksiyon adları ile `pi`, `e`, `ans`, `mem`
kullanılamaz. **Dışa aktar…** formüllerinizi bir dosyaya kaydeder, **İçe aktar…**
başka birinin paylaştığı formülleri ekler. Ayrıntılar:
[formül rehberi](FORMULAS.md) (İngilizce).

## Birim dönüştürücü

Bir kategori seçin (uzunluk, kütle, sıcaklık, alan, hacim, hız, zaman, veri),
üstteki değeri yazın ve birimleri seçin. **Birimleri değiştir** yönü çevirir,
**Sonucu kopyala** sonucu kopyalar. Dönüşüm katsayıları tam tanımlardır (1 mil =
1609,344 m, 1 lb = 0,45359237 kg). Mutlak sıfırın altındaki sıcaklıklar
reddedilir. Veri birimleri SI (1 kB = 1000 B) ve ikili (1 KiB = 1024 B) olarak
ikiye ayrılır.

## Programcı

1. Yazacağınız tabanı seçin: **HEX**, **DEC**, **OCT** veya **BIN**.
2. Kelime boyutunu (**8, 16, 32 veya 64 bit**) ve ikiye tümleyen için
   **İşaretli**'yi seçin.
3. Değeri her tabanda okuyun; kopyalamak için bir değere tıklayın. Bit
   haritasındaki bitlere tıklayarak onları değiştirin.
4. `+ − × ÷ MOD AND OR XOR << n >> n` için ikinci sayıyı (onluk) girin;
   `NOT`, `NEG`, `<< 1` ve `>> 1` yalnızca değer üzerinde çalışır.

Sonuçlar donanımdaki gibi kelime boyutunda başa sarar: işaretli 8 bitte 127 + 1
= −128. Bölme sıfıra doğru keser; işaretli kelimede `>>` işareti korur. DEC
girişi kelimeye sığmalıdır (işaretli 8 bit için −128 ile 127 arası); HEX, OCT ve
BIN girişi en fazla kelime genişliğinde bir bit desenidir. Daha küçük kelimeye
geçmek alt bitleri korur; **İşaretli** anahtarı bitleri değiştirmez, yalnızca
onluk anlamını değiştirir.

## Grafik

**f**, **g** ve **h** olarak x'e bağlı en fazla üç fonksiyon girin (gizlemek için
satırı boş bırakın). Hazır düğmeler **f**'yi doldurur.

- Kaydırmak için sürükleyin; yakınlaştırmak için tekerleği veya iki parmağı
  kullanın; sıfırlamak için çift tıklayın.
- **+ / −** yakınlaştırır, **Görünümü sıfırla** ±10'a döner, **Görünüm ±**
  görünen aralığı ayarlar. Grafiğin üzerindeyken ok tuşları kaydırır, `+`/`-`
  yakınlaştırır, Home sıfırlar.
- Fareyle üzerine gelince her eğrinin o noktadaki değeri görünür.
- **Kökler ve kesişimler** eğrilerin sıfırı veya birbirini kestiği yerleri
  işaretler ve x değerlerini grafiğin altında listeler. Sıfıra değip geri dönen
  eğri (0'da `x^2` gibi) listelenmez.

İki eksen aynı sayısal aralığı gösterir; grafikler her zaman radyan kullanır.
Tanımsız noktalar ve kutuplar sahte dikey çizgi yerine boşluk bırakır. Çizilemeyen
bir fonksiyonun nedeni grafiğin altında yazar.

## Ayarlar ve güncellemeler

- **EN / TR** dili, **Açık / Koyu görünüm** temayı değiştirir; ikisi de
  hatırlanır.
- **Hakkında ve güncellemeler** sürümü gösterir, güncelleme denetler ve otomatik
  güncellemeyi ayarlar: **Kapalı** (varsayılan), **Bana bildir** veya **Otomatik
  kur**. İlk açılışta durum çubuğu bir kez sorar. Otomatik kurulumlar da elle
  kurulumlar gibi doğrulanır ve yeniden başlatınca etkinleşir; RustCalc kendini
  asla kendiliğinden yeniden başlatmaz.
- Komut satırında `rustcalc update --rollback`, son güncellemenin yerine koyduğu
  sürüme geri döner; tekrar çalıştırmak yeni sürüme döndürür.

Her şey bilgisayarınızda kalır: geçmiş, bellek, ayarlar ve formülleriniz
Windows'ta `%APPDATA%\RustCalc\data\app.ron`, Linux'ta
`~/.local/share/rustcalc/app.ron`, macOS'ta
`~/Library/Application Support/RustCalc/app.ron` dosyasında saklanır. Güncelleme
ve kaldırma bu verileri silmez.

RustCalc Windows, Linux ve macOS'ta çalışır. Linux ve macOS'ta kurulum için
`sh install.sh`, kaldırmak için `sh ~/.local/opt/rustcalc/uninstall.sh`
kullanın; ayrıntılar [kurulum belgesinde](INSTALLATION.md) (İngilizce).

## Sık sorulan sorular

**`100 + 10%` neden 110?** Çünkü `+ 10%` "yüzde on ekle" demektir; telefon ve
kasa hesap makineleri de böyle hesaplar. Tek başına `10%` 0,1'dir, `100 * 10%`
ise 10'dur. 0.5 öncesi sürümler yüzdeyi her yerde 100'e bölüyordu (100,1).

**`0.1 + 0.2` neden tam 0.3?** Hesaplar ikili kayan nokta yerine ondalık
sayılarla yapılır; paralar kâğıt üzerindeki gibi toplanır.

**≈ ne demek?** Sonuç bilimsel bir fonksiyonla yaklaşık 15 anlamlı basamağa
yuvarlanmıştır. Tam sonuçlarda görünmez.

**`1.000 + 1` neden 2?** Nokta da ondalık ayırıcıdır, yani `1.000` birdir. Binlik
ayırıcı kullanmadan `1000 + 1` yazın.

**`tan(90)` neden hata veriyor?** 90°'nin tanjantı tanımsızdır. Yanıltıcı büyük
bir sayı yerine hata gösterilir.

**Hesap makinesi neden tarihi sayı olarak gösteriyor?** Tarihler gün sayısıdır;
tarih biçiminde görmek için **Formüller**'deki tarih şablonlarını kullanın.
