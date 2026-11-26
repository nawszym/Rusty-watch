use serde::{Deserialize, Serialize};

use crate::image_fit::FitMode;

pub const SCHEMA_VERSION: u32 = 1;
pub const SCREEN_SIZE: u32 = 466;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub metadata: Metadata,
    pub device: DeviceId,
    pub widgets: Vec<Widget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<Asset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_code: Option<String>,
    /// Kolejność rysowania od spodu do góry (same ids widgetów).
    /// Pusta lista oznacza domyślny porządek według `z`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_order: Vec<String>,
    /// Domyślny system jednostek dla widgetów dystansu (ustawienie aplikacji).
    /// Widget może go nadpisać własnym `measurement_system`.
    #[serde(default)]
    pub measurement_system: MeasurementSystem,
    /// Pierwszy dzień tygodnia używany przez pole „Dzień tygodnia (liczba)"
    /// (ustawienie aplikacji). Dotyczy podglądu i eksportowanej tarczy.
    #[serde(default)]
    pub first_day_of_week: FirstDayOfWeek,
}

/// Pierwszy dzień tygodnia dla numericznego pola daty. Domyślnie poniedziałek
/// (1 = poniedziałek … 7 = niedziela); wariant niedzielny daje 1 = niedziela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirstDayOfWeek {
    #[default]
    Monday,
    Sunday,
}

impl FirstDayOfWeek {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "monday" | "mon" | "poniedzialek" | "pon" => Some(Self::Monday),
            "sunday" | "sun" | "niedziela" | "ndz" => Some(Self::Sunday),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Monday => "monday",
            Self::Sunday => "sunday",
        }
    }
}

/// System jednostek używany do wyświetlania dystansu.
/// `Watch` = jednostka ustawiona w ZeppOS na zegarku (`hmSetting.getMileageUnit()`) —
/// wartość jest odczytywana w runtime tarczy, więc jeden ZIP działa w km i w milach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasurementSystem {
    /// Domyślnie tarcza podąża za ustawieniem jednostki w zegarku.
    #[default]
    Watch,
    Metric,
    Imperial,
}

impl MeasurementSystem {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "watch" | "auto" => Some(Self::Watch),
            "metric" | "km" => Some(Self::Metric),
            "imperial" | "mi" | "miles" => Some(Self::Imperial),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Metric => "metric",
            Self::Imperial => "imperial",
        }
    }

    pub fn is_watch(self) -> bool {
        self == Self::Watch
    }

    /// Dzielnik metrów → jednostka wyświetlana. Dla `Watch` jest to tylko
    /// wartość zapasowa używana przez podgląd — generator JS czyta jednostkę z zegarka.
    pub fn meters_per_unit(self) -> f64 {
        match self {
            Self::Watch | Self::Metric => 1000.0,
            Self::Imperial => 1609.344,
        }
    }

    /// Sufiks zapasowy dla podglądu; `Watch` w JS wybiera `mi`/`km` w runtime.
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Watch | Self::Metric => "km",
            Self::Imperial => "mi",
        }
    }

    /// Tekst pokazywany w podglądzie przed pierwszym odczytem czujnika.
    /// Dla `Watch` PC nie zna ustawienia zegarka — pokazujemy metryczny jako
    /// neutralny placeholder; na zegarku pierwszy tick od razu wstawia właściwą jednostkę.
    pub fn initial_distance_text(self) -> String {
        format!("0.00 {}", self.suffix())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub name: String,
    pub app_id: u32,
    pub author: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceId {
    #[serde(rename = "active-2-round")]
    Active2Round,
}

impl DeviceId {
    pub fn profile(self) -> DeviceProfile {
        match self {
            DeviceId::Active2Round => DeviceProfile {
                target_key: "active2round",
                device_source: 8_913_153,
                design_width: SCREEN_SIZE,
                api_compatible: "3.0.0",
                api_target: "3.6.0",
                api_min: "3.0.0",
                screen_shape: ScreenShape::Round,
            },
        }
    }
}

/// Kształt ekranu urządzenia — używany m.in. do maskowania ikony tarczy
/// wyświetlanej na zegarku. `Round` = okrąg wpisany w kwadrat `design_width`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenShape {
    Round,
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceProfile {
    pub target_key: &'static str,
    pub device_source: u64,
    pub design_width: u32,
    pub api_compatible: &'static str,
    pub api_target: &'static str,
    pub api_min: &'static str,
    pub screen_shape: ScreenShape,
}

/// Warstwa jest domyślnie widoczna — starsze projekty nie mają tego pola.
fn default_visible() -> bool {
    true
}

/// Jednostki są domyślnie pokazywane — starsze projekty nie mają tego pola.
fn default_show_unit() -> bool {
    true
}

/// Pełna nieprzezroczystość — starsze projekty nie mają pola `alpha`.
fn default_alpha() -> u8 {
    255
}

/// Źródło wartości widgetu postępu (łuk/pasek). Wartość jest liczona w JS
/// z istniejących czujników albo wpisana ręcznie (`fixed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProgressSource {
    #[default]
    Battery,
    Steps,
    /// Zachowany tylko dla kompatybilności starych projektów — nie jest już
    /// oferowany jako źródło łuku/paska w inspektorze.
    Distance,
    Heart,
    Calories,
    Stand,
    FatBurning,
    Pai,
    Spo2,
    Sleep,
    Fixed,
}

