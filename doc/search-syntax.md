# Cardinal Search Syntax Reference

> Cardinal adopts an Everything-compatible query syntax that layers boolean logic, grouping, regex, folder scoping, and extension filters on top of the familiar substring search. Start with the README quick queries, then keep this page handy when you need the subset the engine executes today.

## Quick Start

- `report draft` — space means `AND`, so results must contain both tokens anywhere in the path.
- `ext:pdf briefing` — limit to PDF files whose names include “briefing”; Unicode text works out of the box.
- `Pictures vacation` — mix directory fragments and filename fragments freely.
- `parent:/Users demo!.psd` — stay under `/Users` and exclude `.psd` items.
- `regex:^Report.*2025$` — use regular expressions for structured name matches.
- `ext:png;jpg travel|vacation` — search multiple extensions with an `OR` clause.

Jump to [Examples](#examples) for a longer list that mirrors our regression tests.

## Tokens & Wildcards

- Plain text performs **substring** matches in a case-insensitive way. Flip the UI toggle whenever you need case-sensitive evaluation.
- `*` matches zero or more characters and `?` matches exactly one; the pattern must cover the full token, so `*.rs` or `foo??.txt` act on entire filenames. Pair with `parent:` / `infolder:` when you want to constrain the directory scope.
- Use wildcards mid-token for fuzzy spans: `a*b` finds anything starting with `a` and ending with `b`, while `report-??.txt` captures numbered variants. Quote a token (e.g., `"*.rs"`) when you need literal `*` or `?`.
- Wrap phrases or literal paths with spaces in double quotes: `"summer holiday"`, `"/Applications/Cardinal.app"`.

## Boolean Logic & Grouping

| Syntax      | Meaning                                                                 |
| ----------- | ----------------------------------------------------------------------- |
| `foo bar`   | Space = `AND`; both tokens are required.                                |
| `foo|bar`   | `OR`, can also be written `foo OR bar`.                                 |
| `!temp`     | `NOT`, also available as `NOT temp`.                                    |
| `<...>`     | Angle-bracket grouping, matching Everything’s default syntax.           |
| `( ... )`   | Parentheses are also allowed for people who prefer them.                |

NOT > OR > AND, so group expressions (e.g., `good (*.mp3|*.wav)`) any time you want a different order.

## Filter Cheat Sheet

| Filter              | Description & sample usage                                                                 |
| ------------------- | ------------------------------------------------------------------------------------------ |
| `file:` / `folder:` | Restrict the result kind, e.g. `folder: Projects` or `file: notes`.                        |
| `ext:`              | Accept a semicolon-separated extension list: `ext:jpg;png;gif`.                            |
| `parent:`           | Show only direct children of a directory: `parent:/Users/demo/Documents`.                  |
| `infolder:`         | Walk a directory recursively: `infolder:/Users/demo/Projects report`.                      |
| `nosubfolders:`     | Return only the files directly under a folder (no subfolders): `nosubfolders:/Users/demo/Projects`. |
| `type:`             | Category filters such as `type:picture`, `type:video`, `type:doc`, `type:archive`, etc.    |
| `audio:` / `video:` / `doc:` / `exe:` | Shorthand macros equivalent to `type:audio`, `type:video`, etc.          |
| `size:`             | Filter by file size with comparisons (`size:>1GB`), ranges (`size:1mb..10mb`), or keywords (`size:tiny`). |
| `dm:` / `dc:`       | Date modified/created filters with keywords (`dm:today`, `dc:thisweek`) or ranges (`dm:2024/01/01-2024/12/31`). |
| `regex:`            | Regular expressions (`regex:^README\..*`).                                                 |

Keywords such as `today`, `yesterday`, `thisweek`, `lastweek`, `thismonth`, `lastmonth`, `thisyear`, `lastyear`, `pastweek`, `pastmonth`, and `pastyear` work inside `dm:` / `dc:` filters, and you can combine them with comparison operators (`dm:>=2024-01-01`) or explicit ranges.

## Additional Notes

- Boolean expressions still understand parentheses and angle brackets: group segments like `<src|tests> ext:rs`.
- Use `!` to subtract results: `parent:/Users/demo/Documents !ext:pdf`.
- Paths and escaped terms share the same syntax as in Everything, so snippets from existing docs typically paste right in.

## Examples

```text
parent:/Users/demo/Documents ext:md
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
parent:/Users/demo/Documents !ext:pdf
infolder:/Users/demo/Projects report draft
parent:/Users/demo/Scripts *.sh
infolder:/Users/demo/Logs error-??.log
*.psd !parent:/Users/demo/Archive
"Application Support"
regex:^README\\.md$ parent:/Users/demo
Pictures vacation
D:\Projects\cardinal src|docs
parent:/Users demo!.psd
```
