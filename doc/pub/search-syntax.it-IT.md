# Sintassi di ricerca di Cardinal

Il linguaggio di query di Cardinal è volutamente vicino alla sintassi di Everything, ma riflette ciò che l'attuale motore implementa davvero. Questa pagina è la fonte di verità su ciò che il backend in Rust comprende oggi.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Modello mentale

- Ogni query viene analizzata in un albero di:
  - **Parole / frasi** (testo semplice, stringhe tra virgolette, caratteri jolly),
  - **Filtri** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Operatori booleani** (`AND`, `OR`, `NOT` / `!`).
- La corrispondenza avviene sul **percorso completo** di ogni file indicizzato, non solo sul basename.
- La distinzione tra maiuscole/minuscole è controllata dal toggle della UI:
  - Quando è **insensibile al maiuscolo/minuscolo**, il motore converte in minuscolo sia la query sia i candidati per il matching di nome/contenuto.
  - Quando è **sensibile al maiuscolo/minuscolo**, il motore confronta i byte così come sono.

Esempi rapidi:
```text
report draft                  # file il cui percorso contiene sia “report” sia “draft”
ext:pdf briefing              # file PDF il cui nome contiene “briefing”
parent:/Users demo!.psd       # sotto /Users, escludi file .psd
regex:^Report.*2025$          # nomi che corrispondono a una regex
ext:png;jpg travel|vacation   # PNG o JPG i cui nomi contengono “travel” o “vacation”
```

---

## 2. Token, caratteri jolly e segmenti di percorso

### 2.1 Token semplici e frasi

- Un token senza virgolette è una **corrispondenza per sottostringa** nel percorso:
  - `demo` corrisponde a `/Users/demo/Projects/cardinal.md`.
- Le frasi tra virgolette doppie corrispondono alla sequenza esatta, inclusi gli spazi:
  - `"Application Support"` corrisponde a `/Library/Application Support/...`.
- Il toggle di maiuscole/minuscole della UI si applica a entrambi.

### 2.2 Caratteri jolly (`*`, `?`, `**`)

- `*` corrisponde a zero o più caratteri.
- `?` corrisponde esattamente a un carattere.
- `**` è un globstar che attraversa **qualsiasi numero di segmenti di cartella** quando appare tra le barre.
- I caratteri jolly sono interpretati **all'interno di un singolo token**:
  - `*.rs` — qualsiasi nome che termina con `.rs`.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, ecc.
  - `a*b` — nomi che iniziano con `a` e finiscono con `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` ovunque sotto `src/`.
