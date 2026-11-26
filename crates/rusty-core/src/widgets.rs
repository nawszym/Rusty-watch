use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::image_fit::FitMode;
use crate::model::{
    Align, DataSource, MeasurementSystem, ProgressCorner, ProgressOrientation, ProgressSource,
    Project, ShowLevel, SleepStageMode, StatusType, TextStyle, VAlign, Widget, WidgetKind,
    SCREEN_SIZE,
};
use crate::{layers, time_format};

/// Kategoria w katalogu widgetów (zarówno w flyoucie „Dodaj widget", jak i na zakładce Widgets).
pub const CATEGORY_TIME: &str = "time";
pub const CATEGORY_TEXT: &str = "text";
/// Statyczne grafiki z magazynu `%APPDATA%\RustyWatch\Images`.
pub const CATEGORY_IMAGES: &str = "images";
/// Data, dane daty z czujnika TIME oraz zegar światowy.
pub const CATEGORY_DATE: &str = "date";
/// Licznik kroków i dystans — dane z czujników aktywności.
pub const CATEGORY_DISTANCE: &str = "distance";
/// Dzienne cele (kroki, kalorie, stanie, spalanie tłuszczu) z pola `.target` czujników.
pub const CATEGORY_GOALS: &str = "goals";
/// Puls, stres, PAI, temperatura, kalorie i SpO₂.
pub const CATEGORY_HEALTH: &str = "health";
/// Sen: czas, wynik, fazy oraz godziny rozpoczęcia i zakończenia.
pub const CATEGORY_SLEEP: &str = "sleep";
/// Stan urządzenia: bateria oraz ikony statusu systemu.
pub const CATEGORY_SYSTEM: &str = "system";
/// Pogoda: nazwa, miasto, temperatury, wschód/zachód i ikona.
pub const CATEGORY_WEATHER: &str = "weather";
/// Postęp: łuk i pasek zasilane czujnikami albo wartością stałą.
pub const CATEGORY_PROGRESS: &str = "progress";
/// Niewidoczne strefy klikalne (skróty do danych aplikacji).
pub const CATEGORY_SHORTCUT: &str = "shortcut";


/// Pozycja katalogu widgetów czytana przez FFI. `category` pozwala UI grupować
/// wpisy w zwijalne sekcje — lista jest wspólna dla flyoutu i zakładki Widgets.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedWidget {
    pub kind: &'static str,
    pub category: &'static str,
    /// Czy inspektor ma pokazywać pole wzorca czasu/daty. Flaga jest wysyłana
    /// przez FFI, żeby C# nie musiało trzymać własnej listy typów.
    pub has_pattern: bool,
}



impl SupportedWidget {
    fn new(kind: &'static str, category: &'static str) -> Self {
        Self {
            kind,
            category,
            has_pattern: matches!(kind, "time" | "date"),
        }
    }
}



