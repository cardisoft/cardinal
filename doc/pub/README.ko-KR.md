<div align="center">
  <img src="../../cardinal/mac-icon_1024x1024.png" alt="Cardinal 아이콘" width="120" height="120">
  <h1>Cardinal</h1>
  <p>macOS에서 가장 빠르고 정확한 파일 검색 앱입니다.</p>
  <p>
    <a href="#cardinal-사용하기">Cardinal 사용하기</a> ·
    <a href="#cardinal-빌드하기">Cardinal 빌드하기</a>
  </p>
  <img src="UI.gif" alt="Cardinal UI 미리보기" width="720">
</div>

---

[English](../../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Cardinal 사용하기

### 다운로드

Homebrew로 설치:

```bash
brew install --cask cardinal-search
```

[GitHub Releases](https://github.com/cardisoft/cardinal/releases/)에서 최신 패키지 빌드를 받을 수도 있습니다.

### 국제화(i18n) 지원

다른 언어가 필요하신가요? 상태 표시줄의 ⚙️ 버튼을 눌러 바로 전환하세요.

### 검색 기본기

Cardinal은 기존의 부분 문자열/접두사 매칭 위에 Everything과 호환되는 문법을 더했습니다:

- `report draft` – 공백은 `AND`로 동작하므로 두 토큰을 모두 포함한 파일만 표시됩니다.
- `*.pdf briefing` – 이름에 “briefing”이 포함된 PDF 결과만 필터링합니다.
- `*.zip size:>100MB` – 100MB보다 큰 ZIP 파일을 찾습니다.
- `in:/Users demo !.psd` – 검색 루트를 `/Users`로 제한하고, 이름에 `demo`가 있지만 `.psd`는 제외합니다.
- `tag:ProjectA;ProjectB` – Finder 태그(macOS)를 매칭합니다; `;`는 `OR` 역할을 합니다.
- `*.md content:"Bearer "` – 문자열 `Bearer `를 포함한 Markdown만 표시합니다.
- `"Application Support"` – 따옴표로 정확한 구문을 검색합니다.
- `brary/Applicat` – `/`를 경로 구분자로 사용해 `Library/Application Support`와 같은 하위 경로를 매칭합니다.
- `/report` · `draft/` · `/report/` – 토큰 앞뒤에 `/`를 붙여 Everything 문법 이상의 단어 단위 제어가 필요한 경우 접두사/접미사/정확한 이름 매칭을 강제합니다.
- `~/**/.DS_Store` – 글롭스타(`**`)가 홈 디렉터리의 모든 하위 폴더를 탐색해 흩어진 `.DS_Store` 파일을 찾습니다.

지원되는 연산자 카탈로그(불리언 그룹화, 폴더 범위 지정, 확장자 필터, 정규식 사용, 추가 예시)는 [`search-syntax.ko-KR.md`](search-syntax.ko-KR.md)를 참고하세요.

### 단축키와 미리보기

- `Cmd+Shift+Space` – 전역 단축키로 Cardinal 창을 토글합니다.
- `Cmd+,` – 환경설정을 엽니다.
- `Esc` – Cardinal 창을 숨깁니다.
- `ArrowUp`/`ArrowDown` – 선택을 이동합니다.
- `Shift+ArrowUp`/`Shift+ArrowDown` – 선택 범위를 확장합니다.
- `Space` – Cardinal을 떠나지 않고 선택된 행을 Quick Look으로 확인합니다.
- `Cmd+O` – 선택한 결과를 엽니다.
- `Cmd+R` – 선택한 결과를 Finder에서 열어 보여줍니다.
- `Cmd+C` – 선택한 파일을 클립보드에 복사합니다.
- `Cmd+Shift+C` – 선택한 경로를 클립보드에 복사합니다.
- `Cmd+F` – 검색창으로 포커스를 되돌립니다.
- `ArrowUp`/`ArrowDown`(검색창) – 검색 기록을 이동합니다.

즐거운 검색 되세요!

---

## Cardinal 빌드하기

### 요구 사항

- macOS 12+
- Rust 툴체인
- npm이 포함된 Node.js 18+
- Xcode 명령줄 도구 및 Tauri 필수 구성(<https://tauri.app/start/prerequisites/>)

### 개발 모드

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### 프로덕션 빌드

```bash
cd cardinal
npm run tauri build
```
