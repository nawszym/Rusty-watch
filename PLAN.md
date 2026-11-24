# Rusty Watch — plan zbudowania edytora tarcz zegarka dla Amazfit Active 2 (Round)

Wersja: 2026-11-23

> ⚠️ **Dokument historyczny** — opisuje stan i plany z dnia 2026-11-23. Obowiązującą
> dokumentacją jest [`AGENTS.md`](AGENTS.md), a opisem aplikacji [`README.md`](README.md).
> Prototyp Tauri/React w `ui/` został usunięty — jedynym frontendem jest WinUI 3.

## 1. Kontekst i ustalenia wstępne

### 1.1. Cel

Rusty Watch to **desktopowy edytor tarcz (watchface) dla Amazfit Active 2 (Round)**, napisany w Rust, wzorowany funkcjonalnie na edytorze **SashaCX75** (*Watch face editor for Amazfit watch on ZeppOS*). Edytor ma pozwalać na:

- otwieranie istniejących tarcz i projektów,
- wizualną edycję (WYSIWYG) elementów w rozdzielczości 466×466,
- podgląd z symulacją danych (czas, kroki, tętno, bateria itd.),
- zapis do własnego formatu projektu oraz eksport do poprawnego package'u ZeppOS,
- walidację i czytelne komunikaty o błędach.

### 1.2. Urządzenie docelowe (na podstawie dokumentacji Zepp i danych rynkowych)

| Cecha | Wartość |
|---|---|
| Model | Amazfit Active 2 (Round) — **nie** Active 2 Square |
| Ekran | 466 × 466 px, AMOLED, 1,32" |
| ZeppOS | 4.0 (niektóre warianty 4.5) |
| deviceSource (przykłady) | `8913152*`, `8913153`, `8913155`, `8913159`, `10092800*`, `10092801`, `10092803`, `10092807` |
| Kształt | okrągły |

Konsekwencje: profil urządzenia musi być wybierany **przed** otwarciem tarczy — SashaCX75 wyraźnie ostrzega, że zły profil powoduje zniekształcenie kolorów. W planie przewidziano osobny `DeviceProfile` z `deviceSource`, `designWidth=466` i profilem koloru.

### 1.3. Formaty, z którymi trzeba pracować

1. **Paczka ZeppOS (`.zip` / `.zab` / `.zpk`)** — w środku miniprogram:
   - `app.json` (manifest: `configVersion: v3/v4`, `app.appType: "watchface"`, `targets.<key>.platforms[].deviceSource`, `designWidth`, `module.watchface.editable`, `module.watchface.lockscreen`, uprawnienia `data:*`, i18n),
   - `app.js`,
   - `watchface/index.js` (klasyczne API `hmUI` / `hmSensor` / `timer`; nie `@zos/*`),
   - `assets/<targetKey>/…` (PNG, fonty).
2. **Format „designerski" (JSON projektu)** — deklaratywny opis widgetów (czas, data, kroki, bateria, tętno, pogoda, łuki, zestawy obrazków), który edytor SashaCX75 czyta, renderuje i z którego generuje kod JS. To jest główny model, którym będzie operował Rusty Watch.
3. **Tarcze zaszyfrowane (`index.bin` w zipie, tarcze prosto z aplikacji Zepp)** — **nie do edycji**. Edytor musi je wykrywać i jasno odrzucać (Sasha dodaje w tym celu osobne ostrzeżenie).

Uwaga licencyjna: wzorujemy się na **zachowaniu i formacie**, nie kopiujemy kodu źródłowego SashaCX75 ani cudzych zasobów graficznych. W README umieścimy podziękowanie i link do oryginału.

### 1.4. Podjęte decyzje (2026-11-23)