impl ProgressSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Battery => "battery",
            Self::Steps => "steps",
            Self::Distance => "distance",
            Self::Heart => "heart",
            Self::Calories => "calories",
            Self::Stand => "stand",
            Self::FatBurning => "fat-burning",
            Self::Pai => "pai",
            Self::Spo2 => "spo2",
            Self::Sleep => "sleep",
            Self::Fixed => "fixed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "battery" => Some(Self::Battery),
            "steps" => Some(Self::Steps),
            "distance" => Some(Self::Distance),
            "heart" => Some(Self::Heart),
            "calories" => Some(Self::Calories),
            "stand" => Some(Self::Stand),
            "fat-burning" => Some(Self::FatBurning),
            "pai" => Some(Self::Pai),
            "spo2" => Some(Self::Spo2),
            "sleep" => Some(Self::Sleep),
            "fixed" => Some(Self::Fixed),
            _ => None,
        }
    }
}

/// Sposób zaokrąglenia końców łuku (`corner_flag` w ZeppOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProgressCorner {
    #[default]
    Round,
    Flat,
    Start,
    End,
}

impl ProgressCorner {
    /// 0/inny = oba końce, 1 = początek, 2 = koniec, 3 = brak (wg dokumentacji społeczności).
    pub fn corner_flag(self) -> u8 {
        match self {
            Self::Round => 0,
            Self::Start => 1,
            Self::End => 2,
            Self::Flat => 3,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Round => "round",
            Self::Flat => "flat",
            Self::Start => "start",
            Self::End => "end",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "round" => Some(Self::Round),
            "flat" => Some(Self::Flat),
            "start" => Some(Self::Start),
            "end" => Some(Self::End),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProgressOrientation {
    #[default]
    Horizontal,
    Vertical,
}

impl ProgressOrientation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "vertical" => Some(Self::Vertical),
            _ => Some(Self::Horizontal),
        }
    }
}

/// Promień linii środkowej łuku zależy tylko od ramki — zmiana grubości
/// rozszerza pierścień jednakowo do środka i na zewnątrz, bez przesuwania środka.
/// Ewentualne wyjście poza ramkę jest rysowane (edytor nie przycina łuku).
pub fn arc_center_radius(w: i32, h: i32) -> i32 {
    (w.min(h) / 2 - 10).max(4)
}

/// O ile pikseli wykracza łuk poza ramkę widgetu przy danej grubości.
pub fn arc_draw_margin(thickness: u32) -> i32 {
    (thickness as i32 - 16).max(0) / 2 + 3
}

/// Wszystkie ustawienia wizualne i źródło danych widgetu postępu.
/// Jedno pole w `Widget` zamiast kilkunastu osobnych — łatwy round-trip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProgressSettings {
    pub source: ProgressSource,
    /// Wartość minimalna (dolna granica skali).
    pub min: f32,
    /// Wartość maksymalna; gdy <= min, generator używa 100 (albo `.target` czujnika).
    pub max: f32,
    /// Wartość dla źródła `fixed`.
    pub value: f32,
    pub color: String,
    pub track_color: String,
    pub thickness: u32,
    pub start_angle: i32,
    pub end_angle: i32,
    pub corner: ProgressCorner,
    /// Promień rogów paska (FILL_RECT.radius).
    pub radius: u32,
    /// Liczba segmentów; 1 = ciągły pasek/łuk.
    pub segments: u8,
    /// Przerwa: stopnie dla łuku, piksele dla paska.
    pub gap: u32,
    pub orientation: ProgressOrientation,
    pub invert: bool,
    pub show_track: bool,
}

