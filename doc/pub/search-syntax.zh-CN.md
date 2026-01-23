# Cardinal 搜索语法

Cardinal 的查询语言有意贴近 Everything 的语法，同时反映当前引擎实际实现的内容。本页是 Rust 后端目前支持能力的权威参考。

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. 心智模型

- 每个查询都会被解析成一棵树，由以下元素组成：
  - **词 / 短语**（普通文本、引号字符串、通配符），
  - **过滤器**（`ext:`, `type:`, `dm:`, `content:`, …），
  - **布尔运算符**（`AND`, `OR`, `NOT` / `!`）。
- 匹配针对每个已索引文件的 **完整路径**，而不仅是文件名。
- 大小写敏感由 UI 开关控制：
  - **不区分大小写**时，名称/内容匹配会把查询与候选都转为小写。
  - **区分大小写**时，直接按字节比较。

快速示例：
```text
report draft                  # 路径同时包含 “report” 和 “draft” 的文件
ext:pdf briefing              # 名称包含 “briefing” 的 PDF 文件
parent:/Users demo!.psd       # 在 /Users 下排除 .psd 文件
regex:^Report.*2025$          # 符合 regex 的名称
ext:png;jpg travel|vacation   # 名称包含 “travel” 或 “vacation” 的 PNG/JPG
```

---

## 2. 词元、通配符与路径片段

### 2.1 普通词元与短语

- 不带引号的词元是对路径的 **子串匹配**：
  - `demo` 匹配 `/Users/demo/Projects/cardinal.md`。
- 双引号短语匹配包含空格在内的精确序列：
  - `"Application Support"` 匹配 `/Library/Application Support/...`。
- UI 的大小写开关对两者都生效。

### 2.2 通配符（`*`, `?`, `**`）

- `*` 匹配零个或多个字符。
- `?` 匹配恰好一个字符。
- `**` 是 globstar，当出现在斜杠之间时可跨越 **任意数量的文件夹片段**。
- 通配符在 **单个词元内部** 解析：
  - `*.rs` — 任何以 `.rs` 结尾的名称。
  - `report-??.txt` — `report-01.txt`、`report-AB.txt` 等。
  - `a*b` — 以 `a` 开头、以 `b` 结尾的名称。
  - `src/**/Cargo.toml` — `src/` 下任意位置的 `Cargo.toml`。
- 若需要字面量 `*` 或 `?`，请对词元加引号：`"*.rs"`。Globstar 必须是独立的斜杠片段（`foo/**/bar`, `/Users/**`, `**/notes`）。

### 2.3 使用 `/` 的路径式分段

Cardinal 能理解词元中的“斜杠分段”，并将每个分段归类为路径组件的前缀/后缀/精确/子串匹配。示例：

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

这使你可以表达：
- “文件夹必须以 X 结尾” (`foo/`)，
- “文件夹必须以 X 开头” (`/foo`)，
- “路径中部的精确文件夹名” (`gaea/lil/bee/`)。

---

## 3. 布尔逻辑与分组

Cardinal 遵循 Everything 的优先级：

- `NOT` / `!` 结合最紧，
- `OR` / `|` 次之，
- 隐式 / 显式 `AND`（“空格”）优先级 **最低**。

### 3.1 运算符

| 语法            | 含义                                              |
| --------------- | ------------------------------------------------- |
| `foo bar`       | `foo AND bar` — 两个词元都必须匹配。              |
| `foo\|bar`       | `foo OR bar` — 任意一个匹配即可。                |
| `foo OR bar`    | `|` 的文字形式。                                  |
| `!temp`         | `NOT temp` — 排除匹配项。                         |
| `NOT temp`      | 等同于 `!temp`。                                  |
| `( ... )`       | 使用圆括号分组。                                  |
| `< ... >`       | 使用尖括号分组（Everything 风格）。               |

