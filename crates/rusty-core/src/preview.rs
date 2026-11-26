//! Renderer statycznego podglądu tarczy (`preview.png`) i animacji 8 klatek
//! (`preview.gif`). Cała logika jest w Rust — WinUI dostaje tylko gotowe pliki,
//! więc nie ma offscreen-renderowania kontrolek ani artefaktów pod edytorem.
//!
//! Ostatnia klatka jest dokładną kopią 21:37:40 i to z niej powstaje oprawiony
//! statyczny podgląd: przeskalowana tarcza, czarne strefy bezpieczne, brązowy
//! pierścień logo z zębatką i cienki pomarańczowy obrys.
//!
//! Dane klatek mają dwie warstwy:
//! * [`FRAME_PRESETS`] — zahardkodowany szkielet czasu i **prawdziwych dat**
//!   (7 kolejnych poniedziałków–niedziel, 7 różnych miesięcy), ściśle rosnące
//!   godziny/minuty/sekundy oraz wymuszone ekstrema wizualne;
//! * [`PreviewRandomSeries`] — jednorazowo wylosowane wartości (tętno, kalorie,
//!   dystans, kroki itd.). Kroki zawsze wynikają z dystansu, więc widgety nie
//!   mogą pokazać sprzecznych liczb.
//!
//! Podgląd nie czyta Custom Data z edytora — jest niezależny od ustawień UI.

use ab_glyph::{point, Font, FontRef, PxScale, PxScaleFont, ScaleFont};
use gif::{Encoder, Frame, Repeat};
use image::imageops::{self, FilterType};
use image::{ImageEncoder, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::layers::{describe, LayerView};
use crate::locale::WatchfaceLocale;
use crate::model::{MeasurementSystem, ProgressSource, StatusType, SCREEN_SIZE};
use crate::{date_format, time_format};

/// Liczba klatek animacji: 7 oryginalnych (poniedziałek–niedziela) oraz
/// dołączona na końcu dokładna kopia klatki 21:37:40, która zasila `preview.png`.
pub const FRAME_COUNT: usize = 8;
/// Liczba oryginalnych, różnych klatek — używana do wszystkich ciągów danych.
pub const SOURCE_FRAME_COUNT: usize = 7;
/// Indeks klatki 21:37:40 — źródło statycznego podglądu.
pub const PREVIEW_FRAME_INDEX: usize = 7;
/// Opóźnienie jednej klatki GIF-a w setnych sekundy (~1,7 s).
const GIF_FRAME_DELAY: u16 = 170;
/// Promień okręgu tarczy Active 2 (Round).
const SCREEN_RADIUS: f64 = 233.0;
/// Font systemowy ZeppOS (Noto Sans) używany, gdy widget nie ma przypisanej czcionki —
/// dokładnie tak jak wygenerowany JS, który w tym przypadku pomija `font:`.
const SYSTEM_FONT_BYTES: &[u8] =
    include_bytes!("../../../native/RustyWatch/Assets/Fonts/NotoSans-Regular.ttf");

const HOURS: [u8; FRAME_COUNT] = [6, 9, 12, 15, 18, 20, 21, 21];
const MINUTES: [u8; FRAME_COUNT] = [5, 11, 17, 23, 29, 33, 37, 37];
const SECONDS: [u8; FRAME_COUNT] = [4, 10, 16, 22, 28, 34, 40, 40];

/// Predefiniowany moment podglądu edytora, gdy Live Preview jest wyłączone.
/// Wartości pochodzą z klatki źródłowej `preview.png`/GIF, więc edytor i
/// eksportowany podgląd pokazują dokładnie ten sam moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDateTime {
    pub day: u32,
    pub month: u32,
    pub year: i32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// 31 lipca 2022, 21:37:40 — kopia klatki 7 (`PREVIEW_FRAME_INDEX`).
pub const DEFAULT_PREVIEW_DATETIME: PreviewDateTime = PreviewDateTime {
    day: DATES[PREVIEW_FRAME_INDEX].0,
    month: DATES[PREVIEW_FRAME_INDEX].1,
    year: DATES[PREVIEW_FRAME_INDEX].2,
    hour: HOURS[PREVIEW_FRAME_INDEX] as u32,
    minute: MINUTES[PREVIEW_FRAME_INDEX] as u32,
    second: SECONDS[PREVIEW_FRAME_INDEX] as u32,
};

/// Widoczność ikon STATUS w kolejnych klatkach. Bity: ALARM=1, DND=2,
/// BLUETOOTH=4 (oznacza rozłączenie), LOCK=8.
///
/// Ikony w rzeczywistości pojawiają się rzadko — tylko 5 z 7 oryginalnych klatek
/// ma jakąś ikonę. Klatka 7 (21:37:40) ma wszystkie cztery; klatka 8 jest jej
/// dokładną kopią i zasila `preview.png`.
/// Liczby widocznych ikon są celowo nierośnące: 2,2,0,3,0,1,4(+4).
const STATUS_MASKS: [u8; FRAME_COUNT] = [
    0b1010, // 1: noc, DND i blokada
    0b1010, // 2: DND i blokada
    0b0000, // 3: czysty poranek
    0b1011, // 4: alarm + DND + blokada
    0b0000, // 5: pusto
    0b0100, // 6: chwilowe rozłączenie Bluetooth
    0b1111, // 7: pełny zestaw (21:37:40)
    0b1111, // 8: kopia klatki 7 (→ preview.png)
];

/// Która wartość ma być wymuszona na skrajną, żeby przetestować układ widgetu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewExtreme {
    MinHeartRate,
    MaxHeartRate,
}

/// Zahardkodowany szkielet klatki — bez wartości losowych.
#[derive(Debug, Clone, Copy)]
pub struct PreviewFramePreset {
    pub index: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Prawdziwa data klatki: poniedziałek–niedziela, 7 różnych miesięcy 2022.
    pub date: PreviewDate,
    /// Maska widocznych ikon STATUS (`ALARM=1, DND=2, BLUETOOTH=4, LOCK=8`).
    pub status_mask: u8,
    force_extreme: Option<PreviewExtreme>,
}

const fn make_preset(index: usize) -> PreviewFramePreset {
    PreviewFramePreset {
        index: index as u8,
        hour: HOURS[index],
        minute: MINUTES[index],
        second: SECONDS[index],
        date: PreviewDate {
            day: DATES[index].0,
            month: DATES[index].1,
            year: DATES[index].2,
        },
        status_mask: STATUS_MASKS[index],
        force_extreme: if index == 0 {
            Some(PreviewExtreme::MinHeartRate)
        } else if index >= SOURCE_FRAME_COUNT - 1 {
            Some(PreviewExtreme::MaxHeartRate)
        } else {
            None
        },
    }
}

/// (dzień, miesiąc 1-based, rok) — realne daty 2022-01-03 … 2022-07-31,
/// kolejno poniedziałek … niedziela i kolejno styczeń … lipiec.
const DATES: [(u32, u32, i32); FRAME_COUNT] = [
    (3, 1, 2022),
    (8, 2, 2022),
    (9, 3, 2022),
    (14, 4, 2022),
    (20, 5, 2022),
    (25, 6, 2022),
    (31, 7, 2022),
    (31, 7, 2022),
];

/// 7 oryginalnych stanów tarczy (godziny rosną, minuty i sekundy nigdy nie cofają
/// się, każda klatka ma inny prawdziwy dzień tygodnia) + kopia ostatniej z nich.
pub const FRAME_PRESETS: [PreviewFramePreset; FRAME_COUNT] = [
    make_preset(0),
    make_preset(1),
    make_preset(2),
    make_preset(3),
    make_preset(4),
    make_preset(5),
    make_preset(6),
    make_preset(PREVIEW_FRAME_INDEX),
];

/// Data klatki — z predefiniowanego kalendarza 7 prawdziwych dat.
/// Wszystkie widgety daty wewnątrz klatki korzystają z tego samego wpisu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDate {
    pub day: u32,
    /// 1-based (tak jak `Date.getDate()` w ZeppOS).
    pub month: u32,
    pub year: i32,
}

/// Jednorazowa seria wylosowanych danych zwracana przez FFI do edytora.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRandomSeries {
    pub dates: Vec<PreviewDate>,
    pub heart_rates: Vec<u16>,
    pub calories: Vec<u16>,
    pub distances_m: Vec<u32>,
    pub steps_per_km: Vec<u16>,
    pub stress: Vec<u8>,
    pub pai: Vec<u16>,
    pub skin_temp_c: Vec<f32>,
    pub sleep_minutes: Vec<u32>,
    pub spo2: Vec<u8>,
    /// Liczba klatek — C# czyta ją, żeby nie mieć własnej, rozjechanej stałej.
    #[serde(default = "default_frame_count")]
    pub frame_count: usize,
    /// Język nazw miesięcy i oznaczeń AM/PM (`"pl"` albo `"en"`).
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "en".to_string()
}

fn default_frame_count() -> usize {
    FRAME_COUNT
}

/// Finalny, rozwiązany stan pojedynczej klatki przekazywany do renderera.
#[derive(Debug, Clone)]
pub struct PreviewFrameData {
    pub index: usize,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Maska widocznych ikon STATUS w tej klatce.
    pub status_mask: u8,
    pub date: PreviewDate,
    pub battery_pct: u8,
    pub heart_rate: u16,
    pub calories: u16,
    pub distance_m: u32,
    pub steps: u32,
    pub steps_per_km: u16,
    pub stress: u8,
    pub pai: u16,
    pub skin_temp_c: f32,
    pub sleep_minutes: u32,
    pub spo2: u8,
    /// Dzienne cele są stałe w ciągu dnia — nie podlegają losowaniu.
    pub steps_target: u32,
    pub calories_target: u16,
    pub stand_target: u8,
    pub fat_burning_target: u8,
    /// Bieżące wartości z czujników (na PC stabilne, zależne od klatki).
    pub stand_hours: u8,
    pub fat_burning_minutes: u16,
    pub pai_weekly: u16,
    /// Migawka pogody dla widgetów z kategorii Pogoda.
    pub weather: WeatherSnapshot,
}

/// Jedna, deterministyczna migawka pogody w klatce podglądu.
/// `index` to kod pogody ZeppOS v1.0 (0..=28) — ten sam wybiera ikonę i nazwę.
#[derive(Debug, Clone, Copy)]
pub struct WeatherSnapshot {
    /// Nazwa miasta w języku angielskim — Rust tłumaczy ją przez `locale::city_name`.
    pub city: &'static str,
    pub index: u8,
    pub high: i32,
    pub low: i32,
    pub sunrise_hour: u8,
    pub sunrise_minute: u8,
    pub sunset_hour: u8,
    pub sunset_minute: u8,
}

/// Domyślny stan pogody w podglądzie edytora oraz fallback dla Custom Data:
/// pochmurna Warszawa (~20 °C w ciągu dnia). Na zegarku wartości pochodzą z
/// czujnika WEATHER — ten wzorzec dotyczy wyłącznie podglądu na PC.
pub const DEFAULT_PREVIEW_WEATHER: WeatherSnapshot = WeatherSnapshot {
    city: "Warszawa",
    index: 0,
    high: 22,
    low: 16,
    sunrise_hour: 5,
    sunrise_minute: 13,
    sunset_hour: 20,
    sunset_minute: 40,
};

