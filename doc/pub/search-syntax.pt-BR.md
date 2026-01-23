# Sintaxe de busca do Cardinal

A linguagem de consulta do Cardinal é intencionalmente próxima da sintaxe do Everything, mas reflete o que o mecanismo atual realmente implementa. Esta página é a referência oficial do que o backend em Rust entende hoje.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Modelo mental

- Cada consulta é analisada em uma árvore de:
  - **Palavras / frases** (texto simples, strings entre aspas, curingas),
  - **Filtros** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Operadores booleanos** (`AND`, `OR`, `NOT` / `!`).
- A correspondência ocorre sobre o **caminho completo** de cada arquivo indexado, não apenas o nome base.
- A sensibilidade a maiúsculas/minúsculas é controlada pelo toggle da UI:
  - Quando **não diferencia maiúsculas/minúsculas**, o mecanismo coloca em minúsculas tanto a consulta quanto os candidatos para correspondência de nome/conteúdo.
  - Quando **diferencia maiúsculas/minúsculas**, o mecanismo compara os bytes como estão.

Exemplos rápidos:
```text
report draft                  # arquivos cujo caminho contém “report” e “draft”
ext:pdf briefing              # PDFs cujo nome contém “briefing”
parent:/Users demo!.psd       # em /Users, excluir arquivos .psd
regex:^Report.*2025$          # nomes que correspondem a uma regex
ext:png;jpg travel|vacation   # PNG ou JPG cujos nomes contêm “travel” ou “vacation”
```

---

## 2. Tokens, curingas e segmentos de caminho

### 2.1 Tokens simples e frases

- Um token sem aspas é uma **correspondência por substring** no caminho:
  - `demo` corresponde a `/Users/demo/Projects/cardinal.md`.
- Frases entre aspas duplas correspondem à sequência exata, incluindo espaços:
  - `"Application Support"` corresponde a `/Library/Application Support/...`.
- O toggle de sensibilidade a maiúsculas/minúsculas da UI se aplica a ambos.

### 2.2 Curingas (`*`, `?`, `**`)

- `*` corresponde a zero ou mais caracteres.
- `?` corresponde a exatamente um caractere.
- `**` é um globstar que atravessa **qualquer número de segmentos de pasta** quando aparece entre barras.
- Os curingas são interpretados **dentro de um único token**:
  - `*.rs` — qualquer nome que termina em `.rs`.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, etc.
  - `a*b` — nomes que começam com `a` e terminam com `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` em qualquer lugar abaixo de `src/`.
