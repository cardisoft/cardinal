# Sintaxis de búsqueda de Cardinal

El lenguaje de consulta de Cardinal es intencionalmente cercano a la sintaxis de Everything, pero refleja lo que el motor actual realmente implementa. Esta página es la referencia autorizada de lo que el backend en Rust entiende hoy.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Modelo mental

- Cada consulta se analiza en un árbol de:
  - **Palabras / frases** (texto plano, cadenas entre comillas, comodines),
  - **Filtros** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Operadores booleanos** (`AND`, `OR`, `NOT` / `!`).
- La coincidencia se evalúa contra la **ruta completa** de cada archivo indexado, no solo el nombre base.
- La sensibilidad a mayúsculas se controla con el interruptor de la UI:
  - Cuando es **insensible a mayúsculas**, el motor convierte a minúsculas tanto la consulta como los candidatos para coincidencias de nombre/contenido.
  - Cuando es **sensible a mayúsculas**, el motor compara los bytes tal cual.

Ejemplos rápidos:
```text
report draft                  # archivos cuya ruta contiene tanto “report” como “draft”
ext:pdf briefing              # archivos PDF cuyo nombre contiene “briefing”
parent:/Users demo!.psd       # bajo /Users, excluir archivos .psd
regex:^Report.*2025$          # nombres que coinciden con una regex
ext:png;jpg travel|vacation   # PNG o JPG cuyos nombres contienen “travel” o “vacation”
```

---

## 2. Tokens, comodines y segmentos de ruta

### 2.1 Tokens y frases sin comillas

- Un token sin comillas es una **coincidencia por subcadena** en la ruta:
  - `demo` coincide con `/Users/demo/Projects/cardinal.md`.
- Las frases entre comillas dobles coinciden con la secuencia exacta, incluidos los espacios:
  - `"Application Support"` coincide con `/Library/Application Support/...`.
- El interruptor de sensibilidad a mayúsculas de la UI se aplica a ambos.

### 2.2 Comodines (`*`, `?`, `**`)

- `*` coincide con cero o más caracteres.
- `?` coincide con exactamente un carácter.
- `**` es un globstar que atraviesa **cualquier número de segmentos de carpeta** cuando aparece entre barras.
- Los comodines se interpretan **dentro de un solo token**:
  - `*.rs` — cualquier nombre que termina en `.rs`.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, etc.
  - `a*b` — nombres que empiezan con `a` y terminan con `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` en cualquier lugar bajo `src/`.