/// Stolice Europy w 7 oryginalnych klatkach. Klatka 7 (21:37:40, źródło
/// `preview.png`) to Warszawa; klatka 8 jest jej dokładną kopią.
pub const WEATHER_FRAMES: [WeatherSnapshot; FRAME_COUNT] = [
    WeatherSnapshot { city: "Berlin",  index: 0,  high: 4,  low: -1, sunrise_hour: 7,  sunrise_minute: 26, sunset_hour: 16, sunset_minute: 42 },
    WeatherSnapshot { city: "Paris",   index: 5,  high: 11, low: 5,  sunrise_hour: 7,  sunrise_minute: 33, sunset_hour: 17, sunset_minute: 53 },
    WeatherSnapshot { city: "London",  index: 1,  high: 14, low: 8,  sunrise_hour: 6,  sunrise_minute: 42, sunset_hour: 19, sunset_minute: 5 },
    WeatherSnapshot { city: "Rome",    index: 3,  high: 25, low: 14, sunrise_hour: 6,  sunrise_minute: 12, sunset_hour: 20, sunset_minute: 2 },
    WeatherSnapshot { city: "Madrid",  index: 14, high: 32, low: 18, sunrise_hour: 6,  sunrise_minute: 32, sunset_hour: 21, sunset_minute: 21 },
    WeatherSnapshot { city: "Prague",  index: 6,  high: 2,  low: -3, sunrise_hour: 7,  sunrise_minute: 54, sunset_hour: 16, sunset_minute: 12 },
    WeatherSnapshot { city: "Warsaw",  index: 0,  high: 22, low: 16, sunrise_hour: 5,  sunrise_minute: 13, sunset_hour: 20, sunset_minute: 40 },
    WeatherSnapshot { city: "Warsaw",  index: 0,  high: 22, low: 16, sunrise_hour: 5,  sunrise_minute: 13, sunset_hour: 20, sunset_minute: 40 },
];

/// Rozwiązany stan pogody dla podglądu na żywo. W przeciwieństwie do
/// `WeatherSnapshot` ma własny `String` (pole miasta może pochodzić z Custom Data).
#[derive(Debug, Clone)]
pub struct WeatherPreviewState {
    pub city: String,
    /// Pola dnia 0 — zachowane dla zgodności z istniejącym API FFI i testami.
    pub index: u8,
    pub high: i32,
    pub low: i32,
    pub sunrise_hour: u8,
    pub sunrise_minute: u8,
    pub sunset_hour: u8,
    pub sunset_minute: u8,
    /// Deterministyczna prognoza na 5 dni (indeks 0 = dziś).
    pub forecast: Vec<WeatherSnapshot>,
    /// Dzień wybrany w ustawieniach widgetu (0…4).
    pub day_offset: u8,
    /// Minuta dnia używana przez `sun-current` (domyślnie 21:37 z predefiniowanej klatki).
    pub now_minute: i32,
    /// Wartość zastępcza temperatury bieżącej na PC. Tarcza czyta ją z firmware'u
    /// przez `hmUI.data_type.WEATHER_CURRENT` — podgląd nie ma dostępu do tego wiązania.
    pub current: i32,
}

impl WeatherPreviewState {
    /// Dzień faktycznie wyświetlany przez widget; brak wpisu oznacza `--`.
    pub fn resolved_day(&self) -> Option<&WeatherSnapshot> {
        self.forecast.get(self.day_offset as usize)
    }
}

/// Uzupełnia dni 1…4 deterministycznie na podstawie dnia 0. Zegarek czyta
/// `forecastData.data[i]` — podgląd na PC nie ma czujnika, więc dane kolejnych
/// dni muszą być przewidywalne i niezależne od Custom Data.
pub fn forecast_days(base: WeatherSnapshot) -> Vec<WeatherSnapshot> {
    (0..5)
        .map(|day| {
            if day == 0 {
                base
            } else {
                let day = day as i32;
                WeatherSnapshot {
                    city: base.city,
                    index: ((base.index as usize + day as usize * 3) % 29) as u8,
                    high: (base.high + day * 2 - 3).clamp(-99, 99),
                    low: (base.low + day - 2).clamp(-99, 99),
                    sunrise_hour: (base.sunrise_hour + day as u8) % 24,
                    sunrise_minute: (base.sunrise_minute + day as u8 * 3) % 60,
                    sunset_hour: (base.sunset_hour + day as u8) % 24,
                    sunset_minute: (base.sunset_minute + day as u8 * 2) % 60,
                }
            }
        })
        .collect()
}

impl From<WeatherSnapshot> for WeatherPreviewState {
    fn from(value: WeatherSnapshot) -> Self {
        let forecast = forecast_days(value);
        let day = forecast[0];
        Self {
            city: day.city.to_string(),
            index: day.index,
            high: day.high,
            low: day.low,
            sunrise_hour: day.sunrise_hour,
            sunrise_minute: day.sunrise_minute,
            sunset_hour: day.sunset_hour,
            sunset_minute: day.sunset_minute,
            forecast,
            day_offset: 0,
            now_minute: 21 * 60 + 37,
            current: day.high,
        }
    }
}

/// Minuty do najbliższego wschodu albo zachodu. Szukamy zdarzenia w dzisiejszej
/// albo jutrzejszej prognozie — dokładnie tak samo robi helper `_sunCurrentText()`
/// w generowanym JS (bez użycia `Date`).
pub fn sun_current_minutes(forecast: &[WeatherSnapshot], now_minute: i32) -> Option<i32> {
    if now_minute < 0 || now_minute >= 1440 || forecast.is_empty() {
        return None;
    }
    let mut next: Option<i32> = None;
    for (day, snapshot) in forecast.iter().take(2).enumerate() {
        for event in [
            snapshot.sunrise_hour as i32 * 60 + snapshot.sunrise_minute as i32,
            snapshot.sunset_hour as i32 * 60 + snapshot.sunset_minute as i32,
        ] {
            let at = day as i32 * 1440 + event;
            if at >= now_minute && next.map_or(true, |current| at < current) {
                next = Some(at);
            }
        }
    }
    next.map(|at| at - now_minute)
}

fn override_i64(value: &serde_json::Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|item| {
        item.as_i64().or_else(|| item.as_str().and_then(|text| text.trim().parse().ok()))
    })
}

/// Parsuje jedno pole czasu `HH:MM` (z Custom Data). Błędne wpisy są ignorowane,
/// więc podgląd wraca do wartości domyślnej zamiast pokazywać uszkodzony tekst.
fn override_time(value: &serde_json::Value, key: &str) -> Option<(u8, u8)> {
    let text = value.get(key)?.as_str()?.trim();
    let mut parts = text.split(':');
    let hour = parts.next()?.trim().parse::<i64>().ok()?;
    let minute = parts.next()?.trim().parse::<i64>().ok()?;
    if parts.next().is_some() || !(0..=23).contains(&hour) || !(0..=59).contains(&minute) {
        return None;
    }
    Some((hour as u8, minute as u8))
}

/// Nakłada wartości z Custom Data na wzorzec (puste/brakujące pola zostają
/// domyślne — dzięki temu podgląd zawsze ma komplet danych).
impl WeatherPreviewState {
    pub fn with_overrides(mut self, overrides: &serde_json::Value) -> Self {
        // Dzień wybranego widgetu — ten sam indeks trafia do `forecastData.data[N]`.
        if let Some(day) = override_i64(overrides, "dayOffset") {
            self.day_offset = day.clamp(0, 4) as u8;
        }
        if let Some(minute) = override_i64(overrides, "nowMinute") {
            self.now_minute = minute.clamp(0, 1439) as i32;
        }
        if let Some(current) = override_i64(overrides, "current") {
            self.current = current.clamp(-99, 99) as i32;
        }
        if let Some(city) = overrides.get("city").and_then(|v| v.as_str()) {
            if !city.trim().is_empty() {
                self.city = city.trim().to_string();
            }
        }
        if let Some(day) = self.forecast.get_mut(self.day_offset as usize) {
            if let Some(index) = override_i64(overrides, "index") {
                day.index = index.clamp(0, 28) as u8;
            }
            if let Some(high) = override_i64(overrides, "high") {
                day.high = high.clamp(-99, 99) as i32;
            }
            if let Some(low) = override_i64(overrides, "low") {
                day.low = low.clamp(-99, 99) as i32;
            }
            if let Some(hour) = override_i64(overrides, "sunriseHour") {
                day.sunrise_hour = hour.clamp(0, 23) as u8;
            }
            if let Some(minute) = override_i64(overrides, "sunriseMinute") {
                day.sunrise_minute = minute.clamp(0, 59) as u8;
            }
            if let Some(hour) = override_i64(overrides, "sunsetHour") {
                day.sunset_hour = hour.clamp(0, 23) as u8;
            }
            if let Some(minute) = override_i64(overrides, "sunsetMinute") {
                day.sunset_minute = minute.clamp(0, 59) as u8;
            }
            // Jedno pole `HH:MM` zastępuje dawne pary godzina/minuta.
            if let Some((hour, minute)) = override_time(overrides, "sunrise") {
                day.sunrise_hour = hour;
                day.sunrise_minute = minute;
            }
            if let Some((hour, minute)) = override_time(overrides, "sunset") {
                day.sunset_hour = hour;
                day.sunset_minute = minute;
            }
        }
        // Pola „mirror" dla istniejących konsumentów FFI i testów.
        if let Some(day) = self.resolved_day().copied() {
            self.index = day.index;
            self.high = day.high;
            self.low = day.low;
            self.sunrise_hour = day.sunrise_hour;
            self.sunrise_minute = day.sunrise_minute;
            self.sunset_hour = day.sunset_hour;
            self.sunset_minute = day.sunset_minute;
        }
        self
    }
}

/// Tekst pola pogody — jedyna implementacja używana zarówno przez renderer
/// klatek, jak i przez FFI podglądu na żywo (`rw_format_weather_field`).
pub fn weather_text(kind: &str, state: &WeatherPreviewState, locale: WatchfaceLocale) -> String {
    let Some(day) = state.resolved_day() else {
        return match kind {
            "weather-icon" => String::new(),
            "sun-current" => "--:--".to_string(),
            "weather-current" => "--°".to_string(),
            _ => "--".to_string(),
        };
    };
    let unknown = if locale == WatchfaceLocale::Polish {
        "Nieznane"
    } else {
        "Unknown"
    };
    match kind {
        "weather-condition" => locale
            .weather_names()
            .get(day.index as usize)
            .copied()
            .unwrap_or(unknown)
            .to_string(),
        "weather-city" => locale.city_name(&state.city),
        "weather-current" => format!("{}°", state.current),
        "weather-high" => format!("{}°", day.high),
        "weather-low" => format!("{}°", day.low),
        "sunrise" => format!("{:02}:{:02}", day.sunrise_hour, day.sunrise_minute),
        "sunset" => format!("{:02}:{:02}", day.sunset_hour, day.sunset_minute),
        "sun-current" => sun_current_minutes(&state.forecast, state.now_minute)
            .map(|left| format!("{}:{:02}", left / 60, left % 60))
            .unwrap_or_else(|| "--:--".to_string()),
        // Ikona nie ma żadnego tekstu — wcześniej fallback zwracał "--", który
        // edytor rysował w ramce jako artifact przy każdym przeskalowaniu.
        "weather-icon" => String::new(),
        _ => "--".to_string(),
    }
}

