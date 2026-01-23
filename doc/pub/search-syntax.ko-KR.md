# Cardinal 검색 문법

Cardinal의 쿼리 언어는 Everything의 문법에 의도적으로 가깝지만, 현재 엔진이 실제로 구현한 내용을 반영합니다. 이 문서는 Rust 백엔드가 오늘 기준으로 이해하는 내용을 담은 기준 문서입니다.

[English](search-syntax.md) · [Español](search-syntax.es-ES.md) · [한국어](search-syntax.ko-KR.md) · [Русский](search-syntax.ru-RU.md) · [简体中文](search-syntax.zh-CN.md) · [繁體中文](search-syntax.zh-TW.md) · [Português](search-syntax.pt-BR.md) · [Italiano](search-syntax.it-IT.md) · [日本語](search-syntax.ja-JP.md) · [Français](search-syntax.fr-FR.md) · [Deutsch](search-syntax.de-DE.md) · [Українська](search-syntax.uk-UA.md) · [العربية](search-syntax.ar-SA.md) · [हिन्दी](search-syntax.hi-IN.md) · [Türkçe](search-syntax.tr-TR.md)

---

## 1. 멘탈 모델

- 모든 쿼리는 다음으로 이루어진 트리로 파싱됩니다:
  - **단어 / 구문** (일반 텍스트, 따옴표 문자열, 와일드카드),
  - **필터** (`ext:`, `type:`, `dm:`, `content:`, …),
  - **불리언 연산자** (`AND`, `OR`, `NOT` / `!`).
- 매칭은 파일의 **전체 경로**를 기준으로 수행되며 파일명만 보지 않습니다.
- 대/소문자 구분은 UI 토글로 제어됩니다:
  - **대/소문자 구분 없음**일 때는 이름/콘텐츠 매칭을 위해 쿼리와 후보를 모두 소문자로 변환합니다.
  - **대/소문자 구분**일 때는 바이트를 그대로 비교합니다.

빠른 예시:
```text
report draft                  # 경로에 “report”와 “draft”가 모두 포함된 파일
ext:pdf briefing              # 이름에 “briefing”이 포함된 PDF 파일
parent:/Users demo!.psd       # /Users 아래에서 .psd 파일 제외
regex:^Report.*2025$          # regex에 일치하는 이름
ext:png;jpg travel|vacation   # 이름에 “travel” 또는 “vacation”이 포함된 PNG/JPG
```

---

## 2. 토큰, 와일드카드, 경로 세그먼트

### 2.1 일반 토큰과 구문

- 따옴표 없는 토큰은 경로에 대한 **부분 문자열 매칭**입니다:
  - `demo`는 `/Users/demo/Projects/cardinal.md`와 일치합니다.
- 큰따옴표로 감싼 구문은 공백을 포함한 정확한 시퀀스와 일치합니다:
  - `"Application Support"`는 `/Library/Application Support/...`와 일치합니다.
- UI 대/소문자 토글은 둘 다에 적용됩니다.

### 2.2 와일드카드 (`*`, `?`, `**`)

- `*`는 0개 이상의 문자와 일치합니다.
- `?`는 정확히 1개의 문자와 일치합니다.
- `**`는 슬래시 사이에 있을 때 **임의의 개수의 폴더 세그먼트**를 넘나드는 글롭스타입니다.
- 와일드카드는 **단일 토큰 내부에서** 해석됩니다:
  - `*.rs` — `.rs`로 끝나는 모든 이름.
  - `report-??.txt` — `report-01.txt`, `report-AB.txt` 등.
  - `a*b` — `a`로 시작해 `b`로 끝나는 이름.
  - `src/**/Cargo.toml` — `src/` 아래 어디든 있는 `Cargo.toml`.
- 리터럴 `*` 또는 `?`가 필요하면 토큰을 따옴표로 묶으세요: `"*.rs"`. 글롭스타는 독립된 슬래시 세그먼트여야 합니다 (`foo/**/bar`, `/Users/**`, `**/notes`).

