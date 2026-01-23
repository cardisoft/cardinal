# صياغة البحث في Cardinal

لغة الاستعلام في Cardinal قريبة عمدًا من صياغة Everything، لكنها تعكس ما ينفّذه المحرك الحالي فعليًا. هذه الصفحة هي المرجع الموثوق لما تفهمه الواجهة الخلفية المكتوبة بـ Rust حاليًا.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. النموذج الذهني

- يتم تحليل كل استعلام إلى شجرة من:
  - **كلمات / عبارات** (نص عادي، سلاسل بين علامات اقتباس، بدائل)،
  - **مرشحات** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **عوامل بوليانية** (`AND`, `OR`, `NOT` / `!`).
- تتم المطابقة على **المسار الكامل** لكل ملف مُفهرس، وليس فقط اسم الملف.
- تتحكم واجهة المستخدم بحساسية حالة الأحرف:
  - عند **عدم حساسية حالة الأحرف**، يخفض المحرك حالة كلٍ من الاستعلام والمرشحين لمطابقة الاسم/المحتوى.
  - عند **حساسية حالة الأحرف**، يقارن المحرك البايتات كما هي.

أمثلة سريعة:
```text
report draft                  # ملفات يحتوي مسارها على “report” و“draft” معًا
ext:pdf briefing              # ملفات PDF يحتوي اسمها على “briefing”
parent:/Users demo!.psd       # تحت /Users، استبعد ملفات .psd
regex:^Report.*2025$          # أسماء تطابق regex
ext:png;jpg travel|vacation   # PNG أو JPG أسماؤها تحتوي على “travel” أو “vacation”
```

---

## 2. الرموز، البدائل، ومقاطع المسار

### 2.1 الرموز والعبارات البسيطة

- الرمز بدون اقتباس هو **مطابقة جزء من السلسلة** في المسار:
  - `demo` يطابق `/Users/demo/Projects/cardinal.md`.
- العبارات بين علامتي اقتباس تطابق التسلسل الدقيق بما في ذلك المسافات:
  - `"Application Support"` يطابق `/Library/Application Support/...`.
- ينطبق مفتاح حساسية الحالة في واجهة المستخدم على كليهما.

### 2.2 البدائل (`*`, `?`, `**`)

- `*` يطابق صفرًا أو أكثر من الأحرف.
- `?` يطابق حرفًا واحدًا بالضبط.
- `**` هو globstar يعبر **أي عدد من مقاطع المجلدات** عندما يظهر بين الشرطتين المائلتين.
- تُفسَّر البدائل **داخل رمز واحد**:
  - `*.rs` — أي اسم ينتهي بـ `.rs`.
  - `report-??.txt` — `report-01.txt` و`report-AB.txt`، إلخ.
  - `a*b` — أسماء تبدأ بـ `a` وتنتهي بـ `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` في أي مكان تحت `src/`.
