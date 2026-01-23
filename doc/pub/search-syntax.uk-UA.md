# Синтаксис пошуку Cardinal

Мова запитів Cardinal навмисно наближена до синтаксису Everything, але відображає те, що поточний рушій справді реалізує. Ця сторінка — еталонна довідка про те, що сьогодні розуміє бекенд на Rust.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Ментальна модель

- Кожен запит парситься в дерево з:
  - **Слів / фраз** (звичайний текст, рядки в лапках, підстановки),
  - **Фільтрів** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Булевих операторів** (`AND`, `OR`, `NOT` / `!`).
- Зіставлення виконується за **повним шляхом** кожного індексованого файлу, а не лише за базовою назвою.
- Чутливість до регістру керується перемикачем в UI:
  - У режимі **без урахування регістру** рушій приводить до нижнього регістру і запит, і кандидатів для збігів імені/вмісту.
  - У режимі **з урахуванням регістру** рушій порівнює байти як є.

Швидкі приклади:
```text
report draft                  # файли, шлях яких містить і “report”, і “draft”
ext:pdf briefing              # PDF-файли, у назві яких є “briefing”
parent:/Users demo!.psd       # під /Users виключити .psd файли
regex:^Report.*2025$          # назви, що відповідають regex
ext:png;jpg travel|vacation   # PNG або JPG, чиї назви містять “travel” або “vacation”
```

---

## 2. Токени, підстановки та сегменти шляху

### 2.1 Прості токени та фрази

- Токен без лапок — це **пошук за підрядком** у шляху:
  - `demo` збігається з `/Users/demo/Projects/cardinal.md`.
- Фрази в подвійних лапках збігаються з точною послідовністю, включно з пробілами:
  - `"Application Support"` збігається з `/Library/Application Support/...`.
- Перемикач регістру в UI застосовується до обох.

### 2.2 Підстановки (`*`, `?`, `**`)

- `*` відповідає нулю або більшій кількості символів.
- `?` відповідає рівно одному символу.
- `**` — це globstar, який проходить **будь-яку кількість сегментів папок**, коли стоїть між слешами.
- Підстановки розпізнаються **всередині одного токена**:
  - `*.rs` — будь-яка назва, що закінчується на `.rs`.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, тощо.
  - `a*b` — назви, що починаються на `a` і закінчуються на `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` будь-де під `src/`.
