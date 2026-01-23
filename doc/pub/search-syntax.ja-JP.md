# Cardinal 検索構文

Cardinal のクエリ言語は Everything の構文に意図的に近づけていますが、現在のエンジンが実際に実装している内容を反映しています。このページは Rust バックエンドが現時点で理解する内容の公式リファレンスです。

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. メンタルモデル

- すべてのクエリは次の要素からなるツリーに解析されます:
  - **単語 / フレーズ**（プレーンテキスト、引用符付き文字列、ワイルドカード）
  - **フィルタ**（`ext:`, `type:`, `dm:`, `content:`, …）
  - **ブール演算子**（`AND`, `OR`, `NOT` / `!`）
- マッチングは各インデックス済みファイルの **完全なパス** に対して行われ、ベース名だけではありません。
- 大文字/小文字の区別は UI のトグルで制御されます:
  - **大文字小文字を区別しない**場合、エンジンは名前/内容のマッチング用にクエリと候補を小文字化します。
  - **大文字小文字を区別する**場合、エンジンはバイトをそのまま比較します。

簡単な例:
```text
report draft                  # パスに “report” と “draft” の両方を含むファイル
ext:pdf briefing              # 名前に “briefing” を含む PDF
parent:/Users demo!.psd       # /Users 配下で .psd を除外
regex:^Report.*2025$          # regex に一致する名前
ext:png;jpg travel|vacation   # 名前に “travel” または “vacation” を含む PNG/JPG
```

---

## 2. トークン、ワイルドカード、パスセグメント

### 2.1 通常のトークンとフレーズ

- 引用符のないトークンはパスに対する **部分一致** です:
  - `demo` は `/Users/demo/Projects/cardinal.md` に一致します。
- 二重引用符のフレーズは空白を含む正確な並びに一致します:
  - `"Application Support"` は `/Library/Application Support/...` に一致します。
- UI の大文字/小文字トグルは両方に適用されます。

### 2.2 ワイルドカード（`*`, `?`, `**`）

- `*` は 0 文字以上に一致します。
- `?` はちょうど 1 文字に一致します。
- `**` はスラッシュ間にあると **任意の数のフォルダセグメント** をまたぐ globstar です。
- ワイルドカードは **単一トークン内で** 解釈されます:
  - `*.rs` — `.rs` で終わる任意の名前。
  - `report-??.txt` — `report-01.txt`, `report-AB.txt` など。
  - `a*b` — `a` で始まり `b` で終わる名前。
  - `src/**/Cargo.toml` — `src/` 配下のどこかにある `Cargo.toml`。
- リテラルの `*` または `?` が必要な場合はトークンを引用符で囲みます: `"*.rs"`。Globstar は独立したスラッシュセグメントである必要があります（`foo/**/bar`, `/Users/**`, `**/notes`）。

### 2.3 `/` によるパス風セグメント化

Cardinal はトークン内の「スラッシュセグメント」を理解し、各セグメントをパス要素に対する前方/後方/完全/部分一致として分類します。例:

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

これにより次のような表現ができます:
- 「フォルダは X で終わる必要がある」(`foo/`),
- 「フォルダは X で始まる必要がある」(`/foo`),
- 「パスの途中にある正確なフォルダ名」(`gaea/lil/bee/`).

---

## 3. ブール論理とグルーピング

Cardinal は Everything の優先順位に従います:

- `NOT` / `!` が最も強く結合される。
- `OR` / `|` が次。
- 暗黙 / 明示の `AND`（「空白」）は **最も低い** 優先順位。

### 3.1 演算子

| 構文           | 意味                                              |
| -------------- | ------------------------------------------------- |
| `foo bar`      | `foo AND bar` — 両方のトークンが一致する必要があります。 |
| `foo\|bar`      | `foo OR bar` — どちらか一方が一致すればよい。     |
| `foo OR bar`   | `|` の単語形式。                                   |
| `!temp`        | `NOT temp` — 一致を除外。                          |
| `NOT temp`     | `!temp` と同じ。                                   |
| `( ... )`      | 括弧によるグルーピング。                            |
| `< ... >`      | 山括弧によるグルーピング（Everything 風）。       |

