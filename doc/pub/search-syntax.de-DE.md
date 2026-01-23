# Cardinal-Suchsyntax

Die Abfragesprache von Cardinal ist bewusst an die Syntax von Everything angelehnt, spiegelt aber das wider, was die aktuelle Engine tatsächlich implementiert. Diese Seite ist die maßgebliche Referenz dafür, was das Rust-Backend heute versteht.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Mentales Modell

- Jede Abfrage wird in einen Baum aus folgenden Elementen geparst:
  - **Wörter / Phrasen** (Plaintext, Anführungszeichen-Strings, Wildcards),
  - **Filter** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Boolesche Operatoren** (`AND`, `OR`, `NOT` / `!`).
- Das Matching erfolgt gegen den **vollständigen Pfad** jeder indizierten Datei, nicht nur gegen den Basisnamen.
- Die Groß-/Kleinschreibung wird durch den UI-Schalter gesteuert:
  - Bei **nicht case-sensitiv** konvertiert die Engine sowohl Abfrage als auch Kandidaten für Name-/Inhaltsabgleich in Kleinbuchstaben.
  - Bei **case-sensitiv** vergleicht die Engine die Bytes unverändert.

Schnelle Beispiele:
```text
report draft                  # Dateien, deren Pfad sowohl “report” als auch “draft” enthält
ext:pdf briefing              # PDF-Dateien, deren Name “briefing” enthält
parent:/Users demo!.psd       # unter /Users, .psd-Dateien ausschließen
regex:^Report.*2025$          # Namen, die einer Regex entsprechen
ext:png;jpg travel|vacation   # PNG oder JPG, deren Namen “travel” oder “vacation” enthalten
```

---

## 2. Tokens, Wildcards und Pfadsegmente

### 2.1 Einfache Tokens und Phrasen

- Ein Token ohne Anführungszeichen ist ein **Substring-Match** auf dem Pfad:
  - `demo` matcht `/Users/demo/Projects/cardinal.md`.
- Phrasen in doppelten Anführungszeichen matchen die exakte Sequenz inklusive Leerzeichen:
  - `"Application Support"` matcht `/Library/Application Support/...`.
- Der UI-Schalter für Groß-/Kleinschreibung gilt für beide.

### 2.2 Wildcards (`*`, `?`, `**`)

- `*` matcht null oder mehr Zeichen.
- `?` matcht genau ein Zeichen.
- `**` ist ein Globstar, der **beliebig viele Ordnersegmente** durchquert, wenn er zwischen Schrägstrichen steht.
- Wildcards werden **innerhalb eines einzelnen Tokens** verstanden:
  - `*.rs` — jeder Name, der auf `.rs` endet.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, usw.
  - `a*b` — Namen, die mit `a` beginnen und mit `b` enden.
  - `src/**/Cargo.toml` — `Cargo.toml` irgendwo unter `src/`.
