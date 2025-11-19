<div align="center">
  <img src="cardinal/mac-icon_1024x1024.png" alt="Cardinal icon" width="120" height="120">
  <h1>Cardinal</h1>
  <p>A fast file searching tool for macOS.</p>
  <p>
    <a href="#using-cardinal">Using Cardinal</a> ·
    <a href="#building-cardinal">Building Cardinal</a>
  </p>
  <img src="doc/UI.gif" alt="Cardinal UI preview" width="720">
</div>

---

## Using Cardinal

### Download

Grab the latest packaged builds from [GitHub Releases](https://github.com/ldm0/cardinal/releases/).

### i18n support

Need a different language? Click the 🌍 button in the status bar to switch instantly.

### Search basics

Cardinal now speaks an Everything-compatible syntax layer on top of the classic substring/prefix tricks:

- `report draft` – space acts as `AND`, so you only see files whose names contain both tokens.
- `*.pdf briefing` – filter to PDF results whose names include “briefing”.
- `*.zip size:>100MB` – search for ZIP files larger than 100MB.
- `infolder:/Users demo!.psd` – restrict the search root to `/Users`, then search for files whose names contain `demo` but exclude `.psd`.
- `"Application Support"` – quote exact phrases.
- `brary/Applicat` – use `/` as a path separator for sub-path searching, matching directories like `Library/Application Support`.
- `/report` · `draft/` · `/report/` – wrap tokens with leading and/or trailing slashes to force **prefix**, **suffix**, or **exact** name matches when you need whole-word control beyond Everything syntax.

For the supported operator catalog—including boolean grouping, folder scoping, extension filters, regex usage, and more examples—see [`doc/search-syntax.md`](doc/search-syntax.md).

### Keyboard shortcuts & previews

- `Space` – Quick Look the currently selected row without leaving Cardinal.
- `Cmd+R` – reveal the highlighted result in Finder.
- `Cmd+F` – jump focus back to the search bar.
- `Cmd+C` – copy the selected file's path to the clipboard.
- `Cmd+Shift+Space` – toggle the Cardinal window globally via the quick-launch hotkey.

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