impl Default for ProgressSettings {
    fn default() -> Self {
        Self {
            source: ProgressSource::Battery,
            min: 0.0,
            max: 100.0,
            value: 50.0,
            color: "0xFF4FC3F7".to_string(),
            track_color: "0xFF3A3A42".to_string(),
            thickness: 16,
            start_angle: 0,
            end_angle: 360,
            corner: ProgressCorner::Round,
            radius: 8,
            segments: 1,
            gap: 4,
            orientation: ProgressOrientation::Horizontal,
            invert: false,
            show_track: true,
        }
    }
}

/// Sposób prezentacji fazy snu: minuty albo udział procentowy całego snu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SleepStageMode {
    #[default]
    Duration,
    Percent,
}

impl SleepStageMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "duration" | "min" | "minutes" => Some(Self::Duration),
            "percent" | "percentage" | "%" => Some(Self::Percent),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Duration => "duration",
            Self::Percent => "percent",
        }
    }
}

/// Domyślny dzień prognozy — starsze projekty nie mają tego pola.
fn default_weather_day_offset() -> u8 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub path: String,
    pub role: AssetRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetRole {
    Icon,
    /// Czcionka skopiowana z `%APPDATA%\RustyWatch\Fonts` do projektu.
    Font,
    /// Oryginał obrazka zachowany w projekcie.
    Image,
    /// Plik wskazówki skopiowany z `%APPDATA%\RustyWatch\Hands` do projektu.
    Hand,
    /// Ikona statusu systemu skopiowana z `%APPDATA%\RustyWatch\Status` do projektu.
    StatusIcon,
    /// Ikona pogody skopiowana z `%APPDATA%\RustyWatch\Weather` do projektu.
    WeatherIcon,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String,
    pub name: String,
    pub kind: WidgetKind,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default)]
    pub z: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_size: Option<u32>,
    /// Nazwa pliku czcionki z `%APPDATA%\RustyWatch\Fonts` (np. `Inter-Regular.ttf`).
    /// `None` oznacza czcionkę systemową zegarka. Podgląd i eksport używają tego samego pliku.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// Wzorzec wyświetlania czasu, np. `HH:MM:SS`. Dotyczy widgetów z `data_source: time`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// Tryb dopasowania obrazu widgetu IMAGES do ramki (`image_fit` z `image_fit.rs`).
    /// Plik PNG w projekcie jest fizycznie przygotowany do `w × h` przed eksportem.
    #[serde(default)]
    pub image_fit: FitMode,
    /// Nazwa zestawu wskazówek z `%APPDATA%\RustyWatch\Hands` (tylko `AnalogTime`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_set: Option<String>,
    /// Ścieżki plików wskazówek względem `assets/` (tylko `AnalogTime`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hour_src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minute_src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second_src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source: Option<DataSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// Pionowe położenie tekstu wewnątrz ramki (`align_v` w ZeppOS).
    #[serde(default)]
    pub v_align: VAlign,
    #[serde(default)]
    pub show_level: ShowLevel,
    /// Tryb ramki w edytorze: po podwójnym kliknięciu rozmiar czcionki zmienia się
    /// proporcjonalnie do rozmiaru ramki. Nie wpływa bezpośrednio na wygenerowany JS —
    /// zapisany `text_size` jest zawsze końcową wartością widoczną w podglądzie.
    #[serde(default)]
    pub auto_scale: bool,
    /// Sposób dopasowania tekstu do ramki (ucięcie, przewijanie, zawijanie).
    #[serde(default)]
    pub text_style: TextStyle,
    /// Widoczność warstwy w podglądzie i w wygenerowanej tarczy.
    /// Ukryte widgety pozostają w projekcie (i w panelu Warstwy), ale nie są rysowane.
    #[serde(default = "default_visible")]
    pub visible: bool,
    /// Własny system jednostek widgetu dystansu. `None` = użyj domyślnego
    /// systemu z projektu (ustawienie aplikacji).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurement_system: Option<MeasurementSystem>,
    /// Czy widget zdrowotny dołącza jednostkę (`bpm`, `°C`, `kcal`, `%`).
    /// Domyślnie `true`, więc starsze projekty zachowują dotychczasowy wygląd.
    #[serde(default = "default_show_unit")]
    pub show_unit: bool,
    /// Krycie widgetu (0–255) przekazywane do `alpha` w ZeppOS. Domyślnie 255.
    #[serde(default = "default_alpha")]
    pub alpha: u8,
    /// Nazwa paczki ikon statusu z `%APPDATA%\RustyWatch\Status` (tylko StatusIcon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_icon_set: Option<String>,
    /// Który stan systemu reprezentuje ikona (tylko StatusIcon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_type: Option<StatusType>,
    /// Nazwa paczki ikon pogody z `%APPDATA%\RustyWatch\Weather` (tylko WeatherIcon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather_icon_set: Option<String>,
    /// Dzień prognozy dla widgetów pogodowych: 0 = dziś, 1 = jutro, … 4.
    #[serde(default = "default_weather_day_offset")]
    pub weather_day_offset: u8,
    /// Czas fazy snu w minutach albo jako % całego snu.
    #[serde(default)]
    pub sleep_stage_mode: SleepStageMode,
    /// Ustawienia widgetu postępu (łuk/pasek). Starsze projekty ich nie mają.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressSettings>,
    /// Cel niewidocznej strefy klikalnej (`shortcut`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut_target: Option<ShortcutTarget>,
}

