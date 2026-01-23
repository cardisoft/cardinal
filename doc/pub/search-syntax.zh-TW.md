# Cardinal 搜尋語法

Cardinal 的查詢語言刻意貼近 Everything 的語法，同時反映目前引擎實際實作的內容。本頁是 Rust 後端目前支援能力的權威參考。

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. 心智模型

- 每個查詢都會被解析成一棵樹，由以下元素組成：
  - **詞 / 片語**（一般文字、引號字串、萬用字元），
  - **過濾器**（`ext:`, `type:`, `dm:`, `content:`, …），
  - **布林運算子**（`AND`, `OR`, `NOT` / `!`）。
- 比對會針對每個已索引檔案的 **完整路徑**，而不僅是檔名。
- 大小寫敏感由 UI 開關控制：
  - **不區分大小寫**時，名稱/內容比對會把查詢與候選都轉為小寫。
  - **區分大小寫**時，直接按位元組比較。

快速範例：
```text
report draft                  # 路徑同時包含 “report” 與 “draft” 的檔案
ext:pdf briefing              # 名稱包含 “briefing” 的 PDF 檔
parent:/Users demo!.psd       # 在 /Users 下排除 .psd 檔
regex:^Report.*2025$          # 符合 regex 的名稱
ext:png;jpg travel|vacation   # 名稱包含 “travel” 或 “vacation” 的 PNG/JPG
```

---

## 2. 詞元、萬用字元與路徑片段

### 2.1 一般詞元與片語

- 不加引號的詞元是對路徑的 **子字串比對**：
  - `demo` 會匹配 `/Users/demo/Projects/cardinal.md`。
- 雙引號片語會匹配包含空白在內的精確序列：
  - `"Application Support"` 會匹配 `/Library/Application Support/...`。
- UI 的大小寫開關對兩者都有效。

### 2.2 萬用字元（`*`, `?`, `**`）

- `*` 匹配零個或多個字元。
- `?` 匹配恰好一個字元。
- `**` 是 globstar，出現在斜線之間時可跨越 **任意數量的資料夾片段**。
- 萬用字元在 **單一詞元內** 解析：
  - `*.rs` — 任何以 `.rs` 結尾的名稱。
  - `report-??.txt` — `report-01.txt`、`report-AB.txt` 等。
  - `a*b` — 以 `a` 開頭、以 `b` 結尾的名稱。
  - `src/**/Cargo.toml` — `src/` 下任意位置的 `Cargo.toml`。
- 若需要字面量 `*` 或 `?`，請將詞元加上引號：`"*.rs"`。Globstar 必須是獨立的斜線片段（`foo/**/bar`, `/Users/**`, `**/notes`）。

### 2.3 使用 `/` 的路徑式分段

Cardinal 能理解詞元中的「斜線分段」，並將每個分段歸類為路徑元件的前綴/後綴/精確/子字串比對。範例：

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

這讓你可以表達：
- 「資料夾必須以 X 結尾」(`foo/`)，
- 「資料夾必須以 X 開頭」(`/foo`)，
- 「路徑中間的精確資料夾名稱」(`gaea/lil/bee/`)。

---

## 3. 布林邏輯與分組

Cardinal 遵循 Everything 的優先序：

- `NOT` / `!` 綁定最緊，
- `OR` / `|` 次之，
- 隱含 / 顯式 `AND`（「空白」）優先序 **最低**。

### 3.1 運算子

| 語法            | 含義                                              |
| --------------- | ------------------------------------------------- |
| `foo bar`       | `foo AND bar` — 兩個詞元都必須匹配。              |
| `foo\|bar`       | `foo OR bar` — 任一匹配即可。                    |
| `foo OR bar`    | `|` 的文字形式。                                  |
| `!temp`         | `NOT temp` — 排除匹配項。                         |
| `NOT temp`      | 等同於 `!temp`。                                  |
| `( ... )`       | 使用圓括號分組。                                  |
| `< ... >`       | 使用尖括號分組（Everything 風格）。               |

