<div align="center">
  <img src="cardinal/mac-icon_1024x1024.png" alt="Іконка Cardinal" width="120" height="120">
  <h1>Cardinal</h1>
  <p>Найшвидший інструмент пошуку файлів для macOS.</p>
  <p>
    <a href="#використання-cardinal">Використання Cardinal</a> ·
    <a href="#збірка-cardinal">Збірка Cardinal</a>
  </p>
  <img src="doc/UI.gif" alt="Попередній перегляд інтерфейсу Cardinal" width="720">
</div>

---

[English](README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Використання Cardinal

### Завантаження

Встановіть через Homebrew:

```bash
brew install --cask cardinal-search
```

Або завантажте найсвіжіші пакети з [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### Підтримка i18n

Потрібна інша мова? Натисніть кнопку ⚙️ на панелі стану, щоб миттєво перемкнути.

### Основи пошуку

Cardinal підтримує Everything-сумісний синтаксичний шар поверх класичного пошуку за підрядком/префіксом:

- `report draft` – пробіл працює як `AND`, тож ви бачите лише файли, що містять обидва токени.
- `*.pdf briefing` – відфільтровує PDF, у назві яких є «briefing».
- `*.zip size:>100MB` – шукає ZIP-файли більші за 100MB.
- `infolder:/Users demo !.psd` – обмежує корінь пошуку `/Users`, потім шукає `demo`, але виключає `.psd`.
- `tag:ProjectA;ProjectB` – відповідає тегам Finder (macOS); `;` працює як `OR`.
- `*.md content:"Bearer "` – фільтрує Markdown, що містять рядок `Bearer `.
- `"Application Support"` – беріть точні фрази в лапки.
- `brary/Applicat` – використовуйте `/` як розділювач шляху для пошуку підшляхів, наприклад `Library/Application Support`.
- `/report` · `draft/` · `/report/` – обгорніть токени провідним та/або кінцевим `/`, щоб примусити збіг за префіксом, суфіксом або точним ім’ям, коли потрібен контроль цілими словами поза Everything-синтаксисом.
- `~/**/.DS_Store` – globstar (`**`) проходить усі підпапки домашнього каталогу, щоб знайти розкидані `.DS_Store`.

Повний перелік операторів (булеві групи, обмеження папок, фільтри розширень, regex та інші приклади) див. у [`doc/search-syntax.md`](doc/search-syntax.md).

### Гарячі клавіші та перегляди

- `Space` – Quick Look вибраного рядка, не виходячи з Cardinal.
- `Cmd+R` – показати виділений результат у Finder.
- `Cmd+F` – повернути фокус у пошуковий рядок.
- `Cmd+C` – скопіювати шлях вибраного файла в буфер обміну.
- `Cmd+Shift+Space` – глобально перемикає вікно Cardinal гарячою клавішею швидкого запуску.

Приємного пошуку!

---

## Збірка Cardinal

### Вимоги

- macOS 12+
- Toolchain Rust
- Node.js 18+ з npm
- Xcode command-line tools і передумови Tauri (<https://tauri.app/start/prerequisites/>)

### Режим розробки

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Продакшн-збірка

```bash
cd cardinal
npm run tauri build
```