/// Stan systemu wyświetlany przez `hmUI.widget.IMG_STATUS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusType {
    Alarm,
    Dnd,
    Bluetooth,
    Lock,
}

impl StatusType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Alarm => "alarm",
            Self::Dnd => "dnd",
            Self::Bluetooth => "bluetooth",
            Self::Lock => "lock",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "alarm" | "clock" => Some(Self::Alarm),
            "dnd" | "disturb" => Some(Self::Dnd),
            "bluetooth" | "disconnect" => Some(Self::Bluetooth),
            "lock" => Some(Self::Lock),
            _ => None,
        }
    }

    /// Nazwa pliku w paczce — wspólna konwencja dla magazynu i projektu.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Alarm => "alarm.png",
            Self::Dnd => "dnd.png",
            Self::Bluetooth => "bluetooth.png",
            Self::Lock => "lock.png",
        }
    }

    /// Stała klasycznego API tarczy. Uwaga: `DISCONNECT` oznacza brak połączenia.
    pub fn js_constant(self) -> &'static str {
        match self {
            Self::Alarm => "hmUI.system_status.CLOCK",
            Self::Dnd => "hmUI.system_status.DISTURB",
            Self::Bluetooth => "hmUI.system_status.DISCONNECT",
            Self::Lock => "hmUI.system_status.LOCK",
        }
    }

    /// Bit w masce widoczności klatek podglądu.
    pub fn bit(self) -> u8 {
        match self {
            Self::Alarm => 1,
            Self::Dnd => 2,
            Self::Bluetooth => 4,
            Self::Lock => 8,
        }
    }
}

/// Cel klikalnej, niewidocznej strefy (`hmUI.widget.IMG_CLICK`).
/// Wartości odpowiadają stałym `hmUI.data_type.*` w klasycznym API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutTarget {
    #[default]
    Battery,
    Steps,
    Heart,
    Stress,
    Sleep,
    Pai,
    Spo2,
    Weather,
    AlarmClock,
    StopWatch,
    Countdown,
    OutdoorRunning,
    Walking,
    OutdoorCycling,
    FreeTraining,
    PoolSwimming,
    OpenWaterSwimming,
    BreathTrain,
    Moon,
}

