# Cardinal खोज वाक्यविन्यास

Cardinal की क्वेरी भाषा जानबूझकर Everything के वाक्यविन्यास के करीब है, लेकिन यह दर्शाती है कि मौजूदा इंजन वास्तव में क्या लागू करता है। यह पेज बताता है कि Rust बैकएंड आज क्या समझता है—यही आधिकारिक संदर्भ है।

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. मानसिक मॉडल

- हर क्वेरी निम्नलिखित से बनी एक ट्री में पार्स होती है:
  - **शब्द / वाक्यांश** (सादा टेक्स्ट, उद्धृत स्ट्रिंग, वाइल्डकार्ड),
  - **फ़िल्टर** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **बूलियन ऑपरेटर** (`AND`, `OR`, `NOT` / `!`).
- मिलान हर इंडेक्स की गई फ़ाइल के **पूर्ण पथ** पर चलता है, केवल बेसनेम पर नहीं।
- केस संवेदनशीलता UI टॉगल से नियंत्रित होती है:
  - **केस‑इंसेंसिटिव** होने पर इंजन नाम/कंटेंट मिलान के लिए क्वेरी और उम्मीदवारों दोनों को लोअरकेस करता है।
  - **केस‑सेंसिटिव** होने पर इंजन बाइट्स को ज्यों‑का‑त्यों तुलना करता है।

त्वरित उदाहरण:
```text
report draft                  # जिन फ़ाइलों के पथ में “report” और “draft” दोनों हैं
ext:pdf briefing              # जिन PDF फ़ाइलों के नाम में “briefing” है
parent:/Users demo!.psd       # /Users के तहत .psd फ़ाइलें बाहर करें
regex:^Report.*2025$          # regex से मैच होने वाले नाम
ext:png;jpg travel|vacation   # जिन PNG/JPG नामों में “travel” या “vacation” है
```

---

## 2. टोकन, वाइल्डकार्ड, और पाथ सेगमेंट

### 2.1 साधारण टोकन और वाक्यांश

- बिना उद्धरण वाला टोकन पथ पर **सब‑स्ट्रिंग मैच** होता है:
  - `demo` का मैच `/Users/demo/Projects/cardinal.md` से होता है।
- डबल‑क्वोटेड वाक्यांश स्पेस सहित सटीक अनुक्रम से मैच करते हैं:
  - `"Application Support"` का मैच `/Library/Application Support/...` से होता है।
- UI का केस टॉगल दोनों पर लागू होता है।

### 2.2 वाइल्डकार्ड (`*`, `?`, `**`)

- `*` शून्य या अधिक अक्षरों से मेल खाता है।
- `?` ठीक एक अक्षर से मेल खाता है।
- `**` एक globstar है जो स्लैश के बीच होने पर **किसी भी संख्या के फ़ोल्डर सेगमेंट** पार करता है।
- वाइल्डकार्ड **एक ही टोकन के भीतर** समझे जाते हैं:
  - `*.rs` — `.rs` से खत्म होने वाला कोई भी नाम।
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, आदि।
  - `a*b` — `a` से शुरू और `b` पर खत्म होने वाले नाम।
  - `src/**/Cargo.toml` — `src/` के नीचे कहीं भी `Cargo.toml`।
- यदि आपको लिटरल `*` या `?` चाहिए, तो टोकन को उद्धृत करें: `"*.rs"`। Globstar अलग स्लैश सेगमेंट होने चाहिए (`foo/**/bar`, `/Users/**`, `**/notes`)।

### 2.3 `/` के साथ पाथ‑स्टाइल सेगमेंटेशन

Cardinal टोकन के भीतर “स्लैश‑सेगमेंट्स” समझता है और हर सेगमेंट को पाथ कंपोनेंट्स पर प्रीफ़िक्स/सफ़िक्स/एक्ज़ैक्ट/सब‑स्ट्रिंग मैच के रूप में वर्गीकृत करता है। उदाहरण:

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

इससे आप व्यक्त कर सकते हैं:
- “फ़ोल्डर का अंत X से होना चाहिए” (`foo/`),
- “फ़ोल्डर की शुरुआत X से होनी चाहिए” (`/foo`),
- “पथ के बीच में सटीक फ़ोल्डर नाम” (`gaea/lil/bee/`).

---

## 3. बूलियन लॉजिक और ग्रुपिंग

Cardinal Everything की प्राथमिकता का पालन करता है:

- `NOT` / `!` सबसे अधिक बाइंड करता है,
- `OR` / `|` उसके बाद,
- निहित / स्पष्ट `AND` (“स्पेस”) की **सबसे कम** प्राथमिकता होती है।

### 3.1 ऑपरेटर