- إذا احتجت إلى `*` أو `?` حرفيًا، ضع الرمز بين اقتباس: `"*.rs"`. يجب أن تكون globstar مقاطع مائلة مستقلة (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 تقسيم بنمط المسار باستخدام `/`

يفهم Cardinal “مقاطع الشرط المائل” داخل الرمز ويصنّف كل مقطع كمطابقة بادئة/لاحقة/مطابقة تامة/مطابقة جزء من المسار. أمثلة:

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

يتيح لك ذلك التعبير عن:
- “يجب أن ينتهي المجلد بـ X” (`foo/`)،
- “يجب أن يبدأ المجلد بـ X” (`/foo`)،
- “اسم مجلد مطابق في منتصف المسار” (`gaea/lil/bee/`).

---

## 3. المنطق البولياني والتجميع

يتبع Cardinal أولوية Everything:

- `NOT` / `!` أقوى ارتباطًا،
- `OR` / `|` بعده،
- `AND` الضمني / الصريح (“مسافة”) له **أدنى** أولوية.

### 3.1 العوامل

| الصياغة         | المعنى                                              |
| --------------- | --------------------------------------------------- |
| `foo bar`       | `foo AND bar` — يجب أن يتطابق الرمزان معًا.         |
| `foo\|bar`       | `foo OR bar` — يكفي تطابق أحدهما.                  |
| `foo OR bar`    | الصيغة النصية لـ `|`.                               |
| `!temp`         | `NOT temp` — يستبعد المطابقات.                     |
| `NOT temp`      | مماثل لـ `!temp`.                                   |
| `( ... )`       | تجميع بالأقواس.                                     |
| `< ... >`       | تجميع بالأقواس الزاوية (أسلوب Everything).         |

أمثلة على الأولوية:
```text
foo bar|baz        # يُحلَّل كـ foo AND (bar OR baz)
!(ext:zip report)  # يستبعد العناصر التي تطابق ext:zip و“report” معًا
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

استخدم الأقواس أو `<...>` عندما تريد تجاوز الأولوية الافتراضية.

---

## 4. المرشحات

هذا القسم يسرد فقط المرشحات التي يقيّمها المحرك الحالي فعليًا.

> **ملاحظة**: يجب أن تأتي وسيطات المرشح مباشرة بعد النقطتين (`ext:jpg`, `parent:/Users/demo`). كتابة `file: *.md` تضيف مسافة، لذلك يعاملها Cardinal كمرشح `file:` (بدون وسيط) يليه الرمز المنفصل `*.md`.

### 4.1 مرشحات الملفات / المجلدات

| المرشح          | المعنى                          | مثال              |
| --------------- | ------------------------------- | ----------------- |
| `file:`         | ملفات فقط (وليست مجلدات)        | `file: report`    |
| `folder:`       | مجلدات فقط                      | `folder:Projects` |

يمكن دمجها مع شروط أخرى:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 مرشح الامتداد: `ext:`

- يقبل `ext:` امتدادًا واحدًا أو أكثر مفصولًا بـ `;`:
  - `ext:jpg` — صور JPEG.
  - `ext:jpg;png;gif` — أنواع صور ويب شائعة.
- المطابقة غير حساسة لحالة الأحرف ولا تتضمن النقطة.

أمثلة:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 نطاق المجلد: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| المرشح            | المعنى                                                      | مثال                                         |
| ----------------- | ----------------------------------------------------------- | -------------------------------------------- |
| `parent:`         | الأبناء المباشرون للمجلد المحدد فقط                          | `parent:/Users/demo/Documents ext:md`       |
| `infolder:`/`in:` | أي عنصر ضمن المجلد المحدد (بشكل تكراري)                         | `in:/Users/demo/Projects report draft`      |
| `nosubfolders:`   | المجلد نفسه + الأبناء المباشرون من الملفات (بدون مجلدات فرعية) | `nosubfolders:/Users/demo/Projects ext:log` |

تأخذ هذه المرشحات مسارًا مطلقًا كوسيط؛ ويتم توسيع `~` في البداية إلى مجلد المنزل للمستخدم.

### 4.4 مرشح النوع: `type:`

يجمع `type:` امتدادات الملفات ضمن فئات دلالية. تشمل الفئات المدعومة (غير حساسة لحالة الأحرف ومع مرادفات):

- الصور: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- الفيديو: `type:video`, `type:videos`, `type:movie`, `type:movies`
- الصوت: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- المستندات: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- العروض التقديمية: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- الجداول: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- الأرشيفات: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- الشفرة: `type:code`, `type:source`, `type:dev`
- الملفات التنفيذية: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

أمثلة:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 ماكرو النوع: `audio:`, `video:`, `doc:`, `exe:`

اختصارات لحالات `type:` الشائعة:

| الماكرو  | المكافئ          | مثال                 |
| ------- | ---------------- | -------------------- |
| `audio:` | `type:audio`    | `audio: piano`       |
| `video:` | `type:video`    | `video: tutorial`    |
| `doc:`   | `type:doc`      | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`      | `exe: "Cardinal"`   |

تقبل الماكرو وسيطًا اختياريًا:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 مرشح الحجم: `size:`

يدعم `size:` ما يلي:

- **المقارنات**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **النطاقات**: `min..max`
- **الكلمات المفتاحية**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **الوحدات**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

أمثلة:
```text
size:>1GB                 # أكبر من 1 GB
size:1mb..10mb            # بين 1 MB و 10 MB
size:tiny                 # 0–10 KB (نطاق تقريبي للكلمة المفتاحية)
size:empty                # بالضبط 0 بايت
```

### 4.7 مرشحات التاريخ: `dm:`, `dc:`

- `dm:` / `datemodified:` — تاريخ التعديل.
- `dc:` / `datecreated:` — تاريخ الإنشاء.

تقبل:

1. **الكلمات المفتاحية** (نطاقات نسبية):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **تواريخ مطلقة**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - كما تدعم الصيغ الشائعة يوم‑أول / شهر‑أول مثل `DD-MM-YYYY` و `MM/DD/YYYY`.

3. **نطاقات ومقارنات**:
   - النطاقات: `dm:2024-01-01..2024-12-31`
   - المقارنات: `dm:>=2024-01-01`, `dc:<2023/01/01`

أمثلة:
```text
dm:today                      # معدّل اليوم
dc:lastyear                   # تم إنشاؤه في العام السابق
dm:2024-01-01..2024-03-31     # معدّل في الربع الأول من 2024
dm:>=2024/01/01               # معدّل منذ 2024-01-01 فصاعدًا
```

### 4.8 مرشح regex: `regex:`

يعامل `regex:` ما تبقّى من الرمز كتعبير نمطي يطبَّق على مكوّن المسار (اسم ملف أو مجلد).

أمثلة:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

يؤثر مفتاح حساسية الحالة في UI على مطابقة regex.

### 4.9 مرشح المحتوى: `content:`

يفحص `content:` محتويات الملف بحثًا عن **جزء نصي بسيط**:

- لا يوجد regex داخل `content:` — إنها مطابقة جزء نصي على مستوى البايتات.
- حساسية حالة الأحرف تتبع مفتاح UI:
  - في وضع عدم الحساسية، يتم تحويل كلمة البحث والبايتات الممسوحة إلى أحرف صغيرة.
  - في وضع الحساسية، تُقارن البايتات كما هي.
- تُسمح الكلمات القصيرة جدًا، لكن `""` (الفارغ) مرفوض.

أمثلة:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

تتم مطابقة المحتوى بشكل تدفقي عبر الملف؛ ويمكن للتسلسلات متعددة البايتات أن تعبر حدود المخزن المؤقت.

### 4.10 مرشح الوسوم: `tag:` / `t:`

يُرشّح باستخدام وسوم Finder (macOS). يجلب Cardinal الوسوم عند الطلب من بيانات الملف (بدون تخزين مؤقت)، وللمجموعات الكبيرة يستخدم `mdfind` لتضييق المرشحين قبل تطبيق مطابقة الوسوم.

- يقبل وسمًا واحدًا أو أكثر مفصولًا بـ `;` (OR منطقي): `tag:ProjectA;ProjectB`.
- تسلسل عدة مرشحات `tag:` (AND منطقي) لمطابقة عدة وسوم: `tag:Project tag:Important`.
- حساسية حالة الأحرف تتبع مفتاح UI.
- مطابقة أسماء الوسوم تتم عبر جزء من الاسم: `tag:proj` يطابق `Project` و`project`.

أمثلة:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. أمثلة

بعض التركيبات الواقعية:

```text
#  ملاحظات Markdown في Documents (بدون PDF)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  ملفات PDF في Reports تذكر “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  صور من الإجازة
type:picture vacation
ext:png;jpg travel|vacation

#  ملفات سجل حديثة داخل شجرة مشروع
in:/Users/demo/Projects ext:log dm:pastweek

#  سكربتات shell مباشرة تحت مجلد Scripts
parent:/Users/demo/Scripts *.sh

#  أي شيء يحتوي “Application Support” في المسار
"Application Support"

#  مطابقة اسم ملف محدد عبر regex
regex:^README\\.md$ parent:/Users/demo

#  استبعاد ملفات PSD في أي مكان تحت /Users
in:/Users demo!.psd
```

استخدم هذه الصفحة كقائمة موثوقة للعوامل والمرشحات التي ينفّذها المحرك اليوم؛ ميزات Everything الإضافية (مثل تواريخ الوصول/التشغيل أو المرشحات القائمة على السمات) تُحلّل على مستوى الصياغة لكنها تُرفض حاليًا أثناء التقييم.