impl ShortcutTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Battery => "battery",
            Self::Steps => "steps",
            Self::Heart => "heart",
            Self::Stress => "stress",
            Self::Sleep => "sleep",
            Self::Pai => "pai",
            Self::Spo2 => "spo2",
            Self::Weather => "weather",
            Self::AlarmClock => "alarm-clock",
            Self::StopWatch => "stopwatch",
            Self::Countdown => "countdown",
            Self::OutdoorRunning => "outdoor-running",
            Self::Walking => "walking",
            Self::OutdoorCycling => "outdoor-cycling",
            Self::FreeTraining => "free-training",
            Self::PoolSwimming => "pool-swimming",
            Self::OpenWaterSwimming => "open-water-swimming",
            Self::BreathTrain => "breath-train",
            Self::Moon => "moon",
        }
    }

    /// Dokładna nazwa stałej w `hmUI.data_type`.
    pub fn js_constant(self) -> &'static str {
        match self {
            Self::Battery => "BATTERY",
            Self::Steps => "STEP",
            Self::Heart => "HEART",
            Self::Stress => "STRESS",
            Self::Sleep => "SLEEP",
            Self::Pai => "PAI_DAILY",
            Self::Spo2 => "SPO2",
            Self::Weather => "WEATHER_CURRENT",
            Self::AlarmClock => "ALARM_CLOCK",
            Self::StopWatch => "STOP_WATCH",
            Self::Countdown => "COUNT_DOWN",
            Self::OutdoorRunning => "OUTDOOR_RUNNING",
            Self::Walking => "WALKING",
            Self::OutdoorCycling => "OUTDOOR_CYCLING",
            Self::FreeTraining => "FREE_TRAINING",
            Self::PoolSwimming => "POOL_SWIMMING",
            Self::OpenWaterSwimming => "OPEN_WATER_SWIMMING",
            Self::BreathTrain => "BREATH_TRAIN",
            Self::Moon => "MOON",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "battery" => Some(Self::Battery),
            "steps" => Some(Self::Steps),
            "heart" => Some(Self::Heart),
            "stress" => Some(Self::Stress),
            "sleep" => Some(Self::Sleep),
            "pai" => Some(Self::Pai),
            "spo2" => Some(Self::Spo2),
            "weather" => Some(Self::Weather),
            "alarm-clock" => Some(Self::AlarmClock),
            "stopwatch" => Some(Self::StopWatch),
            "countdown" => Some(Self::Countdown),
            "outdoor-running" => Some(Self::OutdoorRunning),
            "walking" => Some(Self::Walking),
            "outdoor-cycling" => Some(Self::OutdoorCycling),
            "free-training" => Some(Self::FreeTraining),
            "pool-swimming" => Some(Self::PoolSwimming),
            "open-water-swimming" => Some(Self::OpenWaterSwimming),
            "breath-train" => Some(Self::BreathTrain),
            "moon" => Some(Self::Moon),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    Text,
    /// Starszy, nieaddowalny widget graficzny (zachowany dla kompatybilności).
    Image,
    /// Statyczny obrazek z magazynu `%APPDATA%\RustyWatch\Images`, skalowany
    /// do ramki jednym z trybów `FitMode` przed eksportem.
    Images,
    TimeText,
    DateText,
    /// Numer dnia tygodnia z `hmSensor.id.TIME.week` (1–7).
    WeekdayNumberText,
    /// Numer tygodnia roku (ISO) liczony wspólnie w Rust i w JS.
    WeekNumberText,
    /// Dzień roku (1–366) liczony wspólnie w Rust i w JS.
    DayOfYearText,
    /// Data chińska (księżycowa) z pól `lunar_month` / `lunar_day`.
    LunarDateText,
    /// Nazwa święta z `solar_festival` / `lunar_festival` / `lunar_solar_term`.
    FestivalText,
    /// Zegar światowy z `hmSensor.id.WORLD_CLOCK` (miasto + godzina).
    WorldClockText,
    /// Licznik kroków — widget tekstowy zasilany czujnikiem `hmSensor.id.STEP`.
    StepText,
    /// Dystans z czujnika `hmSensor.id.DISTANCE`, w km lub milach.
    DistanceText,
    /// Puls z `hmSensor.id.HEART` (wartość bieżąca).
    HeartRateText,
    /// Ostatni prawidłowy puls z `hmSensor.id.HEART.last`.
    HeartRateLastText,
    /// Najniższy puls z `hmSensor.id.HEART.today`.
    HeartRateMinText,
    /// Najwyższy puls z `hmSensor.id.HEART.today`.
    HeartRateMaxText,
    /// Średni puls z `hmSensor.id.HEART.today`.
    HeartRateAvgText,
    /// Poziom stresu z `hmSensor.id.STRESS`.
    StressText,
    /// PAI z `hmSensor.id.PAI` (wynik dzienny).
    PaiText,
    /// Temperatura skóry z `hmSensor.id.BODY_TEMP`.
    SkinTempText,
    /// Czas snu z `hmSensor.id.SLEEP`.
    SleepText,
    /// Wynik snu z `SLEEP.getBasicInfo().score`.
    SleepScoreText,
    /// Czas fazy DEEP z `SLEEP.getSleepStageData()`.
    SleepDeepText,
    /// Czas fazy LIGHT z `SLEEP.getSleepStageData()`.
    SleepLightText,
    /// Czas fazy REM z `SLEEP.getSleepStageData()`.
    SleepRemText,
    /// Czas czuwania w nocy z fazy `WAKE_STAGE`.
    SleepWakeText,
    /// Godzina rozpoczęcia snu z `SLEEP.getBasicInfo().startTime`.
    SleepStartText,
    /// Godzina zakończenia snu z `SLEEP.getBasicInfo().endTime`.
    SleepEndText,
    /// Spalone kalorie z `hmSensor.id.CALORIE`.
    CalorieText,
    /// SpO₂ z `hmSensor.id.SPO2`.
    Spo2Text,
    /// Średnie SpO₂ z bieżącej godziny (`SPO2.hourAvgofDay`).
    Spo2HourAvgText,
    /// Poziom baterii z `hmSensor.id.BATTERY` (widget tekstowy).
    BatteryText,
    /// Cel kroków z `hmSensor.id.STEP.target`.
    StepTargetText,
    /// Cel kalorii z `hmSensor.id.CALORIE.target`.
    CalorieTargetText,
    /// Cel godzin stania z `hmSensor.id.STAND.target`.
    StandTargetText,
    /// Cel minut spalania tłuszczu z `hmSensor.id.FAT_BURRING.target`.
    FatBurningTargetText,
    /// Bieżące godziny stania z `hmSensor.id.STAND.current`.
    StandText,
    /// Bieżące minuty spalania tłuszczu z `hmSensor.id.FAT_BURRING.current`.
    FatBurningText,
    /// Skumulowane PAI z `hmSensor.id.PAI.totalpai`.
    PaiWeeklyText,
    /// Suma PAI z 7 ostatnich dni (`dailypai` + `prepai0…5`).
    PaiPrepaiText,
    /// Cel snu z `hmSetting.getSleepTarget()` (minuty).
    SleepTargetText,
    /// Nazwa pogody z `forecastData.data[dayOffset].index`.
    WeatherConditionText,
    /// Miasto z `weather.getForecastWeather().cityName` (zawsze dzień 0).
    WeatherCityText,
    /// Temperatura bieżąca — zwykły edytowalny tekst, bez wiązania z czujnikiem.
    WeatherCurrentText,
    /// Najwyższa temperatura z `forecastData.data[dayOffset].high`.
    WeatherHighText,
    /// Najniższa temperatura z `forecastData.data[dayOffset].low`.
    WeatherLowText,
    /// Wschód słońca z `tideData.data[dayOffset].sunrise`.
    SunriseText,
    /// Zachód słońca z `tideData.data[dayOffset].sunset`.
    SunsetText,
    /// Czas do najbliższego wschodu/zachodu z `tideData` i czujnika TIME.
    SunCurrentText,
    /// Ikona pogody — jeden z 29 plików wybierany w runtime przez indeks pogody.
    WeatherIcon,
    /// Ikona stanu systemu rysowana przez `hmUI.widget.IMG_STATUS`.
    StatusIcon,
    /// Zegar analogowy: kilka widgetów `IMG` obracanych wokół środka tarczy.
    AnalogTime,
    /// Łuk postępu: `hmUI.widget.ARC_PROGRESS` (podkład + wartość lub segmenty).
    ArcProgress,
    /// Pasek postępu: `hmUI.widget.FILL_RECT` z zaokrąglonymi rogami.
    BarProgress,
    /// Niewidoczna strefa klikalna: `hmUI.widget.IMG_CLICK` z pustym `src`.
    Shortcut,
}

