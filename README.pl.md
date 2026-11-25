<div align="center">

<img src="logo/github-avatar.png" alt="Logo Rusty Watch" width="150" />

# Rusty Watch

**Natywny edytor tarcz dla Amazfit Active 2 (Round) · Zepp OS 4.x**

Projektuj tarcze dla **Amazfit Active 2 (Round)** na Windows 11, sprawdzaj efekt
w podglądzie **1:1 (466 × 466)** i eksportuj gotowy plik **`.zip`**.

[![Rust](https://img.shields.io/badge/Rust-workspace-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![C#](https://img.shields.io/badge/C%23-WinUI%203-9B4F96?logo=csharp&logoColor=white)](https://dotnet.microsoft.com/)
[![WinUI 3](https://img.shields.io/badge/Windows%20App%20SDK-2.7-0F6CBD?logo=windows&logoColor=white)](https://learn.microsoft.com/windows/apps/)
[![Zepp OS](https://img.shields.io/badge/Zepp%20OS-4.x-38B48B)](https://docs.zepp.com/)
[![Windows](https://img.shields.io/badge/Windows-11-0078D6?logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![License](https://img.shields.io/badge/license-GPL--3.0--or--later-4B8BBE)](LICENSE)

**[🇬🇧 Read in English / Czytaj po angielsku](README.md)**

<a href="docs/TECHNICAL.md">🔧 Szczegółowe know-how techniczne</a> ·
<a href="docs/FORMAT.md">Format projektu</a> ·
<a href="PLAN.md">Plan</a> ·
<a href="AGENTS.md">Dla agentów AI</a> ·
<a href="Readme.base">Dokumentacja bazowa</a>

</div>

---

## 📖 Część opisowa

### 🎯 Czym jest Rusty Watch?

Rusty Watch to **dedykowany edytor tarcz zegarka** dla modelu **Amazfit Active 2 (Round)**
(466 × 466 px, Zepp OS 4.x, `deviceSource: 8913153`). Zamiast ręcznie pisać pliki `app.json`
i JavaScript dla ZeppOS, tworzysz tarczę w natywnym interfejsie Windows, układasz warstwy,
podłączasz zasoby i jednym kliknięciem eksportujesz paczkę do wgrania na zegarek.

Projekt powstał jako własna implementacja inspirowana edytorem
[SashaCX75](https://github.com/SashaCX75/Watch-face-editor-for-Amazfit-watch-on-ZeppOS) —
żaden kod ani zasoby tego projektu nie zostały skopiowane.

### ✨ Główne funkcje

| Funkcja | Opis |
| --- | --- |
| 🖥️ **Podgląd 1:1** | Okrągła tarcza 466 × 466 z prawdziwym tłem i wszystkimi widgetami w kolejności eksportu. |
| 🧱 **Warstwy** | Panel z ukrywaniem, przesuwaniem i usuwaniem widgetów. Tło jest na stałe zablokowane na samym dole. |
| 🖱️ **Ramki i przyciąganie** | Przeciąganie i skalowanie widgetów, kratka 16 px, prowadnice wyrównania do krawędzi i środków, przezroczystość widgetu (0–255). |
| 🕐 **Zegary** | Cyfrowy (własny wzorzec formatu) oraz analogowy (obracane wskazówki z zestawów PNG). |
| 📅 **Dane daty i czasu** | Data z edytowalnym wzorcem, dzień tygodnia, tydzień roku, dzień roku, data księżycowa, święta, zegar światowy. |
| ❤️ **Zdrowie i aktywność** | Kroki, dystans, tętno, stres, PAI, temperatura skóry, sen, kalorie i SpO₂. |
| 🎯 **Cele** | Cel kroków, kalorii, stania i spalania tłuszczu z pola `.target` czujników; przełącznik jednostek. |
| 🖼️ **Zasoby** | Czcionki TTF, grafiki widgetu Image, zestawy wskazówek, ikony STATUS i ikony pogody — magazyny współdzielone przez projekty. |
| 🎛️ **Opcje podglądu** | Domyślnie dane predefiniowane; **Podgląd na żywo** przełącza tylko czas i datę na zegar systemowy. Wartości są ustawieniem edytora — nigdy w `.zip`. |
| 🌐 **Język na zegarku** | Eksportowany ZIP sam wykrywa język (`pl-PL` → polskie nazwy miesięcy i AM/PM). |
| 📦 **Eksport `.zip`** | Paczka samowystarczalna: zawiera wygenerowany JS i wszystkie użyte zasoby. |
| 🧪 **Test w symulatorze** | Instalacja eksportu w oficjalnym Zepp OS Simulator jednym przyciskiem. |

### 🧩 Dostępne widgety

- **Czas** — Digital time, Analog time
- **Data** — Date, Day of week (number), Week of year, Day of year, Lunar date, Festival, World clock
- **Tekst** — Text (user defined)
- **Obraz** — Image (niebieska ramka dopasowana do naturalnego rozmiaru i tylko przesuw; dwuklik daje czerwoną ramkę, która rozciąga obraz przy zmianie rozmiaru)
- **Dystans** — Step counter, Distance
- **Cele** — Step target, Calorie target, Stand target, Fat burning target, Sleep target, Sleep target
- **Zdrowie** — Heart rate, Stress, PAI, Skin temperature, Sleep, Calories, SpO₂, Stand hours, Fat burning, PAI (week)
- **Pogoda** — nazwa pogody, miasto, temperatura max/min, wschód/zachód i 29 ikon (Classic)

Nowy widget dodajesz przyciskiem **+ Add widget** na pasku podglądu; katalog po lewej
to tylko podgląd obsługiwanych typów. Numeracja dnia tygodnia domyślnie przyjmuje
**poniedziałek jako 1** (wtorek = 2); wariant „niedziela pierwsza" ustawiasz w App Settings.

Cele czytają pole `.target` czujnika w zegarku. **Cel stania** i **Cel spalania tłuszczu**
mogą nie być dostępne na Amazfit Active 2 — gdy czujnika brakuje, tarcza zachowuje wartość
domyślną z projektu; edytor pokazuje ostrzeżenie w karcie widgetu i w inspektorze.
Dla celów kalorii, stania i spalania tłuszczu jest przełącznik **Pokaż jednostkę**
(`kcal`, `h`, `min`).

### 🎨 Zarządzanie zasobami

| Magazyn (`%APPDATA%\RustyWatch\…`) | Zawartość |
| --- | --- |
| `Fonts` | Czcionki TTF; domyślnie **Inter** (SIL OFL 1.1). Podgląd używa dokładnie tego samego pliku, który trafia do tarczy. |
| `Images` | Grafiki dla widgetu **Image**. |
| `Hands` | Zestawy wskazówek do zegara analogowego (wbudowane **Classic** i **Light**). |
| `Weather` | Paczki ikon pogody — wbudowana **Classic** (29 ikon z masterów SVG OFL, renderowanych do 512×512 PNG); można dodawać własne. |
| `Localization` | Katalogi języków interfejsu — można dodawać własne bez przebudowy aplikacji. |

Wybrane zasoby są kopiowane do katalogu projektu, więc **Save as…** i eksport ZIP nie
wymagają już magazynu aplikacji. Obrazy wskazówek są przycinane do ciasnego prostokąta alfa
z 8-pikselowym marginesem i skalowane w przestrzeni premultiplied alpha (bez białych
krawędzi), ponieważ ZeppOS nie skaluje zasobów `IMG` w runtime.

### 📤 Eksport i symulator

Kliknięcie **Export** najpierw generuje oprawiony `preview.png` (tarcza 21:37:40 pomniejszona w ramce z `logo/gear-outlayer.png`, z czarnymi strefami bezpiecznymi i ciemnym, ledwo widocznym obrysem) oraz 8-klatkowy,
animowany, **zapętlony w nieskończoność** `preview.gif` (pierwsza klatka to oprawiony obraz co `preview.png`, potem 7 surowych klatek). Na przycisku pojawia się
mały pasek postępu (`1/8` … `8/8`), który podczas kodowania GIF-a nadal się ładuje; okno eksportu
otwiera się **dopiero po** zapisaniu GIF-a i pokazuje animację **przyciętą do okręgu tarczy** nad
danymi tarczy oraz przyciskiem **Export ZIP**. Oba pliki lądują obok `project.json` i **nie trafiają
do `.zip`**.

Podglądy rysuje wyłącznie Rust (`crates/rusty-core/src/preview.rs`) — bez offscreen-renderowania
kontrolek WinUI, które dawniej zostawiały artefakt pod edytorem. 7 klatek ma zahardkodowany czas (godziny 6,9,12,15,18,20,21, minuty i sekundy ściśle rosnące, skrajności tętna 56/120), a każda dostaje **inną, prawdziwą datę**: 2022-01-03 (poniedziałek) … 2022-07-31 (niedziela), miesiące styczeń…lipiec. Wszystkie widgety daty
w klatce korzystają z tej samej daty, kroki liczone są z dystansu (`1300`–`1400` kroków/km),
więc `12,3 km` pokazuje około `16 000` kroków. Bateria maleje deterministycznie `100% → 10%`.
Poza promieniem `233` surowe klatki GIF-a mają czarne, nieprzezroczyste tło; oprawiony `preview.png`
ma tam przezroczyste tło wokół ramki logo. Niestandardowe dane podglądu nie wpływają na generowane pliki.

Eksport tworzy plik `.zip` bezpośrednio w folderze projektu — bez okna zapisu i bez
kolejnych kroków. Tarczę możesz przetestować w oficjalnym
[Zepp OS Simulator](https://docs.zepp.com/docs/guides/tools/simulator/download/)
przyciskiem **Test in emulator**. Rust eksportuje do `%APPDATA%\RustyWatch\Emulator\app-<appId>`,
rozpakowuje paczkę i instaluje ją poleceniem `zeus install --target "Amazfit Active 2 NFC (Round)"`.

> Symulator jest oprogramowaniem Zepp Health na własnej licencji. Aplikacja **nie dołącza,
> nie kopiuje i nie instaluje go potajemnie** — otwiera tylko oficjalną stronę pobierania.
> Test w symulatorze sprawdza build i runtime JS, ale nie zastępuje testu na fizycznym zegarku.

Eksport obsługuje wyłącznie format **`.zip`** (nigdy `.zpk`).

### 🖥️ Kategoria SYSTEM: bateria i ikony STATUS

Kategoria **SYSTEM** zawiera **Baterię** (z czujnika `BATTERY`, domyślnie wyśrodkowaną na tarczy) oraz cztery ikony **DND**, **LOCK**, **BLUETOOTH** i **ALARM** — bezpośrednio w SYSTEM, bez podkategorii. Nowe ikony tworzą jedną wycentrowaną linię. Używają `hmUI.widget.IMG_STATUS` — ZeppOS sam pokazuje lub ukrywa ikonę, więc nie generujemy własnych czujników ani timerów. Pakiety są skanowane z `%APPDATA%\RustyWatch\Status\<paczka>\`; ważna paczka musi mieć **wszystkie cztery** kwadratowe pliki: `alarm.png`, `dnd.png`, `bluetooth.png`, `lock.png`. Wbudowany jest tylko pakiet **Classic** (domyślny, jak czarne tło) w `native/RustyWatch/Assets/Status/`. Wybór paczki w inspektorze (lub w **Zasoby → Ikony statusu**) kopiuje ją do `<projekt>/assets/status/<paczka>/`, więc ZIP pozostaje samowystarczalny. Ikony obsługująstandardowy tryb niebieski-przesuw / czerwony-skalowanie (dwuklik włącza skalowanie). W podglądzie 7 klatek ikony pojawiają się tylko w wybranych klatkach; klatka **7** (`21:37:40`) ma wszystkie cztery i to ona trafia do `preview.png`. Miniaturki pakietów generuje Rust (wskazówki na 21:37:11, statusy w układzie 2×2), a zakładka Zasoby pokazuje maksymalnie trzy kafelki w rzędzie.

### 🌐 Języki interfejsu i tarczy

Interfejs edytora ma wbudowane katalogi **EN** i **PL** z natychmiastowym przełączaniem.
Nowe języki wystarczy dodać jako plik `%APPDATA%\RustyWatch\Localization\<kod>.json`
i kliknąć **Reload languages** — bez rebuildu i reinstallu. Osobna opcja **Watchface preview
language** wybiera wariant pokazywany w podglądzie; na zegarku język jest zawsze wykrywany
automatycznie.

### 👤 Dla kogo?

Dla osób, które chcą **własną tarczę pod Amazfit Active 2** bez znajomości ZeppOS API —
od pierwszego projektu po eksport i test w symulatorze. Kod źródłowy edytora jest otwarty,
więc rozszerzenia (nowe widgety, formaty, języki) możesz dodawać samodzielnie.

---

## 🛠️ Część techniczna

### 🔧 Szczegółowe know-how (zacznij tutaj)

**[`docs/TECHNICAL.md`](docs/TECHNICAL.md)** to opis działania aplikacji plik po pliku:
model `Project`/`Widget`, kolejność warstw, matematyka ramek i przyciągania, wszystkie
65 eksportów `rw_*`, pięć magazynów zasobów i algorytmy skalowania, współdzielony silnik
czasu/daty/lokalizacji, generowany `watchface/index.js` (sensory, timery, aktualizacje
widgetów), pakowanie ZIP oraz podgląd, inspektor, nawigacja i integracja z symulatorem
w WinUI. Dokument śledzi dokładną drogę od kliknięcia w edytorze do tarczy na zegarku.

Opcja **Podgląd na żywo** przełącza tylko czas i datę na zegar systemowy, a zębatka otwiera
edycyjne wartości predefiniowane. Żadna z nich nie trafia do `project.json` — eksportowana
tarcza nadal czyta prawdziwe czujniki zegarka. Zobacz `docs/TECHNICAL.md` §11.7.

### 🏗️ Architektura

```
┌──────────────────────────────────────────────────────────────┐
│  native/RustyWatch  — WinUI 3 / C# + XAML (UI, podgląd)     │
│  RustNative.cs  — P/Invoke → rw_* (C ABI, JSON UTF-8)       │
└───────────────────────────┬──────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  crates/rusty-ffi  — jedyny most C ABI (65 eksportów)        │
└───────────────────────────┬──────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  crates/rusty-core  — model, walidacja, codegen, ZIP,       │
│  formaty dat/czasu, warstwy, magazyny zasobów (bez UI)      │
└───────────────────────────┬──────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  generated watchface: app.json · app.js · watchface/index.js│
│  klasyczne API: hmUI / hmSensor / timer · WatchFace({...})  │
└──────────────────────────────────────────────────────────────┘
```

`rusty-core` nie ma zależności od UI. C# nie zawiera logiki biznesowej — wywołuje
funkcje `rw_*`, parsuje JSON i odświeża interfejs. Każda funkcja FFI zwraca
`{"ok":true,"data":…}` albo `{"ok":false,"error":"…"}`, a wywołujący zwalnia string
przez `rw_free_string`. Podgląd i generator korzystają ze **wspólnego** modułu warstw
i formatów, dzięki czemu pozostają w 1:1.

### 🧱 Stack technologiczny

| Warstwa | Technologia |
| --- | --- |
| 🖥️ UI | Windows 11, WinUI 3 / Windows App SDK **2.7.0**, C#/XAML, self-contained |
| 🦀 Logika | Rust — `rusty-core`, `rusty-cli`, `rusty-ffi` (cdylib, C ABI) |
| 🔗 Most | P/Invoke (`DllImport`, `CallingConvention.Cdecl`, UTF-8) |
| ⌚ Wyjście | Zepp OS 4.x, klasyczne API Watchface (`hmUI`, `hmSensor`, `timer`) |
| 🌐 Lokalizacja | Katalogi JSON (EN/PL), ładowane z `%APPDATA%\RustyWatch\Localization` |
| 🧪 Testy | `cargo test --workspace` (Rust), `dotnet build` (MSBuild/XamlCompiler) |
| 📄 Licencja | GPL-3.0-or-later + Generation Exception |

> 🚫 Nie używamy `@zos/ui` / `@zos/sensor` — to API dla Mini Programs. Generowana tarcza
> korzysta z klasycznego API, inaczej na zegarku renders black. `runtime.apiVersion` to
> `1.0.0` / `1.0.1`, bez `runtime.type`.

### 📁 Struktura repozytorium

```
rusty-watch/
├── crates/
│   ├── rusty-core/src/      # model, project, codegen, widgets, layers, formaty,
│   │                        #  zasoby (fonts/images/hands/status), zip, walidacja
│   ├── rusty-cli/src/       # CLI: new | validate | export | inspect
│   └── rusty-ffi/src/       # eksporty rw_* (65 funkcji)
├── native/RustyWatch/       # jedyny frontend WinUI 3
│   ├── MainWindow.xaml(.cs) # shell, HOME, edytor, zasoby, ustawienia, dialogi
│   ├── RustNative.cs        # P/Invoke do rusty_ffi.dll
│   ├── PreviewFonts.cs      # podgląd tym samym TTF, co eksport
│   ├── Assets/              # AppIcon.ico, Logo.png, Inter TTF, wskazówki, licencja czcionki
│   └── Localization/        # seedy en.json / pl.json + przewodnik formatu
├── docs/
│   ├── TECHNICAL.md         # szczegółowe know-how techniczne (zacznij tutaj)
│   └── FORMAT.md            # format pliku projektu i paczki
├── logo/                    # wektorowe źródła logo i wersje PNG/ICO
├── PLAN.md                  # decyzje i historia projektu (dokument historyczny)
├── AGENTS.md                # zasady dla agentów AI (źródło prawdy)
├── README.md                # wersja angielska (domyślna)
├── Readme.base              # dokumentacja bazowa (źródło zmian opisu)
└── LICENSE                  # GPL-3.0-or-later + Generation Exception
```

Dane aplikacji (projekty, ustawienia, magazyny zasobów) żyją poza repozytorium,
w **`%APPDATA%\RustyWatch`** — dzięki czemu przetrwają aktualizacje i podmianę katalogu z programem.

### ✅ Wymagania

- **Windows 11**
- Rust (workspace `Cargo.toml`)
- .NET SDK **10** do zbudowania + .NET Runtime **8**
- Windows App SDK **2.7.0** (pobierany jako pakiet NuGet, aplikacja jest self-contained)
- Opcjonalnie: [Zepp OS Simulator](https://docs.zepp.com/docs/guides/tools/simulator/download/)
  z obrazem **Active 2 v1.1.0** i narzędzie `zeus` do testów

### 🔨 Budowanie i uruchomienie

W PowerShell (w tej powłoce trzeba dodać `dotnet` do `PATH`):

```powershell
# Testy całego workspace'u
cargo test --workspace

# Budowa i uruchomienie natywnego UI
$env:PATH = 'C:\Program Files\dotnet;' + $env:PATH
dotnet build .\native\RustyWatch\RustyWatch.csproj
.\native\RustyWatch\bin\Debug\net8.0-windows10.0.26100.0\win-x64\RustyWatch.exe
```

`dotnet build` najpierw woła `cargo build -p rusty-ffi` i kopiuje `rusty_ffi.dll`
do katalogu wyjściowego (`SkipUnchangedFiles="false"`, aby nie została stara biblioteka).
Oczekiwany wynik: **0 błędów / 0 ostrzeżeń** na obu toolchainach.

Szybki start z CLI:

```powershell
cargo run -p rusty-cli -- new .\moja-tarcza --name "Moja Tarcza" --author "Jan Kowalski" --app-id 12345678
cargo run -p rusty-cli -- validate .\moja-tarcza\project.json
cargo run -p rusty-cli -- export .\moja-tarcza\project.json
cargo run -p rusty-cli -- inspect .\moja-tarcza\Moja_Tarcza.zip
```

`--author` i `--app-id` są opcjonalne (domyślnie autor „Rusty Watch", App ID losowany).
Nazwa, autor i App ID trafiają do `app.json` jako `appName`, `vender`, `appId` i `i18n.*.appName`.

### 🧪 Testy i weryfikacja

1. `cargo test --workspace` — 129 testów core + 33 FFI.
2. `dotnet build .\native\RustyWatch\RustyWatch.csproj` — 0 błędów, 0 ostrzeżeń.
3. Uruchom aplikację i sprawdź ekrany w **EN i PL**.
4. Round-trip `new → export → open → export` musi zachować dane.
5. Weryfikacja w symulatorze, potem na fizycznym zegarku.

### 🔄 Workflow: edycja → eksport → zegarek

1. **Utwórz projekt** na stronie HOME — folder powstaje w katalogu projektów z App Settings.
2. **Dodaj widgety** z flyoutu nad podglądem i ustaw ich inspektora (zmiany zapisują się automatycznie).
3. **Dodaj zasoby** w zakładce Assets (czcionka, obraz, wskazówki, ikony statusu, ikony pogody).
4. **Eksportuj ZIP** przyciskiem Export — plik ląduje obok `project.json`.
5. **Przetestuj** w symulatorze (opcjonalnie) i wgraj na zegarek.

### 📦 Zawartość eksportowanego `.zip`

```
app.json                    # płaski manifest v2 (bez bloku `targets`; defaultLanguage, packageInfo, 8 deviceSource), metadane z projektu
app.js                      # punkt wejścia
watchface/index.js          # generowany kod tarcz (hmUI/hmSensor/timer)
assets/Preview.png          # oprawiony podgląd przeskalowany do 314x314 (maskowany do okręgu)
assets/fonts/font_<hash>.ttf  # użyte TTF
assets/hand_<hash>.png, status_<hash>.png, img_<hash>.png, weather_<hash>.png
                            # płaskie, unikalnie nazwane kopie użytych zasobów
```

### 🤖 Dokumentacja

- [`docs/TECHNICAL.md`](docs/TECHNICAL.md) — szczegółowe know-how techniczne.
  **Zmiany dokumentacji wprowadzaj najpierw do `Readme.base`, potem przenieś do README-ów.**
- [`docs/FORMAT.md`](docs/FORMAT.md) — format `project.json` i struktura paczki.
- [`PLAN.md`](PLAN.md) — cel, zakres i podjęte decyzje (dokument historyczny).
- [`AGENTS.md`](AGENTS.md) — architektura, komendy i pułapki WinUI 3; `CLAUDE.md` i
  `.github/copilot-instructions.md` odsyłają do tego samego pliku.
- [`README.md`](README.md) — angielska wersja tego dokumentu (domyślna).

### 📄 Licencja

**GPL-3.0-or-later** z **Generation Exception** — pełny tekst w [`LICENSE`](LICENSE).

Kod źródłowy edytora i jego pochodne pozostają open source na tej samej licencji
(copyleft). Wyjątek dotyczy wyłącznie **wygenerowanych tarcz `.zip`**: możesz je
swobodnie publikować bez obowiązku udostępniania własnych plików projektu.
Zobacz też [OFL 1.1](native/RustyWatch/Assets/Fonts/OFL-Inter.txt) dla dołączonej czcionki Inter.
