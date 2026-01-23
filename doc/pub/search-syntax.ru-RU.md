# Синтаксис поиска Cardinal

Язык запросов Cardinal намеренно близок к синтаксису Everything, но отражает то, что текущий движок действительно реализует. Эта страница — эталонная справка о том, что сегодня понимает Rust‑бэкенд.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Ментальная модель

- Каждый запрос парсится в дерево из:
  - **Слов / фраз** (обычный текст, строки в кавычках, подстановки),
  - **Фильтров** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Логических операторов** (`AND`, `OR`, `NOT` / `!`).
- Сопоставление выполняется по **полным путям** всех проиндексированных файлов, а не только по базовому имени.
- Чувствительность к регистру управляется переключателем в UI:
  - В режиме **без учета регистра** движок приводит к нижнему регистру и запрос, и кандидаты для совпадений по имени/контенту.
  - В режиме **с учетом регистра** движок сравнивает байты как есть.

Быстрые примеры:
```text
report draft                  # файлы, путь которых содержит и “report”, и “draft”
ext:pdf briefing              # PDF-файлы, в имени которых есть “briefing”
parent:/Users demo!.psd       # под /Users исключить .psd файлы
regex:^Report.*2025$          # имена, совпадающие с regex
ext:png;jpg travel|vacation   # PNG или JPG, имена которых содержат “travel” или “vacation”
```

---

## 2. Токены, подстановки и сегменты пути

### 2.1 Простые токены и фразы

- Токен без кавычек — это **поиск по подстроке** в пути:
  - `demo` совпадает с `/Users/demo/Projects/cardinal.md`.
- Фразы в двойных кавычках совпадают с точной последовательностью, включая пробелы:
  - `"Application Support"` совпадает с `/Library/Application Support/...`.
- Переключатель регистра в UI применяется к обоим.

### 2.2 Подстановки (`*`, `?`, `**`)

- `*` соответствует нулю или большему числу символов.
- `?` соответствует ровно одному символу.
- `**` — это globstar, который пересекает **любое число сегментов папок**, когда стоит между слэшами.
- Подстановки распознаются **внутри одного токена**:
  - `*.rs` — любое имя, оканчивающееся на `.rs`.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt` и т. д.
  - `a*b` — имена, начинающиеся на `a` и заканчивающиеся на `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` где угодно под `src/`.
- Если нужен буквальный `*` или `?`, заключите токен в кавычки: `"*.rs"`. Globstar должен быть отдельным сегментом слеша (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Сегментация пути с `/`

Cardinal понимает “сегменты со слешами” внутри токена и классифицирует каждый сегмент как префиксное/суффиксное/точное/подстрочное совпадение по компонентам пути. Примеры:

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

Это позволяет выражать:
- «Папка должна заканчиваться на X» (`foo/`),
- «Папка должна начинаться с X» (`/foo`),
- «Точное имя папки в середине пути» (`gaea/lil/bee/`).

---

## 3. Логика и группировка

Cardinal следует приоритетам Everything:

- `NOT` / `!` имеет самый высокий приоритет,
- `OR` / `|` следующий,
- неявный / явный `AND` («пробел») имеет **самый низкий** приоритет.

### 3.1 Операторы

| Синтаксис      | Значение                                         |
| -------------- | ------------------------------------------------ |
| `foo bar`      | `foo AND bar` — оба токена должны совпасть.     |
| `foo\|bar`      | `foo OR bar` — может совпасть любой.           |
| `foo OR bar`   | Словесная форма `|`.                            |
| `!temp`        | `NOT temp` — исключает совпадения.             |
| `NOT temp`     | То же, что `!temp`.                             |
| `( ... )`      | Группировка скобками.                           |
| `< ... >`      | Группировка угловыми скобками (стиль Everything). |

Примеры приоритетов:
```text
foo bar|baz        # разбирается как foo AND (bar OR baz)
!(ext:zip report)  # исключает элементы, где совпадают ext:zip И “report”
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Используйте скобки или `<...>`, когда нужно переопределить приоритет по умолчанию.

---

## 4. Фильтры

В этом разделе перечислены только фильтры, которые текущий движок действительно вычисляет.

> **Примечание**: аргументы фильтра должны идти сразу после двоеточия (`ext:jpg`, `parent:/Users/demo`). Запись `file: *.md` вставляет пробел, поэтому Cardinal трактует это как фильтр `file:` (без аргумента), за которым следует отдельный токен `*.md`.

### 4.1 Фильтры файлов / папок

| Фильтр          | Значение                         | Пример             |
| --------------- | -------------------------------- | ------------------ |
| `file:`         | Только файлы (не папки)          | `file: report`     |
| `folder:`       | Только папки                     | `folder:Projects`  |

Их можно комбинировать с другими терминами:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Фильтр расширений: `ext:`

- `ext:` принимает одно или несколько расширений, разделенных `;`:
  - `ext:jpg` — изображения JPEG.
  - `ext:jpg;png;gif` — распространенные форматы веб‑изображений.
- Сопоставление не чувствительно к регистру и не включает точку.