impl WidgetKind {
    /// Tylko czas i data mają edytowalny wzorzec. Pozostałe typy (bateria,
    /// pogoda, czujniki, obrazy) pobierają wartość z czujnika lub z zasobu —
    /// zapisanie `timeFormat` nadpisałoby ich tekst na `00:00:00`.
    pub fn uses_pattern(self) -> bool {
        matches!(self, Self::TimeText | Self::DateText)
    }

    /// Tekst początkowy widgetu zdrowotnego z jednostką lub bez niej.
    /// PC nie ma czujników, więc podgląd i pierwsza klatka tarczy pokazują `--`.
    pub fn health_initial_text(self, show_unit: bool) -> Option<String> {
        let (base, unit) = match self {
            Self::HeartRateText
            | Self::HeartRateLastText
            | Self::HeartRateMinText
            | Self::HeartRateMaxText
            | Self::HeartRateAvgText => ("--", Some(" bpm")),
            Self::StressText => ("--", None),
            Self::PaiText => ("--", None),
            Self::SkinTempText => ("--", Some(" °C")),
            Self::SleepText => ("--:--", None),
            Self::CalorieText => ("--", Some(" kcal")),
            Self::Spo2Text | Self::Spo2HourAvgText => ("--", Some("%")),
            Self::StandText => ("--", Some(" h")),
            Self::FatBurningText => ("--", Some(" min")),
            Self::PaiWeeklyText | Self::PaiPrepaiText => ("--", None),
            _ => return None,
        };
        Some(match (show_unit, unit) {
            (true, Some(unit)) => format!("{base}{unit}"),
            _ => base.to_string(),
        })
    }