| Decyzja | Wybór |
|---|---|
| UI | **WinUI 3 (Windows App SDK 2.7) + Rust FFI** — natywne kontrolki Windows 11, Mica, ciemny pasek tytułu. Shell C#/XAML w `native/RustyWatch`, logika w Rust przez `crates/rusty-ffi`. |
| Zakres | **Tworzenie nowych tarcz + eksport** priorytetowo. Edycja wyłącznie tarcz wyeksportowanych przez Rusty Watch (round-trip własnego formatu). Nie importujemy cudzych tarcz. |
| Eksport | **Tylko `.zip`** (format akceptowany przez amazwatchfaces / Zepp). Bez `.zpk` na start. |
| Licencja | **GPL-3.0-or-later** (strong copyleft). Dodajemy *Generation Exception*: wygenerowane tarcze nie są objęte GPL — użytkownik może je swobodnie publikować. |
| Urządzenia | **Wyłącznie Amazfit Active 2 (Round)**; pozostałe modele w przyszłości. |
| Start | Fundamenty: model, format projektu, generator kodu, CLI i szkielet UI. Funkcje dalej będą dodawane iteracyjnie. |

Ważne wyjaśnienie: wybór UI **nie wpływa na wydajność wyeksportowanej tarczy**. Tarcza to statyczne pliki (JS + PNG) uruchamiane przez ZeppOS; o jej szybkości decyduje generator (liczba widgetów, wstępnie narysowane tło, poprawny AOD), nie technologia edytora.

---

## 2. Architektura (propozycja)

Workspace Rust z trzema crate'ami + natywny shell WinUI 3:

```
rusty-watch/
├── Cargo.toml                 # workspace
├── crates/
│   ├── rusty-core/            # model, walidacja, codegen, writer zip
│   ├── rusty-cli/             # new | validate | export | inspect
│   └── rusty-ffi/             # C ABI (cdylib) dla WinUI 3
├── native/RustyWatch/         # WinUI 3 (C#/XAML) — główny frontend
│   ├── MainWindow.xaml        # układ wg reference-ui.png
│   ├── MainWindow.xaml.cs     # pickery, P/Invoke, akcje
│   └── RustNative.cs          # DllImport -> rusty_ffi
├── templates/                 # szablony tarcz startowych (własne grafiki)
├── tests/fixtures/            # własne przykłady + golden snapshots
├── docs/FORMAT.md             # udokumentowany format .rwf i mapowanie na ZeppOS
├── LICENSE                    # GPL-3.0-or-later + Generation Exception
└── README.md
```

### 2.1. Warstwy i zależności

- **rusty-core** — zero zależności od UI. `serde`(+`serde_json`) dla modelu, `zip` do paczek, `image` do PNG, `minijinja` lub `tera` do generowania JS. Wszystkie operacje czyste i testowalne.
- **rusty-cli** — cienka warstwa nad core; przydatna do testów round-trip i automatyzacji (CI).
- **rusty-ffi** — `cdylib` eksportujący funkcje `rw_*` zwracające JSON (UTF-8, zwalniane przez `rw_free_string`). To jedyny most między C# a logiką Rust.
- **native/RustyWatch** — natywny shell WinUI 3 (Windows App SDK 2.7): MicaBackdrop, ciemny pasek tytułu, kontrolki Fluent, pickery folderów/plików, układ wg `reference-ui.png`. C# nie zawiera logiki biznesowej.

**Dlaczego WinUI 3:** tylko natywne kontrolki Windows App SDK dają 1:1 look & feel Windows 11 (Mica, Segoe UI Variable, focus rectangles, animacje). Tauri/Fluent UI daje ~90–95%, ale nie 100%. Wydajność samej tarczy zależy od generatora JS w `rusty-core`, nie od technologii edytora.

### 2.2. Przepływ danych

```
Now project (template) ─┐
Open own .rwf ──────────┼-> DocumentModel (profile: Active 2 Round, warstwy, zasoby)
                       │     -> GUI (edycja: geometria, wiązania, zestawy obrazków)
                       │     -> Validate -> Render podglądu (symulacja sensorów)
                       │     -> Zapis .rwf
                       │     -> Export .zip: app.json + app.js + watchface/index.js + assets/
                       └     -> Instalacja: QR / tryb deweloperski w Zepp
```