| सिंटैक्स        | अर्थ                                               |
| -------------- | -------------------------------------------------- |
| `foo bar`      | `foo AND bar` — दोनों टोकन मैच होने चाहिए।        |
| `foo\|bar`      | `foo OR bar` — इनमें से कोई एक मैच हो सकता है।   |
| `foo OR bar`   | `|` का शब्द रूप।                                   |
| `!temp`        | `NOT temp` — मैचों को बाहर करें।                  |
| `NOT temp`     | `!temp` के समान।                                   |
| `( ... )`      | कोष्ठकों के साथ ग्रुपिंग।                          |
| `< ... >`      | कोण‑कोष्ठकों के साथ ग्रुपिंग (Everything‑स्टाइल)। |

प्राथमिकता उदाहरण:
```text
foo bar|baz        # foo AND (bar OR baz) के रूप में पार्स होता है
!(ext:zip report)  # जहाँ ext:zip और “report” दोनों मैच हों, उन्हें बाहर करें
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

डिफ़ॉल्ट प्राथमिकता बदलने के लिए कोष्ठक या `<...>` का उपयोग करें।

---

## 4. फ़िल्टर

यह सेक्शन केवल उन फ़िल्टरों की सूची देता है जिन्हें वर्तमान इंजन वास्तव में जाँचता है।

> **नोट**: फ़िल्टर आर्ग्युमेंट्स को तुरंत कोलन के बाद होना चाहिए (`ext:jpg`, `parent:/Users/demo`)। `file: *.md` लिखने पर स्पेस जुड़ जाता है, इसलिए Cardinal इसे `file:` फ़िल्टर (बिना आर्ग्युमेंट) और उसके बाद अलग टोकन `*.md` मानता है।

### 4.1 फ़ाइल / फ़ोल्डर फ़िल्टर

| फ़िल्टर          | अर्थ                         | उदाहरण            |
| --------------- | ---------------------------- | ----------------- |
| `file:`         | केवल फ़ाइलें (फ़ोल्डर नहीं)  | `file: report`    |
| `folder:`       | केवल फ़ोल्डर                  | `folder:Projects` |

इन्हें अन्य शर्तों के साथ जोड़ा जा सकता है:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 एक्सटेंशन फ़िल्टर: `ext:`

- `ext:` `;` से अलग की गई एक या अधिक एक्सटेंशन स्वीकार करता है:
  - `ext:jpg` — JPEG इमेजें।
  - `ext:jpg;png;gif` — सामान्य वेब इमेज प्रकार।
- मैचिंग केस‑इंसेंसिटिव है और डॉट शामिल नहीं करता।

उदाहरण:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 फ़ोल्डर स्कोप: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| फ़िल्टर            | अर्थ                                                      | उदाहरण                                        |
| ----------------- | --------------------------------------------------------- | -------------------------------------------- |
| `parent:`         | निर्दिष्ट फ़ोल्डर के केवल सीधे बच्चे                      | `parent:/Users/demo/Documents ext:md`       |
| `infolder:`/`in:` | निर्दिष्ट फ़ोल्डर का कोई भी वंशज (रिकर्सिव)               | `in:/Users/demo/Projects report draft`      |
| `nosubfolders:`   | फ़ोल्डर स्वयं + सीधे फ़ाइल बच्चे (उप‑फ़ोल्डर नहीं)        | `nosubfolders:/Users/demo/Projects ext:log` |

ये फ़िल्टर आर्ग्युमेंट के रूप में absolute path लेते हैं; अग्रणी `~` यूज़र होम डायरेक्टरी में फैल जाता है।

### 4.4 टाइप फ़िल्टर: `type:`

`type:` फ़ाइल एक्सटेंशन को अर्थपूर्ण श्रेणियों में समूहित करता है। समर्थित श्रेणियाँ (केस‑इंसेंसिटिव, पर्यायवाची सहित) इस प्रकार हैं:

- चित्र: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- वीडियो: `type:video`, `type:videos`, `type:movie`, `type:movies`
- ऑडियो: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- दस्तावेज़: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- प्रेज़ेंटेशन: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- स्प्रेडशीट: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- आर्काइव: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- कोड: `type:code`, `type:source`, `type:dev`
- एग्ज़ीक्यूटेबल: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

उदाहरण:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 टाइप मैक्रो: `audio:`, `video:`, `doc:`, `exe:`

सामान्य `type:` मामलों के लिए शॉर्टकट:

| मैक्रो  | समकक्ष          | उदाहरण                |
| ------ | --------------- | --------------------- |
| `audio:` | `type:audio`  | `audio: piano`        |
| `video:` | `type:video`  | `video: tutorial`     |
| `doc:`   | `type:doc`    | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`    | `exe: "Cardinal"`    |

मैक्रो वैकल्पिक आर्ग्युमेंट स्वीकार करते हैं:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 आकार फ़िल्टर: `size:`

`size:` सपोर्ट करता है:

- **तुलनाएँ**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **रेंज**: `min..max`
- **कीवर्ड**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **इकाइयाँ**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