/// Losuje 7 oryginalnych klatek i dokleja dokładną kopię ostatniej (21:37:40).
/// `seed` pozwala uzyskać powtarzalny wynik w testach.
pub fn random_series(seed: u64, locale: &str) -> PreviewRandomSeries {
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut dates = Vec::with_capacity(FRAME_COUNT);
    let mut heart_rates = Vec::with_capacity(FRAME_COUNT);
    let mut calories = Vec::with_capacity(FRAME_COUNT);
    let mut distances_m = Vec::with_capacity(FRAME_COUNT);
    let mut steps_per_km = Vec::with_capacity(FRAME_COUNT);
    let mut stress = Vec::with_capacity(FRAME_COUNT);
    let mut pai = Vec::with_capacity(FRAME_COUNT);
    let mut skin_temp_c = Vec::with_capacity(FRAME_COUNT);
    let mut sleep_minutes = Vec::with_capacity(FRAME_COUNT);
    let mut spo2 = Vec::with_capacity(FRAME_COUNT);

    let mut heart = 64u16;
    let mut kcal = 0u16;
    let mut metres = 0u32;
    let mut pai_value = 0u16;

    for (index, preset) in FRAME_PRESETS.iter().take(SOURCE_FRAME_COUNT).enumerate() {
        dates.push(preset.date);

        heart = match preset.force_extreme {
            Some(PreviewExtreme::MinHeartRate) => 56,
            Some(PreviewExtreme::MaxHeartRate) => 120,
            None => (heart as i32 + rng.i32(-7..=7)).clamp(56, 118) as u16,
        };
        heart_rates.push(heart);

        // Narastające, ale z losowym wahaniem wokół równomiernego przebiegu tygodnia.
        let kcal_target =
            (2450.0 * index as f64 / (SOURCE_FRAME_COUNT - 1) as f64).round() as i32;
        kcal = (kcal_target + rng.i32(-45..=45)).max(kcal as i32).min(2450) as u16;
        if index == 0 {
            kcal = 0;
        }
        if index == SOURCE_FRAME_COUNT - 1 {
            kcal = 2450;
        }
        calories.push(kcal);

        let metre_target =
            (12300.0 * index as f64 / (SOURCE_FRAME_COUNT - 1) as f64).round() as i32;
        metres = (metre_target + rng.i32(-220..=220))
            .max(metres as i32)
            .min(12300) as u32;
        if index == 0 {
            metres = 0;
        }
        if index == SOURCE_FRAME_COUNT - 1 {
            metres = 12300;
        }
        distances_m.push(metres);
        steps_per_km.push(rng.u16(1300..=1400));

        stress.push(rng.u8(3..=92));
        pai_value = (pai_value + rng.u16(0..=16)).min(268);
        pai.push(pai_value);
        skin_temp_c.push(36.2 + rng.f32() * 1.0);
        sleep_minutes.push(if (1..SOURCE_FRAME_COUNT).contains(&index) {
            rng.u32(300..=420)
        } else if index >= SOURCE_FRAME_COUNT - 2 {
            rng.u32(180..=390)
        } else {
            0
        });
        spo2.push(rng.u8(95..=100));
    }

    // Ostatnia klatka to kopia 21:37:40 — identyczne dane, więc statyczny
    // podgląd i GIF kończą się dokładnie tym samym stanem tarczy.
    let last = SOURCE_FRAME_COUNT - 1;
    let copy = SOURCE_FRAME_COUNT;
    let date_copy = dates[last];
    dates.push(date_copy);
    heart_rates.push(heart_rates[last]);
    calories.push(calories[last]);
    distances_m.push(distances_m[last]);
    steps_per_km.push(steps_per_km[last]);
    stress.push(stress[last]);
    pai.push(pai[last]);
    skin_temp_c.push(skin_temp_c[last]);
    sleep_minutes.push(sleep_minutes[last]);
    spo2.push(spo2[last]);
    debug_assert_eq!(copy, PREVIEW_FRAME_INDEX);

    PreviewRandomSeries {
        dates,
        heart_rates,
        calories,
        distances_m,
        steps_per_km,
        stress,
        pai,
        skin_temp_c,
        sleep_minutes,
        spo2,
        frame_count: FRAME_COUNT,
        locale: locale.to_string(),
    }
}

/// Seed oparty na zegarze i PID — kolejne eksporty dają inne dane.
pub fn random_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ (std::process::id() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Łączy zahardkodowany preset z wylosowaną serią i wylicza spójne kroki.
pub fn frame_data(index: usize, series: &PreviewRandomSeries) -> Result<PreviewFrameData, String> {
    if index >= FRAME_COUNT {
        return Err(format!("Nieprawidłowy indeks klatki: {index}"));
    }
    let expected = [
        series.dates.len(),
        series.heart_rates.len(),
        series.calories.len(),
        series.distances_m.len(),
        series.steps_per_km.len(),
        series.stress.len(),
        series.pai.len(),
        series.skin_temp_c.len(),
        series.sleep_minutes.len(),
        series.spo2.len(),
    ];
    if expected.iter().any(|len| *len != FRAME_COUNT) {
        return Err("Seria danych podglądu musi zawierać dokładnie 7 wpisów".to_string());
    }

    let preset = FRAME_PRESETS[index];
    let distance_m = series.distances_m[index];
    let steps_per_km = series.steps_per_km[index];
    let steps = (distance_m as f64 / 1000.0 * steps_per_km as f64).round() as u32;
    // Bateria maleje liniowo 100% -> 10% i nie jest losowana. Ostatnia klatka
    // jest kopią poprzedniej, więc nie może dostać innego poziomu baterii.
    let source_index = index.min(SOURCE_FRAME_COUNT - 1);
    let battery_pct =
        100 - (source_index as u32 * 90 / (SOURCE_FRAME_COUNT as u32 - 1)) as u8;

    Ok(PreviewFrameData {
        index,
        hour: preset.hour,
        minute: preset.minute,
        second: preset.second,
        status_mask: preset.status_mask,
        date: series.dates[index],
        battery_pct,
        heart_rate: series.heart_rates[index],
        calories: series.calories[index],
        distance_m,
        steps,
        steps_per_km,
        stress: series.stress[index],
        pai: series.pai[index],
        skin_temp_c: series.skin_temp_c[index],
        sleep_minutes: series.sleep_minutes[index],
        spo2: series.spo2[index],
        steps_target: 10_000,
        calories_target: 2_000,
        stand_target: 12,
        fat_burning_target: 45,
        stand_hours: 3 + source_index as u8,
        fat_burning_minutes: 12 + source_index as u16 * 4,
        pai_weekly: 120 + source_index as u16 * 24,
        weather: WEATHER_FRAMES[index],
    })
}

/// Wartość tekstowa widgetu w danej klatce — te same wzorce co podgląd i JS.
pub fn text_for_layer(
    layer: &LayerView,
    frame: &PreviewFrameData,
    project: &crate::model::Project,
    locale: WatchfaceLocale,
) -> String {
    let unit = layer.show_unit;
    match layer.kind.as_str() {
        "time" | "analog-time" => {
            let pattern = layer
                .time_format
                .as_deref()
                .unwrap_or("HH:MM:SS");
            time_format::format_with_locale(
                pattern,
                frame.hour as u32,
                frame.minute as u32,
                frame.second as u32,
                locale,
            )
            .unwrap_or_else(|_| layer.text.clone().unwrap_or_default())
        }
        "date" => {
            let pattern = layer
                .time_format
                .as_deref()
                .unwrap_or(crate::date_format::DEFAULT_PATTERN);
            // `date_format` przyjmuje miesiąc 0-based (jak Date.getMonth()).
            date_format::format_with_locale(
                pattern,
                frame.date.day,
                frame.date.month.saturating_sub(1),
                frame.date.year,
                locale,
            )
            .unwrap_or_else(|_| layer.text.clone().unwrap_or_default())
        }
        "weekday-number" => date_format::weekday_number(
            frame.date.day,
            frame.date.month,
            frame.date.year,
            project.first_day_of_week,
        )
        .to_string(),
        "week-number" => date_format::iso_week(frame.date.day, frame.date.month, frame.date.year).to_string(),
        "day-of-year" => {
            date_format::day_of_year(frame.date.day, frame.date.month, frame.date.year).to_string()
        }
        // ZeppOS nie udostępnia w tarczy danych księżycowych w formacie do
        // wiarygodnego policzenia w PC — uczciwie pokazujemy placeholder.
        "lunar-date" | "festival" => "--".to_string(),
        "world-clock" => format!("{:02}:{:02}", (frame.hour as u32 + 8) % 24, frame.minute),
        "steps" => frame.steps.to_string(),
        "distance" => {
            let (system, suffix) = match layer.measurement_system {
                Some(ref value) => {
                    let parsed = MeasurementSystem::parse(value).unwrap_or(project.measurement_system);
                    (parsed, parsed.suffix())
                }
                None => (project.measurement_system, project.measurement_system.suffix()),
            };
            format!(
                "{:.2} {}",
                frame.distance_m as f64 / system.meters_per_unit(),
                suffix
            )
        }
        "heart" => {
            if unit {
                format!("{} bpm", frame.heart_rate)
            } else {
                frame.heart_rate.to_string()
            }
        }
        "heart-min" => {
            let value = frame.heart_rate.saturating_sub(12).max(1);
            if unit {
                format!("{value} bpm")
            } else {
                value.to_string()
            }
        }
        "heart-max" => {
            let value = frame.heart_rate.saturating_add(18);
            if unit {
                format!("{value} bpm")
            } else {
                value.to_string()
            }
        }
        "heart-avg" => {
            let value = frame.heart_rate.saturating_add(3);
            if unit {
                format!("{value} bpm")
            } else {
                value.to_string()
            }
        }
        "stress" => frame.stress.to_string(),
        "pai" => frame.pai.to_string(),
        "skin-temp" => {
            if unit {
                format!("{:.1} °C", frame.skin_temp_c)
            } else {
                format!("{:.1}", frame.skin_temp_c)
            }
        }
        "sleep" => format!("{}:{:02}", frame.sleep_minutes / 60, frame.sleep_minutes % 60),
        "sleep-score" => (60 + frame.sleep_minutes / 10).min(100).to_string(),
        "sleep-deep" | "sleep-light" | "sleep-rem" | "sleep-wake" => {
            let minutes = match layer.kind.as_str() {
                "sleep-deep" => frame.sleep_minutes / 5,
                "sleep-light" => frame.sleep_minutes / 2,
                "sleep-wake" => frame.sleep_minutes / 8,
                _ => frame.sleep_minutes / 4,
            };
            if layer.sleep_stage_mode == "percent" {
                let percent = if frame.sleep_minutes == 0 {
                    0
                } else {
                    minutes * 100 / frame.sleep_minutes
                };
                format!("{percent}%")
            } else {
                format!("{minutes} min")
            }
        }
        "sleep-start" => "23:40".to_string(),
        "sleep-end" => {
            let minutes = (23 * 60 + 40 + frame.sleep_minutes) % 1440;
            format!("{:02}:{:02}", minutes / 60, minutes % 60)
        }
        "calories" => {
            if unit {
                format!("{} kcal", frame.calories)
            } else {
                frame.calories.to_string()
            }
        }
        "spo2" => {
            if unit {
                format!("{}%", frame.spo2)
            } else {
                frame.spo2.to_string()
            }
        }
        "spo2-hour-avg" => {
            let value = frame.spo2.saturating_sub(1).max(1);
            if unit {
                format!("{value}%")
            } else {
                value.to_string()
            }
        }
        "stand" => {
            if unit {
                format!("{} h", frame.stand_hours)
            } else {
                frame.stand_hours.to_string()
            }
        }
        "fat-burning" => {
            if unit {
                format!("{} min", frame.fat_burning_minutes)
            } else {
                frame.fat_burning_minutes.to_string()
            }
        }
        "pai-weekly" => frame.pai_weekly.to_string(),
        "pai-prepai" => (0u16..7)
            .map(|day| frame.pai.saturating_sub(day * 3) as u32)
            .sum::<u32>()
            .to_string(),
        "sleep-target" => "-- h".to_string(),
        "weather-condition" | "weather-city" | "weather-current" | "weather-high"
        | "weather-low" | "sunrise" | "sunset" | "sun-current" => {
            let mut state = WeatherPreviewState::from(frame.weather);
            if layer.kind == "sun-current" {
                // „Czas do" jest z definicji bieżący — nie zależy od dnia prognozy.
                state.day_offset = 0;
                state.now_minute = frame.hour as i32 * 60 + frame.minute as i32;
            } else {
                state.day_offset = layer.weather_day_offset.min(4);
            }
            weather_text(layer.kind.as_str(), &state, locale)
        }
        "battery" => format!("{}%", frame.battery_pct),
        "steps-target" => frame.steps_target.to_string(),
        "calories-target" => {
            if unit {
                format!("{} kcal", frame.calories_target)
            } else {
                frame.calories_target.to_string()
            }
        }
        "stand-target" => {
            if unit {
                format!("{} h", frame.stand_target)
            } else {
                frame.stand_target.to_string()
            }
        }
        "fat-burning-target" => {
            if unit {
                format!("{} min", frame.fat_burning_target)
            } else {
                frame.fat_burning_target.to_string()
            }
        }
        _ => layer.text.clone().unwrap_or_default(),
    }
}

fn parse_argb(raw: Option<&str>) -> Rgba<u8> {
    let value = raw.unwrap_or("0xFFFFFFFF").trim();
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("#"))
        .unwrap_or(value);
    if hex.len() != 8 {
        return Rgba([255, 255, 255, 255]);
    }
    let a = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let r = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
    Rgba([r, g, b, a])
}