/// Typy widgetów, które edytor potrafi dodać. UI czyta tę listę przez FFI.
pub fn supported_widgets() -> Vec<SupportedWidget> {
    vec![
        SupportedWidget::new("time", CATEGORY_TIME),
        SupportedWidget::new("analog-time", CATEGORY_TIME),
        SupportedWidget::new("date", CATEGORY_DATE),
        SupportedWidget::new("weekday-number", CATEGORY_DATE),
        SupportedWidget::new("week-number", CATEGORY_DATE),
        SupportedWidget::new("day-of-year", CATEGORY_DATE),
        SupportedWidget::new("lunar-date", CATEGORY_DATE),
        SupportedWidget::new("festival", CATEGORY_DATE),
        SupportedWidget::new("world-clock", CATEGORY_DATE),
        SupportedWidget::new("text", CATEGORY_TEXT),
        SupportedWidget::new("images", CATEGORY_IMAGES),
        SupportedWidget::new("steps", CATEGORY_DISTANCE),
        SupportedWidget::new("distance", CATEGORY_DISTANCE),
        SupportedWidget::new("steps-target", CATEGORY_GOALS),
        SupportedWidget::new("calories-target", CATEGORY_GOALS),
        SupportedWidget::new("stand-target", CATEGORY_GOALS),
        SupportedWidget::new("fat-burning-target", CATEGORY_GOALS),
        SupportedWidget::new("heart", CATEGORY_HEALTH),
        SupportedWidget::new("heart-last", CATEGORY_HEALTH),
        SupportedWidget::new("heart-min", CATEGORY_HEALTH),
        SupportedWidget::new("heart-max", CATEGORY_HEALTH),
        SupportedWidget::new("heart-avg", CATEGORY_HEALTH),
        SupportedWidget::new("stress", CATEGORY_HEALTH),
        SupportedWidget::new("pai", CATEGORY_HEALTH),
        SupportedWidget::new("skin-temp", CATEGORY_HEALTH),
        SupportedWidget::new("calories", CATEGORY_HEALTH),
        SupportedWidget::new("spo2", CATEGORY_HEALTH),
        SupportedWidget::new("spo2-hour-avg", CATEGORY_HEALTH),
        SupportedWidget::new("stand", CATEGORY_HEALTH),
        SupportedWidget::new("fat-burning", CATEGORY_HEALTH),
        SupportedWidget::new("pai-weekly", CATEGORY_HEALTH),
        SupportedWidget::new("pai-prepai", CATEGORY_HEALTH),
        SupportedWidget::new("sleep", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-score", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-deep", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-light", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-rem", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-wake", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-start", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-end", CATEGORY_SLEEP),
        SupportedWidget::new("sleep-target", CATEGORY_SLEEP),
        SupportedWidget::new("battery", CATEGORY_SYSTEM),
        SupportedWidget::new("status-alarm", CATEGORY_SYSTEM),
        SupportedWidget::new("status-dnd", CATEGORY_SYSTEM),
        SupportedWidget::new("status-bluetooth", CATEGORY_SYSTEM),
        SupportedWidget::new("status-lock", CATEGORY_SYSTEM),
        SupportedWidget::new("weather-condition", CATEGORY_WEATHER),
        SupportedWidget::new("weather-city", CATEGORY_WEATHER),
        SupportedWidget::new("weather-current", CATEGORY_WEATHER),
        SupportedWidget::new("weather-high", CATEGORY_WEATHER),
        SupportedWidget::new("weather-low", CATEGORY_WEATHER),
        SupportedWidget::new("sunrise", CATEGORY_WEATHER),
        SupportedWidget::new("sunset", CATEGORY_WEATHER),
        SupportedWidget::new("sun-current", CATEGORY_WEATHER),
        SupportedWidget::new("weather-icon", CATEGORY_WEATHER),
        SupportedWidget::new("arc-progress", CATEGORY_PROGRESS),
        SupportedWidget::new("bar-progress", CATEGORY_PROGRESS),
        SupportedWidget::new("shortcut", CATEGORY_SHORTCUT),
    ]
}

/// Specyfikacja widgetu zdrowotnego: (typ, źródło, prefiks id, nazwa, tekst początkowy, rozmiar).
fn health_spec(
    kind: &str,
) -> Option<(WidgetKind, DataSource, &'static str, &'static str, &'static str, u32)> {
    Some(match kind {
        "heart" => (
            WidgetKind::HeartRateText,
            DataSource::HeartRate,
            "heart",
            "Heart",
            "-- bpm",
            40,
        ),
        "heart-last" => (
            WidgetKind::HeartRateLastText,
            DataSource::HeartRateLast,
            "heart-last",
            "Heart last",
            "-- bpm",
            40,
        ),
        "heart-min" => (
            WidgetKind::HeartRateMinText,
            DataSource::HeartRateMin,
            "heart-min",
            "Heart min",
            "-- bpm",
            40,
        ),
        "heart-max" => (
            WidgetKind::HeartRateMaxText,
            DataSource::HeartRateMax,
            "heart-max",
            "Heart max",
            "-- bpm",
            40,
        ),
        "heart-avg" => (
            WidgetKind::HeartRateAvgText,
            DataSource::HeartRateAvg,
            "heart-avg",
            "Heart avg",
            "-- bpm",
            40,
        ),
        "stress" => (
            WidgetKind::StressText,
            DataSource::Stress,
            "stress",
            "Stress",
            "--",
            40,
        ),
        "pai" => (
            WidgetKind::PaiText,
            DataSource::Pai,
            "pai",
            "PAI",
            "--",
            40,
        ),
        "skin-temp" => (
            WidgetKind::SkinTempText,
            DataSource::SkinTemp,
            "skin-temp",
            "Skin temp",
            "-- °C",
            40,
        ),
        "calories" => (
            WidgetKind::CalorieText,
            DataSource::Calorie,
            "calories",
            "Calories",
            "-- kcal",
            40,
        ),
        "spo2" => (
            WidgetKind::Spo2Text,
            DataSource::Spo2,
            "spo2",
            "SpO2",
            "--%",
            40,
        ),
        "spo2-hour-avg" => (
            WidgetKind::Spo2HourAvgText,
            DataSource::Spo2HourAvg,
            "spo2-hour-avg",
            "SpO2 hour avg",
            "--%",
            40,
        ),
        "stand" => (
            WidgetKind::StandText,
            DataSource::Stand,
            "stand",
            "Stand hours",
            "-- h",
            40,
        ),
        "fat-burning" => (
            WidgetKind::FatBurningText,
            DataSource::FatBurning,
            "fat-burning",
            "Fat burning",
            "-- min",
            40,
        ),
        "pai-weekly" => (
            WidgetKind::PaiWeeklyText,
            DataSource::PaiWeekly,
            "pai-weekly",
            "PAI weekly",
            "--",
            40,
        ),
        "pai-prepai" => (
            WidgetKind::PaiPrepaiText,
            DataSource::PaiPrepai,
            "pai-prepai",
            "PAI 7 days",
            "--",
            40,
        ),
        _ => return None,
    })
}

/// Specyfikacja widgetu snu: (typ, źródło, prefiks id, nazwa, tekst początkowy, rozmiar).
/// Dane pochodzą z `hmSensor.id.SLEEP` — `getBasicInfo()` i `getSleepStageData()`.
fn sleep_spec(
    kind: &str,
) -> Option<(WidgetKind, DataSource, &'static str, &'static str, &'static str, u32)> {
    Some(match kind {
        "sleep" => (
            WidgetKind::SleepText,
            DataSource::Sleep,
            "sleep",
            "Sleep",
            "--:--",
            40,
        ),
        "sleep-score" => (
            WidgetKind::SleepScoreText,
            DataSource::SleepScore,
            "sleep-score",
            "Sleep score",
            "--",
            40,
        ),
        "sleep-deep" => (
            WidgetKind::SleepDeepText,
            DataSource::SleepDeep,
            "sleep-deep",
            "Deep sleep",
            "-- min",
            40,
        ),
        "sleep-light" => (
            WidgetKind::SleepLightText,
            DataSource::SleepLight,
            "sleep-light",
            "Light sleep",
            "-- min",
            40,
        ),
        "sleep-rem" => (
            WidgetKind::SleepRemText,
            DataSource::SleepRem,
            "sleep-rem",
            "REM sleep",
            "-- min",
            40,
        ),
        "sleep-wake" => (
            WidgetKind::SleepWakeText,
            DataSource::SleepWake,
            "sleep-wake",
            "Wake time",
            "-- min",
            40,
        ),
        "sleep-start" => (
            WidgetKind::SleepStartText,
            DataSource::SleepStart,
            "sleep-start",
            "Sleep start",
            "--:--",
            40,
        ),
        "sleep-end" => (
            WidgetKind::SleepEndText,
            DataSource::SleepEnd,
            "sleep-end",
            "Sleep end",
            "--:--",
            40,
        ),
        _ => return None,
    })
}

/// Specyfikacja pola celu: (typ, źródło, prefiks id, nazwa, tekst początkowy, rozmiar).
/// Wartość na zegarku pochodzi z pola `.target` czujnika — PC pokazuje tylko
/// sensowny placeholder. `STAND` i `FAT_BURRING` mogą być nieobecne na Active 2,
/// więc generator JS zabezpiecza ich odczyt w `try/catch`.
fn target_spec(
    kind: &str,
) -> Option<(WidgetKind, DataSource, &'static str, &'static str, &'static str, u32)> {
    Some(match kind {
        "steps-target" => (
            WidgetKind::StepTargetText,
            DataSource::StepTarget,
            "steps-target",
            "Step target",
            "10000",
            40,
        ),
        "calories-target" => (
            WidgetKind::CalorieTargetText,
            DataSource::CalorieTarget,
            "calories-target",
            "Calorie target",
            "2000 kcal",
            40,
        ),
        "stand-target" => (
            WidgetKind::StandTargetText,
            DataSource::StandTarget,
            "stand-target",
            "Stand target",
            "12 h",
            40,
        ),
        "fat-burning-target" => (
            WidgetKind::FatBurningTargetText,
            DataSource::FatBurningTarget,
            "fat-burning-target",
            "Fat burning target",
            "45 min",
            40,
        ),
        // `hmSetting.getSleepTarget()` zwraca minuty, nie pole `.target` czujnika.
        "sleep-target" => (
            WidgetKind::SleepTargetText,
            DataSource::SleepTarget,
            "sleep-target",
            "Sleep target",
            "-- h",
            40,
        ),
        _ => return None,
    })
}

/// Specyfikacja pola pogody: (typ, źródło, prefiks id, nazwa, tekst początkowy, rozmiar).
fn weather_spec(
    kind: &str,
) -> Option<(WidgetKind, DataSource, &'static str, &'static str, &'static str, u32)> {
    Some(match kind {
        "weather-condition" => (
            WidgetKind::WeatherConditionText,
            DataSource::WeatherCondition,
            "weather-condition",
            "Weather condition",
            "--",
            40,
        ),
        "weather-city" => (
            WidgetKind::WeatherCityText,
            DataSource::WeatherCity,
            "weather-city",
            "City",
            "--",
            40,
        ),
        // Temperatura bieżąca jest zwykłym, edytowalnym tekstem użytkownika.
        // Klasyczny czujnik WEATHER nie ma pola "current", więc nie udajemy
        // automatycznego odczytu — wartość wpisuje użytkownik w inspektorze.
        "weather-current" => (
            WidgetKind::WeatherCurrentText,
            DataSource::WeatherCurrent,
            "weather-current",
            "Current temperature",
            "22°",
            40,
        ),
        "weather-high" => (
            WidgetKind::WeatherHighText,
            DataSource::WeatherHigh,
            "weather-high",
            "High temperature",
            "--°",
            40,
        ),
        "weather-low" => (
            WidgetKind::WeatherLowText,
            DataSource::WeatherLow,
            "weather-low",
            "Low temperature",
            "--°",
            40,
        ),
        "sunrise" => (
            WidgetKind::SunriseText,
            DataSource::Sunrise,
            "sunrise",
            "Sunrise",
            "--:--",
            40,
        ),
        "sunset" => (
            WidgetKind::SunsetText,
            DataSource::Sunset,
            "sunset",
            "Sunset",
            "--:--",
            40,
        ),
        "sun-current" => (
            WidgetKind::SunCurrentText,
            DataSource::SunCurrent,
            "sun-current",
            "Time to sunrise/sunset",
            "--:--",
            40,
        ),
        "weather-icon" => (
            WidgetKind::WeatherIcon,
            DataSource::WeatherIcon,
            "weather-icon",
            "Weather icon",
            "",
            crate::weather::DEFAULT_ICON_SIZE as u32,
        ),
        _ => return None,
    })
}

pub const MIN_FRAME_SIZE: i32 = 24;

fn default_min_frame_size() -> i32 {
    MIN_FRAME_SIZE
}

fn default_max_frame_size() -> i32 {
    SCREEN_SIZE as i32
}
pub const MIN_TEXT_SIZE: u32 = 8;
pub const MAX_TEXT_SIZE: u32 = 160;
/// Rozmiar kratki przyciągania w pikselach ekranu (466 × 466).
pub const SNAP_GRID: i32 = 16;
/// Maksymalna odległość od linii wyrównania, przy której ramka „przywiera" do niej.
pub const ALIGN_THRESHOLD: i32 = 5;

/// Zaokrągla wartość do najbliższej linii siatki (przy remisie w górę).
fn snap_to_grid(value: i32) -> i32 {
    ((value + SNAP_GRID / 2) / SNAP_GRID) * SNAP_GRID
}

/// Szuka najbliższej linii wyrównania dla lewej krawędzi, środka i prawej krawędzi.
/// Zwraca (przesunięcie, pozycję linii) — linia trafia do podglądu jako czerwona prowadnica.
fn find_alignment(edges: [i32; 3], candidates: &[i32]) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32)> = None;
    for &line in candidates {
        for &edge in &edges {
            let delta = line - edge;
            if delta.abs() <= ALIGN_THRESHOLD
                && best.map_or(true, |(current, _)| delta.abs() < current.abs())
            {
                best = Some((delta, line));
            }
        }
    }
    best
}