- Wenn Sie ein literales `*` oder `?` brauchen, setzen Sie das Token in Anführungszeichen: `"*.rs"`. Globstars müssen eigenständige Slash-Segmente sein (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Pfadartige Segmentierung mit `/`

Cardinal versteht „Slash-Segmente“ innerhalb eines Tokens und klassifiziert jedes Segment als Prefix-/Suffix-/Exact-/Substring-Match auf Pfadkomponenten. Beispiele:

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

Damit können Sie ausdrücken:
- „Ordner muss mit X enden“ (`foo/`),
- „Ordner muss mit X beginnen“ (`/foo`),
- „Exakter Ordnername in der Mitte des Pfads“ (`gaea/lil/bee/`).

---

## 3. Boolesche Logik und Gruppierung

Cardinal folgt der Präzedenz von Everything:

- `NOT` / `!` bindet am stärksten,
- `OR` / `|` als Nächstes,
- implizites / explizites `AND` („Leerzeichen“) hat die **niedrigste** Präzedenz.

### 3.1 Operatoren

| Syntax         | Bedeutung                                         |
| -------------- | ------------------------------------------------- |
| `foo bar`      | `foo AND bar` — beide Tokens müssen matchen.     |
| `foo\|bar`      | `foo OR bar` — eines von beiden kann matchen.    |
| `foo OR bar`   | Wortform von `|`.                                |
| `!temp`        | `NOT temp` — schließt Treffer aus.               |
| `NOT temp`     | Dasselbe wie `!temp`.                            |
| `( ... )`      | Gruppierung mit Klammern.                        |
| `< ... >`      | Gruppierung mit spitzen Klammern (Everything-Stil). |

Präzedenzbeispiele:
```text
foo bar|baz        # wird als foo AND (bar OR baz) geparst
!(ext:zip report)  # schließt Elemente aus, bei denen ext:zip UND “report” matchen
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Verwenden Sie Klammern oder `<...>`, wenn Sie die Standardpräzedenz überschreiben möchten.

---

## 4. Filter

Dieser Abschnitt listet nur Filter auf, die die aktuelle Engine tatsächlich auswertet.

> **Hinweis**: Filterargumente müssen direkt nach dem Doppelpunkt folgen (`ext:jpg`, `parent:/Users/demo`). Das Schreiben von `file: *.md` fügt ein Leerzeichen ein, daher behandelt Cardinal das als `file:`-Filter (ohne Argument) gefolgt vom separaten Token `*.md`.

### 4.1 Datei-/Ordnerfilter

| Filter          | Bedeutung                          | Beispiel          |
| --------------- | ---------------------------------- | ----------------- |
| `file:`         | Nur Dateien (keine Ordner)         | `file: report`    |
| `folder:`       | Nur Ordner                         | `folder:Projects` |

Diese können mit anderen Begriffen kombiniert werden:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Erweiterungsfilter: `ext:`

- `ext:` akzeptiert eine oder mehrere Erweiterungen, getrennt durch `;`:
  - `ext:jpg` — JPEG-Bilder.
  - `ext:jpg;png;gif` — gängige Web-Bildtypen.
- Matching ist nicht case-sensitiv und enthält keinen Punkt.

Beispiele:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Ordnerbereich: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Filter            | Bedeutung                                                     | Beispiel                                        |
| ----------------- | ------------------------------------------------------------- | ---------------------------------------------- |
| `parent:`         | Nur direkte Kinder des angegebenen Ordners                    | `parent:/Users/demo/Documents ext:md`         |
| `infolder:`/`in:` | Jeder Nachkomme des angegebenen Ordners (rekursiv)            | `in:/Users/demo/Projects report draft`        |
| `nosubfolders:`   | Ordner selbst plus direkte Datei-Kinder (keine Unterordner)    | `nosubfolders:/Users/demo/Projects ext:log`   |

Diese Filter erwarten einen absoluten Pfad als Argument; ein führendes `~` wird in das Home-Verzeichnis des Users erweitert.

### 4.4 Typfilter: `type:`

`type:` gruppiert Dateiendungen in semantische Kategorien. Unterstützte Kategorien (case-insensitiv, mit Synonymen) sind:

- Bilder: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Video: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Audio: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Dokumente: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Präsentationen: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Tabellen: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Archive: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Code: `type:code`, `type:source`, `type:dev`
- Ausführbare Dateien: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Beispiele:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Typ-Makros: `audio:`, `video:`, `doc:`, `exe:`

Abkürzungen für gängige `type:`-Fälle:

| Makro   | Entspricht          | Beispiel              |
| ------ | ------------------- | --------------------- |
| `audio:` | `type:audio`       | `audio: piano`        |
| `video:` | `type:video`       | `video: tutorial`     |
| `doc:`   | `type:doc`         | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`         | `exe: "Cardinal"`    |

Makros akzeptieren ein optionales Argument:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Größenfilter: `size:`

`size:` unterstützt:

- **Vergleiche**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Bereiche**: `min..max`
- **Schlüsselwörter**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Einheiten**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Beispiele:
```text
size:>1GB                 # größer als 1 GB
size:1mb..10mb            # zwischen 1 MB und 10 MB
size:tiny                 # 0–10 KB (ungefähre Bereichsangabe)
size:empty                # exakt 0 Bytes
```

### 4.7 Datumsfilter: `dm:`, `dc:`

- `dm:` / `datemodified:` — Änderungsdatum.
- `dc:` / `datecreated:` — Erstellungsdatum.

Sie akzeptieren:

1. **Schlüsselwörter** (relative Bereiche):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Absolute Daten**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - Unterstützt auch gängige Tag‑zuerst / Monat‑zuerst-Formate wie `DD-MM-YYYY` und `MM/DD/YYYY`.

3. **Bereiche und Vergleiche**:
   - Bereiche: `dm:2024-01-01..2024-12-31`
   - Vergleiche: `dm:>=2024-01-01`, `dc:<2023/01/01`

Beispiele:
```text
dm:today                      # heute geändert
dc:lastyear                   # im letzten Kalenderjahr erstellt
dm:2024-01-01..2024-03-31     # in Q1 2024 geändert
dm:>=2024/01/01               # geändert ab 2024-01-01
```

### 4.8 Regex-Filter: `regex:`

`regex:` behandelt den Rest des Tokens als regulären Ausdruck, der auf eine Pfadkomponente (Datei- oder Ordnername) angewendet wird.

Beispiele:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

Der UI-Schalter für Groß-/Kleinschreibung beeinflusst Regex-Matching.

### 4.9 Inhaltsfilter: `content:`

`content:` durchsucht Dateiinhalte nach einer **einfachen Substring**:

- Keine Regex innerhalb von `content:` — es ist ein Byte-Substring-Match.
- Groß-/Kleinschreibung folgt dem UI-Schalter:
  - Im nicht case-sensitiven Modus werden Suchbegriff und gescannte Bytes kleingeschrieben.
  - Im case-sensitiven Modus werden Bytes unverändert verglichen.
- Sehr kurze Suchbegriffe sind erlaubt, aber `""` (leer) wird abgelehnt.

Beispiele:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

Das Inhalts-Matching erfolgt streamend über die Datei; Multibyte-Sequenzen können Puffergrenzen überschreiten.

### 4.10 Tag-Filter: `tag:` / `t:`

Filtert nach Finder-Tags (macOS). Cardinal holt Tags bei Bedarf aus den Metadaten der Datei (ohne Caching) und nutzt bei großen Ergebnismengen `mdfind`, um Kandidaten vorab einzugrenzen, bevor Tag-Matching angewendet wird.

- Akzeptiert ein oder mehrere Tags, getrennt durch `;` (logisches OR): `tag:ProjectA;ProjectB`.
- Ketten Sie mehrere `tag:`-Filter (logisches AND) für Multi-Tag-Matches: `tag:Project tag:Important`.
- Groß-/Kleinschreibung folgt dem UI-Schalter.
- Tag-Namen werden per Substring gematcht: `tag:proj` matcht `Project` und `project`.

Beispiele:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Beispiele

Einige realistische Kombinationen:

```text
#  Markdown-Notizen in Documents (keine PDFs)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDFs in Reports mit „briefing"
ext:pdf briefing parent:/Users/demo/Reports

#  Urlaubsfotos
type:picture vacation
ext:png;jpg travel|vacation

#  Aktuelle Logdateien innerhalb eines Projektbaums
in:/Users/demo/Projects ext:log dm:pastweek

#  Shell-Skripte direkt unter dem Ordner Scripts
parent:/Users/demo/Scripts *.sh

#  Alles mit „Application Support" im Pfad
"Application Support"

#  Einen bestimmten Dateinamen per Regex matchen
regex:^README\\.md$ parent:/Users/demo

#  PSDs überall unter /Users ausschließen
in:/Users demo!.psd
```

Nutzen Sie diese Seite als maßgebliche Liste der Operatoren und Filter, die die Engine heute implementiert; zusätzliche Everything-Funktionen (wie Zugriffs-/Ausführungsdaten oder attributbasierte Filter) werden auf Syntaxebene geparst, aber derzeit bei der Auswertung abgelehnt.
