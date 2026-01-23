<div align="center">
  <img src="../../cardinal/mac-icon_1024x1024.png" alt="Ícone do Cardinal" width="120" height="120">
  <h1>Cardinal</h1>
  <p>App de busca de arquivos para macOS mais rápido e preciso.</p>
  <p>
    <a href="#usar-o-cardinal">Usar o Cardinal</a> ·
    <a href="#compilar-o-cardinal">Compilar o Cardinal</a>
  </p>
  <img src="UI.gif" alt="Prévia da interface do Cardinal" width="720">
</div>

---

[English](../../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Usar o Cardinal

### Download

Instale com o Homebrew:

```bash
brew install --cask cardinal-search
```

Você também pode baixar os pacotes mais recentes em [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### Suporte a i18n

Precisa de outro idioma? Clique no botão ⚙️ na barra de status para alternar na hora.

### Noções básicas de busca

O Cardinal agora traz uma sintaxe compatível com o Everything por cima das correspondências clássicas de substring/prefixo:

- `report draft` – o espaço funciona como `AND`, então você só verá arquivos cujos nomes contenham ambos os termos.
- `*.pdf briefing` – limita aos resultados em PDF cujo nome inclui “briefing”.
- `*.zip size:>100MB` – procura ZIPs maiores que 100MB.
- `in:/Users demo !.psd` – restringe a raiz de busca para `/Users`, depois procura arquivos cujo nome contenha `demo`, mas exclui `.psd`.
- `tag:ProjectA;ProjectB` – corresponde às tags do Finder (macOS); `;` funciona como `OR`.
- `*.md content:"Bearer "` – filtra apenas os Markdown que contenham a string `Bearer `.
- `"Application Support"` – use aspas para frases exatas.
- `brary/Applicat` – use `/` como separador de caminho para buscar subcaminhos, encontrando diretórios como `Library/Application Support`.
- `/report` · `draft/` · `/report/` – adicione barras no início ou final do token para forçar correspondências de prefixo, sufixo ou nome exato quando precisar de controle de palavra inteira além da sintaxe do Everything.
- `~/**/.DS_Store` – o globstar (`**`) percorre todas as subpastas do seu diretório inicial para achar `.DS_Store` esquecidos.

Confira o catálogo completo de operadores—incluindo agrupamento booleano, escopo por pasta, filtros de extensão, uso de regex e mais exemplos—em [`search-syntax.pt-BR.md`](search-syntax.pt-BR.md).

### Atalhos de teclado e pré-visualizações

- `Cmd+Shift+Space` – alterna a janela do Cardinal globalmente pelo atalho rápido.
- `Cmd+,` – abre Preferências.
- `Esc` – oculta a janela do Cardinal.
- `ArrowUp`/`ArrowDown` – move a seleção.
- `Shift+ArrowUp`/`Shift+ArrowDown` – amplia a seleção.
- `Space` – Quick Look da linha selecionada sem sair do Cardinal.
- `Cmd+O` – abre o resultado selecionado.
- `Cmd+R` – revela o resultado destacado no Finder.
- `Cmd+C` – copia os arquivos selecionados para a área de transferência.
- `Cmd+Shift+C` – copia os caminhos selecionados para a área de transferência.
- `Cmd+F` – devolve o foco à barra de busca.
- `ArrowUp`/`ArrowDown` (na barra de busca) – percorre o histórico de busca.

Boas buscas!

---

## Compilar o Cardinal

### Requisitos

- macOS 12+
- Toolchain do Rust
- Node.js 18+ com npm
- Ferramentas de linha de comando do Xcode e pré-requisitos do Tauri (<https://tauri.app/start/prerequisites/>)

### Modo de desenvolvimento

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Build de produção

```bash
cd cardinal
npm run tauri build
```