- Якщо потрібен буквальний `*` або `?`, візьміть токен у лапки: `"*.rs"`. Globstar має бути окремим сегментом слеша (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Сегментація шляхів з `/`

Cardinal розуміє “сегменти зі слешем” у токені та класифікує кожен сегмент як префіксне/суфіксне/точне/підрядкове зіставлення для компонентів шляху. Приклади:

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

Це дозволяє виразити:
- «Папка має закінчуватися на X» (`foo/`),
- «Папка має починатися з X» (`/foo`),
- «Точна назва папки всередині шляху» (`gaea/lil/bee/`).

---

## 3. Булева логіка та групування

Cardinal дотримується пріоритетів Everything:

- `NOT` / `!` має найвищий пріоритет,
- `OR` / `|` — наступний,
- неявний / явний `AND` («пробіл») має **найнижчий** пріоритет.

### 3.1 Оператори

| Синтаксис      | Значення                                         |
| -------------- | ------------------------------------------------ |
| `foo bar`      | `foo AND bar` — обидва токени мають збігтися.   |
| `foo\|bar`      | `foo OR bar` — може збігтися будь-який.         |
| `foo OR bar`   | Словесна форма `|`.                             |
| `!temp`        | `NOT temp` — виключає збіги.                    |
| `NOT temp`     | Те саме, що `!temp`.                            |
| `( ... )`      | Групування дужками.                             |
| `< ... >`      | Групування кутовими дужками (стиль Everything). |

Приклади пріоритетів:
```text
foo bar|baz        # розбирається як foo AND (bar OR baz)
!(ext:zip report)  # виключає елементи, де збігаються ext:zip І “report”
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Використовуйте дужки або `<...>`, коли потрібно перевизначити пріоритет за замовчуванням.

---

## 4. Фільтри

У цьому розділі перелічено лише фільтри, які поточний рушій справді обчислює.

> **Примітка**: аргументи фільтра мають іти одразу після двокрапки (`ext:jpg`, `parent:/Users/demo`). Запис `file: *.md` вставляє пробіл, тож Cardinal трактує це як фільтр `file:` (без аргументу), за яким іде окремий токен `*.md`.

### 4.1 Фільтри файлів / папок

| Фільтр          | Значення                         | Приклад            |
| --------------- | -------------------------------- | ------------------ |
| `file:`         | Лише файли (не папки)            | `file: report`     |
| `folder:`       | Лише папки                       | `folder:Projects`  |

Їх можна комбінувати з іншими термінами:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Фільтр розширень: `ext:`

- `ext:` приймає одне або кілька розширень, розділених `;`:
  - `ext:jpg` — зображення JPEG.
  - `ext:jpg;png;gif` — поширені типи веб‑зображень.
- Зіставлення не чутливе до регістру і не включає крапку.

Приклади:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Область папок: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Фільтр            | Значення                                                     | Приклад                                        |
| ----------------- | ------------------------------------------------------------- | -------------------------------------------- |
| `parent:`         | Лише прямі нащадки вказаної папки                             | `parent:/Users/demo/Documents ext:md`       |
| `infolder:`/`in:` | Будь-який нащадок вказаної папки (рекурсивно)                 | `in:/Users/demo/Projects report draft`      |
| `nosubfolders:`   | Папка сама плюс прямі дочірні файли (без підпапок)            | `nosubfolders:/Users/demo/Projects ext:log` |

Ці фільтри приймають абсолютний шлях як аргумент; початковий `~` розгортається до домашнього каталогу користувача.

### 4.4 Фільтр типу: `type:`

`type:` групує розширення файлів у семантичні категорії. Підтримувані категорії (без урахування регістру, із синонімами) включають:

- Зображення: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Відео: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Аудіо: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Документи: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Презентації: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Таблиці: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Архіви: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Код: `type:code`, `type:source`, `type:dev`
- Виконувані файли: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Приклади:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Макроси типу: `audio:`, `video:`, `doc:`, `exe:`

Скорочення для поширених випадків `type:`:

| Макрос  | Еквівалент        | Приклад                |
| ------ | ----------------- | ---------------------- |
| `audio:` | `type:audio`     | `audio: piano`         |
| `video:` | `type:video`     | `video: tutorial`      |
| `doc:`   | `type:doc`       | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`       | `exe: "Cardinal"`     |

Макроси приймають необов’язковий аргумент:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Фільтр розміру: `size:`

`size:` підтримує:

- **Порівняння**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Діапазони**: `min..max`
- **Ключові слова**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Одиниці**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Приклади:
```text
size:>1GB                 # більше ніж 1 GB
size:1mb..10mb            # між 1 MB та 10 MB
size:tiny                 # 0–10 KB (орієнтовний діапазон за ключовим словом)
size:empty                # рівно 0 байт
```

### 4.7 Фільтри дати: `dm:`, `dc:`

- `dm:` / `datemodified:` — дата зміни.
- `dc:` / `datecreated:` — дата створення.

Вони приймають:

1. **Ключові слова** (відносні діапазони):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Абсолютні дати**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - Також підтримує поширені формати день‑спочатку / місяць‑спочатку, як `DD-MM-YYYY` і `MM/DD/YYYY`.

3. **Діапазони та порівняння**:
   - Діапазони: `dm:2024-01-01..2024-12-31`
   - Порівняння: `dm:>=2024-01-01`, `dc:<2023/01/01`

Приклади:
```text
dm:today                      # змінено сьогодні
dc:lastyear                   # створено минулого календарного року
dm:2024-01-01..2024-03-31     # змінено в Q1 2024
dm:>=2024/01/01               # змінено починаючи з 2024-01-01
```

### 4.8 Фільтр regex: `regex:`

`regex:` сприймає решту токена як регулярний вираз, що застосовується до компонента шляху (назви файлу чи папки).

Приклади:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

Перемикач регістру в UI впливає на збіг regex.

### 4.9 Фільтр вмісту: `content:`

`content:` сканує вміст файлів на **простий підрядок**:

- У `content:` немає regex — це збіг підрядка за байтами.
- Чутливість до регістру слідує перемикачу UI:
  - У режимі без урахування регістру і “голка”, і байти сканування приводяться до нижнього регістру.
  - У режимі з урахуванням регістру байти порівнюються як є.
- Дуже короткі “голки” дозволені, але `""` (порожній) відхиляється.

Приклади:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

Збіг вмісту виконується потоково по файлу; багатобайтові послідовності можуть перетинати межі буфера.

### 4.10 Фільтр тегів: `tag:` / `t:`

Фільтрує за тегами Finder (macOS). Cardinal підтягує теги за потреби з метаданих файлу (без кешу) і для великих наборів результатів використовує `mdfind`, щоб звузити кандидатів перед застосуванням зіставлення тегів.

- Приймає один або кілька тегів, розділених `;` (логічне OR): `tag:ProjectA;ProjectB`.
- Можна ланцюжити кілька фільтрів `tag:` (логічне AND) для збігів за кількома тегами: `tag:Project tag:Important`.
- Чутливість до регістру слідує перемикачу UI.
- Імена тегів зіставляються за підрядком: `tag:proj` збігається з `Project` і `project`.

Приклади:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Приклади

Декілька реалістичних комбінацій:

```text
#  Markdown-нотатки в Documents (без PDF)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDF у Reports з згадкою “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  Фото з відпустки
type:picture vacation
ext:png;jpg travel|vacation

#  Свіжі лог-файли всередині дерева проєкту
in:/Users/demo/Projects ext:log dm:pastweek

#  Shell-скрипти безпосередньо під папкою Scripts
parent:/Users/demo/Scripts *.sh

#  Все з “Application Support” у шляху
"Application Support"

#  Зіставити конкретну назву файлу через regex
regex:^README\\.md$ parent:/Users/demo

#  Виключити PSD будь-де під /Users
in:/Users demo!.psd
```

Використовуйте цю сторінку як авторитетний список операторів і фільтрів, які рушій реалізує сьогодні; додаткові можливості Everything (як-от дати доступу/запуску або фільтри за атрибутами) парсяться на рівні синтаксису, але наразі відхиляються під час оцінювання.
