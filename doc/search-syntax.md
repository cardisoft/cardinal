# Cardinal Search Syntax

Cardinal’s query language is intentionally close to Everything’s syntax, while reflecting what the current engine actually implements. This page is the ground‑truth reference for what the Rust backend understands today.

---

## 1. Mental model

- Every query is parsed into a tree of:
  - **Words / phrases** (plain text, quoted strings, wildcards),
  - **Filters** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Boolean operators** (`AND`, `OR`, `NOT` / `!`).
- Matching runs against the **full path** of every indexed file, not just the basename.
- Case sensitivity is controlled by the UI toggle:
  - When **case-insensitive**, the engine lowercases both query and candidates for name/content matching.
  - When **case-sensitive**, the engine compares bytes as‑is.

Quick examples:
```text
report draft                  # files whose path contains both “report” and “draft”
ext:pdf briefing              # PDF files whose name contains “briefing”
parent:/Users demo!.psd       # under /Users, exclude .psd files
regex:^Report.*2025$          # names matching a regex
ext:png;jpg travel|vacation   # PNG or JPG whose names contain “travel” or “vacation”
```

---

## 2. Tokens, wildcards, and path segments

### 2.1 Plain tokens and phrases

- An unquoted token is a **substring match** on the path:
  - `demo` matches `/Users/demo/Projects/cardinal.md`.
- Double‑quoted phrases match the exact sequence including spaces:
  - `"Application Support"` matches `/Library/Application Support/...`.
- The UI case‑sensitivity toggle applies to both.

### 2.2 Wildcards (`*`, `?`, `**`)

- `*` matches zero or more characters.
- `?` matches exactly one character.
- `**` is a globstar that crosses **any number of folder segments** when it appears between slashes.
- Wildcards are understood **within a single token**:
  - `*.rs` — any name ending with `.rs`.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, etc.
  - `a*b` — names starting with `a` and ending with `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` anywhere below `src/`.