    /// Widgety kategorii DATA — treść jest generowana, nie wpisywana przez użytkownika.
    pub fn is_date_field(self) -> bool {
        matches!(
            self,
            Self::WeekdayNumberText
                | Self::WeekNumberText
                | Self::DayOfYearText
                | Self::LunarDateText
                | Self::FestivalText
                | Self::WorldClockText
        )
    }

    /// Widgety kategorii Zdrowie — nie mają wzorca czasu/daty ani własnej treści.
    pub fn is_health(self) -> bool {
        matches!(
            self,
            Self::HeartRateText
                | Self::HeartRateLastText
                | Self::HeartRateMinText
                | Self::HeartRateMaxText
                | Self::HeartRateAvgText
                | Self::StressText
                | Self::PaiText
                | Self::SkinTempText
                | Self::SleepText
                | Self::CalorieText
                | Self::Spo2Text
                | Self::Spo2HourAvgText
                | Self::StandText
                | Self::FatBurningText
                | Self::PaiWeeklyText
                | Self::PaiPrepaiText
        )
    }

    /// Widgety z `hmSensor.id.SLEEP` (nowa kategoria SNU w katalogu).
    pub fn is_sleep(self) -> bool {
        matches!(
            self,
            Self::SleepText
                | Self::SleepScoreText
                | Self::SleepDeepText
                | Self::SleepLightText
                | Self::SleepRemText
                | Self::SleepWakeText
                | Self::SleepStartText
                | Self::SleepEndText
        )
    }

    /// Początkowy tekst fazy snu zależy od wybranego trybu (minuty albo %).
    pub fn sleep_stage_initial_text(mode: SleepStageMode) -> &'static str {
        match mode {
            SleepStageMode::Duration => "-- min",
            SleepStageMode::Percent => "--%",
        }
    }

    /// Widgety kategorii CELE — czytają pole `.target` czujnika zamiast `.current`.
    pub fn is_goal(self) -> bool {
        matches!(
            self,
            Self::StepTargetText
                | Self::CalorieTargetText
                | Self::StandTargetText
                | Self::FatBurningTargetText
                | Self::SleepTargetText
        )
    }

    /// Widgety kategorii Pogoda — czytają `hmSensor.id.WEATHER`.
    pub fn is_weather(self) -> bool {
        matches!(
            self,
            Self::WeatherConditionText
                | Self::WeatherCityText
                | Self::WeatherCurrentText
                | Self::WeatherHighText
                | Self::WeatherLowText
                | Self::SunriseText
                | Self::SunsetText
                | Self::SunCurrentText
                | Self::WeatherIcon
        )
    }

    /// Tekst początkowy pola celu — na PC i przed pierwszym odczytem czujnika.
    /// `show_unit` decyduje o sufiksie (kcal / h / min), tak samo jak w JS.
    pub fn goal_initial_text(self, show_unit: bool) -> Option<String> {
        let (base, unit) = match self {
            Self::StepTargetText => ("10000", None),
            Self::CalorieTargetText => ("2000", Some(" kcal")),
            Self::StandTargetText => ("12", Some(" h")),
            Self::FatBurningTargetText => ("45", Some(" min")),
            Self::SleepTargetText => ("--", Some(" h")),
            _ => return None,
        };
        Some(match (show_unit, unit) {
            (true, Some(unit)) => format!("{base}{unit}"),
            _ => base.to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Time,
    Date,
    WeekDayNumber,
    WeekNumber,
    DayOfYear,
    LunarDate,
    Festival,
    WorldClock,
    Battery,
    Step,
    Distance,
    HeartRate,
    /// Ostatni prawidłowy odczyt z `HEART.last`.
    HeartRateLast,
    /// Statystyki z `HEART.today` (tablica wartości z poszczególnych minut).
    HeartRateMin,
    HeartRateMax,
    HeartRateAvg,
    Stress,
    Pai,
    SkinTemp,
    Sleep,
    /// Pola `hmSensor.id.SLEEP` (wynik, fazy i godziny snu).
    SleepScore,
    SleepDeep,
    SleepLight,
    SleepRem,
    /// Czas czuwania w nocy z fazy `WAKE_STAGE`.
    SleepWake,
    SleepStart,
    SleepEnd,
    Calorie,
    Spo2,
    /// Średnie SpO₂ z bieżącej godziny (`SPO2.hourAvgofDay`).
    Spo2HourAvg,
    StepTarget,
    CalorieTarget,
    StandTarget,
    FatBurningTarget,
    Stand,
    FatBurning,
    PaiWeekly,
    /// Suma PAI z dzisiejszego dnia i sześciu poprzednich.
    PaiPrepai,
    SleepTarget,
    WeatherCondition,
    WeatherCity,
    /// Edytowalny tekst temperatury bieżącej (bez czujnika).
    WeatherCurrent,
    WeatherHigh,
    WeatherLow,
    Sunrise,
    Sunset,
    /// Czas do najbliższego wschodu/zachodu, liczony z tideData i TIME.
    SunCurrent,
    WeatherIcon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    /// Tarcze ZeppOS używają globalnego `hmUI`, nie modułu `@zos/ui` (ten ostatni
    /// jest przeznaczony dla Mini Program i nie renderuje widgetów na tarczy).
    pub fn as_js(self) -> &'static str {
        match self {
            Align::Left => "hmUI.align.LEFT",
            Align::Center => "hmUI.align.CENTER_H",
            Align::Right => "hmUI.align.RIGHT",
        }
    }
}

/// Pionowe wyrównanie tekstu w ramce (`align_v` w `@zos/ui`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VAlign {
    Top,
    #[default]
    Center,
    Bottom,
}

