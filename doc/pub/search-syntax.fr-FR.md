# Syntaxe de recherche de Cardinal

Le langage de requête de Cardinal est volontairement proche de la syntaxe d’Everything, tout en reflétant ce que le moteur actuel implémente réellement. Cette page est la référence officielle de ce que le backend Rust comprend aujourd’hui.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. Modèle mental

- Chaque requête est analysée en un arbre de:
  - **Mots / phrases** (texte brut, chaînes entre guillemets, jokers),
  - **Filtres** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **Opérateurs booléens** (`AND`, `OR`, `NOT` / `!`).
- La correspondance s’effectue sur le **chemin complet** de chaque fichier indexé, pas uniquement sur le nom de base.
- La sensibilité à la casse est contrôlée par le basculeur de l’UI:
  - En mode **insensible à la casse**, le moteur met en minuscules la requête et les candidats pour le matching nom/contenu.
  - En mode **sensible à la casse**, le moteur compare les octets tels quels.

Exemples rapides:
```text
report draft                  # fichiers dont le chemin contient “report” et “draft”
ext:pdf briefing              # fichiers PDF dont le nom contient “briefing”
parent:/Users demo!.psd       # sous /Users, exclure les fichiers .psd
regex:^Report.*2025$          # noms correspondant à une regex
ext:png;jpg travel|vacation   # PNG ou JPG dont le nom contient “travel” ou “vacation”
```

---

## 2. Jetons, jokers et segments de chemin

### 2.1 Jetons simples et phrases

- Un jeton sans guillemets est une **correspondance par sous-chaîne** sur le chemin:
  - `demo` correspond à `/Users/demo/Projects/cardinal.md`.
- Les phrases entre guillemets doubles correspondent à la séquence exacte, espaces compris:
  - `"Application Support"` correspond à `/Library/Application Support/...`.
- Le basculeur de casse de l’UI s’applique aux deux.

### 2.2 Jokers (`*`, `?`, `**`)

- `*` correspond à zéro ou plusieurs caractères.
- `?` correspond exactement à un caractère.
- `**` est un globstar qui traverse **n’importe quel nombre de segments de dossiers** lorsqu’il apparaît entre des barres.
- Les jokers sont interprétés **au sein d’un seul jeton**:
  - `*.rs` — tout nom se terminant par `.rs`.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt`, etc.
  - `a*b` — noms commençant par `a` et finissant par `b`.
  - `src/**/Cargo.toml` — `Cargo.toml` n’importe où sous `src/`.
- Si vous avez besoin d’un `*` ou `?` littéral, mettez le jeton entre guillemets: `"*.rs"`. Les globstars doivent être des segments de barre autonomes (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 Segmentation de type chemin avec `/`

Cardinal comprend les “segments à barres” au sein d’un jeton et classe chaque segment comme correspondance de préfixe/suffixe/exacte/sous-chaîne sur les composants du chemin. Exemples:

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

Cela permet d’exprimer:
- “Le dossier doit se terminer par X” (`foo/`),
- “Le dossier doit commencer par X” (`/foo`),
- “Nom exact de dossier au milieu du chemin” (`gaea/lil/bee/`).

---

## 3. Logique booléenne et regroupement

Cardinal suit la précédence d’Everything:

- `NOT` / `!` a la précédence la plus élevée,
- `OR` / `|` ensuite,
- `AND` implicite / explicite (“espace”) a la **plus faible** précédence.

### 3.1 Opérateurs

| Syntaxe        | Signification                                          |
| -------------- | ------------------------------------------------------ |
| `foo bar`      | `foo AND bar` — les deux jetons doivent correspondre. |
| `foo\|bar`      | `foo OR bar` — l’un ou l’autre peut correspondre.     |
| `foo OR bar`   | Forme en toutes lettres de `|`.                       |
| `!temp`        | `NOT temp` — exclut les correspondances.              |
| `NOT temp`     | Identique à `!temp`.                                  |
| `( ... )`      | Regroupement avec parenthèses.                        |
| `< ... >`      | Regroupement avec chevrons (style Everything).        |

Exemples de précédence:
```text
foo bar|baz        # analysé comme foo AND (bar OR baz)
!(ext:zip report)  # exclut les éléments où ext:zip ET “report” correspondent
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

Utilisez des parenthèses ou `<...>` dès que vous voulez remplacer la précédence par défaut.

---

## 4. Filtres

Cette section ne liste que les filtres que le moteur actuel évalue réellement.

> **Note**: les arguments des filtres doivent suivre immédiatement les deux-points (`ext:jpg`, `parent:/Users/demo`). Écrire `file: *.md` insère un espace, donc Cardinal le traite comme un filtre `file:` (sans argument) suivi du jeton séparé `*.md`.

### 4.1 Filtres fichier / dossier

| Filtre          | Signification                     | Exemple            |
| --------------- | --------------------------------- | ------------------ |
| `file:`         | Fichiers uniquement (pas dossiers) | `file: report`     |
| `folder:`       | Dossiers uniquement               | `folder:Projects`  |

Ces filtres peuvent être combinés avec d’autres termes:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 Filtre d’extension: `ext:`

- `ext:` accepte une ou plusieurs extensions séparées par `;`:
  - `ext:jpg` — images JPEG.
  - `ext:jpg;png;gif` — types d’images web courants.
- La correspondance est insensible à la casse et n’inclut pas le point.

