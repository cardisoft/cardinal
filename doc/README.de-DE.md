<div align="center">
  <img src="../cardinal/mac-icon_1024x1024.png" alt="Cardinal-Symbol" width="120" height="120">
  <h1>Cardinal</h1>
  <p>Schnellste und genaueste Dateisuche-App für macOS.</p>
  <p>
    <a href="#cardinal-verwenden">Cardinal verwenden</a> ·
    <a href="#cardinal-bauen">Cardinal bauen</a>
  </p>
  <img src="UI.gif" alt="Vorschau der Cardinal-Oberfläche" width="720">
</div>

---

[English](../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Cardinal verwenden

### Download

Installation via Homebrew:

```bash
brew install --cask cardinal-search
```

Du kannst die neuesten Pakete auch von [GitHub Releases](https://github.com/cardisoft/cardinal/releases/) herunterladen.

### i18n-Unterstützung

Du brauchst eine andere Sprache? Klicke auf die ⚙️-Schaltfläche in der Statusleiste, um sofort umzuschalten.

### Suchgrundlagen

Cardinal unterstützt jetzt eine Everything-kompatible Syntaxschicht zusätzlich zu den klassischen Substring-/Prefix-Tricks:

- `report draft` – Leerzeichen wirkt als `AND`, du siehst nur Dateien, deren Namen beide Tokens enthalten.
- `*.pdf briefing` – filtert auf PDF-Ergebnisse, deren Namen „briefing“ enthalten.
- `*.zip size:>100MB` – sucht ZIP-Dateien größer als 100MB.
- `infolder:/Users demo !.psd` – begrenzt die Suchwurzel auf `/Users` und sucht nach `demo`, schließt aber `.psd` aus.
- `tag:ProjectA;ProjectB` – matcht Finder-Tags (macOS); `;` wirkt als `OR`.
- `*.md content:"Bearer "` – filtert auf Markdown-Dateien, die die Zeichenkette `Bearer ` enthalten.
- `"Application Support"` – Anführungszeichen für exakte Phrasen.
- `brary/Applicat` – nutze `/` als Pfadtrenner für Subpfad-Suche, passend zu Verzeichnissen wie `Library/Application Support`.
- `/report` · `draft/` · `/report/` – Tokens mit führendem und/oder nachgestelltem `/` erzwingen Präfix-, Suffix- oder exakte Namensmatches, wenn du ganzes Wort-Controlling über Everything-Syntax hinaus brauchst.
- `~/**/.DS_Store` – Globstar (`**`) durchläuft alle Unterordner unter deinem Home-Verzeichnis, um verstreute `.DS_Store`-Dateien zu finden.

Den vollständigen Operator-Katalog (Boolesche Gruppierung, Ordner-Scopes, Erweiterungsfilter, Regex-Nutzung und mehr Beispiele) findest du in [`search-syntax.md`](search-syntax.md).

### Tastenkürzel & Vorschauen

- `Space` – Quick Look der aktuell markierten Zeile, ohne Cardinal zu verlassen.
- `Cmd+R` – zeigt das markierte Ergebnis im Finder.
- `Cmd+F` – Fokus zurück in die Suchleiste.
- `Cmd+C` – kopiert den Pfad der ausgewählten Datei in die Zwischenablage.
- `Cmd+Shift+Space` – schaltet das Cardinal-Fenster global per Schnellstart-Hotkey um.

Viel Spaß beim Suchen!

---

## Cardinal bauen

### Voraussetzungen

- macOS 12+
- Rust-Toolchain
- Node.js 18+ mit npm
- Xcode-Command-Line-Tools & Tauri-Voraussetzungen (<https://tauri.app/start/prerequisites/>)

### Entwicklungsmodus

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Produktions-Build

```bash
cd cardinal
npm run tauri build
```