- Si necesitas un `*` o `?` literal, pon el token entre comillas: `"*.rs"`. Los globstar deben ser segmentos de barra independientes (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Segmentación estilo ruta con `/`

Cardinal entiende “segmentos con barra” dentro de un token y clasifica cada segmento como coincidencia de prefijo/sufijo/exacta/subcadena en los componentes de la ruta. Ejemplos:

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

Esto te permite expresar:
- “La carpeta debe terminar con X” (`foo/`),
- “La carpeta debe empezar con X” (`/foo`),
- “Nombre de carpeta exacto en medio de la ruta” (`gaea/lil/bee/`).

---

## 3. Lógica booleana y agrupación

Cardinal sigue la precedencia de Everything:

- `NOT` / `!` tiene la precedencia más alta,
- `OR` / `|` después,
- `AND` implícito / explícito (“espacio”) tiene la **precedencia más baja**.

### 3.1 Operadores

| Sintaxis        | Significado                                             |
| --------------- | ------------------------------------------------------- |
| `foo bar`       | `foo AND bar` — ambos tokens deben coincidir.          |
| `foo\|bar`       | `foo OR bar` — cualquiera puede coincidir.             |
| `foo OR bar`    | Forma escrita de `|`.                                  |
| `!temp`         | `NOT temp` — excluye coincidencias.                    |
| `NOT temp`      | Igual que `!temp`.                                     |
| `( ... )`       | Agrupación con paréntesis.                             |
| `< ... >`       | Agrupación con corchetes angulares (estilo Everything). |

Ejemplos de precedencia:
```text
foo bar|baz        # se analiza como foo AND (bar OR baz)
!(ext:zip report)  # excluye elementos donde coinciden ext:zip Y “report”
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Usa paréntesis o `<...>` cuando quieras sobrescribir la precedencia predeterminada.

---

## 4. Filtros

Esta sección solo enumera los filtros que el motor actual realmente evalúa.

> **Nota**: los argumentos del filtro deben ir inmediatamente después de los dos puntos (`ext:jpg`, `parent:/Users/demo`). Escribir `file: *.md` inserta un espacio en blanco, así que Cardinal lo trata como un filtro `file:` (sin argumento) seguido del token separado `*.md`.

### 4.1 Filtros de archivo / carpeta

| Filtro           | Significado                      | Ejemplo            |
| ---------------- | -------------------------------- | ------------------ |
| `file:`          | Solo archivos (no carpetas)      | `file: report`     |
| `folder:`        | Solo carpetas                    | `folder:Projects`  |

Se pueden combinar con otros términos:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Filtro de extensión: `ext:`

- `ext:` acepta una o más extensiones separadas por `;`:
  - `ext:jpg` — imágenes JPEG.
  - `ext:jpg;png;gif` — tipos de imagen web comunes.
- La coincidencia no distingue mayúsculas y no incluye el punto.

Ejemplos:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Alcance de carpeta: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Filtro             | Significado                                                       | Ejemplo                                        |
| ------------------ | ----------------------------------------------------------------- | ---------------------------------------------- |
| `parent:`          | Solo hijos directos de la carpeta indicada                        | `parent:/Users/demo/Documents ext:md`         |
| `infolder:`/`in:`  | Cualquier descendiente de la carpeta indicada (recursivo)         | `in:/Users/demo/Projects report draft`        |
| `nosubfolders:`    | La carpeta en sí más los archivos hijos directos (sin subcarpetas) | `nosubfolders:/Users/demo/Projects ext:log`   |

Estos filtros toman una ruta absoluta como argumento; un `~` inicial se expande al directorio de inicio del usuario.

### 4.4 Filtro de tipo: `type:`

`type:` agrupa extensiones de archivo en categorías semánticas. Las categorías admitidas (insensible a mayúsculas, con sinónimos) incluyen:

- Imágenes: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Video: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Audio: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Documentos: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Presentaciones: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Hojas de cálculo: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Archivos comprimidos: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Código: `type:code`, `type:source`, `type:dev`
- Ejecutables: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Ejemplos:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Macros de tipo: `audio:`, `video:`, `doc:`, `exe:`

Atajos para casos comunes de `type:`:

| Macro    | Equivalente a        | Ejemplo                |
| ------- | -------------------- | ---------------------- |
| `audio:` | `type:audio`        | `audio: piano`         |
| `video:` | `type:video`        | `video: tutorial`      |
| `doc:`   | `type:doc`          | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`          | `exe: "Cardinal"`     |

Las macros aceptan un argumento opcional:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Filtro de tamaño: `size:`

`size:` admite:

- **Comparaciones**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Rangos**: `min..max`
- **Palabras clave**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Unidades**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Ejemplos:
```text
size:>1GB                 # mayor que 1 GB
size:1mb..10mb            # entre 1 MB y 10 MB
size:tiny                 # 0–10 KB (rango aproximado por palabra clave)
size:empty                # exactamente 0 bytes
```

### 4.7 Filtros de fecha: `dm:`, `dc:`

- `dm:` / `datemodified:` — fecha de modificación.
- `dc:` / `datecreated:` — fecha de creación.

Aceptan:

1. **Palabras clave** (rangos relativos):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Fechas absolutas**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - También admite formatos comunes día‑primero / mes‑primero como `DD-MM-YYYY` y `MM/DD/YYYY`.

3. **Rangos y comparaciones**:
   - Rangos: `dm:2024-01-01..2024-12-31`
   - Comparaciones: `dm:>=2024-01-01`, `dc:<2023/01/01`

Ejemplos:
```text
dm:today                      # modificado hoy
dc:lastyear                   # creado el año calendario pasado
dm:2024-01-01..2024-03-31     # modificado en Q1 2024
dm:>=2024/01/01               # modificado desde 2024-01-01 en adelante
```

### 4.8 Filtro de regex: `regex:`

`regex:` trata el resto del token como una expresión regular aplicada a un componente de la ruta (nombre de archivo o carpeta).

Ejemplos:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

El interruptor de sensibilidad a mayúsculas de la UI afecta la coincidencia de regex.

### 4.9 Filtro de contenido: `content:`

`content:` escanea el contenido de los archivos buscando una **subcadena plana**:

- No hay regex dentro de `content:` — es una coincidencia de subcadena de bytes.
- La sensibilidad a mayúsculas sigue el interruptor de la UI:
  - En modo insensible a mayúsculas, el motor pasa a minúsculas tanto la aguja como los bytes escaneados.
  - En modo sensible a mayúsculas, los bytes se comparan tal cual.
- Se permiten agujas muy pequeñas, pero `""` (vacía) se rechaza.

Ejemplos:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

La coincidencia de contenido se hace en modo streaming sobre el archivo; las secuencias multibyte pueden atravesar los límites del buffer.

### 4.10 Filtro de etiquetas: `tag:` / `t:`

Filtra por etiquetas de Finder (macOS). Cardinal obtiene las etiquetas bajo demanda desde los metadatos del archivo (sin caché) y, para conjuntos grandes de resultados, usa `mdfind` para reducir candidatos antes de aplicar la coincidencia de etiquetas.

- Acepta una o más etiquetas separadas por `;` (OR lógico): `tag:ProjectA;ProjectB`.
- Encadena varios filtros `tag:` (AND lógico) para coincidencias con varias etiquetas: `tag:Project tag:Important`.
- La sensibilidad a mayúsculas sigue el interruptor de la UI.
- Coincide los nombres de etiqueta por subcadena: `tag:proj` coincide con `Project` y `project`.

Ejemplos:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Ejemplos

Algunas combinaciones realistas:

```text
#  Notas Markdown en Documents (sin PDFs)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDFs en Reports que mencionan “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  Fotos de vacaciones
type:picture vacation
ext:png;jpg travel|vacation

#  Archivos de log recientes dentro de un árbol de proyecto
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts de shell directamente bajo la carpeta Scripts
parent:/Users/demo/Scripts *.sh

#  Todo con “Application Support” en la ruta
"Application Support"

#  Coincidir un nombre de archivo específico vía regex
regex:^README\\.md$ parent:/Users/demo

#  Excluir PSD en cualquier lugar bajo /Users
in:/Users demo!.psd
```

Usa esta página como la lista autorizada de operadores y filtros que el motor implementa hoy; las funciones adicionales de Everything (como fechas de acceso/ejecución o filtros basados en atributos) se analizan a nivel de sintaxis pero actualmente se rechazan durante la evaluación.
