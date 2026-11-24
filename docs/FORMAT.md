# Format projektu Rusty Watch (`.rwf`) — v1

Projekt to katalog:

```
moja-tarcza/
├── project.json
└── assets/
    ├── bg.png           # tło przygotowane dla tarczy (466 × 466, gdy auto-skalowanie)
    ├── icon.png
    ├── fonts/           # opcjonalnie, gdy widget używa własnej czcionki
    │   └── Inter-Regular.ttf
    ├── backgrounds/     # opcjonalnie, oryginał wybranego obrazu tła (osobny magazyn)
    │   └── zdjecie.png
    ├── images/          # opcjonalnie, grafiki widgetów IMAGES dopasowane do ramki
    │   └── <widget-id>/
    │       └── wykres.png
    └── hands/           # opcjonalnie, pliki zestawu wskazówek zegara analogowego
        └── Classic/
            ├── hour.png
            ├── minute.png
            ├── second.png
            └── cap.png
```

`project.json` (schemaVersion = 1):

| Pole | Opis |
|---|---|
| `metadata.name` | nazwa tarczy (`appName`) |
| `metadata.appId` | identyfikator mini-programu |
| `metadata.author`, `version`, `description` | metadane manifestu |
| `device` | `active-2-round` (jedyny obsługiwany profil) |
| `widgets[]` | widgety w kolejności warstw (pole `z`) |
| `assets[]` | pliki z `assets/`, role: `icon`, `background`, `font`, `image`, `hand`, `other` |
| `background` | opcjonalnie `{ image, autoScale }` — plik z `%APPDATA%/RustyWatch/Backgrounds` (starsze projekty nadal działają dzięki jednorazowej migracji z `Images`) i tryb przygotowania `bg.png`. Domyślnie `black.png` (pełna czerń); wbudowany `starry.png` (dawny gradient) jest zwykłym plikiem magazynu. Starsze zapisy `black`/`starry` i brak pola są nadal akceptowane |
| `customCode` | opcjonalny surowy JS wstrzykiwany do wygenerowanego `index.js` |
| `measurementSystem` | `watch` (domyślnie — jednostka z ustawień zegarka, czytana w JS przez `hmSetting.getMileageUnit()`), `metric` albo `imperial` — domyślny system jednostek dla widgetów dystansu, kopiowany z ustawień aplikacji przy otwarciu projektu |
| `firstDayOfWeek` | `monday` (domyślnie — poniedziałek = 1, wtorek = 2, …, niedziela = 7) albo `sunday` (niedziela = 1, …) — numer pola „Dzień tygodnia (liczba)" w podglądzie i w JS; kopiowany z ustawień aplikacji przy otwarciu projektu |

Widget:

| Pole | Opis |
|---|---|
| `id`, `name` | identyfikator i nazwa w drzewie warstw |
| `kind` | `text`, `image`, `images`, `time_text`, `date_text`, `weekday_number_text`, `week_number_text`, `day_of_year_text`, `lunar_date_text`, `festival_text`, `world_clock_text`, `step_text`, `distance_text`, `heart_rate_text`, `stress_text`, `pai_text`, `skin_temp_text`, `sleep_text`, `calorie_text`, `spo2_text`, `analog_time` |
| `x`, `y`, `w`, `h`, `z` | geometria w przestrzeni 466×466 |
| `text`, `textSize`, `color` | właściwości tekstu (`color` jako `0xAARRGGBB`). Dla `kind: "text"` (widget „Text (user defined)") `text` to dowolna treść wpisana w edytorze — bez `dataSource` i bez `timeFormat` trafia do wygenerowanego JS jako statyczny `text: "..."`. Dla `kind: "step_text"` pole `text` jest wartością początkową (`"0"`), a dla `distance_text` (`"0.00 km"` albo `"0.00 mi"`) — podgląd edytora nie ma czujników |
| `font` | nazwa pliku z `%APPDATA%/RustyWatch/Fonts` (np. `Inter-Regular.ttf`); brak = czcionka systemowa zegarka. Kopia trafia do `assets/fonts/` i jest rejestrowana jako zasób `font` |
| `src` | nazwa pliku z `assets/` dla obrazka. Dla `kind: "images"` to `images/<widget-id>/<plik>` — PNG fizycznie dopasowany do `w × h` przez `image_fit` |
| `imageFit` | tylko `images`: `contain` (domyślnie), `cover` albo `stretch` — tryb skalowania użyty przy przygotowaniu PNG w projekcie i w eksporcie |
| `handSet`, `hourSrc`, `minuteSrc`, `secondSrc`, `capSrc` | tylko `analog_time`: zestaw i ścieżki wskazówek względem `assets/` (np. `hands/Classic/hour.png`). `w`/`h` to rzeczywisty rozmiar wskazówek (początkowo rozmiar plików, użytkownik może je przeskalować zachowując kwadrat), a środek obrotu to `(w/2, h/2)`. Każde z pól `*_src` może być `null` — wtedy dana wskazówka jest ukryta i nie trafia do JS ani do ZIP |
| `dataSource` | `time`, `date`, `week_day_number`, `week_number`, `day_of_year`, `lunar_date`, `festival`, `world_clock`, `step`, `distance`, `heart_rate`, `stress`, `pai`, `skin_temp`, `sleep`, `calorie`, `spo2`, `battery` — `lunar_date`/`festival` czytają `hmSensor.id.TIME`, `world_clock` czyta `hmSensor.id.WORLD_CLOCK`, `week_day_number` (pierwszy dzień wg `firstDayOfWeek`) / `week_number` / `day_of_year` są liczone wspólnym algorytmem w Rust i JS |
| `align` | `left`, `center`, `right` (poziomo) |
| `vAlign` | `top`, `center` (domyślnie), `bottom` — mapowane na `align_v` w ZeppOS |
| `showLevel` | `normal`, `aod`, `edit`, `always` |
| `autoScale` | tylko edytor: czy rozmiar ramki skaluje `textSize` (domyślnie `false`) |
| `textStyle` | `ellipsis` (domyślnie), `none` (przewijanie), `wrap` — mapowane na `text_style` w ZeppOS |
| `visible` | `true` (domyślnie, także gdy pola nie ma w starszych projektach) lub `false` — ukryta warstwa zostaje w projekcie i na liście Warstwy, ale nie jest rysowana w podglądzie ani w wygenerowanym JS |
| `measurementSystem` | tylko `distance_text`: `watch`, `metric`, `imperial` albo brak (`null`) = użyj `measurementSystem` z projektu |
| `showUnit` | tylko widgety zdrowotne z jednostką: `true` (domyślnie) pokazuje `bpm`, `°C`, `kcal`, `%`; `false` zostawia samą liczbę. Starsze projekty bez pola zachowują jednostki |
| `timeFormat` | wzorzec tekstu: dla `time_text` tokeny czasu `HH/H`, `hh/h`, `MM/M`, `SS/S`, `A/a`; dla `date_text` tokeny daty `D/d`, `DD/dd`, `M/m`, `MM/mm`, `MMM/mmm`, `MMMM/mmmm`, `DDD/ddd`, `DDDD/dddd`, `YY/YYYY`. Wielkość litery w tokenach nazw (`MMM`/`mmm`, `DDD`/`ddd`) steruje wielkością wyświetlanej nazwy; 5–6 liter daje WIELKIE LITERY. Parser wybierany jest na podstawie `kind`, więc schema projektu się nie zmienia. `\` wymusza literal. Domyślny wzorzec daty: `MMM D` |

Wzorzec daty jest współdzielony przez podgląd (`rw_format_date`) i generator JS (`date_format::js_expression_localized`) — nazwy miesięcy i dni tygodnia pochodzą z obiektu `L` wybieranego w runtime przez `hmSetting.getLanguage()` (`L.months`, `L.monthsFull`, `L.days`, `L.daysFull`), więc ten sam ZIP działa po polsku i angielsku. Indeks dnia tygodnia jest liczony od niedzieli (0), zgodnie z `Date.getDay()`.

Licznik kroków używa klasycznego API `hmSensor.createSensor(hmSensor.id.STEP)`. Wygenerowany JS czyta `sensor.current`, odświeża widget tekstowy w zdarzeniu `hmSensor.event.CHANGE` (oraz przy okazji tyknięcia zegara) i zdejmuje nasłuchiwanie w `onDestroy`. Utworzenie czujnika jest izolowane próbą `try/catch` — brak sensora nie może położyć całej tarczy, więc tekst pozostaje `"0"`. Widget nie ma wzorca ani edytowalnej treści; ramka, czcionka, kolor i wyrównanie działają jak dla pozostałych widgetów tekstowych.

Dystans działa tak samo, ale czyta `hmSensor.id.DISTANCE` (wartość w **metrach**). Generator przelicza ją na kilometry (`/ 1000`, sufiks `km`) albo mile (`/ 1609.344`, sufiks `mi`) i formatuje do dwóch miejsc po przecinku. Domyślnym systemem jest **`watch`** — wygenerowany JS raz przy starcie tarczy wywołuje `hmSetting.getMileageUnit()` i w zależności od ustawienia zegarka wybiera km albo mile, więc ten sam ZIP działa w obu jednostkach. `metric` i `imperial` to jawne alternatywy. System wybiera `Widget::measurement_system`, a gdy jest `null` — `Project::measurement_system` (ustawienie aplikacji „Domyślny system jednostek"). Inspektor widgetu ma przełącznik „Własny system jednostek": po włączeniu wybierany system (`watch`, `metric` albo `imperial`) zapisuje się w widgetcie, po wyłączeniu wraca do projektu. `rw_set_measurement_system` aktualizuje domyślny system projektu i teksty początkowe widgetów bez nadpisania. W podglądzie edytora `watch` pokazuje neutralny placeholder `0.00 km`, bo PC nie zna ustawienia jednostki zegarka.

Widgety kategorii **Zdrowie** czytają klasyczne czujniki ZeppOS: `HEART.current` (bpm), `STRESS.current` (0–100), `PAI.dailypai`, `BODY_TEMP.current` (°C z jednym miejscem po przecinku), `SLEEP.getTotalTime()` (format `H:MM`), `CALORIE.current` (kcal) i `SPO2.current` (%). Generator trzyma wspólny `healthTick` i po jednym leniwym getterze na czujnik (`try/catch`); odświeża widget przy zdarzeniu czujnika (`CURRENT` dla serca, `CHANGE` dla pozostałych) oraz przy tyknięciu zegara, a w `onDestroy` zdejmuje wszystkie nasłuchiwania. Brak czujnika na zegarku pozostawia tekst początkowy (`--`, `-- bpm`, `--:--` itd.) i nie przerywa działania tarczy. HRV i Gotowość nie są dostępne w katalogu — HRV nie ma sensora w watchface API, a Gotowość istnieje tylko jako `hmUI.data_type.READINESS`.

## Mapowanie przy eksporcie

`.rwf` → pliki w `.zip`:

| Źródło | Wynik |
|---|---|
| `metadata`, `device` | `app.json` (configVersion v3, `defaultLanguage: "en-US"`, `runtime.apiVersion` 1.0.0/1.0.1, bez `runtime.type`, target `active2round`, `deviceSource: 8913153`, `designWidth: 466`, `editable: 1`, `lockscreen: 1`) |
| stały szablon | `app.js` |
| `widgets[]` (tylko `visible: true`), `customCode` | `watchface/index.js` (klasyczne API tarczy: `hmUI.createWidget`, `hmSensor`, `timer`, `WatchFace({...})`) — `x/y/w/h`, `text_size`, `text_style` i `font` trafiają tam bezpośrednio z projektu; ukryte warstwy są pomijane, indeksy `refs` pozostają spójne |
| `assets/*` | `assets/active2round/*` (w tym `fonts/<plik>.ttf`, gdy użyto własnej czcionki) |

Eksport z edytora otwiera natychmiast okno (ContentDialog nad edytorem) tylko z danymi projektu: nazwą tarczy, autorem, wersją i App ID. Nie generujemy żadnego `preview.gif` i nie ma w tym oknie podglądu tarczy — ani animowanego, ani na żywo. Jedyną akcją jest przycisk „Eksportuj ZIP". Paczka trafia bezpośrednio do folderu projektu (obok `project.json`), bez okna wyboru ścieżki. Nazwa pliku powstaje z `metadata.name` — spacje i znaki niedozwolone stają się kropkami (`Moja tarcza` → `Moja.tarcza.zip`). Główny podgląd w edytorze pozostaje nietknięty.

Poziomy wyświetlania mapują się na stałe ZeppOS: `ONLY_NORMAL`, `ONAL_AOD`, `ONLY_EDIT`.

Widget `analog_time` generuje cztery widgety `IMG` (`hour`, `minute`, `second`, `cap`) z tymi samymi `x/y/w/h`; każdy ma `center_x/center_y` w środku obrazu i jest obracany przez `setProperty(hmUI.prop.MORE, { angle })` z wspólnego `tick`. Godzina: `(h % 12) * 30 + m * 0.5`, minuta: `m * 6 + s * 0.1`, sekunda: `s * 6`. Gdy zestaw nie ma sekundy (albo użytkownik ją ukrył), tarcza używa nasłuchiwacza `MINUTEEND`; obecność sekundy wymusza timer 1 s. Dla ukrytych wskazówek generowany jest `null` zamiast widgetu i pomijana jest ich aktualizacja kąta. Pliki trafiają do `assets/active2round/hands/<zestaw>/`.

`assets/bg.png` jest rysowany jako syntetyczny widget `IMG` na całym ekranie. ZeppOS nie przeskalowuje zasobu `IMG` w runtime, więc przy `background.autoScale: true` plik jest wcześniej przycinany do środka i skalowany do 466 × 466 (tryb `Cover` z `image_fit.rs`, nieprzezroczysty RGBA) — bez białych pasków. Gdy `autoScale: false`, zachowany jest oryginalny rozmiar, a edytor rysuje podgląd tak samo. Oryginał zostaje w `assets/backgrounds/`, dzięki czemu przełączenie trybu nie wymaga magazynu aplikacji.

`watchface/index.js` wykrywa język w runtime przez `hmSetting.getLanguage()`: wartość `9` (`pl-PL`) daje polskie miesiące (`Sty…Gru`) i `przedp.`/`pop.`, każda inna wartość daje angielskie (`Jan…Dec`, `AM`/`PM`). Język nie jest zapisywany w `project.json` ani w `app.json` — ten sam ZIP działa w obu wersjach. Edytor ma osobne ustawienie języka **podglądu** (`watchfacePreviewLanguage` w `settings.json`: `auto`/`pl`/`en`), przekazywane przez `rw_format_time` i `rw_month_name`.

Uwaga: numer `deviceSource` jest stałą profilu w `rusty-core/src/model.rs`. Pole `defaultLanguage` jest wymagane przez schemat `app.json` v3 — bez niego `zeus build/install` odrzuca pakiet. Wersja API w manifeście to **1.0.0/1.0.1**, ponieważ tarcza korzysta z klasycznego API `hmUI` — przy wersji 3.x runtime nie udostępnia globalnego `hmUI` i tarcza pozostaje czarna. Nie ustawiamy też `runtime.type` (`2` = bytecode `.bin`, co blokuje ładowanie plików `.js`). Przed pierwszym wgraniem na fizyczny zegarek można zweryfikować eksport w oficjalnym symulatorze poleceniem `zeus install --path <rozpakowany-katalog> --target "Amazfit Active 2 NFC (Round)"` (taką samą nazwę celu używa przycisk „Przetestuj w emulatorze"; zwykła nazwa „Amazfit Active 2 (Round)" jest przez `zeus` odrzucana). Symulator wymaga pobranego obrazu `Active 2 v1.1.0` i otwartego okna zegarka. Własne czcionki są dodatkowo „podgrzewane" przez niewidoczny widget `TEXT` z pełnym zestawem znaków — bez tego ZeppOS potrafi pominąć glify, które pojawiają się dopiero po `setProperty(hmUI.prop.TEXT, ...)` (np. zmieniające się cyfry czasu). Podgląd w edytorze używa dokładnie tego samego pliku TTF dzięki kopiowaniu go do `FontCache` przy `.exe` i ładowaniu przez `ms-appx:///` (WinUI 3 nie czyta czcionek z `%APPDATA%`). Domyślna czcionka Inter jest objęta licencją SIL OFL 1.1 (`native/RustyWatch/Assets/Fonts/OFL-Inter.txt`) — plik można dołączyć do repozytorium projektu.
