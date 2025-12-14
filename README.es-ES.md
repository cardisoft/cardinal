<div align="center">
  <img src="cardinal/mac-icon_1024x1024.png" alt="Ícono de Cardinal" width="120" height="120">
  <h1>Cardinal</h1>
  <p>La herramienta de búsqueda de archivos más rápida para macOS.</p>
  <p>
    <a href="#usar-cardinal">Usar Cardinal</a> ·
    <a href="#compilar-cardinal">Compilar Cardinal</a>
  </p>
  <img src="doc/UI.gif" alt="Vista previa de la interfaz de Cardinal" width="720">
</div>

---

[English](README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md)

## Usar Cardinal

### Descarga

Instala con Homebrew:

```bash
brew install --cask cardinal-search
```

También puedes descargar los últimos paquetes desde [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### Soporte de i18n

¿Necesitas otro idioma? Haz clic en el botón ⚙️ de la barra de estado para cambiarlo al instante.

### Conceptos básicos de búsqueda

Cardinal ahora incorpora una capa de sintaxis compatible con Everything sobre las coincidencias clásicas por subcadena/prefijo:

- `report draft` – el espacio actúa como `AND`, solo verás archivos cuyos nombres contengan ambos términos.
- `*.pdf briefing` – limita los resultados a PDF cuyo nombre incluya “briefing”.
- `*.zip size:>100MB` – busca archivos ZIP de más de 100MB.
- `infolder:/Users demo !.psd` – restringe la raíz de búsqueda a `/Users`, luego busca archivos cuyo nombre contenga `demo` pero excluya `.psd`.
- `tag:ProjectA;ProjectB` – filtra por etiquetas de Finder (macOS); `;` actúa como `OR`.
- `*.md content:"Bearer "` – muestra solo los Markdown que contengan la cadena `Bearer `.
- `"Application Support"` – usa comillas para coincidir frases exactas.
- `brary/Applicat` – usa `/` como separador de ruta para buscar subrutas, coincidiendo con directorios como `Library/Application Support`.
- `/report` · `draft/` · `/report/` – envuelve tokens con barras iniciales o finales para forzar coincidencias de prefijo, sufijo o nombre exacto cuando necesitas control de palabra completa más allá de la sintaxis de Everything.
- `~/**/.DS_Store` – el globstar (`**`) recorre todas las subcarpetas de tu carpeta de inicio para encontrar archivos `.DS_Store` sueltos.

Consulta el catálogo completo de operadores (agrupación booleana, alcance por carpeta, filtros por extensión, uso de regex y más ejemplos) en [`doc/search-syntax.md`](doc/search-syntax.md).

### Atajos de teclado y previsualizaciones

- `Space` – Quick Look de la fila seleccionada sin salir de Cardinal.
- `Cmd+R` – revela el resultado resaltado en Finder.
- `Cmd+F` – devuelve el foco a la barra de búsqueda.
- `Cmd+C` – copia la ruta del archivo seleccionado al portapapeles.
- `Cmd+Shift+Space` – activa o cierra la ventana de Cardinal globalmente mediante el atajo rápido.

¡Feliz búsqueda!

---

## Compilar Cardinal

### Requisitos

- macOS 12+
- Toolchain de Rust
- Node.js 18+ con npm
- Xcode command-line tools y requisitos previos de Tauri (<https://tauri.app/start/prerequisites/>)

### Modo de desarrollo

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Compilación de producción

```bash
cd cardinal
npm run tauri build
```