### 2.3 `/`로 경로 스타일 세그먼트화

Cardinal은 토큰 안의 “슬래시 세그먼트”를 이해하고 각 세그먼트를 경로 구성요소에 대한 접두사/접미사/정확/부분 문자열 매칭으로 분류합니다. 예시:

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

이를 통해 다음을 표현할 수 있습니다:
- “폴더가 X로 끝나야 한다” (`foo/`),
- “폴더가 X로 시작해야 한다” (`/foo`),
- “경로 중간에 정확한 폴더 이름” (`gaea/lil/bee/`).

---

## 3. 불리언 로직과 그룹핑

Cardinal은 Everything의 우선순위를 따릅니다:

- `NOT` / `!`가 가장 강하게 결합됩니다.
- `OR` / `|`가 그 다음입니다.
- 암시적 / 명시적 `AND`(“공백”)는 **가장 낮은** 우선순위를 가집니다.

### 3.1 연산자

| 문법           | 의미                                              |
| -------------- | ------------------------------------------------- |
| `foo bar`      | `foo AND bar` — 두 토큰이 모두 일치해야 합니다.   |
| `foo\|bar`      | `foo OR bar` — 둘 중 하나만 일치하면 됩니다.      |
| `foo OR bar`   | `|`의 단어 형태입니다.                            |
| `!temp`        | `NOT temp` — 일치 항목을 제외합니다.             |
| `NOT temp`     | `!temp`와 같습니다.                                |
| `( ... )`      | 괄호로 그룹핑합니다.                               |
| `< ... >`      | 꺾쇠 괄호로 그룹핑합니다 (Everything 스타일).      |

우선순위 예시:
```text
foo bar|baz        # foo AND (bar OR baz)로 해석됨
!(ext:zip report)  # ext:zip과 “report”가 모두 일치하는 항목 제외
good (<src|tests> ext:rs)
                   # good AND ((src OR tests) AND ext:rs)
```

기본 우선순위를 바꾸고 싶다면 괄호 또는 `<...>`를 사용하세요.

---

## 4. 필터

이 섹션은 현재 엔진이 실제로 평가하는 필터만 나열합니다.

> **참고**: 필터 인자는 콜론 바로 뒤에 와야 합니다 (`ext:jpg`, `parent:/Users/demo`). `file: *.md`처럼 공백을 넣으면 Cardinal이 이를 `file:` 필터(인자 없음)와 별도의 `*.md` 토큰으로 처리합니다.

### 4.1 파일 / 폴더 필터

| 필터           | 의미                         | 예시              |
| -------------- | ---------------------------- | ----------------- |
| `file:`        | 파일만 (폴더 제외)           | `file: report`    |
| `folder:`      | 폴더만                       | `folder:Projects` |

다른 조건과 조합할 수 있습니다:

```text
folder:Pictures vacation
file: invoice dm:pastyear
```

### 4.2 확장자 필터: `ext:`

- `ext:`는 `;`로 구분된 하나 이상의 확장자를 받습니다:
  - `ext:jpg` — JPEG 이미지.
  - `ext:jpg;png;gif` — 일반적인 웹 이미지 유형.
- 매칭은 대/소문자를 구분하지 않으며 점(`.`)을 포함하지 않습니다.

예시:
```text
ext:md content:"TODO"
ext:pdf briefing parent:/Users/demo/Reports
ext:png;jpg travel|vacation
```

### 4.3 폴더 범위: `parent:`, `infolder:` / `in:`, `nosubfolders:`

| 필터             | 의미                                                   | 예시                                         |
| ---------------- | ------------------------------------------------------ | ------------------------------------------- |
| `parent:`        | 지정한 폴더의 직계 하위 항목만                         | `parent:/Users/demo/Documents ext:md`      |
| `infolder:`/`in:`| 지정한 폴더의 모든 하위 항목 (재귀)                     | `in:/Users/demo/Projects report draft`     |
| `nosubfolders:`  | 폴더 자체 + 직계 파일 (하위 폴더 없음)                 | `nosubfolders:/Users/demo/Projects ext:log` |