### 2.3. Własny format projektu `.rwf`

- Katalog lub zip: `project.json` (schemat + wersja), `assets/**`, `src/**` (opcjonalny własny JS).
- Zachowujemy **oryginalne bajty niezmienionych plików**, żeby eksport był wierny.
- Semantyczna wersja schematu (`schema_version`) + migracje przy otwieraniu starszych projektów.
- `project.json` przyjazny diffom w git (ładna kolejność kluczy, brak embeddowanych binarek).

### 2.4. Model widgetów (MVP)

Podstawowe: `IMAGE`, `IMG_TIME`, `TEXT`, `TEXT_IMG`, `TIME_POINTER` (wskazówki), `ARC_PROGRESS`, `IMG_CLICK`.
Wiązania danych: `TIME`, `DATE`, `WEEKDAY`, `BATTERY`, `STEP`, `CALORIE`, `DISTANCE`, `HEART_RATE`, `SPO2`, `PAI`, `WEATHER`, `PRESSURE`.
Właściwości wspólne: `x, y, w, h`, `angle/pivot`, `align_h/v`, `color`, `show_level` (`ONLY_NORMAL`, `ONLY_AOD`, `ONLY_EDIT`), `image_set`, `type`, `editable group id`.

### 2.5. Generator kodu

- Szablony `minijinja` dla ZeppOS v3/v4 (`@zos/ui`, `@zos/sensor`) oraz fallback dla starszego `hmUI` (jeśli kiedyś dodamy inne modele).
- `editable: 1` i `lockscreen: 1` w `app.json` tylko gdy projekt używa odpowiednio edycji na zegarku / AOD.
- Obsługa `WATCHFACE_EDIT_GROUP` (w NORMAL) + `WATCHFACE_EDIT_MASK` (tylko EDIT), `WIDGET_DELEGATE.resume_call` do odświeżania po wyjściu z edycji — pułapki udokumentowane w źródłach Zepp.
- Custom type id dla własnych stylów ≥ 100000.
- Tryb „custom code block": wstrzykiwanie własnego JS do `index.js`, tak jak w edytorze SashaCX75 (np. AQI, własne formatowanie). Edytor nie musi rozumieć tego kodu, ale musi go zachować przy eksporcie i pokazać ostrzeżenie.

---

## 3. Podgląd (preview) — kluczowy element

Dwie strategie:

1. **Renderer w React/Canvas 2D (rekomendowany na start)** — rysowanie obrazków, łuków i tekstu na `<canvas>` 466×466 zgodnie z semantyką widgetów ZeppOS. Łatwe dopasowanie do mockupu i szybkie iteracje; logikę rysowania trzymamy w osobnym module, by później móc ją przetestować. Ryzyko: rozjazd z zachowaniem na zegarku (szczególnie fonty).
2. **Silnik JS (rquickjs/boa) z mockiem `hmUI`** — uruchamiamy wygenerowany `index.js` w Rust. Większa wierność logiki, ale znacznie więcej pracy (mock API sensorów, timerów).

Rekomendacja: strategy 1 + panel symulacji (suwak czasu, suwaki HR/kroków/battery, przełącznik AOD/EDIT) + jawne ostrzeżenie „sprawdź na zegarku" przy fontach i niestandardowym kodzie.

Uwagi wizualne: kolory konwertujemy do RGB565 przy podglądzie (SashaCX75 zniekształca kolory przy złym modelu); neutralne szarości trzymamy jako R=G=B; w trybie AOD ograniczamy OPR (<~15–20%), górny pasek `y≈18..44` zaznaczamy jako strefę zarezerwowaną dla ikon systemowych.

---

## 4. Etapy prac (szacunki dla jednej osoby)