impl VAlign {
    pub fn as_js(self) -> &'static str {
        match self {
            VAlign::Top => "hmUI.align.TOP",
            VAlign::Center => "hmUI.align.CENTER_V",
            VAlign::Bottom => "hmUI.align.BOTTOM",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            VAlign::Top => "top",
            VAlign::Center => "center",
            VAlign::Bottom => "bottom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "top" => Some(Self::Top),
            "center" | "middle" => Some(Self::Center),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

/// Zachowanie ZeppOS przy przepełnieniu tekstu (`text_style` w `@zos/ui`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextStyle {
    /// Pojedyncza linia z wielokropkiem.
    #[default]
    Ellipsis,
    /// Tekst przewijany (scrolling).
    None,
    /// Zawijanie do kolejnych linii.
    Wrap,
}

impl TextStyle {
    pub fn as_js(self) -> &'static str {
        match self {
            TextStyle::Ellipsis => "hmUI.text_style.ELLIPSIS",
            TextStyle::None => "hmUI.text_style.NONE",
            // API tarczy nie ma WRAP — przewijanie jest realizowane przez NONE.
            TextStyle::Wrap => "hmUI.text_style.NONE",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TextStyle::Ellipsis => "ellipsis",
            TextStyle::None => "none",
            TextStyle::Wrap => "wrap",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ellipsis" | "cut" => Some(Self::Ellipsis),
            "none" | "slide" => Some(Self::None),
            "wrap" => Some(Self::Wrap),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShowLevel {
    #[default]
    Normal,
    Aod,
    Edit,
    Always,
}

impl ShowLevel {
    pub fn as_js(self) -> &'static str {
        match self {
            ShowLevel::Normal => "hmUI.show_level.ONLY_NORMAL",
            ShowLevel::Aod => "hmUI.show_level.ONAL_AOD",
            ShowLevel::Edit => "hmUI.show_level.ONLY_EDIT",
            ShowLevel::Always => "hmUI.show_level.ONLY_NORMAL | hmUI.show_level.ONAL_AOD",
        }
    }
}

impl Project {
    pub fn starter(name: impl Into<String>) -> Self {
        Self::starter_with(name, "Rusty Watch", 10_000_101)
    }

    /// Nowy projekt z danymi wpisanymi przez użytkownika przy tworzeniu tarczy.
    pub fn starter_with(
        name: impl Into<String>,
        author: impl Into<String>,
        app_id: u32,
    ) -> Self {
        let name = name.into();
        Project {
            schema_version: SCHEMA_VERSION,
            metadata: Metadata {
                name: name.clone(),
                app_id,
                author: author.into(),
                version: "0.1.0".to_string(),
                description: format!("{} — created with Rusty Watch", name),
            },
            device: DeviceId::Active2Round,
            assets: vec![Asset {
                path: "icon.png".to_string(),
                role: AssetRole::Icon,
            }],
            // Na tym etapie edytor obsługuje wyłącznie jeden widget — wyświetlacz czasu.
            widgets: vec![
                Widget {
                    id: "time".to_string(),
                    name: "Time".to_string(),
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

,
            ],
            custom_code: None,
            measurement_system: MeasurementSystem::Watch,
            first_day_of_week: FirstDayOfWeek::Monday,
            layer_order: vec!["time".to_string()],
        }
    }
}
