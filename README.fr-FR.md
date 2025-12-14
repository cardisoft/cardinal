<div align="center">
  <img src="cardinal/mac-icon_1024x1024.png" alt="Icône de Cardinal" width="120" height="120">
  <h1>Cardinal</h1>
  <p>L’outil de recherche de fichiers le plus rapide pour macOS.</p>
  <p>
    <a href="#utiliser-cardinal">Utiliser Cardinal</a> ·
    <a href="#compiler-cardinal">Compiler Cardinal</a>
  </p>
  <img src="doc/UI.gif" alt="Aperçu de l’interface Cardinal" width="720">
</div>

---

[English](README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md)

## Utiliser Cardinal

### Téléchargement

Installez via Homebrew :

```bash
brew install --cask cardinal-search
```

Vous pouvez aussi récupérer les derniers paquets sur [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### Prise en charge i18n

Besoin d’une autre langue ? Cliquez sur le bouton ⚙️ dans la barre d’état pour changer instantanément.

### Principes de base de la recherche

Cardinal ajoute désormais une couche de syntaxe compatible Everything en plus des correspondances classiques par sous-chaîne/préfixe :

- `report draft` – l’espace agit comme `AND`, vous ne voyez que les fichiers dont le nom contient les deux termes.
- `*.pdf briefing` – filtre les PDF dont le nom inclut « briefing ».
- `*.zip size:>100MB` – recherche des fichiers ZIP de plus de 100MB.
- `infolder:/Users demo !.psd` – limite la racine de recherche à `/Users`, puis cherche les fichiers dont le nom contient `demo` en excluant `.psd`.
- `tag:ProjectA;ProjectB` – filtre sur les tags Finder (macOS) ; `;` agit comme `OR`.
- `*.md content:"Bearer "` – n’affiche que les Markdown contenant la chaîne `Bearer `.
- `"Application Support"` – placez les phrases exactes entre guillemets.
- `brary/Applicat` – utilisez `/` comme séparateur pour chercher des sous-chemins, en correspondant à des dossiers comme `Library/Application Support`.
- `/report` · `draft/` · `/report/` – entourez les tokens de barres en début/fin pour forcer les correspondances de préfixe, suffixe ou nom exact lorsque vous avez besoin d’un contrôle mot à mot au-delà de la syntaxe Everything.
- `~/**/.DS_Store` – le globstar (`**`) descend dans tous les sous-dossiers de votre répertoire personnel pour trouver les `.DS_Store` égarés.

Consultez le catalogue complet des opérateurs (groupement booléen, périmètre de dossiers, filtres d’extension, usage des regex et autres exemples) dans [`doc/search-syntax.md`](doc/search-syntax.md).

### Raccourcis clavier et aperçus

- `Space` – Quick Look de la ligne sélectionnée sans quitter Cardinal.
- `Cmd+R` – affiche le résultat sélectionné dans Finder.
- `Cmd+F` – ramène le focus sur la barre de recherche.
- `Cmd+C` – copie le chemin du fichier sélectionné dans le presse-papiers.
- `Cmd+Shift+Space` – bascule la fenêtre Cardinal globalement via le raccourci rapide.

Bonne recherche !

---

## Compiler Cardinal

### Prérequis

- macOS 12+
- Toolchain Rust
- Node.js 18+ avec npm
- Outils en ligne de commande Xcode et prérequis Tauri (<https://tauri.app/start/prerequisites/>)

### Mode développement

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Build de production

```bash
cd cardinal
npm run tauri build
```
