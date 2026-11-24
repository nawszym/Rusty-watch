use serde::{Deserialize, Serialize};

use crate::image_fit::FitMode;

pub const SCHEMA_VERSION: u32 = 1;
pub const SCREEN_SIZE: u32 = 466;

/// Identyfikator syntetycznej warstwy tła (zasób `role: background`).
pub const BACKGROUND_LAYER_ID: &str = "background";

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
    /// Wybrany obraz tła wraz z trybem przygotowania `assets/bg.png`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<BackgroundConfig>,
    /// Kolejność rysowania od spodu do góry: `"background"` oraz id widgetów.
    /// Pusta lista oznacza domyślny porządek (tło, potem widgety wg `z`).
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
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceProfile {
    pub target_key: &'static str,
    pub device_source: u64,
    pub design_width: u32,
    pub api_compatible: &'static str,
    pub api_target: &'static str,
    pub api_min: &'static str,
}

/// Konfiguracja tła zapisana w projekcie. `image` to nazwa pliku z magazynu
/// `%APPDATA%\RustyWatch\Images`; oryginał jest kopiowany do
/// `assets/backgrounds/<image>`, a `assets/bg.png` to wersja gotowa dla tarczy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundConfig {
    pub image: String,
    #[serde(default = "default_auto_scale")]
    pub auto_scale: bool,
}

fn default_auto_scale() -> bool {
    true
}

/// Warstwa jest domyślnie widoczna — starsze projekty nie mają tego pola.
fn default_visible() -> bool {
    true
}

/// Jednostki są domyślnie pokazywane — starsze projekty nie mają tego pola.
fn default_show_unit() -> bool {
    true
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
    Background,
    /// Czcionka skopiowana z `%APPDATA%\RustyWatch\Fonts` do projektu.
    Font,
    /// Oryginał obrazka (np. źródło tła) zachowany w projekcie.
    Image,
    /// Plik wskazówki skopiowany z `%APPDATA%\RustyWatch\Hands` do projektu.
    Hand,
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
    /// Poziom stresu z `hmSensor.id.STRESS`.
    StressText,
    /// PAI z `hmSensor.id.PAI` (wynik dzienny).
    PaiText,
    /// Temperatura skóry z `hmSensor.id.BODY_TEMP`.
    SkinTempText,
    /// Czas snu z `hmSensor.id.SLEEP`.
    SleepText,
    /// Spalone kalorie z `hmSensor.id.CALORIE`.
    CalorieText,
    /// SpO₂ z `hmSensor.id.SPO2`.
    Spo2Text,
    /// Zegar analogowy: kilka widgetów `IMG` obracanych wokół środka tarczy.
    AnalogTime,
}

impl WidgetKind {
    /// Tekst początkowy widgetu zdrowotnego z jednostką lub bez niej.
    /// PC nie ma czujników, więc podgląd i pierwsza klatka tarczy pokazują `--`.
    pub fn health_initial_text(self, show_unit: bool) -> Option<String> {
        let (base, unit) = match self {
            Self::HeartRateText => ("--", Some(" bpm")),
            Self::StressText => ("--", None),
            Self::PaiText => ("--", None),
            Self::SkinTempText => ("--", Some(" °C")),
            Self::SleepText => ("--:--", None),
            Self::CalorieText => ("--", Some(" kcal")),
            Self::Spo2Text => ("--", Some("%")),
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
                | Self::StressText
                | Self::PaiText
                | Self::SkinTempText
                | Self::SleepText
                | Self::CalorieText
                | Self::Spo2Text
        )
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
    Stress,
    Pai,
    SkinTemp,
    Sleep,
    Calorie,
    Spo2,
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
            assets: vec![
                Asset {
                    path: "bg.png".to_string(),
                    role: AssetRole::Background,
                },
                Asset {
                    path: "icon.png".to_string(),
                    role: AssetRole::Icon,
                },
            ],
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
                    show_level: ShowLevel::Always,
                    auto_scale: false,
                    text_style: TextStyle::Ellipsis,
                    visible: true,
                    measurement_system: None,
                    show_unit: true,
                },
            ],
            custom_code: None,
            background: None,
            measurement_system: MeasurementSystem::Watch,
            first_day_of_week: FirstDayOfWeek::Monday,
            layer_order: vec![
                BACKGROUND_LAYER_ID.to_string(),
                "time".to_string(),
            ],
        }
    }
}