Примеры:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Область папок: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Фильтр            | Значение                                                     | Пример                                        |
| ----------------- | ------------------------------------------------------------- | -------------------------------------------- |
| `parent:`         | Только прямые дочерние элементы указанной папки              | `parent:/Users/demo/Documents ext:md`       |
| `infolder:`/`in:` | Любой потомок указанной папки (рекурсивно)                    | `in:/Users/demo/Projects report draft`      |
| `nosubfolders:`   | Папка сама плюс прямые дочерние файлы (без подпапок)          | `nosubfolders:/Users/demo/Projects ext:log` |

Эти фильтры принимают абсолютный путь в качестве аргумента; начальный `~` разворачивается в домашний каталог пользователя.

### 4.4 Фильтр типа: `type:`

`type:` группирует расширения файлов по смысловым категориям. Поддерживаемые категории (без учета регистра, с синонимами) включают:

- Изображения: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Видео: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Аудио: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Документы: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Презентации: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Таблицы: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Архивы: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Код: `type:code`, `type:source`, `type:dev`
- Исполняемые файлы: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Примеры:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Макросы типа: `audio:`, `video:`, `doc:`, `exe:`

Сокращения для распространенных случаев `type:`:

| Макрос   | Эквивалент        | Пример                |
| ------- | ----------------- | ---------------------- |
| `audio:` | `type:audio`     | `audio: piano`         |
| `video:` | `type:video`     | `video: tutorial`      |
| `doc:`   | `type:doc`       | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`       | `exe: "Cardinal"`     |

Макросы принимают необязательный аргумент:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Фильтр размера: `size:`

`size:` поддерживает:

- **Сравнения**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Диапазоны**: `min..max`
- **Ключевые слова**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Единицы**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Примеры:
```text
size:>1GB                 # больше 1 GB
size:1mb..10mb            # между 1 MB и 10 MB
size:tiny                 # 0–10 KB (приблизительный диапазон по ключевому слову)
size:empty                # ровно 0 байт
```

### 4.7 Фильтры даты: `dm:`, `dc:`

- `dm:` / `datemodified:` — дата изменения.
- `dc:` / `datecreated:` — дата создания.

Они принимают:

1. **Ключевые слова** (относительные диапазоны):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Абсолютные даты**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - Также поддерживаются распространенные форматы день‑сначала / месяц‑сначала, такие как `DD-MM-YYYY` и `MM/DD/YYYY`.

3. **Диапазоны и сравнения**:
   - Диапазоны: `dm:2024-01-01..2024-12-31`
   - Сравнения: `dm:>=2024-01-01`, `dc:<2023/01/01`

Примеры:
```text
dm:today                      # изменено сегодня
dc:lastyear                   # создано в прошлом календарном году
dm:2024-01-01..2024-03-31     # изменено в Q1 2024
dm:>=2024/01/01               # изменено начиная с 2024-01-01
```

### 4.8 Фильтр regex: `regex:`

`regex:` воспринимает оставшуюся часть токена как регулярное выражение, применяемое к компоненту пути (имени файла или папки).

Примеры:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

Переключатель регистра в UI влияет на совпадения regex.

### 4.9 Фильтр содержимого: `content:`

`content:` сканирует содержимое файлов на **простую подстроку**:

- В `content:` нет regex — это совпадение подстроки по байтам.
- Чувствительность к регистру следует переключателю UI:
  - В режиме без учета регистра и строка поиска, и сканируемые байты приводятся к нижнему регистру.
  - В режиме с учетом регистра байты сравниваются как есть.
- Очень короткие подстроки разрешены, но `""` (пустая) отклоняется.

Примеры:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

Сопоставление содержимого выполняется потоково по файлу; многобайтовые последовательности могут пересекать границы буфера.

### 4.10 Фильтр тегов: `tag:` / `t:`

Фильтрует по тегам Finder (macOS). Cardinal запрашивает теги по необходимости из метаданных файла (без кэша) и для больших наборов результатов использует `mdfind`, чтобы сузить кандидатов перед применением сопоставления тегов.

- Принимает один или несколько тегов, разделенных `;` (логическое OR): `tag:ProjectA;ProjectB`.
- Можно цепочкой использовать несколько фильтров `tag:` (логическое AND) для много-тегового совпадения: `tag:Project tag:Important`.
- Чувствительность к регистру следует переключателю UI.
- Имена тегов сопоставляются по подстроке: `tag:proj` совпадает с `Project` и `project`.

Примеры:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Примеры

Несколько реалистичных комбинаций:

```text
#  Markdown-заметки в Documents (без PDF)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDF в Reports с упоминанием “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  Фото из отпуска
type:picture vacation
ext:png;jpg travel|vacation

#  Свежие лог-файлы внутри дерева проекта
in:/Users/demo/Projects ext:log dm:pastweek

#  Shell-скрипты напрямую под папкой Scripts
parent:/Users/demo/Scripts *.sh

#  Все с “Application Support” в пути
"Application Support"

#  Сопоставить конкретное имя файла через regex
regex:^README\\.md$ parent:/Users/demo

#  Исключить PSD везде под /Users
in:/Users demo!.psd
```

Используйте эту страницу как авторитетный список операторов и фильтров, которые движок реализует сегодня; дополнительные возможности Everything (например, даты доступа/запуска или фильтры по атрибутам) разбираются на уровне синтаксиса, но сейчас отклоняются на этапе оценки.
