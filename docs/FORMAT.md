# Format projektu Rusty Watch (`.rwf`) — v1

Projekt to katalog:

```
moja-tarcza/
├── project.json
└── assets/
    ├── icon.png
    ├── fonts/           # opcjonalnie, gdy widget używa własnej czcionki
    │   └── Inter-Regular.ttf
    ├── images/          # opcjonalnie, grafiki widgetów IMAGES dopasowane do ramki
    │   └── <widget-id>/
    │       └── wykres.png
    ├── hands/           # opcjonalnie, pliki zestawu wskazówek zegara analogowego
    │   └── Classic/...
    └── status/          # opcjonalnie, paczki ikon STATUS (4 pliki PNG każda)
        └── Classic/
            ├── alarm.png
            ├── dnd.png
            ├── bluetooth.png
            └── lock.png
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
| `assets[]` | pliki z `assets/`, role: `icon`, `font`, `image`, `hand`, `status_icon`, `weather_icon`, `other` |
| `customCode` | opcjonalny surowy JS wstrzykiwany do wygenerowanego `index.js` |
| `measurementSystem` | `watch` (domyślnie — jednostka z ustawień zegarka, czytana w JS przez `hmSetting.getMileageUnit()`), `metric` albo `imperial` — domyślny system jednostek dla widgetów dystansu, kopiowany z ustawień aplikacji przy otwarciu projektu |
| `firstDayOfWeek` | `monday` (domyślnie — poniedziałek = 1, wtorek = 2, …, niedziela = 7) albo `sunday` (niedziela = 1, …) — numer pola „Dzień tygodnia (liczba)" w podglądzie i w JS; kopiowany z ustawień aplikacji przy otwarciu projektu |

Widget:

| Pole | Opis |
|---|---|
| `id`, `name` | identyfikator i nazwa w drzewie warstw |
| `kind` | `text`, `image` (starszy, nie dodawalny), `images`, `time_text`, `date_text`, `weekday_number_text`, `week_number_text`, `day_of_year_text`, `lunar_date_text`, `festival_text`, `world_clock_text`, `step_text`, `distance_text`, `heart_rate_text`, `heart_rate_last_text`, `heart_rate_min_text`, `heart_rate_max_text`, `heart_rate_avg_text`, `stress_text`, `pai_text`, `pai_prepai_text`, `skin_temp_text`, `sleep_text`, `sleep_score_text`, `sleep_deep_text`, `sleep_light_text`, `sleep_rem_text`, `sleep_wake_text`, `sleep_start_text`, `sleep_end_text`, `calorie_text`, `spo2_text`, `spo2_hour_avg_text`, `battery_text`, `step_target_text`, `calorie_target_text`, `stand_target_text`, `fat_burning_target_text`, `stand_text`, `fat_burning_text`, `pai_weekly_text`, `sleep_target_text`, `weather_condition_text`, `weather_city_text`, `weather_current_text`, `weather_high_text`, `weather_low_text`, `sunrise_text`, `sunset_text`, `sun_current_text`, `weather_icon`, `status_icon`, `analog_time` |
| `x`, `y`, `w`, `h`, `z` | geometria w przestrzeni 466×466 |
| `text`, `textSize`, `color` | właściwości tekstu (`color` jako `0xAARRGGBB`). Dla `kind: "text"` (widget „Text (user defined)") `text` to dowolna treść wpisana w edytorze — bez `dataSource` i bez `timeFormat` trafia do wygenerowanego JS jako statyczny `text: "..."`. Dla `kind: "step_text"` pole `text` jest wartością początkową (`"0"`), a dla `distance_text` (`"0.00 km"` albo `"0.00 mi"`) — podgląd edytora nie ma czujników. Dla `kind: "weather_current_text"` wartość pochodzi z firmware'u (`hmUI.widget.TEXT_FONT` + `type: hmUI.data_type.WEATHER_CURRENT`) — pole `text` w projekcie jest tylko wartością zastęczą dla podglądu PC i **nie trafia do JS** |
| `font` | nazwa pliku z `%APPDATA%/RustyWatch/Fonts` (np. `Inter-Regular.ttf`); brak = czcionka systemowa zegarka. Kopia trafia do `assets/fonts/` i jest rejestrowana jako zasób `font` |
| `src` | nazwa pliku z `assets/` dla obrazka. Dla `kind: "images"` to `images/<widget-id>/<plik>` — PNG fizycznie dopasowany do `w × h` przez `image_fit` |
| `imageFit` | tylko `images`: `contain` (domyślnie), `cover` albo `stretch` — tryb skalowania użyty przy przygotowaniu PNG w projekcie i w eksporcie. Gdy `autoScale: true` (czerwona ramka), `rw_set_widget` wymusza `stretch`, więc rozciągnięcie ramki zawsze rozciąga obraz. `rw_set_widget_image` dopasowuje ramkę do naturalnego rozmiaru obrazka (z zachowaniem proporcji i ograniczeniem do 466×466) i zeruje `autoScale` |
| `handSet`, `hourSrc`, `minuteSrc`, `secondSrc`, `capSrc` | tylko `analog_time`: zestaw i ścieżki wskazówek względem `assets/` (np. `hands/Classic/hour.png`). `w`/`h` to rzeczywisty rozmiar wskazówek (początkowo rozmiar plików, użytkownik może je przeskalować zachowując kwadrat), a środek obrotu to `(w/2, h/2)`. Każde z pól `*_src` może być `null` — wtedy dana wskazówka jest ukryta i nie trafia do JS ani do ZIP |
| `statusIconSet`, `statusType` | tylko `status_icon`: nazwa paczki z `%APPDATA%/RustyWatch/Status` oraz `alarm`, `dnd`, `bluetooth` albo `lock`. `src` wskazuje plik `status/<paczka>/<status>.png`. Ikony są fizycznie skalowane do `w × h` (IMG nie skaluje w runtime) |
| `dataSource` | `time`, `date`, `week_day_number`, `week_number`, `day_of_year`, `lunar_date`, `festival`, `world_clock`, `battery`, `step`, `distance`, `heart_rate`, `heart_rate_last`, `heart_rate_min`, `heart_rate_max`, `heart_rate_avg`, `stress`, `pai`, `pai_prepai`, `skin_temp`, `sleep`, `sleep_score`, `sleep_deep`, `sleep_light`, `sleep_rem`, `sleep_wake`, `sleep_start`, `sleep_end`, `calorie`, `spo2`, `spo2_hour_avg`, `step_target`, `calorie_target`, `stand_target`, `fat_burning_target`, `stand`, `fat_burning`, `pai_weekly`, `sleep_target`, `weather_condition`, `weather_city`, `weather_current`, `weather_high`, `weather_low`, `sunrise`, `sunset`, `sun_current`, `weather_icon` — `lunar_date`/`festival` czytają `hmSensor.id.TIME`, `world_clock` czyta `hmSensor.id.WORLD_CLOCK`, `week_day_number` (pierwszy dzień wg `firstDayOfWeek`) / `week_number` / `day_of_year` są liczone wspólnym algorytmem w Rust i JS. `weather_current` nie ma czujnika — to zwykły edytowalny tekst |
| `align` | `left`, `center`, `right` (poziomo) |
| `vAlign` | `top`, `center` (domyślnie), `bottom` — mapowane na `align_v` w ZeppOS |
| `showLevel` | `normal`, `aod`, `edit`, `always` |
| `autoScale` | tylko edytor: czy rozmiar ramki skaluje `textSize` (domyślnie `false`) |
| `textStyle` | `ellipsis` (domyślnie), `none` (przewijanie), `wrap` — mapowane na `text_style` w ZeppOS |
| `visible` | `true` (domyślnie, także gdy pola nie ma w starszych projektach) lub `false` — ukryta warstwa zostaje w projekcie i na liście Warstwy, ale nie jest rysowana w podglądzie ani w wygenerowanym JS |
| `measurementSystem` | tylko `distance_text`: `watch`, `metric`, `imperial` albo brak (`null`) = użyj `measurementSystem` z projektu |
| `showUnit` | widgety zdrowotne i cele z jednostką: `true` (domyślnie) pokazuje `bpm`, `°C`, `kcal`, `%`, `kcal`, `h`, `min`; `false` zostawia samą liczbę. Starsze projekty bez pola zachowują jednostki |
| `alpha` | krycie widgetu `0`–`255` (domyślnie `255`); wartość poniżej 255 trafia do pola `alpha:` w generowanym JS i do podglądu. Starsze projekty bez pola są w pełni nieprzezroczyste |
| `weatherIconSet` | tylko `weather-icon`: nazwa paczki z `%APPDATA%\RustyWatch\Weather` (wbudowana `Classic` = białe ikony; można dodać własne paczki). Wszystkie 29 ikon fizycznie skalowanych do `w × h` przy zmianie ramki |
| `timeFormat` | wzorzec tekstu: dla `time_text` tokeny czasu `HH/H`, `hh/h`, `MM/M`, `SS/S`, `A/a`; dla `date_text` tokeny daty `D/d`, `DD/dd`, `M/m`, `MM/mm`, `MMM/mmm`, `MMMM/mmmm`, `DDD/ddd`, `DDDD/dddd`, `YY/YYYY`. Wielkość litery w tokenach nazw (`MMM`/`mmm`, `DDD`/`ddd`) steruje wielkością wyświetlanej nazwy; 5–6 liter daje WIELKIE LITERY. Parser wybierany jest na podstawie `kind`, więc schema projektu się nie zmienia. `\` wymusza literal. Domyślny wzorzec daty: `MMM D` |

Wzorzec daty jest współdzielony przez podgląd (`rw_format_date`) i generator JS (`date_format::js_expression_localized`) — nazwy miesięcy i dni tygodnia pochodzą z obiektu `L` wybieranego w runtime przez `hmSetting.getLanguage()` (`L.months`, `L.monthsFull`, `L.days`, `L.daysFull`), więc ten sam ZIP działa po polsku i angielsku. Indeks dnia tygodnia jest liczony od niedzieli (0) — tak samo jak w generowanym JS (algorytm Sakamoto bez API `Date`).

Licznik kroków używa klasycznego API `hmSensor.createSensor(hmSensor.id.STEP)`. Wygenerowany JS czyta `sensor.current`, odświeża widget tekstowy w zdarzeniu `hmSensor.event.CHANGE` (oraz przy okazji tyknięcia zegara) i zdejmuje nasłuchiwanie w `onDestroy`. Utworzenie czujnika jest izolowane próbą `try/catch` — brak sensora nie może położyć całej tarczy, więc tekst pozostaje `"0"`. Widget nie ma wzorca ani edytowalnej treści; ramka, czcionka, kolor i wyrównanie działają jak dla pozostałych widgetów tekstowych.

Dystans działa tak samo, ale czyta `hmSensor.id.DISTANCE` (wartość w **metrach**). Generator przelicza ją na kilometry (`/ 1000`, sufiks `km`) albo mile (`/ 1609.344`, sufiks `mi`) i formatuje do dwóch miejsc po przecinku. Domyślnym systemem jest **`watch`** — wygenerowany JS raz przy starcie tarczy wywołuje `hmSetting.getMileageUnit()` i w zależności od ustawienia zegarka wybiera km albo mile, więc ten sam ZIP działa w obu jednostkach. `metric` i `imperial` to jawne alternatywy. System wybiera `Widget::measurement_system`, a gdy jest `null` — `Project::measurement_system` (ustawienie aplikacji „Domyślny system jednostek"). Inspektor widgetu ma przełącznik „Własny system jednostek": po włączeniu wybierany system (`watch`, `metric` albo `imperial`) zapisuje się w widgetcie, po wyłączeniu wraca do projektu. `rw_set_measurement_system` aktualizuje domyślny system projektu i teksty początkowe widgetów bez nadpisania. W podglądzie edytora `watch` pokazuje neutralny placeholder `0.00 km`, bo PC nie zna ustawienia jednostki zegarka.

Widgety kategorii **Zdrowie** czytają klasyczne czujniki ZeppOS: `HEART.current` (bpm), `STRESS.current` (0–100), `PAI.dailypai`, `BODY_TEMP.current` (°C z jednym miejscem po przecinku), `SLEEP.getTotalTime()` (format `H:MM`), `CALORIE.current` (kcal), `SPO2.current` (%), `STAND.current` (godziny, sufiks `h`), `FAT_BURRING.current` (minuty, sufiks `min`) oraz `PAI.totalpai` (skumulowane PAI). Dwa ostatnie pola z sufiksami mają przełącznik jednostki; `STAND` i `FAT_BURRING` mogą nie być dostępne na Active 2 — wtedy zostaje tekst początkowy i karta katalogu zawiera ostrzeżenie. `PAI.dailypai` i `PAI.totalpai` współdzielą czujnik, ale mają osobne gettery i nasłuchy. Generator trzyma wspólny `healthTick` i po jednym leniwym getterze na czujnik (`try/catch`); odświeża widget przy zdarzeniu czujnika (`CURRENT` dla serca, `CHANGE` dla pozostałych) oraz przy tyknięciu zegara, a w `onDestroy` zdejmuje wszystkie nasłuchiwania. Brak czujnika na zegarku pozostawia tekst początkowy (`--`, `-- bpm`, `--:--` itd.) i nie przerywa działania tarczy. HRV i Gotowość nie są dostępne w katalogu — HRV nie ma sensora w watchface API, a Gotowość istnieje tylko jako `hmUI.data_type.READINESS`. Statystyki tętna (`heart-min`, `heart-max`, `heart-avg`) liczone są lokalnie z dodatnich wartości `HEART.today` (tablica `Array<number>`, maks. 1440 wpisów) i odświeżane raz na minutę; `heart-last` czyta `HEART.last`. `pai-prepai` sumuje `PAI.dailypai` i `prepai0…5` (dziś + 6 poprzednich dni) przez ten sam getter co dzienne PAI. `spo2-hour-avg` czyta `SPO2.hourAvgofDay[godzina]` z czujnika TIME (bez API `Date`). Klasyczne API nie ma natywnej strefy tętna. Widgety kategorii **SEN** czytają `SLEEP.getBasicInfo()` (wynik, `startTime`, `endTime`), `getSleepStageData()` + `getSleepStageModel()` (fazy DEEP/LIGHT/REM/WAKE) oraz `getTotalTime()`; czas snu aktualizuje się co ok. 30 min. Fazy (także `sleep-wake`) mają ustawienie `sleepStageMode` (`duration` = minuty, `percent` = udział % snu).

Pogoda wykorzystuje wielodniowy payload `forecastData.data[]` (indeks 0 = dziś). Pole `weatherDayOffset` (0…4) wybiera dzień dla nazwy/temperatur/wschodu-zachodu/ikony; brakowanie dnia daje `--` i nie zmienia ikony. `weather-city` ignoruje offset. `sun-current` liczy czas do najbliższego wschodu/zachodu z `tideData.data[0..1]` i czujnika TIME (w JS pomaga `_sunCurrentText()`, w podglądzie `sun_current_minutes()`); brak danych daje `--:--`. `weather-current` używa klasycznego (nieudokumentowanego) widgetu `hmUI.widget.TEXT_FONT` z `type: hmUI.data_type.WEATHER_CURRENT` i `unit_type: 0/1` (`showUnit` w inspektorze) — firmware podstawia i odświeża prawdziwą temperaturę, więc generator nie tworzy czujnika, timera ani nasłuchiwania i nie eksportuje statycznego `text:`. Edytowalne są tylko wyrównanie, rozmiar, czcionka, kolor, krycie i geometria. Klasyczny czujnik WEATHER nie ma pola `current`, więc podgląd PC pokazuje wartość zastępczą (`WeatherPreviewState::current`, domyślnie `22°`, nadpisywaną przez Custom Data `current`); klatki eksportu ignorują Custom Data. Podgląd w Rust generuje pięć deterministycznych dni z pojedynczego `WeatherSnapshot` i nakłada Custom Data na wybrany dzień; dla `sun-current` edytor przekazuje dodatkowo `nowMinute` z zegara podglądu.

Kategoria **Cele** (`goals`) ma cztery pola tekstowe czytające pole `.target` czujnika: `steps-target` (`STEP.target`), `calories-target` (`CALORIE.target`, sufiks `kcal`), `stand-target` (`STAND.target`, sufiks `h`) i `fat-burning-target` (`FAT_BURRING.target`, sufiks `min`). Sufiks zależy od `showUnit`. Generator trzyma wspólny `targetTick` i leniwe gettery z `try/catch`; odświeża wartość przy zdarzeniu `CHANGE` i przy tyknięciu zegara, a w `onDestroy` zdejmuje nasłuchiwania. **STAND i FAT_BURRING mogą nie być udostępnione na Amazfit Active 2** — wtedy tarcza zachowuje tekst początkowy z projektu (`10000`, `2000 kcal`, `12 h`, `45 min`), bez przerywania działania. Podgląd na PC używa tych samych stałych celów (PC nie ma czujników).

Kategoria **System** ma widget `battery` (`BATTERY.current`, zdarzenie `CHANGE`, tekst `NN%`) oraz cztery ikony `status-*`. Generowany JS używa `hmUI.widget.IMG_STATUS` z typami `hmUI.system_status.CLOCK` (alarm), `DISTURB` (DND), `DISCONNECT` (brak Bluetooth) i `LOCK`; system sam pokazuje/ukrywa ikonę — nie ma tu sensora, timera ani nasłuchiwania. Pakiety ikon są skanowane dynamicznie z `%APPDATA%/RustyWatch/Status/<id>/`; ważna paczka musi mieć cztery kwadratowe pliki o tej samej wielkości: `alarm.png`, `dnd.png`, `bluetooth.png`, `lock.png`. Wbudowany jest tylko `Classic` (domyślny, jak czarne tło) w `native/RustyWatch/Assets/Status/`. Wybrana paczka jest kopiowana do `<projekt>/assets/status/<id>/` i zaewidencjonowana jako zasób `status_icon`. Nowy widget `battery` jest wyśrodkowany na tarczy (`x=23,y=203,w=420,h=60`); nowe ikony statusu tworzą jedną wycentrowaną linię (`x=145/193/241/289,y=24,w=h=32`) i korzystają z tego samego modelu niebieski-przesuw / czerwony-skalowanie co inne grafiki. Ramki domyślne mieszczą wszystkie cztery ikony wewnątrz okręgu o promieniu 233. Każda zainstalowana paczka dostaje `preview.png`: dla wskazówek złożenie na godzinę 21:37:11, dla STATUS układ 2×2 (góra: alarm, DND; dół: Bluetooth, blokada). W 8 klatkach podglądu maska `ALARM=1/DND=2/BLUETOOTH=4/LOCK=8` daje ciąg `2,2,0,3,0,1,4,4` (2 puste klatki; klatka 4: alarm+DND+blokada, klatka 6: Bluetooth) - klatka 7 (`21:37:40`) ma wszystkie cztery ikony, a klatka 8 jest jej dokładną kopią i zasila statyczny `preview.png`. Każda z siedmiu oryginalnych klatek ma inną, prawdziwą datę: 2022-01-03 (poniedziałek) … 2022-07-31 (niedziela), miesiące styczeń…lipiec. `preview.png` jest oprawiony: tarcza jest pomniejszona w środku, wokół niej są czarne strefy bezpieczne i zewnętrzna warstwa logo z `logo/gear-outlayer.png`, a obrys tarczy jest ciemny i ledwo widoczny; ikona `assets/Preview.png` to ten sam obraz przeskalowany do `314 × 314`. Pierwsza klatka `preview.gif` to ten oprawiony obraz — surowa kopia 21:37 jest usuwana, więc GIF ma nadal 8 klatek (oprawiona, potem 7 surowych).

Kategoria **Postęp** (`progress`) ma dwa widgety: `arc_progress` i `bar_progress`. Ustawienia
mieszczą się w jednym polu `progress` widgetu. W inspektorze są: źródło, grubość linii, dwa kolory
(ten sam `ColorPicker` co dla tekstu), przełącznik podkładu, przełącznik zaokrągleń oraz kąty
start/end dla łuku. Łuk ma jeden suwak **SIZE** (jest kwadratem), pasek ma osobne pola szerokości
i wysokości. Grubość linii dotyczy tylko łuku; pasek zmienia wyłącznie szerokość i wysokość i może mieć nawet
`1 × 1` px (`FrameInput.min_size = 1`; `FILL_RECT` nie ma w ZeppOS praktycznego minimum, pozostałe
widgety zachowują 24 px w geście przeciągania). Nie ma już pola z własnym promieniem rogów:
zaokrąglenia są tylko włączone/wyłączone —
dla łuku daje to `corner_flag` `0/3`, dla paska promień `min(w,h)/2` albo `0` (kwadrat staje się
kołem). Wypełnienie jest zawsze ciągłe — zapisane w starszych projektach `segments` i `gap` są
ignorowane. Generator JS tworzy `hmUI.widget.ARC_PROGRESS` (podkład + wypełnienie) albo
`hmUI.widget.FILL_RECT` z `radius` dla paska; wartość 0…1 liczona jest z wybranego czujnika
(`_createProgSensor`, `_progValue_N`) i aktualizowana w `progressTick` przez
`setProperty(hmUI.prop.MORE, …)`. Wszystkie współrzędne i wymiary w tym wywołaniu są **literałami
z ramki** — użycie gołych identyfikatorów `w`/`h`/`y` powodowało `ReferenceError` i pasek pozostawał
pusty. Nowy łuk domyślnie ma kąty `0…360` (pełny okrąg); w podglądzie WinUI pełny okrąg rysowany
jest jako `EllipseGeometry`. Pasek domyślnie rośnie od lewej do prawej (lewo = pusto, prawo = pełno).
Podkład jest zawsze tworzony w edytorze — `ShowTrack` przełącza tylko jego widoczność. Gruby łuk
może wyjść poza niebieski obrys: edytor daje jego `Path` powiększone wymiary i ujemny margines,
a renderer klatek używa `arc_draw_margin()`. Środek linii pozostaje stały, więc grubość rośnie
symetrycznie. Podgląd na żywo czyta odpowiednie pole Custom Data — bateria domyślnie `73%` — ale
generowane klatki `preview.png/gif` pozostają deterministyczne i nie biorą Custom Data.

## Mapowanie przy eksporcie

`.rwf` → pliki w `.zip`:

| Źródło | Wynik |
|---|---|
| `metadata`, `device` | `app.json` (configVersion v2 (płaski manifest, bez `targets`), `defaultLanguage: "en-US"`, `runtime.apiVersion` 1.0.0/1.0.1, bez `runtime.type`, 8 Active 2 `deviceSource` values (8913152/8913153/8913155/8913159/10092800/10092801/10092803/10092807), `designWidth: 466`, `editable: 0`, `lockscreen: 0`, `hightCost: 0`, `packageInfo.zpm: 2.8.2`, `app.icon = Preview.png`) |
| stały szablon | `app.js` |
| `widgets[]` (tylko `visible: true`), `customCode` | `watchface/index.js` (klasyczne API tarczy: `hmUI.createWidget`, `hmSensor`, `timer`, `WatchFace({...})`) — `x/y/w/h`, `text_size`, `text_style` i `font` trafiają tam bezpośrednio z projektu; ukryte warstwy są pomijane, indeksy `refs` pozostają spójne |
| `assets/*` | `assets/*` pƉasko i z unikalnymi, zahaszowanymi nazwami (`hand_<hash>.png`, `fonts/font_<hash>.ttf`, `status_<hash>.png`, `img_<hash>.png`, `weather_<hash>.png`; `icon.png` bez zmiany, tła brak); `export_paths.rs` zapewnia, Ɖe `src:` w JS wskazuje dokƉadnie te same pliki. `font:` jest generowane jako `fonts/font_<hash>.ttf` — tak jak w dzialajĄcej tarczy referencyjnej. `Preview.png` 314×314 RGBA z przezroczystoƜciƘ poza okrƘg ekranu. Bez `active2round/`) |

Eksport z edytora najpierw generuje oprawiony `preview.png` i 8-klatkowy, zapętlony `preview.gif` (Rust, `crates/rusty-core/src/preview.rs`), a dopiero potem otwiera okno (ContentDialog nad edytorem). Okno pokazuje ten GIF przycięty do okręgu tarczy nad danymi projektu (nazwa, autor, wersja, App ID) i przycisk Eksportuj ZIP. Oba pliki zostają w folderze projektu i nie trafiają do paczki. Paczka trafia bezpośrednio do folderu projektu (obok `project.json`), bez okna wyboru ścieżki. Nazwa pliku powstaje z `metadata.name` — spacje i znaki niedozwolone stają się kropkami (`Moja tarcza` → `Moja.tarcza.zip`). Główny podgląd w edytorze pozostaje nietknięty.

Poziomy wyświetlania w modelu mapują się na stałe ZeppOS (`ONLY_NORMAL`, `ONAL_AOD`, `ONLY_EDIT`), ale **eksport zawsze zapisuje `ONLY_NORMAL`** — `ONAL_AOD` powodował na realnym zegarku czarną nakładkę; AOD obsługuje `WIDGET_DELEGATE`.

Widget `analog_time` generuje cztery widgety `IMG` (`hour`, `minute`, `second`, `cap`) z tymi samymi `x/y/w/h`; każdy ma ramkę całego ekranu (`x/y=0`, `w/h=466`), obraz pozycjonowany przez `pos_x/pos_y`, pivot `center_x/center_y` w środku tarczy i `auto_scale: false` i jest obracany przez `setProperty(hmUI.prop.MORE, { angle })` z wspólnego `tick`. Godzina: `(h % 12) * 30 + m * 0.5`, minuta: `m * 6 + s * 0.1`, sekunda: `s * 6`. Gdy zestaw nie ma sekundy (albo użytkownik ją ukrył), tarcza używa nasłuchiwacza `MINUTEEND`; obecność sekundy wymusza timer 1 s. Dla ukrytych wskazówek generowany jest `null` zamiast widgetu i pomijana jest ich aktualizacja kąta. Pliki trafiają do `assets/hands/<zestaw>/`.

Tła nie ma: generowany JS rysuje na początku `hmUI.widget.FILL_RECT(0,0,466,466,0xFF000000)` — w pełni czarne (celowa zmiana względem referencji `0xFF05070D`). Nie ma zasobu `bg.png`, katalogu `backgrounds/`, pola `background` w `project.json` ani warstwy tła na liście warstw.

`watchface/index.js` wykrywa język w runtime przez `DeviceRuntimeCore.HmUtils.getLanguage()` (z awaryjnym `hmSetting.getLanguage()`), wewnĄtrz bloku `try`/IIFE: wartość `9` (`pl-PL`) daje polskie miesiące (`Sty…Gru`) i `przedp.`/`pop.`, każda inna wartość daje angielskie (`Jan…Dec`, `AM`/`PM`). Język nie jest zapisywany w `project.json` ani w `app.json` — ten sam ZIP działa w obu wersjach. Edytor ma osobne ustawienie języka **podglądu** (`watchfacePreviewLanguage` w `settings.json`: `pl`/`en`, domyślnie `en`, bez `auto` od języka Windows), przekazywane przez `rw_format_time` i `rw_month_name` oraz do generowania klatek `preview.png`/`preview.gif`.

Uwaga: numer `deviceSource` jest stałą profilu w `rusty-core/src/model.rs`. Pole `defaultLanguage` jest wymagane przez schemat `app.json` - bez niego `zeus build/install` odrzuca pakiet. Pakiet dla zegarka jest w wersji v2 i płaski: blok `targets` (format roboczy `zeus`) powoduje na zegarku błąd "Failed to unzip the package"; dodaje go dopiero `prepare_emulator_workspace` do kopii dla symulatora. Wersja API w manifeście to **1.0.0/1.0.1**, ponieważ tarcza korzysta z klasycznego API `hmUI` — przy wersji 3.x runtime nie udostępnia globalnego `hmUI` i tarcza pozostaje czarna. Nie ustawiamy też `runtime.type` (`2` = bytecode `.bin`, co blokuje ładowanie plików `.js`). Przed pierwszym wgraniem na fizyczny zegarek można zweryfikować eksport w oficjalnym symulatorze poleceniem `zeus install --path <rozpakowany-katalog> --target "Amazfit Active 2 NFC (Round)"` (taką samą nazwę celu używa przycisk „Przetestuj w emulatorze"; zwykła nazwa „Amazfit Active 2 (Round)" jest przez `zeus` odrzucana). Symulator wymaga pobranego obrazu `Active 2 v1.1.0` i otwartego okna zegarka. Nie ma widgetów podgrzewających czcionki — initial text każdego widgetu zawiera pełny zestaw znaków, więc glify nie muszą być ładowane dopiero po `setProperty`. Podgląd w edytorze używa dokładnie tego samego pliku TTF dzięki kopiowaniu go do `FontCache` przy `.exe` i ładowaniu przez `ms-appx:///` (WinUI 3 nie czyta czcionek z `%APPDATA%`). Domyślna czcionka Inter jest objęta licencją SIL OFL 1.1 (`native/RustyWatch/Assets/Fonts/OFL-Inter.txt`) — plik można dołączyć do repozytorium projektu.

## Pogoda (kategoria `weather`)

Widgety czytają `hmSensor.id.WEATHER.getForecastWeather()`: `weather-condition` (nazwa z indeksu 0..28 w języku tarczy), `weather-city` (`cityName`, mapowanie Warszawa/Warszawa zależnie od języka), `weather-high`/`weather-low` (`forecastData.data[0].high/low`), `sunrise`/`sunset` (`tideData.data[0]`). `weather-icon` wybiera jeden z 29 plików przez `setProperty(hmUI.prop.SRC, ...)`. Wszystkie odczyty są w `try/catch` i odświeżane w `tick()` oraz `resume_call`. Cel snu (`sleep-target`) czyta `hmSetting.getSleepTarget()` w minutach.

Wbudowane ikony: `erikflowers/weather-icons` (SIL OFL 1.1) — patrz `native/RustyWatch/Assets/Weather/OFL-weather-icons.txt`.

## Podgląd pogody na żywo

`rw_format_weather_field` domyślnie zwraca pochmurną Warszawę (indeks 0, 22°/16°, wschód 05:13, zachód 20:40); pola Custom Data (`city`, `index`, `high`, `low`, `sunrise`, `sunset`) nadpisują tylko wysłane wartości. `sunrise`/`sunset` to pojedyncze ciągi `HH:MM` parsowane w Rust; `index` wybierany jest z listy nazw warunków (`rw_weather_conditions`, kody 0…28). Starsze klucze `sunriseHour`/`sunriseMinute`/`sunsetHour`/`sunsetMinute` są jeszcze odczytywane dla kompatybilności. Dane te nie trafiają do eksportu.

Skalowanie zasobów podczas edycji jest odroczone: `rw_set_widget` z `deferAssets: true` zapisuje tylko geometrię, a `rw_rescale_widget_assets` wykonuje się w wątku tła. Widoczna ikona jest dekodowana raz i skalowana przez WinUI (`Stretch.Fill`).

Ikona pogody nie ma tekstu — `weather_text("weather-icon")` zwraca pusty string, a edytor nie dodaje TextBlocku do ramki ikony. Usuwa to artefakt `--` pojawiający się wcześniej przy przeskalowaniu.

Przełączanie paczki ikon jest natychmiastowe: edytor dekoduje tylko aktualnie widoczną ikonę z `%APPDATA%\RustyWatch\Weather` (`RefreshWeatherIconPreview`), a kopiowanie i skalowanie wszystkich 29 plików do projektu odbywa się w tle (`rw_set_weather_pack`). Szybkie zmiany paczki anulują poprzednie zadanie (token + licznik wersji); wyniki z wątka tła trafiają do UI przez `DispatcherQueue`. Eksport czeka na zakończenie zadania. `attach_to_project` skaluje wszystkie ikony jednokrotnie — nie wołamy już dodatkowo `scale_project_icons` w `rw_set_weather_pack`.

## Live preview i dane predefiniowane

Edytor domyślnie pokazuje dane predefiniowane. Czas i data to stały moment
`DEFAULT_PREVIEW_DATETIME` = **31.07.2022 21:37:40** (ta sama klatka co `preview.png`/GIF),
udostępniany przez `rw_preview_default_datetime`; pola z okna Custom Data nadpisują
wybrane składniki. Włączony przełącznik **Live preview** (`liveTimeDateEnabled` w
`settings.json`) podmienia **wyłącznie** TIME, DATE i pola pochodne (numer dnia
tygodnia, tydzień roku, dzień roku, world clock) na rzeczywisty czas systemowy.
Pogoda, bateria, kroki, tętno, cele, zdrowie i powiadomienia nadal czytają dane
predefiniowane. Po wyłączeniu przełącznika czas i data wracają do stałej daty.
Stary klucz `customPreviewDataEnabled` nie jest już używany — nie migrujemy go.

## Ikony pogody

Wbudowana jest tylko paczka **Classic**. Mastery SVG (OFL 1.1) leżą w
`native/RustyWatch/Assets/Weather/Source/w00.svg` … `w28.svg`, a skrypt
`scripts/generate_weather_icons.py` renderuje je do 512×512 PNG z glifem
zajmującym ~80% kwadratu i ~10% bezpiecznego marginesu — dzięki temu ikona
nie jest przycinana przy żadnym rozmiarze ramki. `weather::resize_png`
skaluje w przestrzeni premultiplied alpha (bez jasnych krawędzi). ZeppOS nie
ładuje SVG w tarczy, więc na zegarek trafiają PNG. Ewentualny stary folder
`Reversed` w `%APPDATA%\RustyWatch\Weather` pozostaje na dysku, ale nie jest
już listowany ani wybieralny.