### Faza 0 — Fundamenty (1–2 tyg.) — **startujemy tutaj**
- Szkielet repo: workspace Cargo (`rusty-core`, `rusty-cli`, `rusty-ffi`), shell WinUI 3, CI.
- Model danych + schemat `.rwf` (własny format, bez importu cudzych tarcz).
- Generator `app.json`, `app.js`, `watchface/index.js` dla profilu Active 2 Round.
- CLI: `new`, `validate`, `export`. Pierwszy działający zip „Rusty Watch v0.1".
- Szkielet UI wg `reference-ui.png` (panele + canvas, bez pełnej logiki).

**Done:** `cargo test` zielone, `rusty-cli new && export` tworzy poprawny zip.

### Faza 1 — Edycja własnych projektów (2–3 tyg.)
- Round-trip własnego `.rwf`: zapis/odczyt, zachowanie niezmienionych plików.
- Walidator (brakujące zasoby, wymiary, zakresy, konflikty id).
- Podgląd na canvasie React, zaznaczanie i przesuwanie widgetów, panel Properties.

**Done:** projekt wyeksportowany w Fazie 0 da się otworzyć i zmodyfikować.

### Faza 2 — Pierwsza tarcza na zegarku (1–2 tyg.)
- Instalacja przez tryb deweloperski Zepp (QR; `zeus preview` jako tymczasowy most).
- Weryfikacja poprawności manifestu i kodu przez `zeus build` / `zeus install` w oficjalnym symulatorze ZeppOS (przycisk „Przetestuj w emulatorze") oraz ostatecznie na fizycznym zegarku. Symulator pobierany jest wyłącznie przez użytkownika z oficjalnej strony (własna licencja Zepp Health — nie dołączamy go do repozytorium ani instalatora aplikacji).
- Sukces: prosta tarcza z własnym tłem i cyfrowym czasem działa na Active 2 Round.

### Faza 3 — Biblioteka widgetów i symulacja (3–4 tyg.)
- Wszystkie widgety z §2.4, zestawy obrazków (akceptacja reguły Sashy: jednakowy rozmiar, pierwszy największy).
- Wskazówki z pivotem, łuki progresu, klikalne pola, AOD i tryb ONLY_EDIT.
- Panel symulacji czujników, golden-image testy podglądu.

**Done:** można odtworzyć większość tarcz z przykładów bez ręcznej edycji JS.

### Faza 4 — Codegen i funkcje advanced (2–3 tyg.)
- Szablony JS v3/v4, `WATCHFACE_EDIT_GROUP`, i18n, custom code block.
- Szablon nowego projektu z biblioteki templates (podstawowy motyw startowy).
- Ostrzeżenia o fontach; eksport wyłącznie `.zip` (bez `.zpk`).

### Faza 5 — Hardening i release (2 tyg.)
- UX: undo/redo, klawiatura, snapping do środka/siatki, dialogi błędów, dokumentacja PL/EN.
- Packaging: `.msi`/`.exe` (Windows), `.deb`/AppImage (Linux), opcjonalnie macOS.
- Beta-testy na prawdziwym Active 2 Round, lista znanych ograniczeń, v1.0 + release notes.

**Szacunek: ~12–16 tygodni pracy w niepełnym wymiarze.**

---

## 5. Ryzyka i mitygacje

| Ryzyko | Mitygacja |
|---|---|
| Zniekształcenie kolorów / zły profil | Stały profil `deviceSource` dla Active 2 Round, podgląd RGB565 |
| Fonty renderują się inaczej niż na zegarku | Preferuj cyfry jako obrazki; przy fontach pokazuj ostrzeżenie i instrukcję restartu zegarka |
| Rozjazd wygenerowanego JS z API ZeppOS | Testy `zeus build` + `zeus preview` + test na fizycznym zegarku za każdym razem |
| Bateria / wycieki pamięci w wygenerowanym kodzie | Generator zawsze dodaje `off*` w `onDestroy`; checklista z guide'a Zepp |
| Kopiowanie GPL do wygenerowanej tarczy | `Generation Exception` w LICENSE: output generatora nie jest objęty GPL |
| Prawo autorskie | Nie kopiujemy kodu SashaCX75; własne grafiki i szablony; GPL-3.0-or-later |
| Złożoność UI | WinUI 3 gotowe kontrolki Fluent; własny tylko podgląd tarczy |

---

## 6. Kryteria akceptacji dla MVP

1. Tworzę nowy projekt z szablonu w Rusty Watch.
2. Zmieniam pozycję/obrazek elementu i zapisuję projekt `.rwf`.
3. Eksportuję `.zip`, instaluję na Active 2 Round przez tryb deweloperski/QR.
4. Tarcza wyświetla się poprawnie w trybie normalnym i AOD.
5. Walidator wykrywa brakujący obrazek w projekcie.
6. Round-trip `new → export → open → export` nie traci żadnych zasobów ani właściwości.

---

## 7. Backlog (po v1.0)

- Profile dla innych okrągłych modeli: GTR 4 (466×466), Balance (480×480), T-Rex 3, GTR Mini (416×416).
- Batch-resize / konwersja grafik, palety kolorów, narzędzie do krojenia cyfr (optical kerning).
- Import/eksport design JSON z oficjalnego Watchface Maker.
- Pluginy/widgety użytkownika, szablon projektu, galeria społeczności.
- Lokalizacja UI (PL/EN), motyw jasny/ciemmy.
- Opcjonalnie: podgląd w przeglądarce (WASM z `rusty-core`).

---

## 8. Decyzje (potwierdzone 2026-11-23)

1. **Stack UI:** WinUI 3 (Windows App SDK 2.7) + Rust FFI — prawdziwy 1:1 look & feel Win11; prototyp Tauri usunięty z repozytorium.
2. **Zakres:** tworzenie nowych tarcz + eksport; edycja tylko własnych wyeksportowanych projektów.
3. **Eksport:** wyłącznie `.zip` (amazwatchfaces / Zepp).
4. **Licencja:** GPL-3.0-or-later + Generation Exception dla wygenerowanych tarcz.
5. **Urządzenie:** tylko Active 2 Round; rozszerzenie na inne modele później.
6. **Tempo:** zaczynamy od fundamentów; dalsze funkcje będą dokładane iteracyjnie na podstawie kolejnych instrukcji.

---

## 9. Źródła

- SashaCX75, *Watch face editor for Amazfit watch on ZeppOS* — https://github.com/SashaCX75/Watch-face-editor-for-Amazfit-watch-on-ZeppOS (lista obsługiwanych modeli zawiera Amazfit Active 2)
- Wątek na forum Amazfit Watch Faces (uwagi o .bin, fontach, kolorach, v18.x) — https://amazfitwatchfaces.com/forum/viewtopic.php?t=2743
- Zepp OS — Device Basic Information (Active 2 Round: 466×466, ZeppOS 4.0/4.5, deviceSource) — https://docs.zepp.com/docs/v2/reference/related-resources/device-list
- Zepp OS — konfiguracja watchface (`app.json`, `editable`, `lockscreen`) — https://docs.zepp.com/docs/watchface/app-json
- Zepp OS — Watchface API / `hmUI.createWidget` — https://docs.zepp.com/docs/watchface/api/hmUI/createWidget/
- Oficjalny Watchface Maker — https://watchface.zepp.com/create
- `novvember/amazfit-watchfaces` — przykłady projektów, struktura i toolchain `zeus-cli` — https://github.com/novvember/amazfit-watchfaces
- Zepp OS watchface development guide (szczegóły API, AOD, edit groups, RGB565) — https://github.com/apopovsky/AmazfitWatchfaces/blob/main/.gemini/skills/zepp-os-watchface/SKILL.md