이 필터들은 절대 경로를 인자로 받으며, 앞의 `~`는 사용자 홈 디렉터리로 확장됩니다.

### 4.4 타입 필터: `type:`

`type:`은 파일 확장자를 의미 있는 카테고리로 묶습니다. 지원되는 카테고리(대/소문자 무시, 동의어 포함)는 다음과 같습니다:

- 이미지: `type:picture`, `type:pictures`, `type:image`, `type:images`, `type:photo`, `type:photos`
- 비디오: `type:video`, `type:videos`, `type:movie`, `type:movies`
- 오디오: `type:audio`, `type:audios`, `type:music`, `type:song`, `type:songs`
- 문서: `type:doc`, `type:docs`, `type:document`, `type:documents`, `type:text`, `type:office`
- 프레젠테이션: `type:presentation`, `type:presentations`, `type:ppt`, `type:slides`
- 스프레드시트: `type:spreadsheet`, `type:spreadsheets`, `type:xls`, `type:excel`, `type:sheet`, `type:sheets`
- PDF: `type:pdf`
- 압축 파일: `type:archive`, `type:archives`, `type:compressed`, `type:zip`
- 코드: `type:code`, `type:source`, `type:dev`
- 실행 파일: `type:exe`, `type:exec`, `type:executable`, `type:executables`, `type:program`, `type:programs`, `type:app`, `type:apps`

예시:
```text
type:picture vacation
type:code "Cardinal"
type:archive dm:pastmonth
```

### 4.5 타입 매크로: `audio:`, `video:`, `doc:`, `exe:`

자주 쓰는 `type:`의 단축 표현:

| 매크로  | 동일한 표현     | 예시                 |
| ------ | --------------- | -------------------- |
| `audio:` | `type:audio`   | `audio: piano`       |
| `video:` | `type:video`   | `video: tutorial`    |
| `doc:`   | `type:doc`     | `doc: invoice dm:2024` |
| `exe:`   | `type:exe`     | `exe: "Cardinal"`   |

매크로는 선택적 인자를 받을 수 있습니다:
```text
audio:soundtrack
video:"Keynote"
```

### 4.6 크기 필터: `size:`

`size:`는 다음을 지원합니다:

- **비교**: `>`, `>=`, `<`, `<=`, `=`, `!=`
- **범위**: `min..max`
- **키워드**: `empty`, `tiny`, `small`, `medium`, `large`, `huge`, `gigantic`, `giant`
- **단위**: bytes (`b`), kilobytes (`k`, `kb`, `kib`, `kilobyte[s]`), megabytes (`m`, `mb`, `mib`, `megabyte[s]`), gigabytes (`g`, `gb`, `gib`, `gigabyte[s]`), terabytes (`t`, `tb`, `tib`, `terabyte[s]`), petabytes (`p`, `pb`, `pib`, `petabyte[s]`).

예시:
```text
size:>1GB                 # 1 GB보다 큼
size:1mb..10mb            # 1 MB에서 10 MB 사이
size:tiny                 # 0–10 KB (대략적인 키워드 범위)
size:empty                # 정확히 0바이트
```

### 4.7 날짜 필터: `dm:`, `dc:`

- `dm:` / `datemodified:` — 수정 날짜.
- `dc:` / `datecreated:` — 생성 날짜.

다음을 허용합니다:

1. **키워드** (상대 범위):
   - `today`, `yesterday`
   - `thisweek`, `lastweek`
   - `thismonth`, `lastmonth`
   - `thisyear`, `lastyear`
   - `pastweek`, `pastmonth`, `pastyear`

2. **절대 날짜**:
   - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
   - `DD-MM-YYYY`, `MM/DD/YYYY` 같은 일반적인 일‑우선 / 월‑우선 형식도 지원합니다.

