<div align="center">
  <img src="../cardinal/mac-icon_1024x1024.png" alt="أيقونة Cardinal" width="120" height="120">
  <h1>Cardinal</h1>
  <p>أسرع وأدق تطبيق للبحث عن الملفات على macOS.</p>
  <p>
    <a href="#استخدام-cardinal">استخدام Cardinal</a> ·
    <a href="#بناء-cardinal">بناء Cardinal</a>
  </p>
  <img src="UI.gif" alt="معاينة واجهة Cardinal" width="720">
</div>

---

[English](../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## استخدام Cardinal

### التحميل

التثبيت عبر Homebrew:

```bash
brew install --cask cardinal-search
```

كما يمكنك تنزيل أحدث الحزم من [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### دعم تعدد اللغات

تحتاج لغة مختلفة؟ انقر زر ⚙️ في شريط الحالة للتبديل فورًا.

### أساسيات البحث

يدعم Cardinal طبقة صياغة متوافقة مع Everything فوق أساليب المطابقة التقليدية (جزء من النص/بادئة):

- `report draft` – المسافة تعمل كـ `AND`، لذا ستظهر الملفات التي تحتوي أسماؤها على كلا الرمزين.
- `*.pdf briefing` – تصفية نتائج PDF التي يتضمن اسمها “briefing”.
- `*.zip size:>100MB` – البحث عن ملفات ZIP أكبر من 100MB.
- `in:/Users demo !.psd` – حصر جذر البحث في `/Users` ثم البحث عن `demo` مع استثناء `.psd`.
- `tag:ProjectA;ProjectB` – مطابقة وسوم Finder (macOS)؛ حيث يعمل `;` كـ `OR`.
- `*.md content:"Bearer "` – تصفية ملفات Markdown التي تحتوي السلسلة `Bearer `.
- `"Application Support"` – استخدم علامات الاقتباس للعبارات الدقيقة.
- `brary/Applicat` – استخدم `/` كفاصل مسار للبحث في المسارات الفرعية، لمطابقة مجلدات مثل `Library/Application Support`.
- `/report` · `draft/` · `/report/` – ضع `/` في بداية و/أو نهاية الرمز لفرض مطابقة البادئة أو اللاحقة أو الاسم الكامل عندما تحتاج تحكمًا أدق من صياغة Everything.
- `~/**/.DS_Store` – الـ globstar (`**`) يتعمق في كل المجلدات الفرعية ضمن مجلد المنزل للعثور على ملفات `.DS_Store` المتناثرة.

للاطلاع على قائمة المعاملات المدعومة (تجميع منطقي، تحديد نطاق المجلدات، فلاتر الامتداد، regex، وأمثلة إضافية)، راجع [`search-syntax.md`](search-syntax.md).

### اختصارات لوحة المفاتيح والمعاينات

- `Cmd+Shift+Space` – إظهار/إخفاء نافذة Cardinal عالميًا عبر اختصار التشغيل السريع.
- `Cmd+,` – فتح التفضيلات.
- `Esc` – إخفاء نافذة Cardinal.
- `ArrowUp`/`ArrowDown` – تحريك التحديد.
- `Shift+ArrowUp`/`Shift+ArrowDown` – توسيع التحديد.
- `Space` – فتح Quick Look للصف المحدد دون مغادرة Cardinal.
- `Cmd+O` – يفتح النتيجة المحددة.
- `Cmd+R` – إظهار النتيجة المحددة في Finder.
- `Cmd+C` – نسخ الملفات المحددة إلى الحافظة.
- `Cmd+Shift+C` – نسخ المسارات المحددة إلى الحافظة.
- `Cmd+F` – إعادة التركيز إلى شريط البحث.
- `ArrowUp`/`ArrowDown` (في شريط البحث) – استعراض سجل البحث.

بحثًا موفقًا!

---

## بناء Cardinal

### المتطلبات

- macOS 12+
- أداة Rust toolchain
- Node.js 18+ مع npm
- أدوات سطر أوامر Xcode ومتطلبات Tauri (<https://tauri.app/start/prerequisites/>)

### وضع التطوير

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### بناء الإنتاج

```bash
cd cardinal
npm run tauri build
```
