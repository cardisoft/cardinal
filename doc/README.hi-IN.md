<div align="center">
  <img src="../cardinal/mac-icon_1024x1024.png" alt="Cardinal आइकन" width="120" height="120">
  <h1>Cardinal</h1>
  <p>macOS के लिए सबसे तेज़ और सबसे सटीक फ़ाइल खोज ऐप।</p>
  <p>
    <a href="#cardinal-का-उपयोग">Cardinal का उपयोग</a> ·
    <a href="#cardinal-बिल्ड-करें">Cardinal बिल्ड करें</a>
  </p>
  <img src="UI.gif" alt="Cardinal UI प्रीव्यू" width="720">
</div>

---

[English](../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Cardinal का उपयोग

### डाउनलोड

Homebrew से इंस्टॉल करें:

```bash
brew install --cask cardinal-search
```

आप [GitHub Releases](https://github.com/cardisoft/cardinal/releases/) से नवीनतम पैकेज भी डाउनलोड कर सकते हैं।

### i18n सपोर्ट

किसी दूसरी भाषा की ज़रूरत है? स्टेटस बार में ⚙️ बटन पर क्लिक करके तुरंत बदलें।

### खोज की बुनियादी बातें

Cardinal अब क्लासिक substring/prefix मैचिंग के ऊपर Everything-संगत सिंटैक्स लेयर सपोर्ट करता है:

- `report draft` – स्पेस `AND` की तरह काम करता है, इसलिए केवल वे फ़ाइलें दिखेंगी जिनके नाम में दोनों टोकन हों।
- `*.pdf briefing` – नाम में “briefing” वाले PDF परिणाम फ़िल्टर करें।
- `*.zip size:>100MB` – 100MB से बड़े ZIP फ़ाइलें खोजें।
- `in:/Users demo !.psd` – खोज रूट को `/Users` तक सीमित करें, फिर `demo` शामिल करें लेकिन `.psd` को बाहर रखें।
- `tag:ProjectA;ProjectB` – Finder टैग (macOS) मैच करें; `;` `OR` की तरह काम करता है।
- `*.md content:"Bearer "` – `Bearer ` स्ट्रिंग शामिल करने वाली Markdown फ़ाइलें फ़िल्टर करें।
- `"Application Support"` – सटीक वाक्यांशों के लिए quotes का उपयोग करें।
- `brary/Applicat` – उप-पथ खोज के लिए `/` को path separator की तरह उपयोग करें, जैसे `Library/Application Support`।
- `/report` · `draft/` · `/report/` – token के आगे/पीछे `/` लगाकर prefix, suffix या exact नाम मैच को मजबूर करें, जब Everything सिंटैक्स से आगे whole-word नियंत्रण चाहिए।
- `~/**/.DS_Store` – globstar (`**`) आपके home directory के सभी subfolders में जाकर कहीं भी पड़े `.DS_Store` ढूँढता है।

समर्थित ऑपरेटर कैटलॉग (boolean grouping, folder scoping, extension filters, regex और अधिक उदाहरण) के लिए [`search-syntax.md`](search-syntax.md) देखें।

### कीबोर्ड शॉर्टकट और प्रीव्यू

- `Space` – Cardinal छोड़े बिना चयनित row का Quick Look।
- `Cmd+R` – हाइलाइट किए गए परिणाम को Finder में दिखाएँ।
- `Cmd+F` – फोकस वापस search bar पर।
- `Cmd+C` – चयनित फ़ाइल का path क्लिपबोर्ड में कॉपी करें।
- `Cmd+Shift+Space` – global hotkey से Cardinal विंडो टॉगल करें।

खोज का आनंद लें!

---

## Cardinal बिल्ड करें

### आवश्यकताएँ

- macOS 12+
- Rust toolchain
- Node.js 18+ (npm सहित)
- Xcode command-line tools और Tauri prerequisites (<https://tauri.app/start/prerequisites/>)

### डेवलपमेंट मोड

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### प्रोडक्शन बिल्ड

```bash
cd cardinal
npm run tauri build
```