优先级示例：
```text
foo bar|baz        # 解析为 foo AND (bar OR baz)
!(ext:zip report)  # 排除 ext:zip 与 “report” 同时匹配的项
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

当你需要覆盖默认优先级时，请使用括号或 `<...>`。

---

## 4. 过滤器

本节只列出当前引擎确实会计算的过滤器。

> **注意**：过滤器参数必须紧跟冒号（`ext:jpg`, `parent:/Users/demo`）。写成 `file: *.md` 会插入空格，Cardinal 会将其视为 `file:` 过滤器（无参数），后面跟一个独立词元 `*.md`。

### 4.1 文件 / 文件夹过滤器

| 过滤器          | 含义                          | 示例              |
| --------------- | ----------------------------- | ----------------- |
| `file:`         | 仅文件（非文件夹）            | `file: report`    |
| `folder:`       | 仅文件夹                      | `folder:Projects` |

这些可以与其他条件组合：

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 扩展名过滤器：`ext:`

- `ext:` 接受一个或多个以 `;` 分隔的扩展名：
  - `ext:jpg` — JPEG 图片。
  - `ext:jpg;png;gif` — 常见的网页图片类型。
- 匹配不区分大小写，且不包含点号。

示例：
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 文件夹范围：`parent:`, `infolder:` / `in:`, `nosubfolders:`

| 过滤器             | 含义                                                       | 示例                                         |
| ------------------ | ---------------------------------------------------------- | -------------------------------------------- |
| `parent:`          | 仅指定文件夹的直接子项                                      | `parent:/Users/demo/Documents ext:md`       |
| `infolder:`/`in:`  | 指定文件夹下的任意后代（递归）                               | `in:/Users/demo/Projects report draft`      |
| `nosubfolders:`    | 文件夹自身 + 直接文件子项（不包含子文件夹）                 | `nosubfolders:/Users/demo/Projects ext:log` |

这些过滤器以绝对路径作为参数；前导 `~` 会展开为用户主目录。

### 4.4 类型过滤器：`type:`

`type:` 将扩展名归类为语义类别。支持的类别（不区分大小写，含同义词）包括：

- 图片：`type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- 视频：`type:video`, `type:videos`, `type:movie`, `type:movies`
- 音频：`type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- 文档：`type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- 演示文稿：`type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- 电子表格：`type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF：`type:pdf`
- 压缩包：`type:archive`, `type:archives`, `type:compressed`, `type:zip`
- 代码：`type:code`, `type:source`, `type:dev`
- 可执行文件：`type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

示例：
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 类型宏：`audio:`, `video:`, `doc:`, `exe:`

常用 `type:` 的快捷方式：

| 宏      | 等价于           | 示例                  |
| ------- | ---------------- | --------------------- |
| `audio:` | `type:audio`    | `audio: piano`        |
| `video:` | `type:video`    | `video: tutorial`     |
| `doc:`   | `type:doc`      | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`      | `exe: "Cardinal"`    |

宏可接受可选参数：
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 大小过滤器：`size:`

`size:` 支持：

- **比较**：`>`, `>=`, `<`, `<=`, `=`, `!=`
- **范围**：`min..max`
- **关键字**：`empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **单位**：bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

示例：
```text
size:>1GB                 # 大于 1 GB
size:1mb..10mb            # 介于 1 MB 和 10 MB 之间
size:tiny                 # 0–10 KB（关键字近似范围）
size:empty                # 恰好 0 字节
```

### 4.7 日期过滤器：`dm:`, `dc:`

- `dm:` / `datemodified:` — 修改日期。
- `dc:` / `datecreated:` — 创建日期。

支持：

1. **关键字**（相对范围）：
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **绝对日期**：
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - 同时支持常见的日‑月/ 月‑日格式，如 `DD-MM-YYYY` 和 `MM/DD/YYYY`。

3. **范围与比较**：
   - 范围：`dm:2024-01-01..2024-12-31`
   - 比较：`dm:>=2024-01-01`, `dc:<2023/01/01`

示例：
```text
dm:today                      # 今天修改
dc:lastyear                   # 上一日历年创建
dm:2024-01-01..2024-03-31     # 2024 年 Q1 修改
dm:>=2024/01/01               # 2024-01-01 及之后修改
```

### 4.8 Regex 过滤器：`regex:`

`regex:` 将词元剩余部分视为应用到路径组件（文件或文件夹名）的正则表达式。

示例：
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

UI 的大小写开关会影响 regex 匹配。

### 4.9 内容过滤器：`content:`

`content:` 扫描文件内容以匹配 **普通子串**：

- `content:` 中不支持 regex —— 是按字节的子串匹配。
- 大小写敏感遵循 UI 开关：
  - 不区分大小写时，搜索词和扫描字节都会转为小写。
  - 区分大小写时，按原字节比较。
- 很短的搜索词允许，但 `""`（空）会被拒绝。

示例：
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

内容匹配采用流式读取文件；多字节序列可能跨越缓冲区边界。

### 4.10 标签过滤器：`tag:` / `t:`

按 Finder 标签（macOS）过滤。Cardinal 按需从文件元数据获取标签（不缓存），并在结果集很大时使用 `mdfind` 缩小候选，再进行标签匹配。

- 接受一个或多个用 `;` 分隔的标签（逻辑 OR）：`tag:ProjectA;ProjectB`。
- 可串联多个 `tag:` 过滤器（逻辑 AND）进行多标签匹配：`tag:Project tag:Important`。
- 大小写敏感遵循 UI 开关。
- 标签名按子串匹配：`tag:proj` 可匹配 `Project` 和 `project`。

示例：
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. 示例

一些现实组合：

```text
#  Documents 中的 Markdown 笔记（无 PDF）
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  Reports 中提到 “briefing” 的 PDF
ext:pdf briefing parent:/Users/demo/Reports

#  旅行照片
type:picture vacation
ext:png;jpg travel|vacation

#  项目目录树内的近期日志文件
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts 文件夹下的 Shell 脚本
parent:/Users/demo/Scripts *.sh

#  路径中包含 “Application Support” 的所有项
"Application Support"

#  通过 regex 匹配特定文件名
regex:^README\\.md$ parent:/Users/demo

#  排除 /Users 下任意位置的 PSD
in:/Users demo!.psd
```

请将本页作为当前引擎已实现的运算符与过滤器的权威列表；Everything 的更多功能（如访问/运行时间或基于属性的过滤器）会在语法层面解析，但目前在评估阶段会被拒绝。