fn blend(canvas: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>, alpha: f32) {
    if x < 0 || y < 0 || x >= SCREEN_SIZE as i32 || y >= SCREEN_SIZE as i32 || alpha <= 0.0 {
        return;
    }
    let index = (y as u32 * SCREEN_SIZE + x as u32) as usize;
    let dst = canvas.as_mut_ptr();
    let source_alpha = (color.0[3] as f32 / 255.0) * alpha;
    if source_alpha <= 0.0 {
        return;
    }
    // Tło jest zawsze nieprzezroczyste (czarne `FILL_RECT`), więc zwykłe
    // alpha-blending daje ten sam efekt co kompozycja ZeppOS.
    unsafe {
        for channel in 0..3 {
            let src = color.0[channel] as f32 / 255.0;
            let back = *dst.add(index * 4 + channel) as f32 / 255.0;
            let mixed = src * source_alpha + back * (1.0 - source_alpha);
            *dst.add(index * 4 + channel) = (mixed * 255.0).round() as u8;
        }
    }
}

fn sample_bilinear(image: &RgbaImage, x: f64, y: f64) -> Rgba<u8> {
    let (w, h) = image.dimensions();
    if x < 0.0 || y < 0.0 || x >= w as f64 - 1.0 || y >= h as f64 - 1.0 {
        return Rgba([0, 0, 0, 0]);
    }
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let fx = (x - x0 as f64) as f32;
    let fy = (y - y0 as f64) as f32;
    let p00 = image.get_pixel(x0, y0).0;
    let p10 = image.get_pixel(x0 + 1, y0).0;
    let p01 = image.get_pixel(x0, y0 + 1).0;
    let p11 = image.get_pixel(x0 + 1, y0 + 1).0;
    let mut out = [0u8; 4];
    for channel in 0..4 {
        let top = p00[channel] as f32 * (1.0 - fx) + p10[channel] as f32 * fx;
        let bottom = p01[channel] as f32 * (1.0 - fx) + p11[channel] as f32 * fx;
        out[channel] = (top * (1.0 - fy) + bottom * fy).round() as u8;
    }
    Rgba(out)
}

/// Obraca kwadratowy obraz wskazówki wokół środka. Dodatni kąt = zgodnie z ruchem zegara.
pub(crate) fn rotate_hand(src: &RgbaImage, degrees: f64) -> RgbaImage {
    let (w, h) = src.dimensions();
    let mut out = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let angle = degrees.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    for y in 0..h {
        for x in 0..w {
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - cy;
            let sx = cos * dx + sin * dy + cx - 0.5;
            let sy = -sin * dx + cos * dy + cy - 0.5;
            out.put_pixel(x, y, sample_bilinear(src, sx, sy));
        }
    }
    out
}

/// Stosuje krycie widgetu do już wczytanego obrazu (0 = niewidoczny, 255 = bez zmian).
pub(crate) fn apply_alpha(image: &mut RgbaImage, alpha: u8) {
    if alpha == 255 {
        return;
    }
    for pixel in image.pixels_mut() {
        let scaled = (pixel.0[3] as u16 * alpha as u16 / 255) as u8;
        pixel.0[3] = scaled;
    }
}

fn overlay_asset(
    canvas: &mut RgbaImage,
    project_dir: &Path,
    src: Option<&str>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    resize: bool,
    alpha: u8,
) {
    let src = match src {
        Some(value) if !value.is_empty() => value,
        _ => return,
    };
    let path = project_dir.join("assets").join(src);
    let image = match image::open(path) {
        Ok(image) => image.to_rgba8(),
        Err(_) => return,
    };
    if resize && image.dimensions() != (w, h) && w > 0 && h > 0 {
        let mut fitted = imageops::resize(&image, w, h, FilterType::Lanczos3);
        apply_alpha(&mut fitted, alpha);
        imageops::overlay(canvas, &fitted, x as i64, y as i64);
    } else {
        let mut source = image;
        apply_alpha(&mut source, alpha);
        imageops::overlay(canvas, &source, x as i64, y as i64);
    }
}

fn measure_width(font: &FontRef<'_>, scaled: &PxScaleFont<&FontRef<'_>>, text: &str) -> f32 {
    text.chars()
        .map(|c| scaled.h_advance(font.glyph_id(c)))
        .sum()
}

fn layout_lines(
    font: &FontRef<'_>,
    scaled: &PxScaleFont<&FontRef<'_>>,
    text: &str,
    max_width: f32,
    style: &str,
) -> Vec<String> {
    if style == "wrap" {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in text.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if measure_width(font, scaled, &candidate) <= max_width || current.is_empty() {
                current = candidate;
            } else {
                lines.push(std::mem::take(&mut current));
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        return if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
    }

    let mut line = text.to_string();
    if style == "ellipsis" && measure_width(font, scaled, &line) > max_width {
        while line.len() > 1 && measure_width(font, scaled, &format!("{line}…")) > max_width {
            line.pop();
        }
        line.push('…');
    }
    vec![line]
}

/// `ab_glyph::PxScale` jest wysokością komórki fontu, a `text_size`/WinUI `FontSize`
/// to rozmiar em. Przeliczamy skalę tak, żeby klatki PNG/GIF miały identyczne
/// metryki tekstu co podgląd w edytorze.
fn px_scale_for(font: &FontRef<'_>, text_size: f32) -> PxScale {
    let units_per_em = font.units_per_em().unwrap_or(1000.0);
    PxScale::from(text_size.max(1.0) * font.height_unscaled() / units_per_em)
}

fn draw_text(canvas: &mut RgbaImage, project_dir: &Path, layer: &LayerView, text: &str) {
    if text.is_empty() || layer.w <= 0 || layer.h <= 0 {
        return;
    }
    // Widget z przypisanym TTF używa dokładnie tego samego pliku w podglądzie i
    // eksporcie. Brak fontu = czcionka systemowa ZeppOS (Noto Sans) — JS też wtedy
    // pomija `font:`, więc klatki PNG/GIF muszą używać tego samego fontu.
    let project_font = layer
        .font
        .as_deref()
        .filter(|name| !name.is_empty())
        .map(|name| project_dir.join("assets").join("fonts").join(name));
    let font_bytes = project_font
        .as_ref()
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_else(|| SYSTEM_FONT_BYTES.to_vec());
    let font = match FontRef::try_from_slice(&font_bytes) {
        Ok(font) => font,
        Err(_) => return,
    };
    let scale = px_scale_for(&font, layer.text_size.unwrap_or(64) as f32);
    let scaled = font.as_scaled(scale);
    let line_height = (scaled.ascent() - scaled.descent() + scaled.line_gap()).max(1.0);
    let lines = layout_lines(&font, &scaled, text, layer.w as f32, &layer.text_style);

    let total_height = line_height * lines.len() as f32;
    let top = match layer.v_align.as_str() {
        "top" => layer.y as f32,
        "bottom" => layer.y as f32 + layer.h as f32 - total_height,
        _ => layer.y as f32 + (layer.h as f32 - total_height) / 2.0,
    };

    let mut color = parse_argb(layer.color.as_deref());
    color.0[3] = (color.0[3] as u16 * layer.alpha as u16 / 255) as u8;
    for (line_index, line) in lines.iter().enumerate() {
        let width = measure_width(&font, &scaled, line);
        let left = match layer.align.as_deref() {
            Some("left") => layer.x as f32,
            Some("right") => layer.x as f32 + layer.w as f32 - width,
            _ => layer.x as f32 + (layer.w as f32 - width) / 2.0,
        };
        let baseline = top + line_index as f32 * line_height + scaled.ascent();
        let mut pen_x = left;
        for c in line.chars() {
            let id = font.glyph_id(c);
            let advance = scaled.h_advance(id);
            let mut glyph = id.with_scale(scale);
            glyph.position = point(pen_x, baseline);
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                let origin_x = bounds.min.x as i32;
                let origin_y = bounds.min.y as i32;
                outlined.draw(|gx, gy, coverage| {
                    blend(
                        canvas,
                        origin_x + gx as i32,
                        origin_y + gy as i32,
                        color,
                        coverage,
                    );
                });
            }
            pen_x += advance;
        }
    }
}

