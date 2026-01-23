# Cardinal Arama Sözdizimi

Cardinal'ın sorgu dili Everything sözdizimine bilinçli olarak yakındır, ancak mevcut motorun gerçekten uyguladıklarını yansıtır. Bu sayfa, Rust arka ucunun bugün anladıklarına ilişkin resmi referanstır.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Zihinsel model

- Her sorgu şu öğelerden oluşan bir ağaca ayrıştırılır:
  - **Kelimeler / ifadeler** (düz metin, tırnaklı dizgeler, jokerler),
  - **Filtreler** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Mantıksal operatörler** (`AND`, `OR`, `NOT` / `!`).
- Eşleştirme, her dizinlenmiş dosyanın yalnızca taban adıyla değil, **tam yoluyla** yapılır.
- Büyük/küçük harf duyarlılığı UI anahtarıyla kontrol edilir:
  - **Büyük/küçük harfe duyarsız** modda, motor ad/içerik eşleşmeleri için hem sorguyu hem adayları küçültür.
  - **Büyük/küçük harfe duyarlı** modda, motor baytları olduğu gibi karşılaştırır.

Hızlı örnekler:
```text
report draft                  # yolu hem “report” hem “draft” içeren dosyalar
ext:pdf briefing              # adı “briefing” içeren PDF dosyaları
parent:/Users demo!.psd       # /Users altında .psd dosyalarını hariç tut
regex:^Report.*2025$          # bir regex'e uyan adlar
ext:png;jpg travel|vacation   # adı “travel” veya “vacation” içeren PNG ya da JPG
```

---

## 2. Belirteçler, jokerler ve yol segmentleri

### 2.1 Basit belirteçler ve ifadeler

- Tırnaksız bir belirteç, yolda **alt dize eşleşmesidir**:
  - `demo`, `/Users/demo/Projects/cardinal.md` ile eşleşir.
- Çift tırnaklı ifadeler, boşluklar dahil tam diziyi eşleştirir:
  - `"Application Support"`, `/Library/Application Support/...` ile eşleşir.
- UI büyük/küçük harf anahtarı ikisine de uygulanır.

### 2.2 Jokerler (`*`, `?`, `**`)

- `*` sıfır veya daha fazla karakterle eşleşir.
- `?` tam olarak bir karakterle eşleşir.
- `**`, eğik çizgiler arasında göründüğünde **herhangi sayıda klasör segmentini** geçen bir globstar'dır.
- Jokerler **tek bir belirteç içinde** yorumlanır:
  - `*.rs` — `.rs` ile biten herhangi bir ad.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt` vb.
  - `a*b` — `a` ile başlayıp `b` ile biten adlar.
  - `src/**/Cargo.toml` — `src/` altında herhangi bir yerde `Cargo.toml`.
- Harfiyen `*` veya `?` gerekiyorsa belirteci tırnaklayın: `"*.rs"`. Globstar'lar bağımsız eğik çizgi segmentleri olmalıdır (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 `/` ile yol tarzı segmentasyon

Cardinal, bir belirteç içindeki “eğik çizgi segmentlerini” anlar ve her segmenti yol bileşenlerinde önek/sonek/tam/alt dize eşleşmesi olarak sınıflandırır. Örnekler:

```text
elloworl        → Substring("elloworl")
/root           → Prefix("root")
root/           → Suffix("root")
/root/          → Exact("root")
/root/bar       → Exact("root"), Prefix("bar")
/root/bar/kksk  → Exact("root"), Exact("bar"), Prefix("kksk")
foo/bar/kks     → Suffix("foo"), Exact("bar"), Prefix("kks")
gaea/lil/bee/   → Suffix("gaea"), Exact("lil"), Exact("bee")
bab/bob/        → Suffix("bab"), Exact("bob")
/byb/huh/good/  → Exact("byb"), Exact("huh"), Exact("good")
```

Bu sayede şunları ifade edebilirsiniz:
- “Klasör X ile bitmeli” (`foo/`),
- “Klasör X ile başlamalı” (`/foo`),
- “Yolun ortasında tam klasör adı” (`gaea/lil/bee/`).

---

## 3. Mantıksal kurallar ve gruplama

Cardinal, Everything'nin öncelik sırasını izler:

- `NOT` / `!` en sıkı bağlanır,
- `OR` / `|` sonraki,
- örtük / açık `AND` (“boşluk”) **en düşük** önceliğe sahiptir.

### 3.1 Operatörler

| Sözdizimi      | Anlamı                                             |
| -------------- | -------------------------------------------------- |
| `foo bar`      | `foo AND bar` — her iki belirteç eşleşmelidir.    |
| `foo\|bar`      | `foo OR bar` — ikisinden biri eşleşebilir.        |
| `foo OR bar`   | `|` simgesinin kelime biçimi.                     |
| `!temp`        | `NOT temp` — eşleşmeleri hariç tutar.             |
| `NOT temp`     | `!temp` ile aynıdır.                              |
| `( ... )`      | Parantezlerle gruplama.                           |
| `< ... >`      | Köşeli ayraçlarla gruplama (Everything tarzı).    |

Öncelik örnekleri:
```text
foo bar|baz        # foo AND (bar OR baz) olarak ayrıştırılır
!(ext:zip report)  # ext:zip ve “report” her ikisinin eşleştiği öğeleri hariç tutar
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Varsayılan önceliği değiştirmek istediğinizde parantez veya `<...>` kullanın.