Exemples:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 Portée de dossier: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| Filtre            | Signification                                                     | Exemple                                        |
| ----------------- | ----------------------------------------------------------------- | ---------------------------------------------- |
| `parent:`         | Uniquement les enfants directs du dossier indiqué                | `parent:/Users/demo/Documents ext:md`         |
| `infolder:`/`in:` | Tout descendant du dossier indiqué (récursif)                     | `in:/Users/demo/Projects report draft`        |
| `nosubfolders:`   | Le dossier lui‑même plus les fichiers enfants directs (sans sous‑dossiers) | `nosubfolders:/Users/demo/Projects ext:log`   |

Ces filtres prennent un chemin absolu comme argument; un `~` initial est développé vers le répertoire personnel de l’utilisateur.

### 4.4 Filtre de type: `type:`

`type:` regroupe les extensions de fichiers en catégories sémantiques. Les catégories prises en charge (insensibles à la casse, avec synonymes) incluent:

- Images: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- Vidéo: `type:video`, `type:videos`, `type:movie`, `type:movies`
- Audio: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- Documents: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- Présentations: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- Tableurs: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- Archives: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- Code: `type:code`, `type:source`, `type:dev`
- Exécutables: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

Exemples:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 Macros de type: `audio:`, `video:`, `doc:`, `exe:`

Raccourcis pour les cas `type:` courants:

| Macro    | Équivalent à       | Exemple                |
| ------- | ------------------ | ---------------------- |
| `audio:` | `type:audio`      | `audio: piano`         |
| `video:` | `type:video`      | `video: tutorial`      |
| `doc:`   | `type:doc`        | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`        | `exe: "Cardinal"`     |

Les macros acceptent un argument optionnel:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 Filtre de taille: `size:`

`size:` prend en charge:

- **Comparaisons**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **Intervalles**: `min..max`
- **Mots-clés**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **Unités**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

Exemples:
```text
size:>1GB                 # supérieur à 1 GB
size:1mb..10mb            # entre 1 MB et 10 MB
size:tiny                 # 0–10 KB (plage approximative par mot‑clé)
size:empty                # exactement 0 octet
```

### 4.7 Filtres de date: `dm:`, `dc:`

- `dm:` / `datemodified:` — date de modification.
- `dc:` / `datecreated:` — date de création.

Ils acceptent:

1. **Mots-clés** (plages relatives):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **Dates absolues**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - Prend aussi en charge les formats courants jour‑d’abord / mois‑d’abord comme `DD-MM-YYYY` et `MM/DD/YYYY`.

3. **Plages et comparaisons**:
   - Plages: `dm:2024-01-01..2024-12-31`
   - Comparaisons: `dm:>=2024-01-01`, `dc:<2023/01/01`

Exemples:
```text
dm:today                      # modifié aujourd’hui
dc:lastyear                   # créé l’an dernier (année civile)
dm:2024-01-01..2024-03-31     # modifié au T1 2024
dm:>=2024/01/01               # modifié à partir du 2024-01-01
```

### 4.8 Filtre regex: `regex:`

`regex:` traite le reste du jeton comme une expression régulière appliquée à un composant du chemin (nom de fichier ou de dossier).

Exemples:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

Le basculeur de casse de l’UI affecte la correspondance regex.

### 4.9 Filtre de contenu: `content:`

`content:` parcourt le contenu des fichiers à la recherche d’une **sous-chaîne simple**:

- Pas de regex dans `content:` — c’est une correspondance de sous-chaîne d’octets.
- La sensibilité à la casse suit le basculeur de l’UI:
  - En mode insensible, le moteur met en minuscules l’aiguille et les octets analysés.
  - En mode sensible, les octets sont comparés tels quels.
- Les aiguilles très courtes sont autorisées, mais `""` (vide) est refusé.

Exemples:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

La correspondance du contenu se fait en streaming sur le fichier; les séquences multioctets peuvent traverser les limites de buffer.

### 4.10 Filtre de tags: `tag:` / `t:`

Filtre par tags Finder (macOS). Cardinal récupère les tags à la demande depuis les métadonnées du fichier (sans cache) et, pour de grands ensembles de résultats, utilise `mdfind` pour réduire les candidats avant d’appliquer le matching des tags.

- Accepte un ou plusieurs tags séparés par `;` (OR logique): `tag:ProjectA;ProjectB`.
- Enchaîne plusieurs filtres `tag:` (AND logique) pour des correspondances multi‑tags: `tag:Project tag:Important`.
- La sensibilité à la casse suit le basculeur de l’UI.
- Correspond aux noms de tags par sous-chaîne: `tag:proj` correspond à `Project` et `project`.

Exemples:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. Exemples

Quelques combinaisons réalistes:

```text
#  Notes Markdown dans Documents (sans PDF)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  PDF dans Reports mentionnant “briefing”
ext:pdf briefing parent:/Users/demo/Reports

#  Photos de vacances
type:picture vacation
ext:png;jpg travel|vacation

#  Fichiers de log récents dans un arbre de projet
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts shell directement sous le dossier Scripts
parent:/Users/demo/Scripts *.sh

#  Tout avec “Application Support” dans le chemin
"Application Support"

#  Correspondre à un nom de fichier précis via regex
regex:^README\\.md$ parent:/Users/demo

#  Exclure les PSD partout sous /Users
in:/Users demo!.psd
```

Utilisez cette page comme la liste autorisée des opérateurs et filtres que le moteur implémente aujourd’hui; des fonctionnalités supplémentaires d’Everything (comme les dates d’accès/exécution ou les filtres basés sur des attributs) sont analysées au niveau de la syntaxe mais actuellement rejetées lors de l’évaluation.