fn apply_round_cutout(canvas: &mut RgbaImage) {
    let center = (SCREEN_SIZE as f64 - 1.0) / 2.0;
    for y in 0..SCREEN_SIZE {
        for x in 0..SCREEN_SIZE {
            let dx = x as f64 + 0.5 - center - 0.5;
            let dy = y as f64 + 0.5 - center - 0.5;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > SCREEN_RADIUS - 1.0 {
                let factor = ((SCREEN_RADIUS - distance).max(0.0)).min(1.0) as f32;
                let pixel = canvas.get_pixel(x, y);
                let black = Rgba([0, 0, 0, 255]);
                let mut mixed = [0u8; 4];
                for channel in 0..4 {
                    mixed[channel] = (pixel.0[channel] as f32 * factor
                        + black.0[channel] as f32 * (1.0 - factor))
                        .round() as u8;
                }
                canvas.put_pixel(x, y, Rgba(mixed));
            }
        }
    }
}

/// Rysuje jedną klatkę w kolejności warstw z `layers::describe()`.
pub fn render_frame(
    project_dir: &Path,
    project: &crate::model::Project,
    frame: &PreviewFrameData,
    series: &PreviewRandomSeries,
) -> RgbaImage {
    let mut canvas = RgbaImage::from_pixel(SCREEN_SIZE, SCREEN_SIZE, Rgba([0, 0, 0, 255]));
    let locale = WatchfaceLocale::from_code(&series.locale);

    for layer in describe(project) {
        if !layer.visible {
            continue;
        }
        match layer.kind.as_str() {
            "image" => overlay_asset(
                &mut canvas,
                project_dir,
                layer.src.as_deref(),
                layer.x,
                layer.y,
                layer.w as u32,
                layer.h as u32,
                true,
                layer.alpha,
            ),
            "images" => overlay_asset(
                &mut canvas,
                project_dir,
                layer.src.as_deref(),
                layer.x,
                layer.y,
                layer.w as u32,
                layer.h as u32,
                true,
                layer.alpha,
            ),
            "analog-time" => {
                let w = layer.w as u32;
                let h = layer.h as u32;
                let hands = [
                    (
                        layer.hour_src.as_deref(),
                        frame.hour as f64 % 12.0 * 30.0 + frame.minute as f64 * 0.5,
                    ),
                    (
                        layer.minute_src.as_deref(),
                        frame.minute as f64 * 6.0 + frame.second as f64 * 0.1,
                    ),
                    (layer.second_src.as_deref(), frame.second as f64 * 6.0),
                    (layer.cap_src.as_deref(), 0.0),
                ];
                for (src, angle) in hands {
                    let Some(src) = src else { continue };
                    let path = project_dir.join("assets").join(src);
                    let Ok(image) = image::open(path) else { continue };
                    let mut hand = image.to_rgba8();
                    if hand.dimensions() != (w, h) && w > 0 && h > 0 {
                        hand = imageops::resize(&hand, w, h, FilterType::Lanczos3);
                    }
                    apply_alpha(&mut hand, layer.alpha);
                    let rotated = rotate_hand(&hand, angle);
                    imageops::overlay(&mut canvas, &rotated, layer.x as i64, layer.y as i64);
                }
            }
            "status-alarm" | "status-dnd" | "status-bluetooth" | "status-lock" => {
                let status = match layer.kind.as_str() {
                    "status-alarm" => StatusType::Alarm,
                    "status-dnd" => StatusType::Dnd,
                    "status-bluetooth" => StatusType::Bluetooth,
                    _ => StatusType::Lock,
                };
                // Ikona jest rysowana tylko wtedy, gdy dany stan jest aktywny
                // w tej klatce — dokładnie tak zachowuje się IMG_STATUS na zegarku.
                if frame.status_mask & status.bit() == 0 {
                    continue;
                }
                overlay_asset(
                    &mut canvas,
                    project_dir,
                    layer.src.as_deref(),
                    layer.x,
                    layer.y,
                    layer.w as u32,
                    layer.h as u32,
                    true,
                    layer.alpha,
                );
            }
            "weather-icon" => {
                // Ikona jest wybierana z 29 plików przez kod pogody klatki —
                // dokładnie tak, jak JS wybiera element tablicy weatherIcons.
                let set = layer
                    .weather_icon_set
                    .as_deref()
                    .unwrap_or(crate::weather::DEFAULT_WEATHER_PACK);
                let days = forecast_days(frame.weather);
                let day = days
                    .get(layer.weather_day_offset.min(4) as usize)
                    .copied()
                    .unwrap_or(frame.weather);
                let index = day.index.min(28) as usize;
                let src = crate::weather::js_icon_path(set, index);
                overlay_asset(
                    &mut canvas,
                    project_dir,
                    Some(&src),
                    layer.x,
                    layer.y,
                    layer.w as u32,
                    layer.h as u32,
                    true,
                    layer.alpha,
                );
            }
            "arc-progress" | "bar-progress" => {
                draw_progress(&mut canvas, &layer, frame);
            }
            _ => {
                let value = text_for_layer(&layer, frame, project, locale);
                draw_text(&mut canvas, project_dir, &layer, &value);
            }
        }
    }

    apply_round_cutout(&mut canvas);
    canvas
}

/// Zapisuje klatkę jako PNG (pełny, nieprzezroczysty RGBA).
pub fn write_png(image: &RgbaImage, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("Nie można zapisać PNG: {e}"))?;
    image::codecs::png::PngEncoder::new(file)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Nie można zakodować PNG: {e}"))
}

/// Koduje klatki do animowanego GIF-a (~1,7 s na klatkę).
pub fn write_gif(frames: &[RgbaImage], path: &Path) -> Result<(), String> {
    if frames.is_empty() {
        return Err("Brak klatek do zapisu GIF".to_string());
    }
    let file = File::create(path).map_err(|e| format!("Nie można zapisać GIF: {e}"))?;
    let mut encoder = Encoder::new(
        file,
        frames[0].width() as u16,
        frames[0].height() as u16,
        &[],
    )
    .map_err(|e| format!("Nie można utworzyć kodera GIF: {e}"))?;
    // Animacja ma krążyć w nieskończoność — bez tego ZeppOS/Windows
    // odtwarzają 7 klatek tylko raz i zatrzymują się na ostatniej.
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|e| format!("Nie można ustawić zapętlenia GIF: {e}"))?;
    for image in frames {
        let mut pixels = image.as_raw().to_vec();
        let mut frame = Frame::from_rgba_speed(
            image.width() as u16,
            image.height() as u16,
            &mut pixels,
            10,
        );
        // Jednostka to 1/100 s — 170 daje ~1,7 s na klatkę.
        frame.delay = GIF_FRAME_DELAY;
        encoder
            .write_frame(&frame)
            .map_err(|e| format!("Nie można zakodować klatki GIF: {e}"))?;
    }
    Ok(())
}

/// Gotowa zewnętrzna warstwa logo (brązowy łuk + zębatka) z `logo/`.
const GEAR_OUTLAYER_PNG: &[u8] = include_bytes!("../../../logo/gear-outlayer.png");
/// Promienie łuku w źródłowym rysunku 1024 × 1024 wokół środka (512,512).
const GEAR_OUTER_RADIUS: f64 = 333.6;
const GEAR_INNER_RADIUS: f64 = 275.0;
const RING_BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
/// Ciemny obrys tarczy (#404040) — ma być ledwo widoczny na tle tarczy.
const RING_OUTLINE: Rgba<u8> = Rgba([64, 64, 64, 255]);

// Promienie kolejnych stref jako ułamek połowy boku kwadratu:
// przezroczyste tło → czarna strefa bezpieczna → zewnętrzna warstwa logo
// → wewnętrzna czarna strefa → przeskalowana tarcza z ciemnym obrysem.
const R_EDGE: f64 = 0.98;
const R_BROWN_OUT: f64 = 0.90;
const R_FACE: f64 = 0.66;

/// Wypełnienie koła o promieniu `radius` w odległości `distance` (1 px wygładzenie).
fn disc_coverage(radius: f64, distance: f64) -> f64 {
    (radius - distance + 0.5).clamp(0.0, 1.0)
}

fn annulus_coverage(distance: f64, inner: f64, outer: f64) -> f64 {
    disc_coverage(outer, distance) * (1.0 - disc_coverage(inner, distance))
}

