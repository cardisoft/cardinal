<div align="center">
  <img src="cardinal/mac-icon_1024x1024.png" alt="Cardinal icon" width="120" height="120">
  <h1>Cardinal</h1>
  <p>Fastest and most accurate file search app for macOS.</p>
  <p>
    <a href="#using-cardinal">Using Cardinal</a> ·
    <a href="#building-cardinal">Building Cardinal</a>
  </p>
  <img src="doc/pub/UI.gif" alt="Cardinal UI preview" width="720">
</div>

---

[English](README.md) · [Español](doc/pub/README.es-ES.md) · [한국어](doc/pub/README.ko-KR.md) · [Русский](doc/pub/README.ru-RU.md) · [简体中文](doc/pub/README.zh-CN.md) · [繁體中文](doc/pub/README.zh-TW.md) · [Português](doc/pub/README.pt-BR.md) · [Italiano](doc/pub/README.it-IT.md) · [日本語](doc/pub/README.ja-JP.md) · [Français](doc/pub/README.fr-FR.md) · [Deutsch](doc/pub/README.de-DE.md) · [Українська](doc/pub/README.uk-UA.md) · [العربية](doc/pub/README.ar-SA.md) · [हिन्दी](doc/pub/README.hi-IN.md) · [Türkçe](doc/pub/README.tr-TR.md)

## Using Cardinal

### Download

Use homebrew:

```bash
brew install --cask cardinal-search
```

You can also grab the latest packaged builds from [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### i18n support

Need a different language? Click the ⚙️ button in the status bar to switch instantly.

### Search basics

Cardinal now speaks an Everything-compatible syntax layer on top of the classic substring/prefix tricks:

- `report draft` – space acts as `AND`, so you only see files whose names contain both tokens.
- `*.pdf briefing` – filter to PDF results whose names include “briefing”.
- `*.zip size:>100MB` – search for ZIP files larger than 100MB.
- `in:/Users demo !.psd` – restrict the search root to `/Users`, then search for files whose names contain `demo` but exclude `.psd`.
- `tag:ProjectA;ProjectB` – match Finder tags (macOS); `;` acts as `OR`.
- `*.md content:"Bearer "` – filter to Markdown files containing the string `Bearer `.
- `"Application Support"` – quote exact phrases.
- `brary/Applicat` – use `/` as a path separator for sub-path searching, matching directories like `Library/Application Support`.
- `/report` · `draft/` · `/report/` – wrap tokens with leading and/or trailing slashes to force **prefix**, **suffix**, or **exact** name matches when you need whole-word control beyond Everything syntax.
- `~/**/.DS_Store` – globstar (`**`) dives through every subfolder under your home directory to find stray `.DS_Store` files anywhere in the tree.

For the supported operator catalog—including boolean grouping, folder scoping, extension filters, regex usage, and more examples—see [`doc/pub/search-syntax.md`](doc/pub/search-syntax.md).

### Keyboard shortcuts & previews

- `Cmd+Shift+Space` – toggle the Cardinal window globally via the quick-launch hotkey.
- `Cmd+,` – open Preferences.
- `Esc` – hide the Cardinal window.
- `ArrowUp`/`ArrowDown` – move the selection.
- `Shift+ArrowUp`/`Shift+ArrowDown` – extend the selection.
- `Space` – Quick Look the currently selected row without leaving Cardinal.
- `Cmd+O` – open the highlighted result.
- `Cmd+R` – reveal the highlighted result in Finder.
- `Cmd+C` – copy the selected files to the clipboard.
- `Cmd+Shift+C` – copy the selected paths to the clipboard.
- `Cmd+F` – jump focus back to the search bar.
- `ArrowUp`/`ArrowDown` (in search bar) – cycle search history.

Happy searching!

---

## Building Cardinal

### Requirements

- macOS 12+
- Rust toolchain
- Node.js 18+ with npm
- Xcode command-line tools & Tauri prerequisites (<https://tauri.app/start/prerequisites/>)

### Development mode

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Production build

```bash
cd cardinal
npm run tauri build
```
