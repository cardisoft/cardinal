<div align="center">
  <img src="../cardinal/mac-icon_1024x1024.png" alt="Icona di Cardinal" width="120" height="120">
  <h1>Cardinal</h1>
  <p>App di ricerca file per macOS più veloce e precisa.</p>
  <p>
    <a href="#utilizzare-cardinal">Utilizzare Cardinal</a> ·
    <a href="#compilare-cardinal">Compilare Cardinal</a>
  </p>
  <img src="UI.gif" alt="Anteprima dell'interfaccia di Cardinal" width="720">
</div>

---

[English](../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Utilizzare Cardinal

### Download

Installa con Homebrew:

```bash
brew install --cask cardinal-search
```

Puoi anche scaricare i pacchetti più recenti da [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### Supporto i18n

Ti serve un'altra lingua? Clicca sul pulsante ⚙️ nella barra di stato per cambiare al volo.

### Basi di ricerca

Cardinal ora affianca alla classica corrispondenza per sottostringa/prefisso una sintassi compatibile con Everything:

- `report draft` – lo spazio funge da `AND`, quindi vedi solo i file i cui nomi contengono entrambi i token.
- `*.pdf briefing` – filtra i risultati PDF il cui nome include “briefing”.
- `*.zip size:>100MB` – cerca file ZIP più grandi di 100MB.
- `in:/Users demo !.psd` – limita la radice di ricerca a `/Users`, poi cerca file con `demo` nel nome escludendo `.psd`.
- `tag:ProjectA;ProjectB` – corrisponde alle etichette del Finder (macOS); `;` agisce come `OR`.
- `*.md content:"Bearer "` – mostra solo i Markdown che contengono la stringa `Bearer `.
- `"Application Support"` – usa le virgolette per frasi esatte.
- `brary/Applicat` – usa `/` come separatore di percorso per cercare sottopercorsi, trovando directory come `Library/Application Support`.
- `/report` · `draft/` · `/report/` – avvolgi i token con barre iniziali o finali per forzare corrispondenze di prefisso, suffisso o nome esatto quando serve controllo per parola oltre la sintassi Everything.
- `~/**/.DS_Store` – il globstar (`**`) attraversa tutte le sottocartelle della tua home per trovare file `.DS_Store` sparsi.

Consulta il catalogo degli operatori supportati—inclusi raggruppamento booleano, ambito cartelle, filtri per estensione, uso di regex e altri esempi—in [`search-syntax.md`](search-syntax.md).

### Scorciatoie da tastiera e anteprime

- `Cmd+Shift+Space` – attiva o chiude la finestra di Cardinal globalmente tramite scorciatoia rapida.
- `Cmd+,` – apre le Preferenze.
- `Esc` – nasconde la finestra di Cardinal.
- `ArrowUp`/`ArrowDown` – sposta la selezione.
- `Shift+ArrowUp`/`Shift+ArrowDown` – estende la selezione.
- `Space` – Quick Look della riga selezionata senza lasciare Cardinal.
- `Cmd+O` – apre il risultato selezionato.
- `Cmd+R` – mostra il risultato evidenziato nel Finder.
- `Cmd+C` – copia i file selezionati negli appunti.
- `Cmd+Shift+C` – copia i percorsi selezionati negli appunti.
- `Cmd+F` – riporta il focus sulla barra di ricerca.
- `ArrowUp`/`ArrowDown` (nella barra di ricerca) – scorre la cronologia di ricerca.

Buona ricerca!

---

## Compilare Cardinal

### Requisiti

- macOS 12+
- Toolchain Rust
- Node.js 18+ con npm
- Strumenti da riga di comando Xcode e prerequisiti Tauri (<https://tauri.app/start/prerequisites/>)

### Modalità sviluppo

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Build di produzione

```bash
cd cardinal
npm run tauri build
```