- Se precisar de `*` ou `?` literal, coloque o token entre aspas: `"*.rs"`. Globstars devem ser segmentos de barra independentes (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Segmentação em estilo de caminho com `/`

Cardinal entende “segmentos com barra” dentro de um token e classifica cada segmento como correspondência de prefixo/sufixo/exata/substring nos componentes do caminho. Exemplos:

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

Isso permite expressar:
- “A pasta deve terminar com X” (`foo/`),
- “A pasta deve começar com X” (`/foo`),
- “Nome exato de pasta no meio do caminho” (`gaea/lil/bee/`).

---

## 3. Lógica booleana e agrupamento

Cardinal segue a precedência do Everything:

- `NOT` / `!` tem a precedência mais alta,
- `OR` / `|` em seguida,
- `AND` implícito / explícito (“espaço”) tem a **menor** precedência.

### 3.1 Operadores

| Sintaxe        | Significado                                         |
| -------------- | --------------------------------------------------- |
| `foo bar`      | `foo AND bar` — ambos os tokens devem corresponder. |
| `foo\|bar`      | `foo OR bar` — qualquer um pode corresponder.       |
| `foo OR bar`   | Forma escrita de `|`.                               |
| `!temp`        | `NOT temp` — exclui correspondências.               |
| `NOT temp`     | Igual a `!temp`.                                    |
| `( ... )`      | Agrupamento com parênteses.                         |
| `< ... >`      | Agrupamento com colchetes angulares (estilo Everything). |

Exemplos de precedência:
```text
foo bar|baz        # analisado como foo AND (bar OR baz)
!(ext:zip report)  # exclui itens onde ext:zip E “report” correspondem
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Use parênteses ou `<...>` sempre que quiser substituir a precedência padrão.

---

## 4. Filtros

Esta seção lista apenas os filtros que o mecanismo atual realmente avalia.

> **Nota**: os argumentos de filtro devem vir imediatamente após os dois-pontos (`ext:jpg`, `parent:/Users/demo`). Escrever `file: *.md` insere um espaço em branco, então o Cardinal trata isso como um filtro `file:` (sem argumento) seguido do token separado `*.md`.

### 4.1 Filtros de arquivo / pasta

| Filtro          | Significado                       | Exemplo            |
| --------------- | --------------------------------- | ------------------ |
| `file:`         | Somente arquivos (não pastas)     | `file: report`     |
| `folder:`       | Somente pastas                    | `folder:Projects`  |

Eles podem ser combinados com outros termos:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Filtro de extensão: `ext:`

- `ext:` aceita uma ou mais extensões separadas por `;`:
  - `ext:jpg` — imagens JPEG.
  - `ext:jpg;png;gif` — tipos comuns de imagem para web.
- A correspondência não diferencia maiúsculas/minúsculas e não inclui o ponto.

Exemplos:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Escopo de pasta: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Filtro            | Significado                                                     | Exemplo                                        |
| ----------------- | --------------------------------------------------------------- | ---------------------------------------------- |
| `parent:`         | Apenas filhos diretos da pasta indicada                         | `parent:/Users/demo/Documents ext:md`         |
| `infolder:`/`in:` | Qualquer descendente da pasta indicada (recursivo)              | `in:/Users/demo/Projects report draft`        |
| `nosubfolders:`   | A pasta em si mais os arquivos filhos diretos (sem subpastas)    | `nosubfolders:/Users/demo/Projects ext:log`   |

Esses filtros recebem um caminho absoluto como argumento; um `~` inicial é expandido para a pasta home do usuário.

### 4.4 Filtro de tipo: `type:`

`type:` agrupa extensões de arquivo em categorias semânticas. As categorias suportadas (sem diferenciação de maiúsculas/minúsculas, com sinônimos) incluem:

- Imagens: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Vídeo: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Áudio: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Documentos: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Apresentações: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Planilhas: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Arquivos compactados: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Código: `type:code`, `type:source`, `type:dev`
- Executáveis: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Exemplos:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Macros de tipo: `audio:`, `video:`, `doc:`, `exe:`

Atalhos para casos comuns de `type:`:

| Macro    | Equivalente a        | Exemplo                |
| ------- | -------------------- | ---------------------- |
| `audio:` | `type:audio`        | `audio: piano`         |
| `video:` | `type:video`        | `video: tutorial`      |
| `doc:`   | `type:doc`          | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`          | `exe: "Cardinal"`     |

As macros aceitam um argumento opcional:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Filtro de tamanho: `size:`

`size:` suporta:

- **Comparações**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Intervalos**: `min..max`
- **Palavras-chave**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Unidades**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Exemplos:
```text
size:>1GB                 # maior que 1 GB
size:1mb..10mb            # entre 1 MB e 10 MB
size:tiny                 # 0–10 KB (intervalo aproximado por palavra-chave)
size:empty                # exatamente 0 bytes
```

### 4.7 Filtros de data: `dm:`, `dc:`

- `dm:` / `datemodified:` — data de modificação.
- `dc:` / `datecreated:` — data de criação.

Eles aceitam:

1. **Palavras-chave** (intervalos relativos):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Datas absolutas**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - Também suporta formatos comuns dia‑primeiro / mês‑primeiro como `DD-MM-YYYY` e `MM/DD/YYYY`.

3. **Intervalos e comparações**:
   - Intervalos: `dm:2024-01-01..2024-12-31`
   - Comparações: `dm:>=2024-01-01`, `dc:<2023/01/01`

Exemplos:
```text
dm:today                      # modificado hoje
dc:lastyear                   # criado no ano calendário passado
dm:2024-01-01..2024-03-31     # modificado no 1º trimestre de 2024
dm:>=2024/01/01               # modificado a partir de 2024-01-01
```

### 4.8 Filtro de regex: `regex:`

`regex:` trata o restante do token como uma expressão regular aplicada a um componente do caminho (nome de arquivo ou pasta).

Exemplos:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

O toggle de sensibilidade a maiúsculas/minúsculas da UI afeta a correspondência de regex.

### 4.9 Filtro de conteúdo: `content:`

`content:` varre o conteúdo do arquivo em busca de uma **substring simples**:

- Não há regex dentro de `content:` — é uma correspondência de substring por bytes.
- A sensibilidade a maiúsculas/minúsculas segue o toggle da UI:
  - No modo sem diferenciação, o mecanismo coloca em minúsculas tanto a agulha quanto os bytes analisados.
  - No modo com diferenciação, os bytes são comparados como estão.
- Agulhas muito pequenas são permitidas, mas `""` (vazia) é rejeitada.

Exemplos:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

A correspondência de conteúdo é feita em streaming pelo arquivo; sequências multibyte podem atravessar limites de buffer.

### 4.10 Filtro de tags: `tag:` / `t:`

Filtra por tags do Finder (macOS). O Cardinal busca tags sob demanda a partir dos metadados do arquivo (sem cache) e, para conjuntos grandes de resultados, usa `mdfind` para reduzir candidatos antes de aplicar a correspondência de tags.

- Aceita uma ou mais tags separadas por `;` (OR lógico): `tag:ProjectA;ProjectB`.
- Encadeie vários filtros `tag:` (AND lógico) para combinar várias tags: `tag:Project tag:Important`.
- A sensibilidade a maiúsculas/minúsculas segue o toggle da UI.
- Corresponde a nomes de tags por substring: `tag:proj` corresponde a `Project` e `project`.

Exemplos:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Exemplos

Algumas combinações realistas:

```text
#  Notas Markdown em Documents (sem PDFs)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDFs em Reports mencionando “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  Fotos de férias
type:picture vacation
ext:png;jpg travel|vacation

#  Arquivos de log recentes dentro de uma árvore de projeto
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts de shell diretamente na pasta Scripts
parent:/Users/demo/Scripts *.sh

#  Tudo com “Application Support” no caminho
"Application Support"

#  Corresponder a um nome de arquivo específico via regex
regex:^README\\.md$ parent:/Users/demo

#  Excluir PSDs em qualquer lugar sob /Users
in:/Users demo!.psd
```

Use esta página como a lista oficial de operadores e filtros que o mecanismo implementa hoje; recursos adicionais do Everything (como datas de acesso/execução ou filtros baseados em atributos) são analisados no nível de sintaxe, mas atualmente são rejeitados durante a avaliação.
