<div align="center">
  <img src="../cardinal/mac-icon_1024x1024.png" alt="Cardinal simgesi" width="120" height="120">
  <h1>Cardinal</h1>
  <p>macOS için en hızlı ve en doğru dosya arama uygulaması.</p>
  <p>
    <a href="#cardinali-kullanma">Cardinal’i kullanma</a> ·
    <a href="#cardinali-derleme">Cardinal’i derleme</a>
  </p>
  <img src="UI.gif" alt="Cardinal arayüz önizlemesi" width="720">
</div>

---

[English](../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Cardinal’i kullanma

### İndirme

Homebrew ile kurun:

```bash
brew install --cask cardinal-search
```

Ayrıca en güncel paketleri [GitHub Releases](https://github.com/cardisoft/cardinal/releases/) üzerinden indirebilirsiniz.

### i18n desteği

Farklı bir dil mi lazım? Durum çubuğundaki ⚙️ düğmesine tıklayarak anında değiştirin.

### Arama temelleri

Cardinal artık klasik alt dize/ön ek eşleştirmesinin üzerine Everything uyumlu bir söz dizimi katmanı ekliyor:

- `report draft` – boşluk `AND` gibi davranır; yalnızca adında her iki belirteci de içeren dosyaları görürsünüz.
- `*.pdf briefing` – adında “briefing” geçen PDF sonuçlarını filtreler.
- `*.zip size:>100MB` – 100MB’den büyük ZIP dosyalarını arar.
- `infolder:/Users demo !.psd` – arama kökünü `/Users` ile sınırlar; ardından adında `demo` olanları bulur ama `.psd`’yi hariç tutar.
- `tag:ProjectA;ProjectB` – Finder etiketlerini (macOS) eşleştirir; `;` `OR` gibi çalışır.
- `*.md content:"Bearer "` – `Bearer ` dizgesini içeren Markdown dosyalarını filtreler.
- `"Application Support"` – tam ifadeler için tırnak kullanın.
- `brary/Applicat` – alt yol araması için `/` yol ayırıcısı olarak kullanılır; `Library/Application Support` gibi dizinlerle eşleşir.
- `/report` · `draft/` · `/report/` – belirtecin başına/sonuna `/` ekleyerek Everything söz diziminin ötesinde, ön ek, son ek veya tam ad eşleştirmesini zorlayın.
- `~/**/.DS_Store` – globstar (`**`) ev dizininizin altındaki tüm klasörlere iner ve her yerdeki `.DS_Store` dosyalarını bulur.

Desteklenen operatör kataloğu (mantıksal gruplama, klasör kapsamı, uzantı filtreleri, regex kullanımı ve daha fazla örnek) için [`search-syntax.md`](search-syntax.md) dosyasına bakın.

### Klavye kısayolları ve önizlemeler

- `Space` – Cardinal’den ayrılmadan seçili satırı Quick Look ile önizleyin.
- `Cmd+R` – vurgulanan sonucu Finder’da gösterin.
- `Cmd+F` – odağı arama çubuğuna geri alın.
- `Cmd+C` – seçili dosyanın yolunu panoya kopyalayın.
- `Cmd+Shift+Space` – hızlı başlatma kısayoluyla Cardinal penceresini global olarak aç/kapatın.

İyi aramalar!

---

## Cardinal’i derleme

### Gereksinimler

- macOS 12+
- Rust toolchain
- npm’li Node.js 18+
- Xcode command-line tools ve Tauri prerequisites (<https://tauri.app/start/prerequisites/>)

### Geliştirme modu

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### Üretim derlemesi

```bash
cd cardinal
npm run tauri build
```