- If you need literal `*` or `?`, quote the token: `"*.rs"`. Globstars must be standalone slash segments (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Path‑style segmentation with `/`

Cardinal understands “slash‑segments” inside a token and classifies each segment as a prefix/suffix/exact/substring match on path components. Examples:

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

This lets you express:
- “Folder must end with X” (`foo/`),
- “Folder must start with X” (`/foo`),
- “Exact folder name in the middle of the path” (`gaea/lil/bee/`).

---

## 3. Boolean logic and grouping

Cardinal follows Everything’s precedence:

- `NOT` / `!` binds tightest,
- `OR` / `|` next,
- implicit / explicit `AND` (“space”) has the **lowest** precedence.

### 3.1 Operators

| Syntax         | Meaning                                               |
| -------------- | ----------------------------------------------------- |
| `foo bar`      | `foo AND bar` — both tokens must match.              |
| `foo\|bar`      | `foo OR bar` — either can match.                     |
| `foo OR bar`   | Word form of `|`.                                    |
| `!temp`        | `NOT temp` — exclude matches.                        |
| `NOT temp`     | Same as `!temp`.                                     |
| `( ... )`      | Grouping with parentheses.                           |
| `< ... >`      | Grouping with angle brackets (Everything-style).     |

Precedence examples:
```text
foo bar|baz        # parsed as foo AND (bar OR baz)
!(ext:zip report)  # exclude items where both ext:zip AND “report” match
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Use parentheses or `<...>` any time you want to override the default precedence.

---

## 4. Filters

This section only lists filters that the current engine actually evaluates.

> **Note**: filter arguments must follow the colon immediately (`ext:jpg`, `parent:/Users/demo`). Writing `file: *.md` inserts whitespace, so Cardinal treats it as a `file:` filter (with no argument) followed by the separate token `*.md`.

### 4.1 File / folder filters

| Filter              | Meaning                                       | Example                                |
| ------------------- | --------------------------------------------- | -------------------------------------- |
| `file:`             | Only files (not folders)                      | `file: report`                         |
| `folder:`           | Only folders                                  | `folder:Projects`                      |

These can be combined with other terms:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Extension filter: `ext:`

- `ext:` accepts one or more extensions separated by `;`:
  - `ext:jpg` — JPEG images.
  - `ext:jpg;png;gif` — common web image types.
- Matching is case-insensitive and does not include the dot.

Examples:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Folder scope: `parent:`, `infolder:`, `nosubfolders:`

| Filter          | Meaning                                                   | Example                                           |
| --------------- | --------------------------------------------------------- | ------------------------------------------------- |
| `parent:`       | Direct children of the given folder only                  | `parent:/Users/demo/Documents ext:md`            |
| `infolder:`     | Any descendant of the given folder (recursive)           | `infolder:/Users/demo/Projects report draft`     |
| `nosubfolders:` | Files in a folder but not in any of its subfolders       | `nosubfolders:/Users/demo/Projects ext:log`      |

These filters take an absolute path as their argument.

### 4.4 Type filter: `type:`

`type:` groups file extensions into semantic categories. Supported categories (case-insensitive, with synonyms) include:

- Pictures: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Video: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Audio: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Documents: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Presentations: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Spreadsheets: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Archives: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Code: `type:code`, `type:source`, `type:dev`
- Executables: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Examples:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Type macros: `audio:`, `video:`, `doc:`, `exe:`

Shortcuts for common `type:` cases:

| Macro    | Equivalent to                     | Example                 |
| -------- | --------------------------------- | ----------------------- |
| `audio:` | `type:audio`                      | `audio: piano`          |
| `video:` | `type:video`                      | `video: tutorial`       |
| `doc:`   | `type:doc`                        | `doc: invoice dm:2024`  |
| `exe:`   | `type:exe`                        | `exe: "Cardinal"`       |

Macros accept an optional argument:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Size filter: `size:`

`size:` supports:

- **Comparisons**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Ranges**: `min..max`
- **Keywords**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Units**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Examples:
```text
size:>1GB                 # larger than 1 GB
size:1mb..10mb            # between 1 MB and 10 MB
size:tiny                 # 0–10 KB (approximate keyword range)
size:empty                # exactly 0 bytes
```

### 4.7 Date filters: `dm:`, `dc:`

- `dm:` / `datemodified:` — date modified.
- `dc:` / `datecreated:` — date created.

They accept:

1. **Keywords** (relative ranges):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Absolute dates**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - Also supports common day‑first / month‑first layouts like `DD-MM-YYYY` and `MM/DD/YYYY`.

3. **Ranges and comparisons**:
   - Ranges: `dm:2024-01-01..2024-12-31`
   - Comparisons: `dm:>=2024-01-01`, `dc:<2023/01/01`

Examples:
```text
dm:today                      # changed today
dc:lastyear                   # created last calendar year
dm:2024-01-01..2024-03-31     # modified in Q1 2024
dm:>=2024/01/01               # modified from 2024-01-01 onwards
```

### 4.8 Regex filter: `regex:`

`regex:` treats the rest of the token as a regular expression applied to the filename (within the path). It uses Rust’s `regex` engine.

Examples:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

The UI case-sensitivity toggle affects regex matching.

### 4.9 Content filter: `content:`

`content:` scans file contents for a **plain substring**:

- No regex inside `content:` — it is a byte substring match.
- Case-sensitivity follows the UI toggle:
  - In case-insensitive mode, the needle and scanned bytes are lowercased.
  - In case-sensitive mode, bytes are compared as-is.
- Very small needles are allowed, but `""` (empty) is rejected.

Examples:
```text
*.md content:"Bearer "
ext:md content:"API key"
infolder:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

Content matching is done in streaming fashion over the file; multi-byte sequences can span buffer boundaries.

### 4.10 Tag filter: `tag:`

Filters by Finder tags (macOS). Cardinal fetches tags on demand from the file’s metadata (no caching) and matches the needle against each tag name.

- Accepts one or more tags separated by `;` (logical OR): `tag:ProjectA;ProjectB`.
- Chain multiple `tag:` filters (logical AND) for multi-tag matches: `tag:Project tag:Important`.
- Case-sensitivity follows the UI toggle.
- Matches tag names by substring: `tag:proj` matches `Project` and `project`.

Examples:
```text
tag:Important
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
infolder:/Users/demo/Documents tag:"Q4"
```

---

## 5. Examples

Some realistic combinations:

```text
#  Markdown notes in Documents (no PDFs)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDFs in Reports mentioning “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  Pictures from vacations
type:picture vacation
ext:png;jpg travel|vacation

#  Recent log files inside a project tree
infolder:/Users/demo/Projects ext:log dm:pastweek

#  Shell scripts directly under Scripts folder
parent:/Users/demo/Scripts *.sh

#  Everything with “Application Support” in the path
"Application Support"

#  Matching a specific filename via regex
regex:^README\\.md$ parent:/Users/demo

#  Exclude PSDs anywhere under /Users
infolder:/Users demo!.psd
```

Use this page as the authoritative list of operators and filters that the engine implements today; additional Everything features (like access/run dates or attribute-based filters) are parsed at the syntax level but currently rejected during evaluation.