優先順位の例:
```text
foo bar|baz        # foo AND (bar OR baz) として解析
!(ext:zip report)  # ext:zip と “report” の両方が一致する項目を除外
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

デフォルトの優先順位を変えたい場合は括弧または `<...>` を使ってください。

---

## 4. フィルタ

このセクションは、現在のエンジンが実際に評価するフィルタのみを列挙しています。

> **注意**: フィルタ引数はコロンの直後に続ける必要があります（`ext:jpg`, `parent:/Users/demo`）。`file: *.md` のように空白を挟むと、Cardinal は `file:` フィルタ（引数なし）と別トークン `*.md` として扱います。

### 4.1 ファイル / フォルダフィルタ

| フィルタ        | 意味                             | 例                 |
| --------------- | -------------------------------- | ------------------ |
| `file:`         | ファイルのみ（フォルダを除く）   | `file: report`     |
| `folder:`       | フォルダのみ                     | `folder:Projects`  |

これらは他の条件と組み合わせられます:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 拡張子フィルタ: `ext:`

- `ext:` は `;` で区切られた 1 つ以上の拡張子を受け取ります:
  - `ext:jpg` — JPEG 画像。
  - `ext:jpg;png;gif` — 一般的な Web 画像タイプ。
- マッチングは大文字/小文字を区別せず、ドットは含みません。

例:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 フォルダ範囲: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| フィルタ           | 意味                                                         | 例                                         |
| ------------------ | ------------------------------------------------------------ | ----------------------------------------- |
| `parent:`          | 指定フォルダの直下の子だけ                                   | `parent:/Users/demo/Documents ext:md`     |
| `infolder:`/`in:`  | 指定フォルダのすべての子孫（再帰）                             | `in:/Users/demo/Projects report draft`    |
| `nosubfolders:`    | フォルダ自身＋直下のファイル（サブフォルダなし）              | `nosubfolders:/Users/demo/Projects ext:log` |

これらのフィルタは引数に絶対パスを取ります。先頭の `~` はユーザーホームに展開されます。

### 4.4 タイプフィルタ: `type:`

`type:` は拡張子を意味的なカテゴリにまとめます。サポートされるカテゴリ（大文字/小文字無視、同義語含む）は次のとおりです:

- 画像: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- 動画: `type:video`, `type:videos`, `type:movie`, `type:movies`
- 音声: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- ドキュメント: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- プレゼンテーション: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- スプレッドシート: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- アーカイブ: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- コード: `type:code`, `type:source`, `type:dev`
- 実行ファイル: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

例:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 タイプマクロ: `audio:`, `video:`, `doc:`, `exe:`

一般的な `type:` のショートカット:

| マクロ  | 相当           | 例                   |
| ------ | -------------- | -------------------- |
| `audio:` | `type:audio`  | `audio: piano`       |
| `video:` | `type:video`  | `video: tutorial`    |
| `doc:`   | `type:doc`    | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`    | `exe: "Cardinal"`   |

マクロは任意の引数を受け取れます:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 サイズフィルタ: `size:`

`size:` は次をサポートします:

- **比較**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **範囲**: `min..max`
- **キーワード**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **単位**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

例:
```text
size:>1GB                 # 1 GB より大きい
size:1mb..10mb            # 1 MB から 10 MB の間
size:tiny                 # 0–10 KB（おおよそのキーワード範囲）
size:empty                # ちょうど 0 バイト
```

### 4.7 日付フィルタ: `dm:`, `dc:`

- `dm:` / `datemodified:` — 更新日。
- `dc:` / `datecreated:` — 作成日。

次を受け付けます:

1. **キーワード**（相対範囲）:
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **絶対日付**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - `DD-MM-YYYY` や `MM/DD/YYYY` のような一般的な日付形式もサポートします。

3. **範囲と比較**:
   - 範囲: `dm:2024-01-01..2024-12-31`
   - 比較: `dm:>=2024-01-01`, `dc:<2023/01/01`

例:
```text
dm:today                      # 今日更新
dc:lastyear                   # 前の暦年に作成
dm:2024-01-01..2024-03-31     # 2024 年 Q1 に更新
dm:>=2024/01/01               # 2024-01-01 以降に更新
```

### 4.8 Regex フィルタ: `regex:`

`regex:` はトークンの残りを、パス構成要素（ファイル名またはフォルダ名）に適用される正規表現として扱います。

例:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

UI の大文字/小文字トグルは regex のマッチングに影響します。

### 4.9 内容フィルタ: `content:`

`content:` はファイル内容を **単純な部分文字列** で検索します:

- `content:` 内に regex はありません — バイトの部分一致です。
- 大文字/小文字の扱いは UI トグルに従います:
  - 大文字/小文字を区別しない場合、検索語と走査バイトを小文字化します。
  - 大文字/小文字を区別する場合、バイトはそのまま比較されます。
- 非常に短い検索語は許可されますが、`""`（空）は拒否されます。

例:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

内容の一致はファイルをストリーミングしながら行われ、マルチバイトのシーケンスはバッファ境界をまたぐことがあります。

### 4.10 タグフィルタ: `tag:` / `t:`

Finder タグ（macOS）でフィルタします。Cardinal はファイルのメタデータからタグをオンデマンドで取得し（キャッシュなし）、結果が大きい場合は `mdfind` で候補を絞ってからタグ一致を適用します。

- `;` 区切りで 1 つ以上のタグを指定できます（論理 OR）: `tag:ProjectA;ProjectB`。
- 複数の `tag:` フィルタを連結すると論理 AND になります: `tag:Project tag:Important`。
- 大文字/小文字の扱いは UI トグルに従います。
- タグ名は部分一致で照合されます: `tag:proj` は `Project` と `project` に一致します。

例:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. 例

現実的な組み合わせ例:

```text
#  Documents 内の Markdown ノート（PDF なし）
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  Reports 内で “briefing” を含む PDF
ext:pdf briefing parent:/Users/demo/Reports

#  休暇の写真
type:picture vacation
ext:png;jpg travel|vacation

#  プロジェクトツリー内の最近のログファイル
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts フォルダ直下のシェルスクリプト
parent:/Users/demo/Scripts *.sh

#  パスに “Application Support” を含むすべて
"Application Support"

#  regex で特定のファイル名を一致
regex:^README\\.md$ parent:/Users/demo

#  /Users 配下の PSD を除外
in:/Users demo!.psd
```

このページを、現在エンジンが実装している演算子とフィルタの権威ある一覧として使用してください。アクセス/実行日時や属性ベースのフィルタなど Everything の追加機能は構文レベルでは解析されますが、現時点では評価時に拒否されます。