---

## 4. Filtreler

Bu bölüm yalnızca mevcut motorun gerçekten değerlendirdiği filtreleri listeler.

> **Not**: filtre argümanları iki noktadan hemen sonra gelmelidir (`ext:jpg`, `parent:/Users/demo`). `file: *.md` yazmak boşluk ekler; bu yüzden Cardinal bunu `file:` filtresi (argüman yok) ve ardından ayrı bir `*.md` belirteci olarak yorumlar.

### 4.1 Dosya / klasör filtreleri

| Filtre          | Anlamı                           | Örnek             |
| --------------- | -------------------------------- | ----------------- |
| `file:`         | Yalnızca dosyalar (klasör değil) | `file: report`    |
| `folder:`       | Yalnızca klasörler               | `folder:Projects` |

Bunlar diğer terimlerle birleştirilebilir:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Uzantı filtresi: `ext:`

- `ext:`, `;` ile ayrılmış bir veya daha fazla uzantı kabul eder:
  - `ext:jpg` — JPEG görüntüler.
  - `ext:jpg;png;gif` — yaygın web görüntü türleri.
- Eşleşme büyük/küçük harfe duyarlı değildir ve noktayı içermez.

Örnekler:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Klasör kapsamı: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Filtre            | Anlamı                                                     | Örnek                                        |
| ----------------- | ----------------------------------------------------------- | ------------------------------------------- |
| `parent:`         | Yalnızca belirtilen klasörün doğrudan çocukları            | `parent:/Users/demo/Documents ext:md`       |
| `infolder:`/`in:` | Belirtilen klasörün herhangi bir alt öğesi (özyinelemeli)  | `in:/Users/demo/Projects report draft`      |
| `nosubfolders:`   | Klasörün kendisi + doğrudan dosya çocukları (alt klasör yok) | `nosubfolders:/Users/demo/Projects ext:log` |

Bu filtreler argüman olarak mutlak yol alır; baştaki `~` kullanıcı ana dizinine genişletilir.

### 4.4 Tür filtresi: `type:`

`type:`, dosya uzantılarını anlamsal kategorilere gruplar. Desteklenen kategoriler (büyük/küçük harf duyarsız, eşanlamlılarla) şunlardır:

- Görseller: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Video: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Ses: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Belgeler: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Sunumlar: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Elektronik tablolar: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Arşivler: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Kod: `type:code`, `type:source`, `type:dev`
- Çalıştırılabilirler: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Örnekler:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Tür makroları: `audio:`, `video:`, `doc:`, `exe:`

Yaygın `type:` durumları için kısayollar:

| Makro   | Karşılığı           | Örnek                |
| ------ | ------------------- | -------------------- |
| `audio:` | `type:audio`       | `audio: piano`       |
| `video:` | `type:video`       | `video: tutorial`    |
| `doc:`   | `type:doc`         | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`         | `exe: "Cardinal"`   |

Makrolar isteğe bağlı bir argüman kabul eder:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Boyut filtresi: `size:`

`size:` şunları destekler:

- **Karşılaştırmalar**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Aralıklar**: `min..max`
- **Anahtar sözcükler**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Birimler**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Örnekler:
```text
size:>1GB                 # 1 GB'den büyük
size:1mb..10mb            # 1 MB ile 10 MB arası
size:tiny                 # 0–10 KB (yaklaşık anahtar sözcük aralığı)
size:empty                # tam olarak 0 bayt
```

### 4.7 Tarih filtreleri: `dm:`, `dc:`

- `dm:` / `datemodified:` — değiştirilme tarihi.
- `dc:` / `datecreated:` — oluşturulma tarihi.

Şunları kabul eder:

1. **Anahtar sözcükler** (göreli aralıklar):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Kesin tarihler**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - `DD-MM-YYYY` ve `MM/DD/YYYY` gibi yaygın gün‑önce / ay‑önce biçimlerini de destekler.

3. **Aralıklar ve karşılaştırmalar**:
   - Aralıklar: `dm:2024-01-01..2024-12-31`
   - Karşılaştırmalar: `dm:>=2024-01-01`, `dc:<2023/01/01`

Örnekler:
```text
dm:today                      # bugün değiştirilmiş
dc:lastyear                   # geçen takvim yılında oluşturulmuş
dm:2024-01-01..2024-03-31     # 2024 Q1 içinde değiştirilmiş
dm:>=2024/01/01               # 2024-01-01 ve sonrasında değiştirilmiş
```

### 4.8 Regex filtresi: `regex:`

`regex:`, belirtecin geri kalanını yol bileşenine (dosya veya klasör adı) uygulanan bir düzenli ifade olarak ele alır.

Örnekler:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

UI büyük/küçük harf anahtarı regex eşleşmesini etkiler.

### 4.9 İçerik filtresi: `content:`

`content:`, dosya içeriklerini **düz bir alt dize** için tarar:

- `content:` içinde regex yoktur — bayt alt dize eşleşmesidir.
- Büyük/küçük harf duyarlılığı UI anahtarını takip eder:
  - Büyük/küçük harfe duyarsız modda, hem aranan dize hem taranan baytlar küçültülür.
  - Büyük/küçük harfe duyarlı modda, baytlar olduğu gibi karşılaştırılır.
- Çok kısa dizeler kabul edilir, ancak `""` (boş) reddedilir.

Örnekler:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

İçerik eşleştirme dosya üzerinde akış halinde yapılır; çok baytlı diziler tampon sınırlarını aşabilir.

### 4.10 Etiket filtresi: `tag:` / `t:`

Finder etiketlerine (macOS) göre filtreler. Cardinal, etiketleri dosyanın meta verilerinden istek üzerine alır (önbellek yoktur) ve büyük sonuç kümelerinde etiket eşleşmesini uygulamadan önce adayları daraltmak için `mdfind` kullanır.

- `;` ile ayrılmış bir veya daha fazla etiket kabul eder (mantıksal OR): `tag:ProjectA;ProjectB`.
- Birden fazla `tag:` filtresini (mantıksal AND) zincirleyerek çoklu etiket eşleşmesi yapabilirsiniz: `tag:Project tag:Important`.
- Büyük/küçük harf duyarlılığı UI anahtarını takip eder.
- Etiket adları alt dize eşleşmesiyle bulunur: `tag:proj`, `Project` ve `project` ile eşleşir.

Örnekler:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Örnekler

Bazı gerçekçi kombinasyonlar:

```text
#  Documents içinde Markdown notları (PDF yok)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  Reports içinde “briefing” geçen PDF'ler
ext:pdf briefing parent:/Users/demo/Reports

#  Tatil fotoğrafları
type:picture vacation
ext:png;jpg travel|vacation

#  Proje ağacı içinde yakın tarihli log dosyaları
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts klasörü altında doğrudan shell betikleri
parent:/Users/demo/Scripts *.sh

#  Yolda “Application Support” olan her şey
"Application Support"

#  Regex ile belirli bir dosya adını eşleştir
regex:^README\\.md$ parent:/Users/demo

#  /Users altında her yerde PSD'leri hariç tut
in:/Users demo!.psd
```

Bu sayfayı, motorun bugün uyguladığı operatörler ve filtreler için yetkili liste olarak kullanın; Everything’nin ek özellikleri (erişim/çalıştırma tarihleri veya öznitelik tabanlı filtreler gibi) sözdizimi düzeyinde ayrıştırılır ancak şu anda değerlendirme sırasında reddedilir.
