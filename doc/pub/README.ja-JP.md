<div align="center">
  <img src="../../cardinal/mac-icon_1024x1024.png" alt="Cardinal アイコン" width="120" height="120">
  <h1>Cardinal</h1>
  <p>macOS 向け最速・最高精度のファイル検索アプリ。</p>
  <p>
    <a href="#cardinalの使い方">Cardinalの使い方</a> ·
    <a href="#cardinalのビルド">Cardinalのビルド</a>
  </p>
  <img src="UI.gif" alt="Cardinal UI プレビュー" width="720">
</div>

---

[English](../../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Cardinalの使い方

### ダウンロード

Homebrew でインストール:

```bash
brew install --cask cardinal-search
```

[GitHub Releases](https://github.com/cardisoft/cardinal/releases/) から最新のパッケージ版を入手することもできます。

### 国際化サポート

他の言語が必要ですか？ ステータスバーの ⚙️ ボタンをクリックするとすぐに切り替えられます。

### 基本の検索

Cardinal は従来の部分一致/プレフィックス一致に加えて、Everything 互換の構文レイヤーに対応しました:

- `report draft` – スペースは `AND` として動作し、両方のトークンを含むファイルだけが表示されます。
- `*.pdf briefing` – 名前に “briefing” を含む PDF だけに絞り込みます。
- `*.zip size:>100MB` – 100MB を超える ZIP ファイルを検索します。
- `in:/Users demo !.psd` – 検索ルートを `/Users` に制限し、名前に `demo` を含み `.psd` を除外します。
- `tag:ProjectA;ProjectB` – Finder タグ(macOS) でフィルターします。`;` は `OR` として機能します。
- `*.md content:"Bearer "` – 文字列 `Bearer ` を含む Markdown だけを表示します。
- `"Application Support"` – 正確なフレーズは引用符で囲みます。
- `brary/Applicat` – `/` をパス区切りとして使い、`Library/Application Support` のようなサブパスをマッチさせます。
- `/report` · `draft/` · `/report/` – トークンの前後に `/` を付けて、Everything 構文を超えた単語単位の制御が必要なときにプレフィックス/サフィックス/完全一致を強制します。
- `~/**/.DS_Store` – グロブスター(`**`)がホームディレクトリ以下のすべてのサブフォルダーをたどり、散らばった `.DS_Store` を見つけます。

対応している演算子一覧（ブールグループ化、フォルダーのスコープ指定、拡張子フィルター、正規表現の例など）は [`search-syntax.ja-JP.md`](search-syntax.ja-JP.md) を参照してください。

### キーボードショートカットとプレビュー

- `Cmd+Shift+Space` – グローバルショートカットで Cardinal ウィンドウを切り替えます。
- `Cmd+,` – 環境設定を開きます。
- `Esc` – Cardinal ウィンドウを隠します。
- `ArrowUp`/`ArrowDown` – 選択を移動します。
- `Shift+ArrowUp`/`Shift+ArrowDown` – 選択範囲を拡張します。
- `Space` – Cardinal を離れずに選択行を Quick Look します。
- `Cmd+O` – 選択した結果を開きます。
- `Cmd+R` – ハイライトされた結果を Finder で表示します。
- `Cmd+C` – 選択したファイルをクリップボードにコピーします。
- `Cmd+Shift+C` – 選択したパスをクリップボードにコピーします。
- `Cmd+F` – 検索バーにフォーカスを戻します。
- `ArrowUp`/`ArrowDown`（検索バー内）– 検索履歴を移動します。

よい検索を！

---

## Cardinalのビルド

### 必要条件

- macOS 12+
- Rust ツールチェーン
- npm 付きの Node.js 18+
- Xcode コマンドラインツールと Tauri の前提条件 (<https://tauri.app/start/prerequisites/>)

### 開発モード

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### 本番ビルド

```bash
cd cardinal
npm run tauri build
```