उदाहरण:
```text
size:>1GB                 # 1 GB से बड़ा
size:1mb..10mb            # 1 MB से 10 MB के बीच
size:tiny                 # 0–10 KB (कीवर्ड आधारित अनुमानित रेंज)
size:empty                # बिल्कुल 0 बाइट
```

### 4.7 तिथि फ़िल्टर: `dm:`, `dc:`

- `dm:` / `datemodified:` — संशोधित तिथि।
- `dc:` / `datecreated:` — निर्माण तिथि।

ये स्वीकार करते हैं:

1. **कीवर्ड** (सापेक्ष रेंज):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **निश्चित तिथियाँ**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - `DD-MM-YYYY` और `MM/DD/YYYY` जैसे सामान्य दिन‑पहले / महीने‑पहले प्रारूप भी समर्थित हैं।

3. **रेंज और तुलनाएँ**:
   - रेंज: `dm:2024-01-01..2024-12-31`
   - तुलनाएँ: `dm:>=2024-01-01`, `dc:<2023/01/01`

उदाहरण:
```text
dm:today                      # आज संशोधित
dc:lastyear                   # पिछले कैलेंडर वर्ष में बनाया गया
dm:2024-01-01..2024-03-31     # 2024 के Q1 में संशोधित
dm:>=2024/01/01               # 2024-01-01 से आगे संशोधित
```

### 4.8 Regex फ़िल्टर: `regex:`

`regex:` टोकन के बाकी हिस्से को पथ घटक (फ़ाइल या फ़ोल्डर नाम) पर लागू नियमित अभिव्यक्ति मानता है।

उदाहरण:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

UI का केस टॉगल regex मैचिंग को प्रभावित करता है।

### 4.9 कंटेंट फ़िल्टर: `content:`

`content:` फ़ाइल कंटेंट को **सादा सब‑स्ट्रिंग** के लिए स्कैन करता है:

- `content:` के अंदर regex नहीं है — यह बाइट सब‑स्ट्रिंग मैच है।
- केस संवेदनशीलता UI टॉगल के अनुसार होती है:
  - केस‑इंसेंसिटिव मोड में, सुई और स्कैन किए गए बाइट दोनों लोअरकेस किए जाते हैं।
  - केस‑सेंसिटिव मोड में, बाइट्स ज्यों‑के‑त्यों तुलना होते हैं।
- बहुत छोटी सुइयाँ अनुमत हैं, लेकिन `""` (खाली) अस्वीकृत है।

उदाहरण:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

कंटेंट मैचिंग फ़ाइल पर स्ट्रीमिंग ढंग से होती है; मल्टी‑बाइट क्रम बफ़र सीमाओं को पार कर सकते हैं।

### 4.10 टैग फ़िल्टर: `tag:` / `t:`

Finder टैग (macOS) से फ़िल्टर करता है। Cardinal फ़ाइल के मेटाडेटा से टैग मांग पर लाता है (कोई कैश नहीं), और बड़े परिणाम सेट के लिए `mdfind` का उपयोग करके टैग मिलान से पहले उम्मीदवारों को सीमित करता है।

- `;` से अलग किए गए एक या अधिक टैग स्वीकार करता है (तार्किक OR): `tag:ProjectA;ProjectB`।
- कई `tag:` फ़िल्टर को श्रृंखला में जोड़ें (तार्किक AND): `tag:Project tag:Important`।
- केस संवेदनशीलता UI टॉगल के अनुसार है।
- टैग नाम सब‑स्ट्रिंग से मिलते हैं: `tag:proj` का मैच `Project` और `project` से होता है।

उदाहरण:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. उदाहरण

कुछ वास्तविक संयोजन:

```text
#  Documents में Markdown नोट्स (PDF नहीं)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  Reports में “briefing” का उल्लेख करने वाले PDFs
ext:pdf briefing parent:/Users/demo/Reports

#  छुट्टियों की तस्वीरें
type:picture vacation
ext:png;jpg travel|vacation

#  प्रोजेक्ट ट्री के भीतर हाल के लॉग फ़ाइलें
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts फ़ोल्डर के ठीक नीचे शेल स्क्रिप्ट्स
parent:/Users/demo/Scripts *.sh

#  पथ में “Application Support” वाले सभी आइटम
"Application Support"

#  regex के जरिए किसी खास फ़ाइल नाम का मैच
regex:^README\\.md$ parent:/Users/demo

#  /Users के नीचे कहीं भी PSD को बाहर करें
in:/Users demo!.psd
```

इस पेज को आज के इंजन द्वारा लागू ऑपरेटरों और फ़िल्टरों की आधिकारिक सूची मानें; Everything की अतिरिक्त सुविधाएँ (जैसे access/run तारीखें या एट्रिब्यूट‑आधारित फ़िल्टर) वाक्यविन्यास स्तर पर पार्स होती हैं, लेकिन वर्तमान में मूल्यांकन के दौरान अस्वीकृत रहती हैं।