- Se ti serve un `*` o `?` letterale, racchiudi il token tra virgolette: `"*.rs"`. I globstar devono essere segmenti di barra autonomi (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Segmentazione in stile percorso con `/`

Cardinal comprende i “segmenti con barra” all'interno di un token e classifica ogni segmento come corrispondenza di prefisso/suffisso/esatta/sottostringa sui componenti del percorso. Esempi:

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

Questo ti permette di esprimere:
- “La cartella deve terminare con X” (`foo/`),
- “La cartella deve iniziare con X” (`/foo`),
- “Nome cartella esatto nel mezzo del percorso” (`gaea/lil/bee/`).

---

## 3. Logica booleana e raggruppamento

Cardinal segue la precedenza di Everything:

- `NOT` / `!` ha la precedenza più alta,
- `OR` / `|` viene dopo,
- `AND` implicito / esplicito (“spazio”) ha la **precedenza più bassa**.

### 3.1 Operatori

| Sintassi        | Significato                                      |
| --------------- | ------------------------------------------------ |
| `foo bar`       | `foo AND bar` — entrambi i token devono combaciare. |
| `foo\|bar`       | `foo OR bar` — può combaciare uno qualunque.       |
| `foo OR bar`    | Forma testuale di `|`.                           |
| `!temp`         | `NOT temp` — esclude corrispondenze.             |
| `NOT temp`      | Uguale a `!temp`.                                |
| `( ... )`       | Raggruppamento con parentesi.                    |
| `< ... >`       | Raggruppamento con parentesi angolari (stile Everything). |

Esempi di precedenza:
```text
foo bar|baz        # analizzato come foo AND (bar OR baz)
!(ext:zip report)  # esclude elementi dove corrispondono ext:zip E “report”
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Usa le parentesi o `<...>` quando vuoi sovrascrivere la precedenza predefinita.

---

## 4. Filtri

Questa sezione elenca solo i filtri che il motore attuale valuta davvero.

> **Nota**: gli argomenti del filtro devono seguire subito i due punti (`ext:jpg`, `parent:/Users/demo`). Scrivere `file: *.md` inserisce uno spazio, quindi Cardinal lo tratta come un filtro `file:` (senza argomento) seguito dal token separato `*.md`.

### 4.1 Filtri file / cartella

| Filtro           | Significato                        | Esempio            |
| ---------------- | ---------------------------------- | ------------------ |
| `file:`          | Solo file (non cartelle)           | `file: report`     |
| `folder:`        | Solo cartelle                      | `folder:Projects`  |

Questi possono essere combinati con altri termini:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Filtro estensione: `ext:`

- `ext:` accetta una o più estensioni separate da `;`:
  - `ext:jpg` — immagini JPEG.
  - `ext:jpg;png;gif` — tipi comuni di immagini web.
- La corrispondenza non è sensibile al maiuscolo/minuscolo e non include il punto.

Esempi:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Ambito cartella: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Filtro            | Significato                                                      | Esempio                                        |
| ----------------- | ---------------------------------------------------------------- | ---------------------------------------------- |
| `parent:`         | Solo figli diretti della cartella indicata                       | `parent:/Users/demo/Documents ext:md`         |
| `infolder:`/`in:` | Qualsiasi discendente della cartella indicata (ricorsivo)         | `in:/Users/demo/Projects report draft`        |
| `nosubfolders:`   | La cartella stessa più i file figli diretti (senza sottocartelle) | `nosubfolders:/Users/demo/Projects ext:log`   |

Questi filtri accettano un percorso assoluto come argomento; un `~` iniziale viene espanso alla home dell'utente.

### 4.4 Filtro di tipo: `type:`

`type:` raggruppa le estensioni dei file in categorie semantiche. Le categorie supportate (non sensibili al maiuscolo/minuscolo, con sinonimi) includono:

- Immagini: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Video: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Audio: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Documenti: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Presentazioni: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Fogli di calcolo: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Archivi: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Codice: `type:code`, `type:source`, `type:dev`
- Eseguibili: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Esempi:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Macro di tipo: `audio:`, `video:`, `doc:`, `exe:`

Scorciatoie per casi comuni di `type:`:

| Macro    | Equivalente a       | Esempio                |
| ------- | ------------------- | ---------------------- |
| `audio:` | `type:audio`       | `audio: piano`         |
| `video:` | `type:video`       | `video: tutorial`      |
| `doc:`   | `type:doc`         | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`         | `exe: "Cardinal"`     |

Le macro accettano un argomento opzionale:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Filtro dimensione: `size:`

`size:` supporta:

- **Confronti**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Intervalli**: `min..max`
- **Parole chiave**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Unità**: byte (`b`), kilobyte (`k`, `kb`, `kib`, `kilobyte[s]`), megabyte (`m`, `mb`, `mib`, `megabyte[s]`), gigabyte (`g`, `gb`, `gib`, `gigabyte[s]`), terabyte (`t`, `tb`, `tib`, `terabyte[s]`), petabyte (`p`, `pb`, `pib`, `petabyte[s]`).

Esempi:
```text
size:>1GB                 # maggiore di 1 GB
size:1mb..10mb            # tra 1 MB e 10 MB
size:tiny                 # 0–10 KB (intervallo approssimativo per parola chiave)
size:empty                # esattamente 0 byte
```

### 4.7 Filtri data: `dm:`, `dc:`

- `dm:` / `datemodified:` — data di modifica.
- `dc:` / `datecreated:` — data di creazione.

Accettano:

1. **Parole chiave** (intervalli relativi):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Date assolute**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - Supporta anche formati comuni giorno‑prima / mese‑prima come `DD-MM-YYYY` e `MM/DD/YYYY`.

3. **Intervalli e confronti**:
   - Intervalli: `dm:2024-01-01..2024-12-31`
   - Confronti: `dm:>=2024-01-01`, `dc:<2023/01/01`

Esempi:
```text
dm:today                      # modificato oggi
dc:lastyear                   # creato lo scorso anno solare
dm:2024-01-01..2024-03-31     # modificato nel Q1 2024
dm:>=2024/01/01               # modificato dal 2024-01-01 in poi
```

### 4.8 Filtro regex: `regex:`

`regex:` tratta il resto del token come un'espressione regolare applicata a un componente del percorso (nome file o cartella).

Esempi:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

Il toggle di sensibilità alle maiuscole/minuscole della UI influisce sul matching regex.

### 4.9 Filtro contenuto: `content:`

`content:` esegue la scansione dei contenuti dei file cercando una **sottostringa semplice**:

- Nessuna regex dentro `content:` — è una corrispondenza di sottostringa di byte.
- La sensibilità a maiuscole/minuscole segue il toggle della UI:
  - In modalità non sensibile, il motore converte in minuscolo sia l'ago sia i byte analizzati.
  - In modalità sensibile, i byte vengono confrontati così come sono.
- Sono ammessi aghi molto piccoli, ma `""` (vuoto) viene rifiutato.

Esempi:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

La corrispondenza del contenuto avviene in streaming sul file; le sequenze multibyte possono attraversare i limiti del buffer.

### 4.10 Filtro tag: `tag:` / `t:`

Filtra per tag di Finder (macOS). Cardinal recupera i tag su richiesta dai metadati del file (senza caching) e, per set di risultati grandi, usa `mdfind` per restringere i candidati prima di applicare il matching dei tag.

- Accetta uno o più tag separati da `;` (OR logico): `tag:ProjectA;ProjectB`.
- Concatena più filtri `tag:` (AND logico) per match multi‑tag: `tag:Project tag:Important`.
- La sensibilità a maiuscole/minuscole segue il toggle della UI.
- Corrisponde ai nomi dei tag per sottostringa: `tag:proj` corrisponde a `Project` e `project`.

Esempi:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Esempi

Alcune combinazioni realistiche:

```text
#  Note Markdown in Documents (senza PDF)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDF in Reports che menzionano “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  Foto di vacanze
type:picture vacation
ext:png;jpg travel|vacation

#  File di log recenti all'interno di un albero di progetto
in:/Users/demo/Projects ext:log dm:pastweek

#  Script di shell direttamente sotto la cartella Scripts
parent:/Users/demo/Scripts *.sh

#  Tutto con “Application Support” nel percorso
"Application Support"

#  Corrispondenza di un nome file specifico via regex
regex:^README\\.md$ parent:/Users/demo

#  Escludere PSD ovunque sotto /Users
in:/Users demo!.psd
```

Usa questa pagina come elenco autorevole di operatori e filtri che il motore implementa oggi; funzionalità aggiuntive di Everything (come date di accesso/esecuzione o filtri basati su attributi) vengono analizzate a livello di sintassi ma attualmente sono rifiutate durante la valutazione.