/// Wybiera prowadnicę na pozycji sprzed przyciągnięcia do siatki. Siatka ma 16 px,
/// a próg wyrównania tylko 5 px — po wstępnym przyciągnięciu środek ramki byłby
/// zbyt daleko od środka tarczy/widgetu i czerwona prowadnica nigdy by się nie
/// pojawiła przy włączonym „Przyciągaj do siatki". Wyrównanie ma więc pierwszeństwo.
fn resolve_axis(
    value: i32,
    edges: [i32; 3],
    candidates: &[i32],
    snap: bool,
    max: i32,
) -> (i32, Option<i32>) {
    if let Some((delta, line)) = find_alignment(edges, candidates) {
        let aligned = (value + delta).clamp(0, max);
        // Po clampie wybrana krawędź/środek musi faktycznie leżeć na prowadnicy —
        // przy samym brzegu ekranu przesunięcie może zostać obcięte.
        let center_offset = edges[1] - value;
        let right_offset = edges[2] - value;
        if [aligned, aligned + center_offset, aligned + right_offset].contains(&line) {
            return (aligned, Some(line));
        }
    }
    if snap {
        (snap_to_grid(value).clamp(0, max), None)
    } else {
        (value, None)
    }
}

/// Wejście do czystej funkcji geometrii ramki — pozycja na początku gestu,
/// przesunięcie kursora i uchwyt, który jest przeciągany.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameInput {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub text_size: u32,
    /// `move`, `nw`, `ne`, `sw` albo `se`.
    pub handle: String,
    pub dx: i32,
    pub dy: i32,
    pub auto_scale: bool,
    /// Czy przeciąganie ma przyciągać krawędzie do kratki (`SNAP_GRID`).
    #[serde(default)]
    pub snap: bool,
    /// Kandydaci do wyrównania w osi X (środki/krawędzie płótna i pozostałych widgetów).
    #[serde(default)]
    pub align_x: Vec<i32>,
    /// Kandydaci do wyrównania w osi Y.
    #[serde(default)]
    pub align_y: Vec<i32>,
    /// Wskazówki zegara są kwadratowe — zwężanie jednej osi zniekształciłoby je.
    /// Gdy `true`, rozciąganie zachowuje `w == h`.
    #[serde(default)]
    pub square: bool,
    /// Minimalny wymiar ramki. Pasek postępu pozwala na 1 px — `FILL_RECT`
    /// nie ma w ZeppOS praktycznego minimum (poza w/h > 0).
    #[serde(default = "default_min_frame_size")]
    pub min_size: i32,
    /// Maksymalny wymiar ramki (domyślnie cały ekran). Ikona pogody dostaje 256 px,
    /// żeby przeciąganie rogu i suwak SIZE miały dokładnie to samo ograniczenie.
    #[serde(default = "default_max_frame_size")]
    pub max_size: i32,
}

/// Geometria zwracana podglądowi. `text_size` to ta sama wartość, która trafia
/// do `project.json` i do wygenerowanego JS — dzięki temu podgląd = eksport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameGeometry {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub text_size: u32,
    /// Pozycja pionowej prowadnicy wyrównania (`null`, gdy nie wyrównano).
    pub guide_x: Option<i32>,
    /// Pozycja poziomej prowadnicy wyrównania (`null`, gdy nie wyrównano).
    pub guide_y: Option<i32>,
}

/// Oblicza nową ramkę podczas przeciągania. Nie zapisuje projektu — edytor
/// wywołuje to na każdym ruchu kursora, a zapis następuje dopiero po zwolnieniu.
pub fn frame_geometry(input: &FrameInput) -> FrameGeometry {
    let screen = SCREEN_SIZE as i32;
    let min_size = input.min_size.max(1);
    let max_size = input.max_size.clamp(min_size, screen);
    let mut x = input.x.clamp(0, screen - min_size);
    let mut y = input.y.clamp(0, screen - min_size);
    let mut w = input.w.clamp(min_size, (screen - x).min(max_size));
    let mut h = input.h.clamp(min_size, (screen - y).min(max_size));
    x = x.min(screen - w);
    y = y.min(screen - h);
    let start_w = w;
    let mut guide_x = None;
    let mut guide_y = None;

    if input.handle == "move" || input.handle.is_empty() {
        x = (x + input.dx).clamp(0, screen - w);
        y = (y + input.dy).clamp(0, screen - h);
        // Najpierw środek/krawędzie wobec środka tarczy i innych widgetów, dopiero
        // potem siatka — inaczej 16-px siatka zabija 5-px próg wyrównania.
        let (aligned_x, line_x) =
            resolve_axis(x, [x, x + w / 2, x + w], &input.align_x, input.snap, screen - w);
        x = aligned_x;
        guide_x = line_x;
        let (aligned_y, line_y) =
            resolve_axis(y, [y, y + h / 2, y + h], &input.align_y, input.snap, screen - h);
        y = aligned_y;
        guide_y = line_y;
    } else if input.square {
        // Analog: wskazówki są kwadratowymi PNG — skalujemy je jednym wymiarem,
        // wybierając oś, którą użytkownik pociągnął dalej. Zasoby `IMG` na tarczy
        // dostają `w/h` z projektu, więc podgląd pozostaje 1:1 z eksportem.
        let west = input.handle.contains('w');
        let north = input.handle.contains('n');
        let right = x + w;
        let bottom = y + h;
        let size_x = if west { w - input.dx } else { w + input.dx };
        let size_y = if north { h - input.dy } else { h + input.dy };
        let use_vertical = (size_y - h).abs() > (size_x - w).abs();
        let mut size = if use_vertical { size_y } else { size_x };

        let max_x = if west { right } else { screen - x };
        let max_y = if north { bottom } else { screen - y };
        let limit_size = max_x.min(max_y).min(max_size);
        size = size.clamp(min_size, limit_size);

        // Jak przy przesuwaniu: prowadnica przed siatką, żeby środek wskazówek
        // mógł trafić w środek tarczy albo innego widgetu.
        let raw_size = size;
        let raw_x = if west { right - raw_size } else { x };
        let raw_y = if north { bottom - raw_size } else { y };
        let (aligned_x, line_x) = resolve_axis(
            raw_x,
            [raw_x, raw_x + raw_size / 2, raw_x + raw_size],
            &input.align_x,
            false,
            screen - raw_size,
        );
        let (aligned_y, line_y) = resolve_axis(
            raw_y,
            [raw_y, raw_y + raw_size / 2, raw_y + raw_size],
            &input.align_y,
            false,
            screen - raw_size,
        );
        guide_x = line_x;
        guide_y = line_y;

        if guide_x.is_none() && guide_y.is_none() && input.snap {
            let (edge, min_edge, max_edge) = if use_vertical && north {
                (bottom - size, 0, bottom - min_size)
            } else if use_vertical {
                (y + size, y + min_size, screen)
            } else if west {
                (right - size, 0, right - min_size)
            } else {
                (x + size, x + min_size, screen)
            };
            let snapped = snap_to_grid(edge).clamp(min_edge, max_edge);
            let from_snap = if use_vertical {
                if north { bottom - snapped } else { snapped - y }
            } else if west {
                right - snapped
            } else {
                snapped - x
            };
            size = from_snap.clamp(min_size, limit_size);
        }

        if guide_x.is_some() || guide_y.is_some() {
            x = aligned_x;
            y = aligned_y;
        } else {
            if west {
                x = right - size;
            }
            if north {
                y = bottom - size;
            }
        }
        w = size;
        h = size;
    } else {
        let west = input.handle.contains('w');
        let north = input.handle.contains('n');
        if west {
            // Lewa krawędź się przesuwa, prawa zostaje na miejscu.
            let right = x + w;
            let left_limit = (right - max_size).max(0);
            x = (x + input.dx).clamp(left_limit, right - min_size);
            if input.snap {
                x = snap_to_grid(x).clamp(left_limit, right - min_size);
            }
            w = right - x;
        } else {
            // Prawa krawędź się przesuwa, lewa zostaje na miejscu.
            w = (w + input.dx).clamp(min_size, (screen - x).min(max_size));
            if input.snap {
                let right = snap_to_grid(x + w).clamp(x + min_size, (x + max_size).min(screen));
                w = right - x;
            }
        }
        if north {
            let bottom = y + h;
            let top_limit = (bottom - max_size).max(0);
            y = (y + input.dy).clamp(top_limit, bottom - min_size);
            if input.snap {
                y = snap_to_grid(y).clamp(top_limit, bottom - min_size);
            }
            h = bottom - y;
        } else {
            h = (h + input.dy).clamp(min_size, (screen - y).min(max_size));
            if input.snap {
                let bottom = snap_to_grid(y + h).clamp(y + min_size, (y + max_size).min(screen));
                h = bottom - y;
            }
        }
    }

    let text_size = if input.auto_scale && start_w > 0 {
        // Zaokrąglenie do najbliższej liczby całkowitej, bez zmiennoprzecinkowych.
        let scaled = (input.text_size as i64 * w as i64 + start_w as i64 / 2) / start_w as i64;
        scaled.clamp(MIN_TEXT_SIZE as i64, MAX_TEXT_SIZE as i64) as u32
    } else {
        input.text_size
    };

    FrameGeometry {
        x,
        y,
        w,
        h,
        text_size,
        guide_x,
        guide_y,
    }
}