/// Straight-alpha `source-over`: kładzie `color` z kryciem `factor` na `dst`.
fn paint_over(dst: &mut Rgba<u8>, color: Rgba<u8>, factor: f64) {
    if factor <= 0.0 {
        return;
    }
    let src_alpha = (color.0[3] as f64 / 255.0) * factor;
    if src_alpha <= 0.0 {
        return;
    }
    let dst_alpha = dst.0[3] as f64 / 255.0;
    let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);
    if out_alpha <= 0.0 {
        *dst = Rgba([0, 0, 0, 0]);
        return;
    }
    for channel in 0..3 {
        let src = color.0[channel] as f64 * src_alpha;
        let back = dst.0[channel] as f64 * dst_alpha * (1.0 - src_alpha);
        dst.0[channel] = ((src + back) / out_alpha).round().clamp(0.0, 255.0) as u8;
    }
    dst.0[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

/// Krycie prostokąta z zaokrąglonymi rogami (`FILL_RECT.radius`).
fn rounded_rect_coverage(x: i32, y: i32, w: i32, h: i32, radius: u32, px: i32, py: i32) -> f64 {
    if w <= 0 || h <= 0 || px < x || py < y || px >= x + w || py >= y + h {
        return 0.0;
    }
    let r = (radius as i32).min(w.min(h) / 2).max(0);
    if r == 0 {
        return 1.0;
    }
    let cx = if px < x + r {
        x + r
    } else if px >= x + w - r {
        x + w - r - 1
    } else {
        px
    };
    let cy = if py < y + r {
        y + r
    } else if py >= y + h - r {
        y + h - r - 1
    } else {
        py
    };
    let distance = (((px - cx) as f64).powi(2) + ((py - cy) as f64).powi(2)).sqrt();
    disc_coverage(r as f64, distance)
}

/// Krycie łuku o zadanym kącie (0° = góra, zgodnie z `ARC_PROGRESS`).
fn arc_coverage(
    cx: i32,
    cy: i32,
    radius: i32,
    thickness: u32,
    start: i32,
    end: i32,
    rounded: bool,
    px: i32,
    py: i32,
) -> f64 {
    let dx = px as f64 - cx as f64;
    let dy = py as f64 - cy as f64;
    let distance = (dx * dx + dy * dy).sqrt();
    let cap = thickness as f64 / 2.0;
    let ring = annulus_coverage(
        distance,
        radius as f64 - cap,
        radius as f64 + cap,
    );
    if ring <= 0.0 {
        return 0.0;
    }
    let sweep = (end - start) as f64;
    // Tak samo jak `ArcGeometry` w edytorze: |sweep| >= 359.999 to pełne koło,
    // a ujemny sweep to łuk rysowany przeciwnie do ruchu wskazówek zegara.
    let full = sweep.abs() >= 359.999;
    let span = if full { 360.0 } else { sweep.abs() };
    if full {
        // Zamknięty okrąg nie ma końców do zaokrąglenia.
        return ring;
    }
    // Odwrotność `x = cx + r*sin(θ)`, `y = cy - r*cos(θ)` (0° = góra).
    // `atan2(-dy, dx)` dawał inny układ niż kapki i odrywał je od łuku.
    let angle = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
    let from = (start as f64).rem_euclid(360.0);
    let delta = if sweep >= 0.0 {
        (angle - from).rem_euclid(360.0)
    } else {
        (from - angle).rem_euclid(360.0)
    };
    let angular = if delta <= span { ring } else { 0.0 };
    if !rounded {
        return angular;
    }
    // `ARC_PROGRESS` z corner_flag rysuje półokrągłe kapki na końcach łuku —
    // edytor WinUI robi to samo przez StrokeStart/EndLineCap=Round. Dodajemy
    // je do klatek PNG/GIF, żeby podgląd nie miał ostrych ściętych końców.
    let mut cover = angular;
    for degrees in [start as f64, end as f64] {
        let radians = degrees.to_radians();
        let cap_x = cx as f64 + radius as f64 * radians.sin();
        let cap_y = cy as f64 - radius as f64 * radians.cos();
        let cap_distance = ((px as f64 - cap_x).powi(2) + (py as f64 - cap_y).powi(2)).sqrt();
        cover = cover.max(disc_coverage(cap, cap_distance));
    }
    cover
}

/// Wartość postępu 0…1 dla klatek podglądu — ta sama skala co w JS.
fn progress_frame_value(layer: &LayerView, frame: &PreviewFrameData) -> f64 {
    let Some(progress) = &layer.progress else { return 0.0 };
    let raw = match progress.source {
        ProgressSource::Battery => frame.battery_pct as f64,
        ProgressSource::Steps => frame.steps as f64,
        ProgressSource::Distance => frame.distance_m as f64,
        ProgressSource::Heart => frame.heart_rate as f64,
        ProgressSource::Calories => frame.calories as f64,
        ProgressSource::Stand => frame.stand_hours as f64,
        ProgressSource::FatBurning => frame.fat_burning_minutes as f64,
        ProgressSource::Pai => frame.pai as f64,
        ProgressSource::Spo2 => frame.spo2 as f64,
        ProgressSource::Sleep => frame.sleep_minutes as f64,
        ProgressSource::Fixed => progress.value as f64,
    };
    let lo = progress.min as f64;
    let hi = match progress.source {
        // Cel z aplikacji Zepp ma pierwszeństwo przed zapisanym max — tak samo
        // jak w wygenerowanym JS (2500/5000 kroków = połowa paska).
        ProgressSource::Steps => frame.steps_target as f64,
        ProgressSource::Calories => frame.calories_target as f64,
        ProgressSource::Stand => frame.stand_target.max(1) as f64,
        ProgressSource::FatBurning => frame.fat_burning_target.max(1) as f64,
        ProgressSource::Heart => 165.0,
        _ if progress.max > progress.min => progress.max as f64,
        _ => 100.0,
    };
    let mut value = if hi > lo { (raw - lo) / (hi - lo) } else { 0.0 };
    if progress.invert {
        value = 1.0 - value;
    }
    value.clamp(0.0, 1.0)
}

/// Rysuje łuk albo pasek postępu do klatki podglądu. Podkład i wypełnienie
/// używają dokładnie tych samych parametrów, które trafiają do JS.
fn draw_progress(canvas: &mut RgbaImage, layer: &LayerView, frame: &PreviewFrameData) {
    use crate::model::ProgressOrientation;

    let Some(progress) = &layer.progress else { return };
    let value = progress_frame_value(layer, frame);
    let alpha = layer.alpha as f64 / 255.0;
    let fg = {
        let mut c = parse_argb(Some(&progress.color));
        c.0[3] = (c.0[3] as f64 * alpha).round().clamp(0.0, 255.0) as u8;
        c
    };
    let bg = {
        let mut c = parse_argb(Some(&progress.track_color));
        c.0[3] = (c.0[3] as f64 * alpha).round().clamp(0.0, 255.0) as u8;
        c
    };
    let arc = layer.kind == "arc-progress";
    let bar_radius = if progress.corner.as_str() == "flat" {
        0
    } else {
        // Włączone zaokrąglenia = połowa krótszego boku (kwadrat to koło),
        // ale nigdy 0 — tak jak `Math.Max(1, …)` w `UpdateProgressVisual`.
        (layer.w.min(layer.h) / 2).max(1) as u32
    };
    let (cx, cy, radius) = (
        layer.x + layer.w / 2,
        layer.y + layer.h / 2,
        crate::model::arc_center_radius(layer.w, layer.h),
    );

    // Gruby łuk może wyjść poza ramkę widgetu — rysujemy z marginesem, żeby
    // krawędź zewnętrzna nie została ucięta (ramka pozostaje bez zmian).
    let margin = if arc {
        crate::model::arc_draw_margin(progress.thickness)
    } else {
        0
    };
    let y0 = (layer.y - margin).max(0);
    let y1 = (layer.y + layer.h + margin).min(SCREEN_SIZE as i32);
    let x0 = (layer.x - margin).max(0);
    let x1 = (layer.x + layer.w + margin).min(SCREEN_SIZE as i32);
    for py in y0..y1 {
        for px in x0..x1 {
            if arc {
                let rounded = progress.corner.as_str() != "flat";
                if progress.show_track {
                    let cover = arc_coverage(
                        cx, cy, radius, progress.thickness,
                        progress.start_angle, progress.end_angle, rounded, px, py,
                    );
                    paint_over(canvas.get_pixel_mut(px as u32, py as u32), bg, cover);
                }
                if value > 0.0 {
                    let end = progress.start_angle
                        + ((progress.end_angle - progress.start_angle) as f64 * value).round() as i32;
                    let cover = arc_coverage(
                        cx, cy, radius, progress.thickness, progress.start_angle, end, rounded, px, py,
                    );
                    paint_over(canvas.get_pixel_mut(px as u32, py as u32), fg, cover);
                }
            } else {
                if progress.show_track {
                    let cover = rounded_rect_coverage(
                        layer.x, layer.y, layer.w, layer.h, bar_radius, px, py,
                    );
                    paint_over(canvas.get_pixel_mut(px as u32, py as u32), bg, cover);
                }
                let (fx, fy, fw, fh) = match progress.orientation {
                    ProgressOrientation::Horizontal => (
                        layer.x, layer.y, (layer.w as f64 * value).round() as i32, layer.h,
                    ),
                    ProgressOrientation::Vertical => {
                        let fh = (layer.h as f64 * value).round() as i32;
                        (layer.x, layer.y + layer.h - fh, layer.w, fh)
                    }
                };
                let cover = rounded_rect_coverage(fx, fy, fw, fh, bar_radius, px, py);
                paint_over(canvas.get_pixel_mut(px as u32, py as u32), fg, cover);
            }
        }
    }
}

/// Składa oprawiony podgląd: pełna klatka tarczy jest zmniejszana do środkowego
/// okręgu z ledwo widocznym ciemnym obrysem, wokół niej są czarne strefy
/// bezpieczne i zewnętrzna warstwa logo z `logo/gear-outlayer.png`. Poza
/// outermost okręgiem piksele są w pełni przezroczyste.
pub fn compose_branded_preview(frame: &RgbaImage, size: u32) -> RgbaImage {
    // Zewnętrzna warstwa logo jest gotowym rysunkiem — nie odtwarzamy jej w kodzie.
    let mut gear = image::load_from_memory(GEAR_OUTLAYER_PNG)
        .expect("osadzony gear-outlayer.png")
        .to_rgba8();
    // Przezroczyste piksele mają białe RGB — wyzerujemy je, żeby przy próbkowaniu
    // nie wnosiły jasnej obwódki na krawędzi łuku.
    for pixel in gear.pixels_mut() {
        if pixel.0[3] == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
    let gear_center = (gear.width() as f64 - 1.0) / 2.0;

    let mut out = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    let half = size as f64 / 2.0;
    let center = (size as f64 - 1.0) / 2.0;
    let edge = R_EDGE * half;
    let brown_out = R_BROWN_OUT * half;
    // Łuk w rysunku ma promień ~333.6 px — dopasowujemy go do `brown_out`.
    let gear_scale = brown_out / GEAR_OUTER_RADIUS;
    let inner_black = GEAR_INNER_RADIUS * gear_scale;
    let face = R_FACE * half;
    let outline_px = 2.0;
    // Tarcza ma promień 233 px na 466 px boku — odwzorowujemy ten okrąg na `face`.
    let scale = 2.0 * face / frame.width() as f64;
    let src_center = (frame.width() as f64 - 1.0) / 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > edge + 1.0 {
                continue;
            }
            let mut pixel = Rgba([0, 0, 0, 0]);

            // Zewnętrzna czarna strefa bezpieczna.
            paint_over(&mut pixel, RING_BLACK, disc_coverage(edge, distance));

            // Gotowa zewnętrzna warstwa logo (łuk + zębatka) zamiast rysowania w kodzie.
            let gx = gear_center + dx / gear_scale;
            let gy = gear_center + dy / gear_scale;
            paint_over(&mut pixel, sample_bilinear(&gear, gx, gy), 1.0);

            // Wewnętrzna czarna strefa bezpieczna zaczyna się przy krawędzi łuku.
            paint_over(&mut pixel, RING_BLACK, disc_coverage(inner_black, distance));

            // Przeskalowana pełna klatka tarczy i ciemny, ledwo widoczny obrys.
            if distance <= face + 1.0 {
                let sx = src_center + dx / scale;
                let sy = src_center + dy / scale;
                let face_pixel = sample_bilinear(frame, sx, sy);
                paint_over(&mut pixel, face_pixel, disc_coverage(face, distance));
                paint_over(
                    &mut pixel,
                    RING_OUTLINE,
                    annulus_coverage(distance, face - outline_px, face),
                );
            }

            out.put_pixel(x, y, pixel);
        }
    }
    out
}

/// Kolor „morskiej bryzy" używany do oznaczania stref klikalnych w edytorze
/// i na następnej klatce GIF-u (tej po oprawionym podglądzie).
const SHORTCUT_HATCH_COLOR: Rgba<u8> = Rgba([125, 226, 209, 255]);
const SHORTCUT_ICON_PNG: &[u8] =
    include_bytes!("../../../native/RustyWatch/Assets/Icons/click-zone.png");