優先序範例：
```text
foo bar|baz        # 解析為 foo AND (bar OR baz)
!(ext:zip report)  # 排除 ext:zip 與 “report” 同時匹配的項目
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

當你需要覆寫預設優先序時，請使用括號或 `<...>`。

---

## 4. 過濾器

本節只列出目前引擎實際會計算的過濾器。

> **注意**：過濾器參數必須緊接在冒號之後（`ext:jpg`, `parent:/Users/demo`）。寫成 `file: *.md` 會插入空白，Cardinal 會將其視為 `file:` 過濾器（無參數），後面接一個獨立詞元 `*.md`。

### 4.1 檔案 / 資料夾過濾器

| 過濾器          | 含義                          | 範例              |
| --------------- | ----------------------------- | ----------------- |
| `file:`         | 僅檔案（非資料夾）            | `file: report`    |
| `folder:`       | 僅資料夾                      | `folder:Projects` |

這些可以與其他條件組合：

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 副檔名過濾器：`ext:`

- `ext:` 接受一個或多個以 `;` 分隔的副檔名：
  - `ext:jpg` — JPEG 圖片。
  - `ext:jpg;png;gif` — 常見的網頁圖片類型。
- 比對不區分大小寫，且不包含點號。

範例：
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 資料夾範圍：`parent:`, `infolder:` / `in:`, `nosubfolders:`

| 過濾器             | 含義                                                       | 範例                                         |
| ------------------ | ---------------------------------------------------------- | -------------------------------------------- |
| `parent:`          | 僅指定資料夾的直接子項                                      | `parent:/Users/demo/Documents ext:md`       |
| `infolder:`/`in:`  | 指定資料夾下的任意後代（遞迴）                               | `in:/Users/demo/Projects report draft`      |
| `nosubfolders:`    | 資料夾本身 + 直接檔案子項（不含子資料夾）                   | `nosubfolders:/Users/demo/Projects ext:log` |

這些過濾器以絕對路徑作為參數；前導 `~` 會展開為使用者家目錄。

### 4.4 類型過濾器：`type:`

`type:` 將副檔名歸類為語義類別。支援的類別（不區分大小寫，含同義詞）包括：

- 圖片：`type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- 影片：`type:video`, `type:videos`, `type:movie`, `type:movies`
- 音訊：`type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- 文件：`type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- 簡報：`type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- 試算表：`type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF：`type:pdf`
- 壓縮檔：`type:archive`, `type:archives`, `type:compressed`, `type:zip`
- 程式碼：`type:code`, `type:source`, `type:dev`
- 可執行檔：`type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

範例：
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 類型巨集：`audio:`, `video:`, `doc:`, `exe:`

常見 `type:` 的捷徑：

| 巨集   | 等價於           | 範例                  |
| ------ | ---------------- | --------------------- |
| `audio:` | `type:audio`    | `audio: piano`        |
| `video:` | `type:video`    | `video: tutorial`     |
| `doc:`   | `type:doc`      | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`      | `exe: "Cardinal"`    |

巨集可接受可選參數：
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 大小過濾器：`size:`

`size:` 支援：

- **比較**：`>`, `>=`, `<`, `<=`, `=`, `!=`
- **範圍**：`min..max`
- **關鍵字**：`empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **單位**：bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

範例：
```text
size:>1GB                 # 大於 1 GB
size:1mb..10mb            # 介於 1 MB 與 10 MB 之間
size:tiny                 # 0–10 KB（關鍵字近似範圍）
size:empty                # 恰好 0 位元組
```

### 4.7 日期過濾器：`dm:`, `dc:`

- `dm:` / `datemodified:` — 修改日期。
- `dc:` / `datecreated:` — 建立日期。

支援：

1. **關鍵字**（相對範圍）：
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **絕對日期**：
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - 也支援常見的日‑月 / 月‑日格式，如 `DD-MM-YYYY` 和 `MM/DD/YYYY`。

3. **範圍與比較**：
   - 範圍：`dm:2024-01-01..2024-12-31`
   - 比較：`dm:>=2024-01-01`, `dc:<2023/01/01`

範例：
```text
dm:today                      # 今天修改
dc:lastyear                   # 上一曆年建立
dm:2024-01-01..2024-03-31     # 2024 年 Q1 修改
dm:>=2024/01/01               # 2024-01-01 之後修改
```

### 4.8 Regex 過濾器：`regex:`

`regex:` 會將詞元剩餘部分視為套用到路徑元件（檔名或資料夾名）的正則表達式。

範例：
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

UI 的大小寫開關會影響 regex 比對。

### 4.9 內容過濾器：`content:`

`content:` 會掃描檔案內容以匹配 **一般子字串**：

- `content:` 中不支援 regex —— 是按位元組的子字串比對。
- 大小寫敏感遵循 UI 開關：
  - 不區分大小寫時，搜尋詞與掃描位元組會轉為小寫。
  - 區分大小寫時，按原位元組比較。
- 允許非常短的搜尋詞，但 `""`（空）會被拒絕。

範例：
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

內容比對以串流方式進行；多位元組序列可能跨越緩衝區邊界。

### 4.10 標籤過濾器：`tag:` / `t:`

依 Finder 標籤（macOS）過濾。Cardinal 會按需從檔案中繼資料取得標籤（不快取），並在結果集很大時先使用 `mdfind` 縮小候選，再進行標籤比對。

- 接受一個或多個以 `;` 分隔的標籤（邏輯 OR）：`tag:ProjectA;ProjectB`。
- 可串聯多個 `tag:` 過濾器（邏輯 AND）以達到多標籤匹配：`tag:Project tag:Important`。
- 大小寫敏感遵循 UI 開關。
- 標籤名稱以子字串匹配：`tag:proj` 會匹配 `Project` 與 `project`。

範例：
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. 範例

一些實際的組合：

```text
#  Documents 中的 Markdown 筆記（無 PDF）
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  Reports 中提到 “briefing” 的 PDF
ext:pdf briefing parent:/Users/demo/Reports

#  旅行照片
type:picture vacation
ext:png;jpg travel|vacation

#  專案樹內近期的日誌檔
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts 資料夾下直接的 Shell 腳本
parent:/Users/demo/Scripts *.sh

#  路徑中包含 “Application Support” 的所有項目
"Application Support"

#  透過 regex 匹配特定檔名
regex:^README\\.md$ parent:/Users/demo

#  排除 /Users 下任意位置的 PSD
in:/Users demo!.psd
```

請將本頁作為目前引擎已實作的運算子與過濾器的權威清單；Everything 的更多功能（如存取/執行日期或基於屬性的過濾器）會在語法層解析，但目前在評估時會被拒絕。