fn string_field(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("missing or invalid '{key}'"))
}

fn i64_field(value: &Value, key: &str) -> Result<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .with_context(|| format!("missing or invalid '{key}'"))
}

/// Specyfikacja pola daty: (typ, źródło, prefiks id, nazwa, tekst początkowy, rozmiar).
fn date_field_spec(
    kind: &str,
) -> Option<(WidgetKind, DataSource, &'static str, &'static str, &'static str, u32)> {
    Some(match kind {
        "weekday-number" => (
            WidgetKind::WeekdayNumberText,
            DataSource::WeekDayNumber,
            "weekday-number",
            "Day of week",
            "--",
            40,
        ),
        "week-number" => (
            WidgetKind::WeekNumberText,
            DataSource::WeekNumber,
            "week-number",
            "Week",
            "--",
            40,
        ),
        "day-of-year" => (
            WidgetKind::DayOfYearText,
            DataSource::DayOfYear,
            "day-of-year",
            "Day of year",
            "--",
            40,
        ),
        "lunar-date" => (
            WidgetKind::LunarDateText,
            DataSource::LunarDate,
            "lunar-date",
            "Lunar date",
            "--/--",
            40,
        ),
        "festival" => (
            WidgetKind::FestivalText,
            DataSource::Festival,
            "festival",
            "Festival",
            "--",
            32,
        ),
        "world-clock" => (
            WidgetKind::WorldClockText,
            DataSource::WorldClock,
            "world-clock",
            "World clock",
            "--:--",
            40,
        ),
        _ => return None,
    })
}

/// Typ ikony statusu dla wpisu katalogu `status-*`.
fn status_icon_spec(kind: &str) -> Option<StatusType> {
    match kind {
        "status-alarm" => Some(StatusType::Alarm),
        "status-dnd" => Some(StatusType::Dnd),
        "status-bluetooth" => Some(StatusType::Bluetooth),
        "status-lock" => Some(StatusType::Lock),
        _ => None,
    }
}

/// Dodaje łuk albo pasek postępu. To jedyne widgety, których wygląd jest
/// opisany jedną strukturą `ProgressSettings` zamiast pol tekstu.
fn add_progress_widget(project: &mut Project, kind: &str) -> Result<Widget> {
    use crate::model::{ProgressSettings, WidgetKind};

    let (arc, prefix, name) = match kind {
        "arc-progress" => (true, "arc", "Arc progress"),
        _ => (false, "bar", "Bar progress"),
    };
    let mut index = 1u32;
    let id = loop {
        let candidate = if index == 1 {
            prefix.to_string()
        } else {
            format!("{prefix}-{index}")
        };
        if !project.widgets.iter().any(|w| w.id == candidate) {
            break candidate;
        }
        index += 1;
    };
    let suffix = if index == 1 {
        String::new()
    } else {
        format!(" {index}")
    };
    let (x, y, w, h) = if arc {
        (33, 33, 400, 400)
    } else {
        (23, 203, 420, 60)
    };
    let widget = Widget {
        id,
        name: format!("{name}{suffix}"),
        kind: if arc {
            WidgetKind::ArcProgress
        } else {
            WidgetKind::BarProgress
        },
        x,
        y,
        w,
        h,
        z: 0,
        text: None,
        text_size: None,
        font: None,
        time_format: None,
        color: Some(ProgressSettings::default().color),
        src: None,
        image_fit: FitMode::default(),
        hand_set: None,
        hour_src: None,
        minute_src: None,
        second_src: None,
        cap_src: None,
        data_source: None,
        align: None,
        v_align: VAlign::Center,
        show_level: ShowLevel::Normal,
        auto_scale: false,
        text_style: TextStyle::Ellipsis,
        visible: true,
        measurement_system: None,
        show_unit: true,
        alpha: 255,
        status_icon_set: None,
        status_type: None,
        weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: Some(ProgressSettings::default()),
    shortcut_target: None,
    }
;
    project.widgets.push(widget.clone());
    layers::normalize(project);
    Ok(widget)
}

/// Dodaje niewidoczną strefę klikalną — na tarczy jest to przezroczysty
/// `IMG_CLICK`, w edytorze kreskowana ramka z ikoną dotknięcia.
fn add_shortcut_widget(project: &mut Project) -> Result<Widget> {
    use crate::model::ShortcutTarget;

    let mut index = 1u32;
    let id = loop {
        let candidate = if index == 1 {
            "shortcut".to_string()
        } else {
            format!("shortcut-{index}")
        };
        if !project.widgets.iter().any(|w| w.id == candidate) {
            break candidate;
        }
        index += 1;
    };
    let suffix = if index == 1 {
        String::new()
    } else {
        format!(" {index}")
    };
    let widget = Widget {
        id,
        name: format!("Shortcut{suffix}"),
        kind: WidgetKind::Shortcut,
        x: 173,
        y: 203,
        w: 120,
        h: 60,
        z: 0,
        text: None,
        text_size: None,
        font: None,
        time_format: None,
        color: None,
        src: None,
        image_fit: FitMode::default(),
        hand_set: None,
        hour_src: None,
        minute_src: None,
        second_src: None,
        cap_src: None,
        data_source: None,
        align: None,
        v_align: VAlign::Center,
        show_level: ShowLevel::Normal,
        auto_scale: false,
        text_style: TextStyle::Ellipsis,
        visible: true,
        measurement_system: None,
        show_unit: true,
        alpha: 255,
        status_icon_set: None,
        status_type: None,
        weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: Some(ShortcutTarget::Battery),
    };
    project.widgets.push(widget.clone());
    layers::normalize(project);
    Ok(widget)
}