/// Dorysowuje widoczne pola klikania na surowej klatce 466×466.
/// Na zegarku strefy są niewidoczne — ten narzut jest tylko informacyjny.
pub fn draw_shortcut_overlays(canvas: &mut RgbaImage, project: &crate::model::Project) {
    let icon_source = match image::load_from_memory(SHORTCUT_ICON_PNG) {
        Ok(image) => image.to_rgba8(),
        Err(_) => return,
    };
    let alpha = |layer: &LayerView| layer.alpha as f64 / 255.0;

    for layer in describe(project) {
        if layer.kind != "shortcut" || !layer.visible {
            continue;
        }
        let x0 = layer.x.max(0);
        let x1 = (layer.x + layer.w).min(SCREEN_SIZE as i32);
        let y0 = layer.y.max(0);
        let y1 = (layer.y + layer.h).min(SCREEN_SIZE as i32);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        // Ramka wyznacza pole kliknięcia — ikona i gęstość kresek rosną z jej rozmiarem.
        let shorter = layer.w.min(layer.h).max(1);
        let icon_size = (shorter / 2).clamp(1, shorter) as u32;
        let hatch_step = (shorter / 4).clamp(6, 24) as i32;
        let icon = image::imageops::resize(
            &icon_source,
            icon_size,
            icon_size,
            FilterType::Lanczos3,
        );
        let icon_x = layer.x + layer.w / 2 - icon_size as i32 / 2;
        let icon_y = layer.y + layer.h / 2 - icon_size as i32 / 2;
        let center = SCREEN_SIZE as f64 / 2.0;

        for py in y0..y1 {
            for px in x0..x1 {
                // Nie rysujemy poza okręgiem tarczy.
                let dx = px as f64 - center;
                let dy = py as f64 - center;
                if (dx * dx + dy * dy).sqrt() > 233.0 {
                    continue;
                }
                let pixel = canvas.get_pixel_mut(px as u32, py as u32);
                if (px + py).rem_euclid(hatch_step) < 2 {
                    paint_over(pixel, SHORTCUT_HATCH_COLOR, 0.45 * alpha(&layer));
                }
                let ix = px - icon_x;
                let iy = py - icon_y;
                if ix >= 0 && iy >= 0 && (ix as u32) < icon_size && (iy as u32) < icon_size {
                    let sample = *icon.get_pixel(ix as u32, iy as u32);
                    paint_over(pixel, sample, alpha(&layer));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FirstDayOfWeek;

    fn sample_series() -> PreviewRandomSeries {
        random_series(1234, "en")
    }

    #[test]
    fn text_scale_matches_editor_font_size() {
        // `ab_glyph` bierze PxScale jako wysokość komórki; edytor i ZeppOS
        // podają rozmiar em. 64 px musi dać dokładnie 64 px w klatce PNG/GIF.
        let font = FontRef::try_from_slice(SYSTEM_FONT_BYTES).unwrap();
        let scale = px_scale_for(&font, 64.0);
        let upm = font.units_per_em().unwrap_or(1000.0);
        let effective_em = scale.x * upm / font.height_unscaled();
        assert!((effective_em - 64.0).abs() < 0.01, "effective em = {effective_em}");
    }

    #[test]
    fn arc_round_caps_extend_past_flat_ends() {
        // Łuk 0→90° zakończony w punkcie (333,233); punkt tuż za końcem
        // powinien być pusty przy płaskich końcach, ale pokryty przez okrągłą kapkę.
        let px = 338;
        let py = 240;
        let flat = arc_coverage(233, 233, 100, 20, 0, 90, false, px, py);
        let round = arc_coverage(233, 233, 100, 20, 0, 90, true, px, py);
        assert_eq!(flat, 0.0, "flat cap must not cover outside the arc");
        assert!(round > 0.0, "round cap must cover just past the arc end");

        // Pełny okrąg nie ma kapek — oba warianty dają ten sam pierścień.
        let full_flat = arc_coverage(233, 233, 100, 20, 0, 360, false, 233, 133);
        let full_round = arc_coverage(233, 233, 100, 20, 0, 360, true, 233, 133);
        assert_eq!(full_flat, full_round);
    }

    #[test]
    fn arc_draw_margin_matches_editor() {
        // `ArcStrokeMargin` w WinUI: max(0, thickness/2 - 8) + 6.
        assert_eq!(crate::model::arc_draw_margin(8), 6);
        assert_eq!(crate::model::arc_draw_margin(16), 6);
        assert_eq!(crate::model::arc_draw_margin(32), 14);
    }

    #[test]
    fn reversed_arc_sweep_is_not_a_full_circle() {
        let (cx, cy, radius, thickness) = (233, 233, 100, 16);
        // 135° → -135° to łuk 270° przeciwnie do ruchu wskazówek — edytor
        // rysuje go przez SweepDirection.Counterclockwise, nie pełne koło.
        assert!(
            arc_coverage(cx, cy, radius, thickness, 135, -135, false, cx, cy - radius) > 0.0,
            "top (0°) must be covered by the reversed arc"
        );
        assert_eq!(
            arc_coverage(cx, cy, radius, thickness, 135, -135, false, cx, cy + radius),
            0.0,
            "bottom (180°) must stay empty"
        );
        // Pełny obrót wstecz nadal daje zamknięty pierścień.
        assert!(
            arc_coverage(cx, cy, radius, thickness, 135, -225, false, cx, cy + radius) > 0.0
        );
    }

    #[test]
    fn arc_angle_matches_point_placement() {
        let cx = 233;
        let cy = 233;
        let radius = 100;
        let thickness = 16;
        // 0° = góra, 90° = prawo, 180° = dół — zgodnie z ARC_PROGRESS i ArcPoint.
        assert!(arc_coverage(cx, cy, radius, thickness, 0, 90, false, cx + radius, cy) > 0.0);
        assert_eq!(arc_coverage(cx, cy, radius, thickness, 0, 90, false, cx - radius, cy), 0.0);
        assert!(arc_coverage(cx, cy, radius, thickness, 90, 180, false, cx, cy + radius) > 0.0);
        assert_eq!(arc_coverage(cx, cy, radius, thickness, 90, 180, false, cx, cy - radius), 0.0);
    }

    #[test]
    fn rendered_arc_has_no_detached_caps() {
        use std::collections::HashSet;

        let mut project = crate::model::Project::starter("ArcTest");
        let widget = crate::widgets::add_widget(&mut project, "arc-progress").unwrap();
        let settings = serde_json::json!({
            "id": widget.id,
            "x": 29, "y": 194, "w": 67, "h": 67,
            "progress": {
                "source": "battery", "min": 0, "max": 100,
                "color": "0xFF4FC3F7", "trackColor": "0xFF3A3A42",
                "thickness": 16, "startAngle": 0, "endAngle": 360,
                "corner": "round", "showTrack": true
            }
        });
        crate::widgets::update_widget(&mut project, &settings).unwrap();

        let series = random_series(1234, "en");
        // Klatka 3 ma baterię 55% → łuk 0…198°, czyli dokładnie przypadek,
        // w którym wcześniej odrywała się kapka końcowa.
        let frame = frame_data(3, &series).unwrap();
        assert_eq!(frame.battery_pct, 55);
        let image = render_frame(Path::new("."), &project, &frame, &series);

        let mut points = HashSet::new();
        for py in 194..261 {
            for px in 29..96 {
                let p = image.get_pixel(px as u32, py as u32);
                if p[2] > 150 && p[1] > 110 && p[0] < 160 {
                    points.insert((px, py));
                }
            }
        }
        assert!(!points.is_empty(), "arc was not rendered");

        let mut components = Vec::new();
        while let Some(&start) = points.iter().next() {
            points.remove(&start);
            let mut stack = vec![start];
            let mut size = 0usize;
            while let Some((x, y)) = stack.pop() {
                size += 1;
                for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                    if points.remove(&(nx, ny)) {
                        stack.push((nx, ny));
                    }
                }
            }
            components.push(size);
        }
        assert_eq!(
            components.len(),
            1,
            "detached arc caps / broken ring: {components:?}"
        );
    }

    #[test]
    fn shortcut_overlay_scales_with_the_frame() {
        let changed_for = |size: i32| {
            let mut project = crate::model::Project::starter("ShortcutScale");
            let widget = crate::widgets::add_widget(&mut project, "shortcut").unwrap();
            let settings = serde_json::json!({
                "id": widget.id,
                "x": 233 - size / 2, "y": 233 - size / 2,
                "w": size, "h": size,
                "shortcutTarget": "battery",
            });
            crate::widgets::update_widget(&mut project, &settings).unwrap();
            let series = random_series(1234, "en");
            let frame = frame_data(0, &series).unwrap();
            let mut image = render_frame(Path::new("."), &project, &frame, &series);
            let before = image.clone();
            draw_shortcut_overlays(&mut image, &project);
            before.pixels().zip(image.pixels()).filter(|(a, b)| a.0 != b.0).count()
        };
        let small = changed_for(40);
        let large = changed_for(120);
        assert!(small > 0 && large > small, "overlay must grow with the frame: {small} vs {large}");
    }

    #[test]
    fn shortcut_overlay_goes_on_second_gif_frame() {
        let mut project = crate::model::Project::starter("GifShortcut");
        crate::widgets::add_widget(&mut project, "shortcut").unwrap();
        let series = random_series(1234, "en");
        let mut frames: Vec<_> = (0..FRAME_COUNT)
            .map(|index| {
                let frame = frame_data(index, &series).unwrap();
                render_frame(Path::new("."), &project, &frame, &series)
            })
            .collect();
        let branded = compose_branded_preview(&frames[PREVIEW_FRAME_INDEX], SCREEN_SIZE);
        frames.pop();
        frames.insert(0, branded.clone());
        assert_eq!(frames[0], branded, "first frame must stay the branded preview");

        let original = frames[1].clone();
        draw_shortcut_overlays(&mut frames[1], &project);
        let changed = original
            .pixels()
            .zip(frames[1].pixels())
            .filter(|(a, b)| a.0 != b.0)
            .count();
        assert!(changed > 0, "second frame must show the click zone");
    }

    #[test]
    fn progress_preview_uses_watch_goal_and_bpm_scale() {
        use crate::model::ProgressSource;

        let mut project = crate::model::Project::starter("PreviewGoals");
        let widget = crate::widgets::add_widget(&mut project, "bar-progress").unwrap();
        crate::widgets::update_widget(
            &mut project,
            &serde_json::json!({
                "id": widget.id,
                "x": 23, "y": 203, "w": 420, "h": 60,
                "progress": { "source": "steps", "max": 100 },
            }),
        )
        .unwrap();
        let layer = describe(&project)
            .into_iter()
            .find(|view| view.id == widget.id)
            .unwrap();
        let mut frame = frame_data(0, &random_series(1234, "en")).unwrap();
        frame.steps = 2500;
        frame.steps_target = 5000;
        let value = progress_frame_value(&layer, &frame);
        assert!((value - 0.5).abs() < 0.001, "2500/5000 must be half: {value}");

        let mut settings = layer.progress.clone().unwrap();
        settings.source = ProgressSource::Heart;
        let layer = LayerView { progress: Some(settings), ..layer };
        frame.heart_rate = 165;
        assert!(
            (progress_frame_value(&layer, &frame) - 1.0).abs() < 0.001,
            "165 bpm must fill the arc"
        );
        frame.heart_rate = 82;
        assert!((progress_frame_value(&layer, &frame) - 82.0 / 165.0).abs() < 0.001);
    }

    #[test]
    fn presets_have_strictly_increasing_time_and_real_dates() {
        for i in 1..SOURCE_FRAME_COUNT {
            assert!(FRAME_PRESETS[i].hour > FRAME_PRESETS[i - 1].hour);
            assert!(FRAME_PRESETS[i].minute > FRAME_PRESETS[i - 1].minute);
            assert!(FRAME_PRESETS[i].second > FRAME_PRESETS[i - 1].second);
            assert!(FRAME_PRESETS[i].date.month > FRAME_PRESETS[i - 1].date.month);
            assert!(FRAME_PRESETS[i].date.day > FRAME_PRESETS[i - 1].date.day);
        }
        assert_eq!(
            FRAME_PRESETS[0].date,
            PreviewDate { day: 3, month: 1, year: 2022 }
        );
        assert_eq!(
            FRAME_PRESETS[6].date,
            PreviewDate { day: 31, month: 7, year: 2022 }
        );
        // Wszystkie daty są prawdziwe — 2022 nie jest rokiem przestępnym.
        assert!(FRAME_PRESETS
            .iter()
            .all(|p| p.date.year == 2022 && (1..=7).contains(&p.date.month)));
    }

    #[test]
    fn every_source_frame_has_a_distinct_real_weekday_and_month() {
        let months: std::collections::HashSet<u32> = FRAME_PRESETS
            .iter()
            .take(SOURCE_FRAME_COUNT)
            .map(|p| p.date.month)
            .collect();
        assert_eq!(months.len(), SOURCE_FRAME_COUNT);
        let mut previous = 0;
        for (index, preset) in FRAME_PRESETS.iter().take(SOURCE_FRAME_COUNT).enumerate() {
            let weekday = crate::date_format::weekday_number(
                preset.date.day, preset.date.month, preset.date.year, FirstDayOfWeek::Monday,
            );
            assert_eq!(weekday, index as u32 + 1, "frame {}", index + 1);
            assert!(weekday > previous);
            previous = weekday;
        }
    }

    #[test]
    fn last_frame_is_exact_21_37_copy_and_feeds_preview() {
        let source = FRAME_PRESETS[SOURCE_FRAME_COUNT - 1];
        let copy = FRAME_PRESETS[PREVIEW_FRAME_INDEX];
        assert_eq!((source.hour, source.minute, source.second), (21, 37, 40));
        assert_eq!(source.status_mask, 0b1111);
        assert_eq!(
            (copy.hour, copy.minute, copy.second, copy.date, copy.status_mask),
            (source.hour, source.minute, source.second, source.date, source.status_mask)
        );
        assert_eq!(FRAME_COUNT, SOURCE_FRAME_COUNT + 1);
    }

    #[test]
    fn preview_frame_reuses_all_random_values() {
        let series = sample_series();
        let source = frame_data(SOURCE_FRAME_COUNT - 1, &series).unwrap();
        let copy = frame_data(PREVIEW_FRAME_INDEX, &series).unwrap();
        assert_eq!(
            (
                source.date,
                source.battery_pct,
                source.heart_rate,
                source.calories,
                source.distance_m,
                source.steps,
                source.stress,
                source.pai,
                source.skin_temp_c,
                source.sleep_minutes,
                source.spo2,
            ),
            (
                copy.date,
                copy.battery_pct,
                copy.heart_rate,
                copy.calories,
                copy.distance_m,
                copy.steps,
                copy.stress,
                copy.pai,
                copy.skin_temp_c,
                copy.sleep_minutes,
                copy.spo2,
            )
        );
    }

    #[test]
    fn status_visibility_covers_all_five_states_and_is_not_monotonic() {
        let counts: Vec<u32> = FRAME_PRESETS
            .iter()
            .map(|frame| frame.status_mask.count_ones())
            .collect();
        assert_eq!(counts, [2, 2, 0, 3, 0, 1, 4, 4]);
        let states: std::collections::HashSet<u32> = counts.iter().copied().collect();
        assert_eq!(states, [0, 1, 2, 3, 4].into_iter().collect());
        assert_eq!(counts.iter().filter(|count| **count == 0).count(), 2);
        assert!(counts.windows(2).any(|pair| pair[0] > pair[1]));
        assert!(counts.windows(2).any(|pair| pair[0] < pair[1]));
        // Klatka 4 ma alarm+DND+blokada, klatka 6 tylko Bluetooth.
        assert_eq!(FRAME_PRESETS[3].status_mask, 0b1011);
        assert_eq!(FRAME_PRESETS[5].status_mask, 0b0100);
    }

    #[test]
    fn series_covers_visual_extremes_and_uses_preset_dates() {
        let series = sample_series();
        assert_eq!(series.frame_count, FRAME_COUNT);
        assert_eq!(series.heart_rates[0], 56);
        assert_eq!(series.heart_rates[FRAME_COUNT - 1], 120);
        assert_eq!(series.calories[0], 0);
        assert_eq!(series.calories[FRAME_COUNT - 1], 2450);
        assert_eq!(series.distances_m[0], 0);
        assert_eq!(series.distances_m[FRAME_COUNT - 1], 12300);
        assert!(series
            .steps_per_km
            .iter()
            .all(|value| (1300..=1400).contains(value)));
        assert_eq!(series.dates.len(), FRAME_COUNT);
        for (date, preset) in series.dates.iter().zip(FRAME_PRESETS.iter()) {
            assert_eq!(*date, preset.date);
        }
    }

    #[test]
    fn steps_are_derived_from_distance_and_battery_decreases() {
        let series = sample_series();
        let mut previous_battery = 255u8;
        for index in 0..SOURCE_FRAME_COUNT {
            let frame = frame_data(index, &series).unwrap();
            let expected =
                (frame.distance_m as f64 / 1000.0 * frame.steps_per_km as f64).round() as u32;
            assert_eq!(frame.steps, expected);
            assert!(frame.battery_pct < previous_battery);
            previous_battery = frame.battery_pct;
        }
        assert_eq!(frame_data(0, &series).unwrap().battery_pct, 100);
        assert_eq!(
            frame_data(FRAME_COUNT - 1, &series).unwrap().battery_pct,
            10
        );
        let last = frame_data(FRAME_COUNT - 1, &series).unwrap();
        assert!((12_000..=17_500).contains(&last.steps), "steps={}", last.steps);
    }

    #[test]
    fn date_widgets_are_consistent_with_the_same_frame_date() {
        let series = sample_series();
        let frame = frame_data(3, &series).unwrap();
        // 14 kwietnia 2022 to czwartek: poniedziałek-first daje 4.
        let fixed = PreviewDate { day: 14, month: 4, year: 2022 };
        assert_eq!(frame.date, series.dates[3]);
        assert_eq!(frame.date, fixed);
        assert_eq!(
            date_format::weekday_number(fixed.day, fixed.month, fixed.year, FirstDayOfWeek::Monday),
            4
        );
        assert_eq!(
            date_format::weekday_number(fixed.day, fixed.month, fixed.year, FirstDayOfWeek::Sunday),
            5
        );
        assert_eq!(date_format::iso_week(fixed.day, fixed.month, fixed.year), 15);
        assert_eq!(date_format::day_of_year(fixed.day, fixed.month, fixed.year), 104);
        let text = date_format::format_with_locale(
            "DD.MM.YYYY",
            fixed.day,
            fixed.month - 1,
            fixed.year,
            WatchfaceLocale::Polish,
        )
        .unwrap();
        assert_eq!(text, "14.04.2022");
    }

    #[test]
    fn gif_frame_delay_is_about_1_7_seconds() {
        // Jednostka GIF-a to 1/100 s, więc 170 = 1,7 s na klatkę.
        assert_eq!(GIF_FRAME_DELAY, 170);
        let image = RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]));
        let path = std::env::temp_dir().join(format!("rw-delay-test-{}.gif", std::process::id()));
        write_gif(&[image.clone(), image], &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.windows(2).any(|w| w == [0xAA, 0x00]), "brak opóźnienia 170");
        assert!(bytes.windows(11).any(|w| w == b"NETSCAPE2.0"));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn gif_last_frame_uses_the_branded_preview() {
        let face = RgbaImage::from_pixel(SCREEN_SIZE, SCREEN_SIZE, Rgba([30, 60, 90, 255]));
        let branded = compose_branded_preview(&face, SCREEN_SIZE);
        let mut frames = vec![face; FRAME_COUNT];
        frames[0] = branded;
        let path = std::env::temp_dir().join(format!("rw-branded-gif-{}.gif", std::process::id()));
        write_gif(&frames, &path).unwrap();

        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(File::open(&path).unwrap()).unwrap();
        let mut decoded = Vec::new();
        while decoder.read_next_frame().unwrap().is_some() {
            let rgba = decoder.current_frame_info().unwrap().buffer.to_vec();
            decoded.push(rgba);
        }
        std::fs::remove_file(&path).ok();

        assert_eq!(decoded.len(), FRAME_COUNT);
        let first = &decoded[0];
        assert_eq!(first[3], 0, "pierwsza klatka nie jest oprawionym podglądem");
        let ring = (40 * SCREEN_SIZE as usize + 233) * 4;
        assert!(
            first[ring] > 60 && first[ring] > first[ring + 2],
            "brak brązowego pierścienia w pierwszej klatce"
        );
        assert_eq!(decoded.last().unwrap()[3], 255, "ostatnia klatka jest surowa");
    }

    #[test]
    fn cutout_paints_corners_black() {
        let mut image = RgbaImage::from_pixel(SCREEN_SIZE, SCREEN_SIZE, Rgba([200, 100, 50, 255]));
        apply_round_cutout(&mut image);
        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(image.get_pixel(465, 465).0, [0, 0, 0, 255]);
        assert_ne!(image.get_pixel(233, 233).0, [0, 0, 0, 255]);
    }

    #[test]
    fn branded_preview_layers_match_the_mockup() {
        // Syntetyczna tarcza: środek ma być widoczny w blue area, a rogi mają
        // być przezroczyste (na zrzucie pokazane na zielono).
        let face = RgbaImage::from_pixel(SCREEN_SIZE, SCREEN_SIZE, Rgba([30, 60, 90, 255]));
        let image = compose_branded_preview(&face, SCREEN_SIZE);
        assert_eq!(image.dimensions(), (466, 466));
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        assert_eq!(image.get_pixel(465, 465).0[3], 0);

        // Zewnętrzna strefa bezpieczna (między brązem a krawędzią).
        assert_eq!(image.get_pixel(233, 20).0, [0, 0, 0, 255]);
        // Brązowy pierścień na górze.
        let brown = image.get_pixel(233, 40).0;
        assert!(brown[0] > brown[2], "oczekiwany brąz: {brown:?}");
        // Zębatka po prawej stronie (na zewnątrz brązowego pierścienia).
        let gear = image.get_pixel(448, 233).0;
        assert!(gear[0] > 150 && gear[1] > 60, "oczekiwana zębatka: {gear:?}");
        // Wewnętrzna czarna strefa bezpieczna (między pierścieniem a tarczą).
        assert_eq!(image.get_pixel(233, 66).0, [0, 0, 0, 255]);
        // Przeskalowana tarcza w środku.
        assert_eq!(image.get_pixel(233, 233).0, [30, 60, 90, 255]);
        // Ciemny, ledwo widoczny obrys na granicy tarczy (#404040).
        let outline = (70..85)
            .map(|y| image.get_pixel(233, y).0)
            .max_by_key(|pixel| pixel[0])
            .unwrap();
        assert!(
            (55..=70).contains(&outline[0]) && outline[0] == outline[1] && outline[1] == outline[2],
            "zły obrys: {outline:?}"
        );
    }

    #[test]
    fn branded_preview_scales_to_watch_icon_size() {
        let face = RgbaImage::from_pixel(SCREEN_SIZE, SCREEN_SIZE, Rgba([30, 60, 90, 255]));
        let icon = compose_branded_preview(&face, 314);
        assert_eq!(icon.dimensions(), (314, 314));
        assert_eq!(icon.get_pixel(0, 0).0[3], 0);
        assert_eq!(icon.get_pixel(157, 157).0, [30, 60, 90, 255]);
    }
}