3. **범위 및 비교**:
   - 범위: `dm:2024-01-01..2024-12-31`
   - 비교: `dm:>=2024-01-01`, `dc:<2023/01/01`

예시:
```text
dm:today                      # 오늘 수정됨
dc:lastyear                   # 지난 달력연도에 생성됨
dm:2024-01-01..2024-03-31     # 2024년 1분기 내 수정됨
dm:>=2024/01/01               # 2024-01-01 이후 수정됨
```

### 4.8 정규식 필터: `regex:`

`regex:`는 토큰의 나머지를 경로 구성요소(파일 또는 폴더 이름)에 적용되는 정규식으로 취급합니다.

예시:
```text
regex:^README\\.md$ parent:/Users/demo
regex:Report.*2025
```

UI 대/소문자 토글은 정규식 매칭에 영향을 줍니다.

### 4.9 내용 필터: `content:`

`content:`는 파일 내용을 **일반 문자열 부분**으로 검색합니다:

- `content:` 안에는 regex가 없습니다 — 바이트 부분 문자열 매칭입니다.
- 대/소문자 구분은 UI 토글을 따릅니다:
  - 구분하지 않는 모드에서는 검색 문자열과 스캔된 바이트를 모두 소문자로 바꿉니다.
  - 구분하는 모드에서는 바이트를 그대로 비교합니다.
- 매우 짧은 문자열은 허용되지만 `""`(빈 문자열)은 거부됩니다.

예시:
```text
*.md content:"Bearer "
ext:md content:"API key"
in:/Users/demo/Projects content:deadline
type:doc content:"Q4 budget"
```

콘텐츠 매칭은 파일을 스트리밍 방식으로 처리하며, 멀티바이트 시퀀스가 버퍼 경계를 넘을 수 있습니다.

### 4.10 태그 필터: `tag:` / `t:`

Finder 태그(macOS)로 필터링합니다. Cardinal은 파일 메타데이터에서 태그를 필요할 때 가져오며(캐시 없음), 결과가 큰 경우 `mdfind`로 후보를 좁힌 뒤 태그 매칭을 적용합니다.

- `;`로 구분된 하나 이상의 태그를 허용합니다(논리 OR): `tag:ProjectA;ProjectB`.
- 여러 `tag:` 필터를 연결하면 논리 AND가 되어 여러 태그 매칭을 수행합니다: `tag:Project tag:Important`.
- 대/소문자 구분은 UI 토글을 따릅니다.
- 태그 이름은 부분 문자열로 매칭합니다: `tag:proj`는 `Project`와 `project` 모두에 일치합니다.

예시:
```text
tag:Important
t:Urgent
tag:ProjectA;ProjectB report
tag:Project tag:Archive report
in:/Users/demo/Documents tag:"Q4"
```

---

## 5. 예시

현실적인 조합 몇 가지:

```text
#  Documents의 Markdown 노트 (PDF 제외)
parent:/Users/demo/Documents ext:md
parent:/Users/demo/Documents !ext:pdf

#  Reports에서 “briefing”이 언급된 PDF
ext:pdf briefing parent:/Users/demo/Reports

#  휴가 사진
type:picture vacation
ext:png;jpg travel|vacation

#  프로젝트 트리 안의 최근 로그 파일
in:/Users/demo/Projects ext:log dm:pastweek

#  Scripts 폴더 바로 아래의 셸 스크립트
parent:/Users/demo/Scripts *.sh

#  경로에 “Application Support”가 포함된 모든 항목
"Application Support"

#  regex로 특정 파일 이름 매칭
regex:^README\\.md$ parent:/Users/demo

#  /Users 아래의 모든 PSD 제외
in:/Users demo!.psd
```

이 페이지를 현재 엔진이 구현한 연산자와 필터의 권위 있는 목록으로 사용하세요. Everything의 추가 기능(접근/실행 날짜나 속성 기반 필터 등)은 구문 수준에서는 파싱되지만 현재는 평가 단계에서 거부됩니다.