/// Dodaje widget do projektu. Obsługiwane typy: `time` (cyfrowy), `analog-time`,
/// `date`, `text` (stały tekst), `steps`, `distance`, widgety zdrowotne
/// z kategorii `health`, bateria oraz ikony statusu z kategorii `system`.
pub fn add_widget(project: &mut Project, kind: &str) -> Result<Widget> {
    if kind == "shortcut" {
        return add_shortcut_widget(project);
    }
    if matches!(kind, "arc-progress" | "bar-progress") {
        return add_progress_widget(project, kind);
    }
    let health = health_spec(kind);
    let sleep = sleep_spec(kind);
    let target = target_spec(kind);
    let weather = weather_spec(kind);
    let date_field = date_field_spec(kind);
    let status_icon = status_icon_spec(kind);
    let battery = kind == "battery";
    let (analog, date, user_text, steps, distance, images) = match kind {
        "time" | "battery" => (false, false, false, false, false, false),
        "analog-time" => (true, false, false, false, false, false),
        "date" => (false, true, false, false, false, false),
        "text" => (false, false, true, false, false, false),
        "images" => (false, false, false, false, false, true),
        "steps" => (false, false, false, true, false, false),
        "distance" => (false, false, false, false, true, false),
        _ if health.is_some()
            || sleep.is_some()
            || target.is_some()
            || weather.is_some()
            || date_field.is_some()
            || status_icon.is_some() =>
        {
            (false, false, false, false, false, false)
        }
        _ => bail!("Nieobsługiwany typ widgetu: {kind}"),
    };
    let prefix = if analog {
        "analog"
    } else if images {
        "image"
    } else if date {
        "date"
    } else if user_text {
        "text"
    } else if steps {
        "steps"
    } else if distance {
        "distance"
    } else if battery {
        "battery"
    } else if let Some(status) = status_icon {
        match status {
            StatusType::Alarm => "status-alarm",
            StatusType::Dnd => "status-dnd",
            StatusType::Bluetooth => "status-bluetooth",
            StatusType::Lock => "status-lock",
        }
    } else if let Some((_, _, health_prefix, _, _, _)) = health {
        health_prefix
    } else if let Some((_, _, sleep_prefix, _, _, _)) = sleep {
        sleep_prefix
    } else if let Some((_, _, target_prefix, _, _, _)) = target {
        target_prefix
    } else if let Some((_, _, weather_prefix, _, _, _)) = weather {
        weather_prefix
    } else if let Some((_, _, date_prefix, _, _, _)) = date_field {
        date_prefix
    } else {
        "time"
    };

    let mut index = 1u32;
    let id = loop {
        let candidate = if index == 1 {
            prefix.to_string()
        } else {
            format!("{prefix}-{index}")
        };
        if !project.widgets.iter().any(|w| w.id == candidate) {
            break candidate;
        }
        index += 1;
    };
    let suffix = if index == 1 {
        String::new()
    } else {
        format!(" {index}")
    };

    let widget = if analog {
        // Rozmiar i pozycja zostaną ustawione przez `hands::attach_to_project`
        // na podstawie wymiarów plików wybranego zestawu wskazówek.
        Widget {
            id: id.clone(),
            name: format!("Analog{suffix}"),
            kind: WidgetKind::AnalogTime,
            x: 33,
            y: 33,
            w: 400,
            h: 400,
            z: 0,
            text: None,
            text_size: None,
            font: None,
            time_format: None,
            color: None,
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: None,
            align: None,
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if images {
        // Statyczny obrazek — plik PNG jest przygotowywany do rozmiaru ramki
        // w `images::set_widget_image` (to samo `image_fit`, co dla tła).
        Widget {
            id: id.clone(),
            name: format!("Image{suffix}"),
            kind: WidgetKind::Images,
            x: 133,
            y: 133,
            w: 200,
            h: 200,
            z: 0,
            text: None,
            text_size: None,
            font: None,
            time_format: None,
            color: None,
            src: None,
            image_fit: FitMode::Contain,
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: None,
            align: None,
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if date {
        // Data to widget tekstowy z danymi `Date`. `time_format` przechowuje tu
        // wzorzec daty (inny parser niż dla czasu) — podgląd i wygenerowany JS
        // czytają go przez `date_format`, więc oba widoki pozostają 1:1.
        Widget {
            id: id.clone(),
            name: format!("Date{suffix}"),
            kind: WidgetKind::DateText,
            x: 23,
            y: 250,
            w: 420,
            h: 80,
            z: 0,
            text: Some("Jan 1".to_string()),
            text_size: Some(48),
            font: None,
            time_format: Some(crate::date_format::DEFAULT_PATTERN.to_string()),
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(DataSource::Date),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if user_text {
        // Zwykły tekst wpisany w inspektorze. Bez `dataSource` i bez wzorca —
        // generator JS emituje statyczne `text:`, bez timerów i nasłuchiwania sensorów.
        Widget {
            id: id.clone(),
            name: format!("Text{suffix}"),
            kind: WidgetKind::Text,
            x: 23,
            y: 350,
            w: 420,
            h: 60,
            z: 0,
            text: Some("Text".to_string()),
            text_size: Some(32),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: None,
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if steps {
        // Licznik kroków czyta wartość z czujnika Zegarka — w podglądzie edytora
        // pokazujemy „0", bo na PC nie ma czujnika kroków.
        Widget {
            id: id.clone(),
            name: format!("Steps{suffix}"),
            kind: WidgetKind::StepText,
            x: 23,
            y: 320,
            w: 420,
            h: 60,
            z: 0,
            text: Some("0".to_string()),
            text_size: Some(32),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(DataSource::Step),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if distance {
        // Dystans czyta czujnik ZeppOS w metrach. Jednostka domyślna pochodzi
        // z ustawień aplikacji zapisanych w projekcie; widget może ją nadpisać.
        Widget {
            id: id.clone(),
            name: format!("Distance{suffix}"),
            kind: WidgetKind::DistanceText,
            x: 23,
            y: 260,
            w: 420,
            h: 60,
            z: 0,
            text: Some(project.measurement_system.initial_distance_text()),
            text_size: Some(32),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(DataSource::Distance),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if let Some((kind, source, _, name, initial, size)) = date_field {
        // Pole daty — wartość jest generowana przez czujnik TIME / WORLD_CLOCK
        // albo wspólny algorytm JS. Na PC pokazujemy neutralny symbol.
        Widget {
            id: id.clone(),
            name: format!("{name}{suffix}"),
            kind,
            x: 23,
            y: 200,
            w: 420,
            h: 60,
            z: 0,
            text: Some(initial.to_string()),
            text_size: Some(size),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(source),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if let Some((target_kind, target_source, _, target_name, initial, size)) = target {
        // Pole celu — wartość czyta czujnik na zegarku (pole `.target`). Na PC
        // i przed pierwszym odczytem zostaje sensowny placeholder z katalogu.
        Widget {
            id: id.clone(),
            name: format!("{target_name}{suffix}"),
            kind: target_kind,
            x: 23,
            y: 150,
            w: 420,
            h: 60,
            z: 0,
            text: Some(initial.to_string()),
            text_size: Some(size),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(target_source),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if let Some((sleep_kind, sleep_source, _, sleep_name, sleep_text, sleep_size)) = sleep {
        // Widget snu — dane z `hmSensor.id.SLEEP`, na PC tekst predefiniowany.
        Widget {
            id: id.clone(),
            name: format!("{sleep_name}{suffix}"),
            kind: sleep_kind,
            x: 23,
            y: 150,
            w: 420,
            h: 60,
            z: 0,
            text: Some(sleep_text.to_string()),
            text_size: Some(sleep_size),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(sleep_source),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
            weather_day_offset: 0,
            sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if let Some((health_kind, health_source, _, health_name, _, health_size)) = health {
        // Widget zdrowotny — wartość czyta czujnik na zegarku. Na PC podgląd
        // pokazuje neutralny symbol, bo komputer nie ma czujników biometrycznych.
        Widget {
            id: id.clone(),
            name: format!("{health_name}{suffix}"),
            kind: health_kind,
            x: 23,
            y: 150,
            w: 420,
            h: 60,
            z: 0,
            text: health_kind.health_initial_text(true),
            text_size: Some(health_size),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(health_source),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if battery {
        // Poziom baterii — tekst zasilany `hmSensor.id.BATTERY`. Na PC pokazujemy
        // neutralne 100%, dopóki zegarek nie prześle pierwszego odczytu.
        Widget {
            id: id.clone(),
            name: format!("Battery{suffix}"),
            kind: WidgetKind::BatteryText,
            // Środek ramki w środku tarczy: x = (466-420)/2, y = (466-60)/2.
            x: 23,
            y: 203,
            w: 420,
            h: 60,
            z: 0,
            text: Some("100%".to_string()),
            text_size: Some(32),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(DataSource::Battery),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if let Some(status) = status_icon {
        // Ikona statusu systemu. Pakiety są przypisywane w FFI zaraz po dodaniu
        // widgetu — tutaj ustawiamy tylko domyślną ramkę w górnym pasku stanu.
        // Jedna pozioma linia wycentrowana na środku tarczy: 4 × 32 px
        // z 16 px odstępami = 176 px, start x = (466 - 176) / 2 = 145.
        let (x, y) = match status {
            StatusType::Alarm => (145, 24),
            StatusType::Dnd => (193, 24),
            StatusType::Bluetooth => (241, 24),
            StatusType::Lock => (289, 24),
        };
        Widget {
            id: id.clone(),
            name: format!("{}{}", status.as_str(), suffix),
            kind: WidgetKind::StatusIcon,
            x,
            y,
            w: 32,
            h: 32,
            z: 0,
            text: None,
            text_size: None,
            font: None,
            time_format: None,
            color: None,
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: None,
            align: None,
            v_align: VAlign::Center,
            // Ikony nie powinny świecić w trybie AOD (reguła ZeppOS: <10% pikseli).
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: Some(status),
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else if let Some((weather_kind, weather_source, _, weather_name, initial, size)) = weather {
        // Widget pogody — wartości czyta `hmSensor.id.WEATHER` na zegarku.
        // Ikona dostaje wyśrodkowaną kwadratową ramkę i domyślny zestaw
        // przypisywany przez FFI zaraz po dodaniu (tak jak ikony STATUS).
        let (ix, iy, iw, ih) = if weather_kind == WidgetKind::WeatherIcon {
            crate::weather::default_frame()
        } else {
            (23, 150, 420, 60)
        };
        Widget {
            id: id.clone(),
            name: format!("{weather_name}{suffix}"),
            kind: weather_kind,
            x: ix,
            y: iy,
            w: iw,
            h: ih,
            z: 0,
            text: if initial.is_empty() {
                None
            } else {
                Some(initial.to_string())
            },
            text_size: Some(size),
            font: None,
            time_format: None,
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(weather_source),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    } else {
        Widget {
            id: id.clone(),
            name: format!("Time{suffix}"),
            kind: WidgetKind::TimeText,
            x: 23,
            y: 183,
            w: 420,
            h: 100,
            z: 0,
            text: Some("00:00:00".to_string()),
            text_size: Some(64),
            font: None,
            time_format: Some("HH:MM:SS".to_string()),
            color: Some("0xFFFFFFFF".to_string()),
            src: None,
            image_fit: FitMode::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(DataSource::Time),
            align: Some(Align::Center),
            v_align: VAlign::Center,
            show_level: ShowLevel::Normal,
            auto_scale: false,
            text_style: TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        progress: None,
        shortcut_target: None,
        }


    };
    project.widgets.push(widget.clone());
    // Nowy widget ląduje na samej górze; synchronize layer_order i z.
    layers::normalize(project);
    Ok(widget)
}

/// Aktualizuje ustawienia wskazanego widgetu z panelu inspektora edytora.
/// Zapisuje ustawienia widgetu postępu. Zwraca `None`, gdy warstwa nie jest
/// łukiem ani paskiem — wtedy obsługuje ją zwykła ścieżka tekstowa.
fn update_progress_widget(project: &mut Project, settings: &Value) -> Result<Option<Widget>> {
    let id = string_field(settings, "id")?;
    let is_progress = project.widgets.iter().find(|w| w.id == id).is_some_and(|w| {
        matches!(w.kind, WidgetKind::ArcProgress | WidgetKind::BarProgress)
    });
    if !is_progress {
        return Ok(None);
    }
    let x = i64_field(settings, "x")?;
    let y = i64_field(settings, "y")?;
    let w = i64_field(settings, "w")?;
    let h = i64_field(settings, "h")?;
    if w <= 0 || h <= 0 || x < 0 || y < 0 || x + w > SCREEN_SIZE as i64 || y + h > SCREEN_SIZE as i64 {
        bail!("Widget musi mieścić się w ekranie 466×466");
    }
    let widget = project
        .widgets
        .iter_mut()
        .find(|widget| widget.id == id)
        .unwrap();

    let mut progress = widget.progress.clone().unwrap_or_default();
    let nested = settings.get("progress").unwrap_or(&Value::Null);
    let valid_color = |value: &str| {
        let hex = value.strip_prefix("0x").unwrap_or(value);
        hex.len() == 8 && hex.chars().all(|c| c.is_ascii_hexdigit())
    };
    let str_val = |key: &str| -> Option<String> {
        nested
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let f_val = |key: &str| -> Option<f32> {
        nested.get(key).and_then(Value::as_f64).map(|v| v as f32)
    };
    let u_val = |key: &str| -> Option<u32> {
        nested.get(key).and_then(Value::as_u64).map(|v| v.min(u32::MAX as u64) as u32)
    };
    if let Some(source) = str_val("source").and_then(|v| ProgressSource::parse(&v)) {
        progress.source = source;
    }
    if let Some(value) = f_val("min") {
        progress.min = value.clamp(-99999.0, 99999.0);
    }
    if let Some(value) = f_val("max") {
        progress.max = value.clamp(-99999.0, 99999.0);
    }
    if let Some(value) = f_val("value") {
        progress.value = value.clamp(-99999.0, 99999.0);
    }
    if let Some(value) = str_val("color") {
        if valid_color(&value) {
            progress.color = value;
        }
    }
    if let Some(value) = str_val("trackColor") {
        if valid_color(&value) {
            progress.track_color = value;
        }
    }
    if let Some(value) = u_val("thickness") {
        progress.thickness = value.clamp(1, 233);
    }
    if let Some(value) = nested.get("startAngle").and_then(Value::as_i64) {
        progress.start_angle = value.clamp(-360, 360) as i32;
    }
    if let Some(value) = nested.get("endAngle").and_then(Value::as_i64) {
        progress.end_angle = value.clamp(-360, 360) as i32;
    }
    if let Some(value) = str_val("corner").and_then(|v| ProgressCorner::parse(&v)) {
        progress.corner = value;
    }
    if let Some(value) = u_val("radius") {
        progress.radius = value.clamp(0, 233);
    }
    if let Some(value) = u_val("segments") {
        progress.segments = value.clamp(1, 24) as u8;
    }
    if let Some(value) = u_val("gap") {
        progress.gap = value.clamp(0, 466);
    }
    if let Some(value) = str_val("orientation").and_then(|v| ProgressOrientation::parse(&v)) {
        progress.orientation = value;
    }
    if let Some(value) = nested.get("invert").and_then(Value::as_bool) {
        progress.invert = value;
    }
    if let Some(value) = nested.get("showTrack").and_then(Value::as_bool) {
        progress.show_track = value;
    }
    widget.x = x as i32;
    widget.y = y as i32;
    widget.w = w as i32;
    widget.h = h as i32;
    widget.alpha = settings
        .get("alpha")
        .and_then(Value::as_u64)
        .map(|v| v.clamp(0, 255) as u8)
        .unwrap_or(widget.alpha);
    widget.progress = Some(progress);
    Ok(Some(widget.clone()))
}

/// Zapisuje cel klikalnej strefy (reszta geometryczna jest wspólna).
fn update_shortcut_widget(project: &mut Project, settings: &Value) -> Result<Option<Widget>> {
    use crate::model::ShortcutTarget;

    let id = string_field(settings, "id")?;
    let is_shortcut = project
        .widgets
        .iter()
        .find(|w| w.id == id)
        .is_some_and(|w| w.kind == WidgetKind::Shortcut);
    if !is_shortcut {
        return Ok(None);
    }
    let x = i64_field(settings, "x")?;
    let y = i64_field(settings, "y")?;
    let w = i64_field(settings, "w")?;
    let h = i64_field(settings, "h")?;
    if w <= 0 || h <= 0 || x < 0 || y < 0 || x + w > SCREEN_SIZE as i64 || y + h > SCREEN_SIZE as i64 {
        bail!("Widget musi mieścić się w ekranie 466×466");
    }
    let widget = project.widgets.iter_mut().find(|w| w.id == id).unwrap();
    widget.x = x as i32;
    widget.y = y as i32;
    widget.w = w as i32;
    widget.h = h as i32;
    if let Some(value) = settings.get("shortcutTarget").and_then(Value::as_str) {
        widget.shortcut_target = Some(
            ShortcutTarget::parse(value)
                .with_context(|| format!("Nieprawidłowy cel skrótu: {value}"))?,
        );
    }
    widget.alpha = settings
        .get("alpha")
        .and_then(Value::as_u64)
        .map(|v| v.clamp(0, 255) as u8)
        .unwrap_or(widget.alpha);
    Ok(Some(widget.clone()))
}

pub fn update_widget(project: &mut Project, settings: &Value) -> Result<Widget> {
    let id = string_field(settings, "id")?;
    if let Some(widget) = update_shortcut_widget(project, settings)? {
        return Ok(widget);
    }
    if let Some(widget) = update_progress_widget(project, settings)? {
        return Ok(widget);
    }
    // Ikona statusu nie ma czcionki ani wzorca — zapisujemy tylko geometrię.
    let is_status_icon = project
        .widgets
        .iter()
        .find(|widget| widget.id == id)
        .is_some_and(|widget| widget.kind == WidgetKind::StatusIcon);
    let is_weather_icon = project
        .widgets
        .iter()
        .find(|widget| widget.id == id)
        .is_some_and(|widget| widget.kind == WidgetKind::WeatherIcon);
    if is_weather_icon {
        let x = i64_field(settings, "x")?;
        let y = i64_field(settings, "y")?;
        let w = i64_field(settings, "w")?;
        let h = i64_field(settings, "h")?;
        if w <= 0 || h <= 0 || x < 0 || y < 0 || x + w > SCREEN_SIZE as i64 || y + h > SCREEN_SIZE as i64 {
            bail!("Widget musi mieścić się w ekranie 466×466");
        }
        let auto_scale = settings
            .get("autoScale")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let widget = project
            .widgets
            .iter_mut()
            .find(|widget| widget.id == id)
            .unwrap();
        widget.x = x as i32;
        widget.y = y as i32;
        widget.w = w as i32;
        widget.h = h as i32;
        widget.auto_scale = auto_scale;
        widget.alpha = settings
            .get("alpha")
            .and_then(Value::as_u64)
            .map(|value| value.clamp(0, 255) as u8)
            .unwrap_or(widget.alpha);
        return Ok(widget.clone());
    }
    if is_status_icon {
        let x = i64_field(settings, "x")?;
        let y = i64_field(settings, "y")?;
        let w = i64_field(settings, "w")?;
        let h = i64_field(settings, "h")?;
        if w <= 0 || h <= 0 || x < 0 || y < 0 || x + w > SCREEN_SIZE as i64 || y + h > SCREEN_SIZE as i64 {
            bail!("Widget musi mieścić się w ekranie 466×466");
        }
        let (existing_auto_scale, existing_alpha) = project
            .widgets
            .iter()
            .find(|widget| widget.id == id)
            .map(|widget| (widget.auto_scale, widget.alpha))
            .unwrap_or((false, 255));
        let auto_scale = settings
            .get("autoScale")
            .and_then(Value::as_bool)
            .unwrap_or(existing_auto_scale);
        let widget = project
            .widgets
            .iter_mut()
            .find(|widget| widget.id == id)
            .unwrap();
        widget.x = x as i32;
        widget.y = y as i32;
        widget.w = w as i32;
        widget.h = h as i32;
        widget.auto_scale = auto_scale;
        widget.alpha = settings
            .get("alpha")
            .and_then(Value::as_u64)
            .map(|value| value.clamp(0, 255) as u8)
            .unwrap_or(existing_alpha);
        return Ok(widget.clone());
    }
    let (
        existing_auto_scale,
        existing_text_style,
        existing_v_align,
        existing_font,
        is_analog,
        is_images,
        existing_image_fit,
        is_date,
        uses_pattern,
        is_user_text,
        is_distance,
        is_health,
        is_goal,
        is_sleep,
        is_weather,
        existing_measurement_system,
        existing_show_unit,
        existing_alpha,
        existing_weather_day_offset,
        existing_sleep_stage_mode,
    ) = project
        .widgets
        .iter()
        .find(|widget| widget.id == id)
        .map(|widget| {
            (
                widget.auto_scale,
                widget.text_style,
                widget.v_align,
                widget.font.clone(),
                widget.kind == WidgetKind::AnalogTime,
                widget.kind == WidgetKind::Images,
                widget.image_fit,
                widget.kind == WidgetKind::DateText,
                widget.kind.uses_pattern(),
                widget.kind == WidgetKind::Text,
                widget.kind == WidgetKind::DistanceText,
                widget.kind.is_health(),
                widget.kind.is_goal(),
                widget.kind.is_sleep(),
                widget.kind.is_weather(),
                widget.measurement_system,
                widget.show_unit,
                widget.alpha,
                widget.weather_day_offset,
                widget.sleep_stage_mode,
            )
        })
        .with_context(|| format!("warstwa nie istnieje: {id}"))?;
    // Tylko czas i data mają wzorzec. Dla pozostałych typów (bateria, pogoda,
    // czujniki, tekst użytkownika) `timeFormat` z inspektora musi być ignorowany,
    // bo nadpisałby ich treść na `00:00:00`.
    let no_pattern = !uses_pattern;
    let project_default = project.measurement_system;
    let x = i64_field(settings, "x")?;
    let y = i64_field(settings, "y")?;
    let w = i64_field(settings, "w")?;
    let h = i64_field(settings, "h")?;
    if w <= 0 || h <= 0 || x < 0 || y < 0 || x + w > SCREEN_SIZE as i64 || y + h > SCREEN_SIZE as i64 {
        bail!("Widget musi mieścić się w ekranie 466×466");
    }
    if is_analog {
        // Wskazówki nie mają czcionki ani formatu — zapisujemy geometrię (to nią
        // skalujemy obrazy) oraz tryb skalowania zapalany podwójnym kliknięciem.
        let auto_scale = settings
            .get("autoScale")
            .and_then(Value::as_bool)
            .unwrap_or(existing_auto_scale);
        let widget = project
            .widgets
            .iter_mut()
            .find(|widget| widget.id == id)
            .unwrap();
        widget.x = x as i32;
        widget.y = y as i32;
        widget.w = w as i32;
        widget.h = h as i32;
        widget.auto_scale = auto_scale;
        widget.alpha = settings
            .get("alpha")
            .and_then(Value::as_u64)
            .map(|value| value.clamp(0, 255) as u8)
            .unwrap_or(existing_alpha);
        return Ok(widget.clone());
    }
    if is_images {
        // Obrazek nie ma czcionki, koloru ani wzorca — zapisujemy geometrię,
        // tryb skalowania i przełączanie ramki (niebieski = przesuw, czerwony = skala).
        let image_fit = match settings.get("imageFit").and_then(Value::as_str) {
            Some(value) => FitMode::parse(value)
                .with_context(|| format!("Nieprawidłowy tryb skalowania obrazka: {value}"))?,
            None => existing_image_fit,
        };
        let auto_scale = settings
            .get("autoScale")
            .and_then(Value::as_bool)
            .unwrap_or(existing_auto_scale);
        let widget = project
            .widgets
            .iter_mut()
            .find(|widget| widget.id == id)
            .unwrap();
        widget.x = x as i32;
        widget.y = y as i32;
        widget.w = w as i32;
        widget.h = h as i32;
        // Czerwona ramka (auto_scale) zawsze rozciąga obraz do nowej ramki —
        // niezależnie od wybranego trybu dopasowania z inspektora.
        widget.image_fit = if auto_scale {
            FitMode::Stretch
        } else {
            image_fit
        };
        widget.auto_scale = auto_scale;
        widget.alpha = settings
            .get("alpha")
            .and_then(Value::as_u64)
            .map(|value| value.clamp(0, 255) as u8)
            .unwrap_or(existing_alpha);
        return Ok(widget.clone());
    }

    let text_size = i64_field(settings, "textSize")?;
    if !(8..=160).contains(&text_size) {
        bail!("Rozmiar czcionki musi być w zakresie 8–160");
    }

    let color = string_field(settings, "color")?;
    let hex = color.strip_prefix("0x").context("Kolor musi mieć postać 0xAARRGGBB")?;
    if hex.len() != 8 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("Kolor musi mieć postać 0xAARRGGBB");
    }

    let align = match string_field(settings, "align")?.as_str() {
        "left" => Align::Left,
        "right" => Align::Right,
        _ => Align::Center,
    };

    // Pola ramki są opcjonalne — starsi klienci (i stare projekty) zachowują wartości domyślne.
    let auto_scale = settings
        .get("autoScale")
        .and_then(Value::as_bool)
        .unwrap_or(existing_auto_scale);
    let text_style = match settings.get("textStyle").and_then(Value::as_str) {
        Some(value) => TextStyle::parse(value)
            .with_context(|| format!("Nieprawidłowy sposób dopasowania tekstu: {value}"))?,
        None => existing_text_style,
    };
    let v_align = settings
        .get("vAlign")
        .and_then(Value::as_str)
        .and_then(VAlign::parse)
        .unwrap_or(existing_v_align);
    // Pusty string = świadomy wybór "System font" (zegarek używa Noto Sans, JS
    // pomija `font:`). Brak klucza = zachowaj poprzedni wybór.
    let font = match settings.get("font").and_then(Value::as_str) {
        Some("") => None,
        None => existing_font,
        Some(value) => Some(value.to_string()),
    };

    let pattern = settings
        .get("timeFormat")
        .and_then(Value::as_str)
        .map(str::to_string);
    // Widget bez wzorca nie może nadpisać treści formatem czasu/daty.
    if let Some(pattern) = &pattern {
        if !no_pattern {
            if is_date {
                crate::date_format::parse(pattern)?;
            } else {
                time_format::parse(pattern)?;
            }
        }
    }
    let user_text = settings
        .get("text")
        .and_then(Value::as_str)
        .map(str::to_string);
    // Brak klucza zachowuje poprzedni wybór; pusty string czyści nadpisanie
    // i wraca do systemu z ustawień aplikacji.
    let measurement_system = match settings.get("measurementSystem").and_then(Value::as_str) {
        None => existing_measurement_system,
        Some("") => None,
        Some(value) => Some(
            MeasurementSystem::parse(value)
                .with_context(|| format!("Nieprawidłowy system jednostek: {value}"))?,
        ),
    };

    let show_unit = settings
        .get("showUnit")
        .and_then(Value::as_bool)
        .unwrap_or(existing_show_unit);
    // Brak klucza zachowuje poprzednie krycie; wartość z UI jest w zakresie 0–255.
    let alpha = settings
        .get("alpha")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(0, 255) as u8)
        .unwrap_or(existing_alpha);
    let weather_day_offset = settings
        .get("weatherDayOffset")
        .and_then(Value::as_u64)
        .map(|value| value.min(4) as u8)
        .unwrap_or(existing_weather_day_offset);
    let sleep_stage_mode = settings
        .get("sleepStageMode")
        .and_then(Value::as_str)
        .and_then(SleepStageMode::parse)
        .unwrap_or(existing_sleep_stage_mode);

    let widget = project
        .widgets
        .iter_mut()
        .find(|widget| widget.id == id)
        .with_context(|| format!("warstwa nie istnieje: {id}"))?;
    widget.x = x as i32;
    widget.y = y as i32;
    widget.w = w as i32;
    widget.h = h as i32;
    widget.text_size = Some(text_size as u32);
    widget.color = Some(color);
    widget.align = Some(align);
    widget.auto_scale = auto_scale;
    widget.text_style = text_style;
    widget.v_align = v_align;
    widget.font = font;
    widget.show_unit = show_unit;
    widget.alpha = alpha;
    if is_weather {
        widget.weather_day_offset = weather_day_offset;
    }
    if is_sleep {
        widget.sleep_stage_mode = sleep_stage_mode;
        if matches!(
            widget.kind,
            WidgetKind::SleepDeepText
            | WidgetKind::SleepLightText
            | WidgetKind::SleepRemText
            | WidgetKind::SleepWakeText
        ) {
            widget.text = Some(WidgetKind::sleep_stage_initial_text(sleep_stage_mode).to_string());
        }
    }
    if is_health {
        widget.text = widget.kind.health_initial_text(show_unit);
    }
    if is_goal {
        widget.text = widget.kind.goal_initial_text(show_unit);
    }
    if is_distance {
        widget.measurement_system = measurement_system;
        let system = measurement_system.unwrap_or(project_default);
        widget.text = Some(system.initial_distance_text());
    }
    if let Some(pattern) = pattern {
        if !no_pattern {
            widget.text = Some(if is_date {
                crate::date_format::format(&pattern, 1, 0, 2026)?
            } else {
                time_format::format(&pattern, 0, 0, 0)?
            });
            widget.time_format = Some(pattern);
        }
    }
    if is_user_text {
        if let Some(text) = user_text {
            widget.text = Some(text);
        }
    }
    Ok(widget.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn status_icon_defaults_fit_inside_round_watchface() {
        let mut project = crate::model::Project::starter("Test");
        let mut boxes = Vec::new();
        for kind in ["status-alarm", "status-dnd", "status-bluetooth", "status-lock"] {
            let widget = add_widget(&mut project, kind).unwrap();
            let cx = widget.x as f64 + widget.w as f64 / 2.0;
            let cy = widget.y as f64 + widget.h as f64 / 2.0;
            let distance = ((cx - 233.0).powi(2) + (cy - 233.0).powi(2)).sqrt();
            assert!(distance < 233.0, "{kind}: center distance {distance}");
            boxes.push((widget.x, widget.y, widget.w, widget.h));
        }
        // Jedna linia w poziomie, bez nachodzenia i wycentrowana na środku tarczy.
        assert!(boxes.iter().all(|(_, y, _, h)| *y == 24 && *h == 32));
        let left = boxes.iter().map(|b| b.0).min().unwrap();
        let right = boxes.iter().map(|b| b.0 + b.2).max().unwrap();
        assert_eq!((left + right) / 2, 233);
        let mut sorted = boxes.clone();
        sorted.sort_by_key(|b| b.0);
        for pair in sorted.windows(2) {
            assert!(pair[0].0 + pair[0].2 < pair[1].0, "ikony nachodzą na siebie");
        }
    }

    #[test]
    fn battery_default_is_centered_on_watchface() {
        let mut project = crate::model::Project::starter("Test");
        let widget = add_widget(&mut project, "battery").unwrap();
        assert_eq!(widget.x + widget.w / 2, 233);
        assert_eq!(widget.y + widget.h / 2, 233);
    }

    #[test]
    fn empty_font_selects_watch_system_font() {
        let mut project = crate::model::Project::starter("Test");
        let mut settings = json!({
            "id": "time",
            "x": 23, "y": 183, "w": 420, "h": 100,
            "textSize": 64,
            "color": "0xFFFFFFFF",
            "align": "center",
            "font": "Inter-Regular.ttf",
        });
        let updated = update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.font.as_deref(), Some("Inter-Regular.ttf"));

        // Wybór "System font" w inspektorze musi naprawdę czyścić przypisaną czcionkę,
        // inaczej podgląd i eksport nadal używają Inter zamiast Noto Sans.
        settings["font"] = json!("");
        let updated = update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.font, None);

        // Brak klucza natomiast zachowuje wybór.
        settings.as_object_mut().unwrap().remove("font");
        let updated = update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.font, None);
    }

    #[test]
    fn status_widget_persists_auto_scale() {
        let mut project = crate::model::Project::starter("Test");
        let widget = add_widget(&mut project, "status-dnd").unwrap();
        let settings = serde_json::json!({
            "id": widget.id,
            "x": 145, "y": 24, "w": 32, "h": 32,
            "autoScale": true,
        });
        let updated = update_widget(&mut project, &settings).unwrap();
        assert!(updated.auto_scale);
    }

    fn frame(handle: &str, dx: i32, dy: i32, snap: bool) -> FrameInput {
        FrameInput {
            x: 83,
            y: 83,
            w: 300,
            h: 300,
            text_size: 64,
            handle: handle.to_string(),
            dx,
            dy,
            auto_scale: true,
            snap,
            align_x: vec![],
            align_y: vec![],
            square: true,
            min_size: MIN_FRAME_SIZE,
            max_size: SCREEN_SIZE as i32,
        }
    }

    #[test]
    fn square_frame_keeps_hands_square_on_all_corners() {
        let se = frame_geometry(&frame("se", 50, 0, false));
        assert_eq!((se.x, se.y, se.w, se.h), (83, 83, 350, 350));

        let nw = frame_geometry(&frame("nw", -20, -20, false));
        assert_eq!((nw.x, nw.y, nw.w, nw.h), (63, 63, 320, 320));

        // Przeciągnięcie w osi Y również skaluje obie osie.
        let ne = frame_geometry(&frame("ne", 0, -40, false));
        assert_eq!((ne.x, ne.y, ne.w, ne.h), (83, 43, 340, 340));
    }

    #[test]
    fn square_frame_snaps_and_clamps_to_screen() {
        let snapped = frame_geometry(&frame("se", 7, 0, true));
        assert_eq!(snapped.w, snapped.h);
        assert_eq!(snapped.x + snapped.w, 384); // najbliższa linia kratki 16px

        let huge = frame_geometry(&frame("se", 1000, 1000, false));
        assert_eq!((huge.x, huge.y, huge.w, huge.h), (83, 83, 383, 383));

        // Ikona pogody ma wspólne z suwakiem SIZE maksimum 256 px.
        let mut capped_input = frame("se", 1000, 1000, false);
        capped_input.max_size = 256;
        let capped = frame_geometry(&capped_input);
        assert_eq!((capped.w, capped.h), (256, 256));
        assert!(capped.x + capped.w <= 466 && capped.y + capped.h <= 466);
    }

    #[test]
    fn analog_settings_persist_scale_mode() {
        let mut project = crate::model::Project::starter("Analog");
        project.widgets[0].kind = WidgetKind::AnalogTime;
        let settings = json!({
            "id": "time", "x": 33, "y": 33, "w": 400, "h": 400,
            "textSize": 64, "color": "0xFFFFFFFF", "align": "center",
            "autoScale": true,
        });
        update_widget(&mut project, &settings).unwrap();
        assert!(project.widgets[0].auto_scale);
    }

    #[test]
    fn sensor_text_widgets_ignore_time_format() {
        // Regresja: inspektor wysyłał timeFormat="HH:MM:SS" dla baterii i pól
        // pogody, co zapisywało wzorzec i nadpisywało tekst na "00:00:00".
        for kind in [
            "battery",
            "weather-condition",
            "weather-city",
            "weather-high",
            "weather-low",
            "sunrise",
            "sunset",
        ] {
            let mut project = crate::model::Project::starter("Test");
            let widget = add_widget(&mut project, kind).unwrap();
            let (id, before_text, before_pattern) =
                (widget.id.clone(), widget.text.clone(), widget.time_format.clone());
            let settings = json!({
                "id": id,
                "x": widget.x, "y": widget.y, "w": widget.w, "h": widget.h,
                "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
                "timeFormat": "HH:MM:SS",
            });
            let updated = update_widget(&mut project, &settings).unwrap();
            assert_eq!(updated.time_format, before_pattern, "{kind}: zapisano wzorzec");
            assert_eq!(updated.text, before_text, "{kind}: nadpisano tekst");
        }
    }

    #[test]
    fn pattern_widgets_still_store_time_format() {
        let mut project = crate::model::Project::starter("Test");
        let widget = project.widgets[0].clone();
        let settings = json!({
            "id": widget.id,
            "x": widget.x, "y": widget.y, "w": widget.w, "h": widget.h,
            "textSize": 64, "color": "0xFFFFFFFF", "align": "center",
            "timeFormat": "HH:MM",
        });
        let updated = update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.time_format.as_deref(), Some("HH:MM"));
        assert_eq!(updated.text.as_deref(), Some("00:00"));
    }

    #[test]
    fn status_and_analog_persist_alpha() {
        let mut project = crate::model::Project::starter("Test");
        let status = add_widget(&mut project, "status-dnd").unwrap();
        let settings = json!({
            "id": status.id,
            "x": status.x, "y": status.y, "w": status.w, "h": status.h,
            "alpha": 128,
        });
        assert_eq!(update_widget(&mut project, &settings).unwrap().alpha, 128);

        project.widgets[0].kind = WidgetKind::AnalogTime;
        let settings = json!({
            "id": "time", "x": 33, "y": 33, "w": 400, "h": 400,
            "alpha": 64,
        });
        assert_eq!(update_widget(&mut project, &settings).unwrap().alpha, 64);
    }
}
