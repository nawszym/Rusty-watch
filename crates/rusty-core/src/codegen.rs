use std::collections::HashMap;

use serde_json::json;

use crate::locale::WatchfaceLocale;
use crate::model::{
    Align, DataSource, ProgressSettings, ProgressSource, Project, StatusType, Widget, WidgetKind,
};
use crate::time_format;

/// Czujnik zdrowotny obsługiwany przez wspólny mechanizm w generowanym JS.
/// Wszystkie instancje tworzone są leniwie i w `try/catch`, a nasłuchiwanie
/// zdejmowane w `onDestroy` — brak czujnika na zegarku nie może położyć tarczy.
#[derive(Clone, Copy)]
struct HealthSensor {
    source: DataSource,
    var: &'static str,
    id: &'static str,
    getter: &'static str,
    event: &'static str,
    read_var: &'static str,
    /// Wyrażenie wartości bez jednostki.
    value: &'static str,
    /// Opcjonalna jednostka dołączana, gdy widget ma `show_unit`.
    unit: Option<&'static str>,
}

static HEALTH_SENSORS: &[HealthSensor] = &[
    HealthSensor {
        source: DataSource::HeartRate,
        var: "healthHeartSensor",
        id: "HEART",
        getter: "_getHealthHeart",
        event: "LAST",
        read_var: "_hHeart",
        // `current` wymaga ciągłego pomiaru (onCurrentChange) — zjadłoby baterię.
        // `last` to ostatni zarejestrowany pomiar i ma własne zdarzenie LAST.
        value: "_hHeart.last ? String(_hHeart.last) : '--'",
        unit: Some("bpm"),
    },
    HealthSensor {
        source: DataSource::HeartRateLast,
        var: "healthHeartSensor",
        id: "HEART",
        getter: "_getHealthHeart",
        event: "LAST",
        read_var: "_hHeart",
        value: "_hHeart.last ? String(_hHeart.last) : '--'",
        unit: Some("bpm"),
    },
    HealthSensor {
        source: DataSource::HeartRateMin,
        var: "healthHeartSensor",
        id: "HEART",
        getter: "_getHealthHeart",
        event: "LAST",
        read_var: "_hHeart",
        value: "_hrStats.min",
        unit: Some("bpm"),
    },
    HealthSensor {
        source: DataSource::HeartRateMax,
        var: "healthHeartSensor",
        id: "HEART",
        getter: "_getHealthHeart",
        event: "LAST",
        read_var: "_hHeart",
        value: "_hrStats.max",
        unit: Some("bpm"),
    },
    HealthSensor {
        source: DataSource::HeartRateAvg,
        var: "healthHeartSensor",
        id: "HEART",
        getter: "_getHealthHeart",
        event: "LAST",
        read_var: "_hHeart",
        value: "_hrStats.avg",
        unit: Some("bpm"),
    },
    HealthSensor {
        source: DataSource::Stress,
        var: "healthStressSensor",
        id: "STRESS",
        getter: "_getHealthStress",
        event: "CHANGE",
        read_var: "_hStress",
        value: "_hStress.current ? String(_hStress.current) : '--'",
        unit: None,
    },
    HealthSensor {
        source: DataSource::Pai,
        var: "healthPaiSensor",
        id: "PAI",
        getter: "_getHealthPai",
        event: "CHANGE",
        read_var: "_hPai",
        value: "String(_hPai.dailypai || 0)",
        unit: None,
    },
    HealthSensor {
        source: DataSource::SkinTemp,
        var: "healthSkinTempSensor",
        id: "BODY_TEMP",
        getter: "_getHealthSkinTemp",
        event: "CHANGE",
        read_var: "_hTemp",
        value: "_hTemp.current ? String(_hTemp.current.toFixed(1)) : '--'",
        unit: Some("°C"),
    },
    HealthSensor {
        source: DataSource::Sleep,
        var: "healthSleepSensor",
        id: "SLEEP",
        getter: "_getHealthSleep",
        event: "CHANGE",
        read_var: "_hSleep",
        value: "_fmtDuration(_hSleep.getTotalTime())",
        unit: None,
    },
    HealthSensor {
        source: DataSource::Calorie,
        var: "healthCalorieSensor",
        id: "CALORIE",
        getter: "_getHealthCalorie",
        event: "CHANGE",
        read_var: "_hCalorie",
        value: "String(Math.round(_hCalorie.current || 0))",
        unit: Some("kcal"),
    },
    HealthSensor {
        source: DataSource::Spo2,
        var: "healthSpo2Sensor",
        id: "SPO2",
        getter: "_getHealthSpo2",
        event: "CHANGE",
        read_var: "_hSpo2",
        value: "_hSpo2.current ? String(_hSpo2.current) : '--'",
        unit: Some("%"),
    },
    HealthSensor {
        source: DataSource::Spo2HourAvg,
        var: "healthSpo2Sensor",
        id: "SPO2",
        getter: "_getHealthSpo2",
        event: "CHANGE",
        read_var: "_hSpo2",
        value: "(_hSpo2.hourAvgofDay && _hSpo2.hourAvgofDay[_spo2Hour] > 0) ? String(_hSpo2.hourAvgofDay[_spo2Hour]) : '--'",
        unit: Some("%"),
    },
    HealthSensor {
        source: DataSource::Stand,
        var: "healthStandSensor",
        id: "STAND",
        getter: "_getHealthStand",
        event: "CHANGE",
        read_var: "_hStand",
        value: "_hStand.current ? String(_hStand.current) : '--'",
        unit: Some("h"),
    },
    HealthSensor {
        source: DataSource::FatBurning,
        var: "healthFatBurnSensor",
        id: "FAT_BURRING",
        getter: "_getHealthFatBurning",
        event: "CHANGE",
        read_var: "_hFatBurn",
        value: "_hFatBurn.current ? String(_hFatBurn.current) : '--'",
        unit: Some("min"),
    },
    // Suma z dzisiejszego `dailypai` i sześciu poprzednich dni (`prepai0…5`).
    // Współdzieli getter i czujnik z dziennym PAI — nie tworzymy drugiego listenera.
    HealthSensor {
        source: DataSource::PaiPrepai,
        var: "healthPaiSensor",
        id: "PAI",
        getter: "_getHealthPai",
        event: "CHANGE",
        read_var: "_hPai",
        value: "String((_hPai.dailypai || 0) + (_hPai.prepai0 || 0) + (_hPai.prepai1 || 0) + (_hPai.prepai2 || 0) + (_hPai.prepai3 || 0) + (_hPai.prepai4 || 0) + (_hPai.prepai5 || 0))",
        unit: None,
    },
    // `totalpai` to skumulowane PAI (tydzień). Własny getter, więc nie koliduje
    // z dziennym `dailypai` — oba czytają ten sam czujnik `hmSensor.id.PAI`.
    HealthSensor {
        source: DataSource::PaiWeekly,
        var: "healthPaiWeeklySensor",
        id: "PAI",
        getter: "_getHealthPaiWeekly",
        event: "CHANGE",
        read_var: "_hPaiWeekly",
        value: "String(_hPaiWeekly.totalpai || 0)",
        unit: None,
    },
];

fn health_sensor(source: Option<DataSource>) -> Option<&'static HealthSensor> {
    let source = source?;
    HEALTH_SENSORS.iter().find(|sensor| sensor.source == source)
}

/// Pole celu czytane z czujnika. W przeciwieństwie do zdrowotnych `.current`,
/// cele używają `.target`. Czujniki STAND i FAT_BURRING mogą być nieobecne na
/// Active 2 — getter zawsze sprawdza `hmSensor.id` i łapie wyjątek.
#[derive(Clone, Copy)]
struct TargetSensor {
    source: DataSource,
    var: &'static str,
    id: &'static str,
    getter: &'static str,
    read_var: &'static str,
    unit: Option<&'static str>,
}

static TARGET_SENSORS: &[TargetSensor] = &[
    // Kroki współdzielą już istniejący `stepSensor` z `_getStep()`.
    TargetSensor {
        source: DataSource::StepTarget,
        var: "stepSensor",
        id: "STEP",
        getter: "_getStep",
        read_var: "_tStep",
        unit: None,
    },
    TargetSensor {
        source: DataSource::CalorieTarget,
        var: "calorieTargetSensor",
        id: "CALORIE",
        getter: "_getTargetCalorie",
        read_var: "_tCalorie",
        unit: Some("kcal"),
    },
    TargetSensor {
        source: DataSource::StandTarget,
        var: "standSensor",
        id: "STAND",
        getter: "_getTargetStand",
        read_var: "_tStand",
        unit: Some("h"),
    },
    TargetSensor {
        source: DataSource::FatBurningTarget,
        var: "fatBurnSensor",
        id: "FAT_BURRING",
        getter: "_getTargetFatBurning",
        read_var: "_tFat",
        unit: Some("min"),
    },
];

fn target_sensor(source: Option<DataSource>) -> Option<&'static TargetSensor> {
    let source = source?;
    TARGET_SENSORS.iter().find(|sensor| sensor.source == source)
}

/// Pole pogody czytane z `hmSensor.id.WEATHER.getForecastWeather()`.
/// Wszystkie korzystają z jednego czujnika i jednego odczytu w `weatherTick`.
#[derive(Clone, Copy)]
struct WeatherField {
    source: DataSource,
    /// Wyrażenie JS zwracające tekst (zmienne `_wData`, `_wForecast`, `_wTide`).
    value: &'static str,
}

static WEATHER_FIELDS: &[WeatherField] = &[
    WeatherField {
        source: DataSource::WeatherCondition,
        value: "L.weather[Math.max(0, Math.min(28, (_wForecast.data[$D] ? _wForecast.data[$D].index : 25) | 0))] || L.weather[25]",
    },
    WeatherField {
        source: DataSource::WeatherCity,
        value: "_cityName(_wData && _wData.cityName)",
    },
    WeatherField {
        source: DataSource::WeatherHigh,
        value: "(_wForecast.data[$D] && _wForecast.data[$D].high !== undefined) ? String(_wForecast.data[$D].high) + '°' : '--°'",
    },
    WeatherField {
        source: DataSource::WeatherLow,
        value: "(_wForecast.data[$D] && _wForecast.data[$D].low !== undefined) ? String(_wForecast.data[$D].low) + '°' : '--°'",
    },
    WeatherField {
        source: DataSource::Sunrise,
        value: "(_wTide.data[$D] && _wTide.data[$D].sunrise) ? p2(_wTide.data[$D].sunrise.hour) + ':' + p2(_wTide.data[$D].sunrise.minute) : '--:--'",
    },
    WeatherField {
        source: DataSource::Sunset,
        value: "(_wTide.data[$D] && _wTide.data[$D].sunset) ? p2(_wTide.data[$D].sunset.hour) + ':' + p2(_wTide.data[$D].sunset.minute) : '--:--'",
    },
    // Czas do najbliższego wschodu/zachodu — dzień jest wybierany wewnątrz
    // helpera (dziś albo jutro), więc nie używa `$D` jak pozostałe pola.
    WeatherField {
        source: DataSource::SunCurrent,
        value: "_sunCurrentText()",
    },
];

fn weather_field(source: Option<DataSource>) -> Option<&'static WeatherField> {
    let source = source?;
    WEATHER_FIELDS.iter().find(|field| field.source == source)
}

/// Wyrażenie JS dla pola snu. Fazy mogą być pokazane w minutach albo jako
/// udział procentowy całego snu — wybór zapisujemy w ustawieniach widgetu.
fn sleep_value(source: DataSource, mode: crate::model::SleepStageMode) -> String {
    let percent = mode == crate::model::SleepStageMode::Percent;
    match source {
        DataSource::SleepScore => {
            "(_sBasic && _sBasic.score !== undefined && _sBasic.score !== null) ? String(_sBasic.score) : '--'".to_string()
        }
        DataSource::SleepDeep => format!("_sleepStageText(_sModels && _sModels.DEEP_STAGE, {percent})"),
        DataSource::SleepLight => format!("_sleepStageText(_sModels && _sModels.LIGHT_STAGE, {percent})"),
        DataSource::SleepRem => format!("_sleepStageText(_sModels && _sModels.REM_STAGE, {percent})"),
        DataSource::SleepWake => format!("_sleepStageText(_sModels && _sModels.WAKE_STAGE, {percent})"),
        DataSource::SleepStart => "_fmtSleepHM(_sBasic ? _sBasic.startTime : null)".to_string(),
        DataSource::SleepEnd => {
            "_fmtSleepHM(_sBasic && _sBasic.endTime !== undefined ? _sBasic.endTime + 1 : null)"
                .to_string()
        }
        _ => "'--'".to_string(),
    }
}

/// Wszystkie warianty Active 2, które ZeppOS rozpoznaje na tarczy.
/// Emulator używa zwykle 8913153, ale prawdziwy zegarek może zgłosić inny
/// `deviceSource` — manifest musi zawierać całą rodzinę, inaczej instalacja
/// z telefonu kończy się „download failed".
pub const COMPATIBLE_DEVICE_SOURCES: [(&str, u64); 8] = [
    ("Amazfit Active 2", 8_913_152),
    ("Amazfit Active 2", 8_913_153),
    ("Amazfit Active 2", 8_913_155),
    ("Amazfit Active 2", 8_913_159),
    ("Amazfit Active 2", 10_092_800),
    ("Amazfit Active 2", 10_092_801),
    ("Amazfit Active 2", 10_092_803),
    ("Amazfit Active 2", 10_092_807),
];

/// Nazwa pliku miniatury tarczy wewnątrz paczki. Zegarek szuka dokładnie
/// `assets/Preview.png` (314 × 314 RGBA) — tak samo jak referencyjna tarcza
/// z innego edytora. Zmiana wielkości liter lub rozdzielczości powoduje, że
/// tarcza nie ma podglądu w menu zegarka.
pub const WATCHFACE_PREVIEW_ICON: &str = "Preview.png";

/// Generuje `app.json` w płaskim formacie v2 — takim, jaki przyjmuje instalator
/// na prawdziwym zegarku. Blok `targets` (v3) jest formatem roboczym `zeus` i na
/// zegarku kończy się błędem „Failed to unzip the package"; dodajemy go wyłącznie
/// w kopii przeznaczonej dla symulatora (patrz `prepare_emulator_workspace`).
pub fn generate_app_json(project: &Project) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let profile = project.device.profile();
    let platforms: Vec<serde_json::Value> = COMPATIBLE_DEVICE_SOURCES
        .iter()
        .map(|(name, source)| json!({ "name": name, "deviceSource": source }))
        .collect();
    let time_stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let icon = WATCHFACE_PREVIEW_ICON;
    let app = json!({
        "configVersion": "v2",
        "app": {
            "appIdType": 0,
            "appId": project.metadata.app_id,
            "appName": project.metadata.name,
            "appType": "watchface",
            "version": { "code": 1, "name": project.metadata.version },
            // Manifest w 100% zgodny z działającą tarczą referencyjną.
            "vender": "zepp",
            "description": "",
            "icon": icon,
            "cover": [icon],
            "extraInfo": { "madeBy": 1, "fromZoom": false }
        },
        "permissions": [],
        "runtime": {
            // Tarcza korzysta z klasycznego API `hmUI`. Wersje 3.x ładują
            // moduły `@zos/*` i nie udostępniają globalnego `hmUI`.
            // Nie ustawiamy `type`: 0 = interpreter QuickJS dla plików .js.
            "apiVersion": {
                "compatible": "1.0.0",
                "target": "1.0.1",
                "minVersion": "1.0.0"
            }
        },
        // Referencja ma tylko `en-US`. Dodatkowy `pl-PL` może skłonić firmware
        // do ładowania nieistniejącego katalogu językowego i przerwać build tarczy.
        "i18n": {
            "en-US": { "appName": project.metadata.name, "icon": icon }
        },
        "defaultLanguage": "en-US",
        "debug": false,
        "module": {
            "watchface": {
                "path": "watchface/index",
                "main": 1,
                "editable": 0,
                "lockscreen": 0,
                "hightCost": 0
            }
        },
        "designWidth": profile.design_width,
        "platforms": platforms,
        "packageInfo": {
            "mode": "production",
            "timeStamp": time_stamp,
            "expiredTime": 172800,
            "zpm": "2.8.2"
        }
    });
    serde_json::to_string_pretty(&app).expect("app json is serializable")
}

/// Bootstrap aplikacji — dosłowny szablon z oficjalnych paczek `zeus`
/// (`wfeditor/templates/app.js` w działającym generatorze Python).
///
/// Realny firmware oczekuje nie tylko `__globals__`, ale też `gettextFactory`
/// i helperów `getApp`/`getCurrentPage`. Uproszczona wersja bez tych elementów
/// działała w symulatorze, ale na zegarku przerywała rejestrację modułu.
pub fn generate_app_js() -> String {
    // Szablon w paczce referencyjnej ma zakończenia CRLF — zwracamy identyczne bajty.
    r#"try {
    (() => {
        const __$$app$$__ = __$$hmAppManager$$__.currentApp;
        function getApp() {
            return __$$app$$__.app;
        }
        function getCurrentPage() {
            return __$$app$$__.current && __$$app$$__.current.module;
        }
        __$$app$$__.__globals__ = {
            lang: new DeviceRuntimeCore.HmUtils.Lang(DeviceRuntimeCore.HmUtils.getLanguage()),
            px: DeviceRuntimeCore.HmUtils.getPx(454)
        };
        const {px} = __$$app$$__.__globals__;
        const languageTable = {};
        __$$app$$__.__globals__.gettext = DeviceRuntimeCore.HmUtils.gettextFactory(languageTable, __$$app$$__.__globals__.lang, 'en-US');
        __$$app$$__.app = DeviceRuntimeCore.App({
            globalData: {},
            onCreate(options) {
            },
            onDestroy(options) {
            },
            onError(error) {
            },
            onPageNotFound(obj) {
            },
            onUnhandledRejection(obj) {
            }
        });
        ;
    })();
} catch (e) {
    console.log('Mini Program Error', e);
    e && e.stack && e.stack.split(/\n/).forEach(i => console.log('error stack', i));
    ;
}
"#.replace('\n', "\r\n")
}

/// Ustawienia postępu są zawsze zapisane w `Widget::progress`.
fn progress_of(w: &Widget) -> &ProgressSettings {
    w.progress
        .as_ref()
        .expect("progress widget without settings")
}

/// Parametry `ARC_PROGRESS` współdzielone przez `createWidget` i `setProperty`.
fn arc_params(p: &ProgressSettings, cx: i32, cy: i32, radius: i32, color: &str, level: &str) -> String {
    format!(
        "center_x: {cx}, center_y: {cy}, radius: {radius}, start_angle: {}, end_angle: {}, line_width: {}, color: {color}, corner_flag: {}, level: {level}, show_level: {}",
        p.start_angle, p.end_angle, p.thickness, p.corner.corner_flag(), JS_SHOW_LEVEL
    )
}

/// Promień rogów paska — „Zaokrąglone końce" wyłączone oznacza radius 0.
fn progress_radius(w: &Widget, p: &ProgressSettings) -> u32 {
    if p.corner.as_str() == "flat" {
        0
    } else {
        // Zaokrąglenia są włączone lub wyłączone — kwadrat staje się kołem.
        w.w.min(w.h).max(0) as u32 / 2
    }
}

/// Parametry `FILL_RECT` współdzielone przez `createWidget` i `setProperty`.
fn rect_params(x: i32, y: i32, w: i32, h: i32, color: &str, radius: u32) -> String {
    format!(
        "x: {x}, y: {y}, w: {w}, h: {h}, color: {color}, radius: {radius}, show_level: {}",
        JS_SHOW_LEVEL
    )
}

/// Geometria łuku wyliczona z ramki widgetu (zachowuje kwadrat `w == h`).
fn arc_geometry(w: &Widget) -> (i32, i32, i32) {
    let cx = w.x + w.w / 2;
    let cy = w.y + w.h / 2;
    let radius = crate::model::arc_center_radius(w.w, w.h);
    (cx, cy, radius)
}

/// Getter czujnika dla wybranego źródła postępu (bez duplikowania ticków tekstu).
fn progress_sensor_id(source: ProgressSource) -> Option<&'static str> {
    match source {
        ProgressSource::Battery => Some("BATTERY"),
        ProgressSource::Steps => Some("STEP"),
        ProgressSource::Distance => Some("DISTANCE"),
        ProgressSource::Heart => Some("HEART"),
        ProgressSource::Calories => Some("CALORIE"),
        ProgressSource::Stand => Some("STAND"),
        ProgressSource::FatBurning => Some("FAT_BURRING"),
        ProgressSource::Pai => Some("PAI"),
        ProgressSource::Spo2 => Some("SPO2"),
        ProgressSource::Sleep => Some("SLEEP"),
        ProgressSource::Fixed => None,
    }
}

/// Zdarzenie czujnika, które odświeża postęp — tak samo jak w pozostałych
/// widgetach (HEART używa CURRENT, reszta CHANGE).
fn progress_sensor_event(source: ProgressSource) -> Option<&'static str> {
    match source {
        ProgressSource::Fixed => None,
        ProgressSource::Heart => Some("LAST"),
        _ => Some("CHANGE"),
    }
}

/// Funkcja zwracająca postęp 0…1 dla jednego widgetu.
fn progress_value_fn(index: usize, w: &Widget) -> String {
    let p = progress_of(w);
    let mut out = format!("let _progSensor_{index} = null\n");
    if p.source == ProgressSource::Sleep {
        out.push_str(&format!("let _progSleepMinute_{index} = -1\n"));
    }
    out.push_str(&format!("function _progValue_{index}() {{\n"));
    match p.source {
        ProgressSource::Fixed => {
            out.push_str(&format!("  return {}\n", (p.value / 100.0).clamp(0.0, 1.0)));
        }
        ProgressSource::Sleep => {
            out.push_str(&format!(
                "  const s = _progSensor_{index} || (_progSensor_{index} = _createProgSensor('SLEEP'))\n"
            ));
            // Na zegarku `getTotalTime()` zwraca 0 dopóki czujnik nie pobierze danych —
            // text widget snu robi to samo w sleepTick.
            out.push_str(&format!(
                "  if (s && s.updateInfo) {{\n    const _pt = _getTime()\n    const _pm = _pt ? (_pt.hour * 60 + _pt.minute) : -1\n    if (_progSleepMinute_{index} !== _pm) {{ _progSleepMinute_{index} = _pm\n      try {{ s.updateInfo() }} catch (e) {{}}\n    }}\n  }}\n"
            ));
            out.push_str("  let raw = 0\n  if (s && s.getTotalTime) { try { raw = s.getTotalTime() || 0 } catch (e) { raw = 0 } }\n");
            out.push_str("  let hi = 100\n  try { if (typeof hmSetting !== 'undefined' && hmSetting.getSleepTarget) hi = hmSetting.getSleepTarget() || 100 } catch (e) { hi = 100 }\n");
            out.push_str(&format!("  const lo = {}\n", p.min));
        }
        source => {
            let id = progress_sensor_id(source).unwrap();
            let field = match source {
                ProgressSource::Pai => "dailypai",
                // Ostatni pomiar — bez uruchamiania ciągłego monitoringu.
                ProgressSource::Heart => "last",
                _ => "current",
            };
            out.push_str(&format!(
                "  const s = _progSensor_{index} || (_progSensor_{index} = _createProgSensor('{id}'))\n"
            ));
            out.push_str(&format!("  const raw = s ? (s.{field} || 0) : 0\n"));
            let hi = if matches!(
                source,
                ProgressSource::Steps
                    | ProgressSource::Calories
                    | ProgressSource::Stand
                    | ProgressSource::FatBurning
            ) {
                // Cel ustawiony w aplikacji Zepp ma zawsze pierwszeństwo przed
                // zapisanym max — inaczej 2500/5000 kroków dawało 25%, nie 50%.
                "(s && s.target ? s.target : 100)".to_string()
            } else if source == ProgressSource::Heart {
                "165".to_string()
            } else if p.max > p.min {
                p.max.to_string()
            } else {
                "100".to_string()
            };
            out.push_str(&format!("  const lo = {}, hi = {}\n", p.min, hi));
        }
    }
    out.push_str("  let v = hi > lo ? (raw - lo) / (hi - lo) : 0\n");
    if p.invert {
        out.push_str("  v = 1 - v\n");
    }
    out.push_str("  return v < 0 ? 0 : (v > 1 ? 1 : v)\n}\n");
    out
}

/// Funkcja aktualizująca widgety postępu ( Continuous lub segmenty ).
fn progress_apply_fn(index: usize, w: &Widget) -> String {
    let p = progress_of(w);
    let alpha = if w.alpha < 255 {
        format!("alpha: {}, ", w.alpha)
    } else {
        String::new()
    };
    let mut out = format!(
        "function _progApply_{index}() {{\n  const view = refs[{index}]\n  if (!view) return\n  const v = _progValue_{index}()\n"
    );
    if w.kind == WidgetKind::ArcProgress {
        let (cx, cy, radius) = arc_geometry(w);
        let params = arc_params(p, cx, cy, radius, &p.color, "Math.round(v * 100)");
        out.push_str(&format!(
            "  if (view.fill) view.fill.setProperty(hmUI.prop.MORE, {{ {alpha}{params} }})\n"
        ));
    } else {
        // Wszystkie wymiary są literałami z ramki — w JS nie ma zmiennych
        // `x`, `y`, `w`, `h`. Stąd wcześniejszy ReferenceError i pusty pasek.
        let (x, y, bw, bh) = match p.orientation {
            crate::model::ProgressOrientation::Horizontal => (
                w.x.to_string(),
                w.y.to_string(),
                format!("Math.round({} * v)", w.w),
                w.h.to_string(),
            ),
            crate::model::ProgressOrientation::Vertical => (
                w.x.to_string(),
                format!("{} + {} - Math.round({} * v)", w.y, w.h, w.h),
                w.w.to_string(),
                format!("Math.round({} * v)", w.h),
            ),
        };
        let params = rect_params_js(&x, &y, &bw, &bh, &p.color, progress_radius(w, p), &alpha);
        out.push_str(&format!(
            "  if (view.fill) view.fill.setProperty(hmUI.prop.MORE, {{ {params} }})\n"
        ));
    }
    out.push_str("}\n");
    out
}

/// `rect_params` z wyrażeniami JS zamiast stałych (używane przy aktualizacji paska).
fn rect_params_js(
    x: &str,
    y: &str,
    bw: &str,
    bh: &str,
    color: &str,
    radius: u32,
    alpha: &str,
) -> String {
    format!(
        "{alpha}x: {x}, y: {y}, w: {bw}, h: {bh}, color: {color}, radius: {radius}, show_level: {}",
        JS_SHOW_LEVEL
    )
}

/// `createWidget` dla łuku/paska — zwraca obiekt z podkładem, wypełnieniem
/// i tablicą segmentów (analogicznie do grupy wskazówek).
fn progress_widget_call(w: &Widget) -> String {
    let p = progress_of(w);
    let alpha = if w.alpha < 255 {
        format!("alpha: {}, ", w.alpha)
    } else {
        String::new()
    };
    let mut parts = Vec::new();
    if p.show_track {
        let track = if w.kind == WidgetKind::ArcProgress {
            let (cx, cy, radius) = arc_geometry(w);
            format!(
                "hmUI.createWidget(hmUI.widget.ARC_PROGRESS, {{ {alpha}{} }})",
                arc_params(p, cx, cy, radius, &p.track_color, "100")
            )
        } else {
            format!(
                "hmUI.createWidget(hmUI.widget.FILL_RECT, {{ {alpha}{} }})",
                rect_params(w.x, w.y, w.w, w.h, &p.track_color, progress_radius(w, p))
            )
        };
        parts.push(format!("track: {track}"));
    }
    // Zawsze jedno ciągłe wypełnienie — segmenty nie są już opcją w edytorze.
    let fill = if w.kind == WidgetKind::ArcProgress {
        let (cx, cy, radius) = arc_geometry(w);
        format!(
            "hmUI.createWidget(hmUI.widget.ARC_PROGRESS, {{ {alpha}{} }})",
            arc_params(p, cx, cy, radius, &p.color, "0")
        )
    } else {
        format!(
            "hmUI.createWidget(hmUI.widget.FILL_RECT, {{ {alpha}{} }})",
            rect_params(w.x, w.y, 0, w.h, &p.color, progress_radius(w, p))
        )
    };
    parts.push(format!("fill: {fill}"));
    format!("{{ {} }}", parts.join(", "))
}

/// Generuje `watchface/index.js` zgodnie z oficjalnym Watchface API (`hmUI`).
///
/// `createWidget` z modułu `@zos/ui` przeznaczony jest dla Mini Program i nie
/// renderuje widgetów na tarczy. Generujemy więc globalne `hmUI`, `hmSensor`
/// i `timer` — dokładnie tak, jak w działających tarczach Active 2.
pub fn generate_index_js(project: &Project) -> String {
    generate_index_js_with_hands(project, &HashMap::new())
}

/// Wariant używany przy eksporcie: przyjęte z zewnątrz pozycje wskazówek
/// (po przycięciu przezroczystych marginesów) są wstawiane do wygenerowanego JS.
pub fn generate_index_js_with_hands(
    project: &Project,
    placements: &HashMap<String, HandPlacement>,
) -> String {
    // Podgląd w edytorze i wygenerowany JS rysują dokładnie te same warstwy w tej samej kolejności.
    // Ukryte warstwy nie są rysowane ani w podglądzie, ani w wygenerowanym JS.
    let widgets = crate::layers::ordered_widgets(project);
    let needs_seconds = widgets.iter().any(|w| {
        (w.data_source == Some(DataSource::Time)
            && w.time_format
                .as_deref()
                .map(time_format::has_seconds)
                .unwrap_or(false))
            || (w.kind == WidgetKind::AnalogTime && w.second_src.is_some())
    });
    let step_indexes: Vec<usize> = widgets
        .iter()
        .enumerate()
        .filter(|(_, w)| w.data_source == Some(DataSource::Step))
        .map(|(index, _)| index)
        .collect();
    let battery_indexes: Vec<usize> = widgets
        .iter()
        .enumerate()
        .filter(|(_, w)| w.data_source == Some(DataSource::Battery))
        .map(|(index, _)| index)
        .collect();
    let default_system = project.measurement_system;
    let distance_units: Vec<(usize, crate::model::MeasurementSystem)> = widgets
        .iter()
        .enumerate()
        .filter(|(_, w)| w.data_source == Some(DataSource::Distance))
        .map(|(index, w)| (index, w.measurement_system.unwrap_or(default_system)))
        .collect();
    // „Zgodnie z zegarkiem" — jednostka jest odczytywana raz w runtime z hmSetting.
    let needs_auto_mileage = distance_units.iter().any(|(_, system)| system.is_watch());
    let needs_weekday_number = widgets
        .iter()
        .any(|w| w.data_source == Some(DataSource::WeekDayNumber));
    let needs_iso_week = widgets
        .iter()
        .any(|w| w.data_source == Some(DataSource::WeekNumber));
    let needs_day_of_year = widgets
        .iter()
        .any(|w| w.data_source == Some(DataSource::DayOfYear));
    let needs_world_clock = widgets
        .iter()
        .any(|w| w.data_source == Some(DataSource::WorldClock));
    let weather_widgets: Vec<(usize, &'static WeatherField, u8)> = widgets
        .iter()
        .enumerate()
        .filter_map(|(index, w)| {
            weather_field(w.data_source)
                .map(|field| (index, field, w.weather_day_offset.min(4)))
        })
        .collect();
    let weather_icons: Vec<(usize, String, u8)> = widgets
        .iter()
        .enumerate()
        .filter(|(_, w)| w.kind == WidgetKind::WeatherIcon)
        .filter_map(|(index, w)| {
            let set = w.weather_icon_set.as_deref().unwrap_or(crate::weather::DEFAULT_WEATHER_PACK);
            // `js_string` already emits a complete JS string literal with double
            // quotes. Wrapping it in apostrophes produced entries like
            // '"weather_xxxx.png"' — `setProperty(SRC, …)` then received a path
            // containing literal quotes and the icon never rendered on the watch.
            let names: Vec<String> = (0..crate::weather::ICON_COUNT)
                .map(|i| js_string(&crate::export_paths::exported_asset_name(&crate::weather::js_icon_path(set, i))))
                .collect();
            Some((index, names.join(", "), w.weather_day_offset.min(4)))
        })
        .collect();
    let progress_widgets: Vec<(usize, &Widget)> = widgets
        .iter()
        .enumerate()
        .filter(|(_, w)| matches!(w.kind, WidgetKind::ArcProgress | WidgetKind::BarProgress))
        .collect();
    let sleep_target_indexes: Vec<usize> = widgets
        .iter()
        .enumerate()
        .filter(|(_, w)| w.data_source == Some(DataSource::SleepTarget))
        .map(|(index, _)| index)
        .collect();
    let sleep_widgets: Vec<(usize, DataSource, crate::model::SleepStageMode)> = widgets
        .iter()
        .enumerate()
        .filter_map(|(index, w)| {
            if w.kind.is_sleep() && w.data_source != Some(DataSource::Sleep) {
                w.data_source.map(|source| (index, source, w.sleep_stage_mode))
            } else {
                None
            }
        })
        .collect();
    let health_widgets: Vec<(usize, &'static HealthSensor, bool)> = widgets
        .iter()
        .enumerate()
        .filter_map(|(index, w)| {
            health_sensor(w.data_source).map(|sensor| (index, sensor, w.show_unit))
        })
        .collect();
    let mut used_health: Vec<&'static HealthSensor> = Vec::new();
    for (_, sensor, _) in &health_widgets {
        // Dwa pola mogą czytać ten sam czujnik (np. PAI dzień i tydzień) —
        // rozróżniamy je getterem, żeby wygenerować oba i oba nasłuchy.
        if !used_health.iter().any(|s| s.getter == sensor.getter) {
            used_health.push(sensor);
        }
    }
    let target_widgets: Vec<(usize, &'static TargetSensor, bool)> = widgets
        .iter()
        .enumerate()
        .filter_map(|(index, w)| {
            target_sensor(w.data_source).map(|sensor| (index, sensor, w.show_unit))
        })
        .collect();
    let mut used_targets: Vec<&'static TargetSensor> = Vec::new();
    for (_, sensor, _) in &target_widgets {
        if !used_targets.iter().any(|s| s.id == sensor.id) {
            used_targets.push(sensor);
        }
    }

    let mut prelude = String::new();
    prelude.push_str("// Generated by Rusty Watch v0.1.0 — do not edit by hand.\n");
    prelude.push_str("// Watchface API ZeppOS: hmUI / hmSensor / timer.\n\n");
    // Język tarczy jest wykrywany w runtime — ten sam ZIP działa po polsku i angielsku.
    prelude.push_str(WatchfaceLocale::js_detection());
    prelude.push_str("const p2 = (value) => String(value).padStart(2, '0')\n");
    prelude.push_str("function _screenIsAod() {\n");
    prelude.push_str("  try {\n");
    prelude.push_str("    return typeof hmSetting !== 'undefined' && hmSetting.getScreenType && hmSetting.screen_type &&\n");
    prelude.push_str("      hmSetting.getScreenType() === hmSetting.screen_type.AOD\n");
    prelude.push_str("  } catch (e) { return false }\n");
    prelude.push_str("}\n");
    // Klasyczny firmware tarczy nie udostępnia obiektu `Date` — w emulatorze działa,
    // na zegarku wywala build. Wszystkie pola daty liczymy sami z czujnika TIME.
    prelude.push_str(crate::date_format::JS_WEEKDAY_INDEX);
    prelude.push_str(crate::date_format::JS_DATE_FROM_T);
    if needs_weekday_number {
        prelude.push_str(&crate::date_format::js_weekday_number(
            project.first_day_of_week,
        ));
    }
    if needs_iso_week {
        prelude.push_str(crate::date_format::JS_ISO_WEEK);
    }
    if needs_day_of_year {
        prelude.push_str(crate::date_format::JS_DAY_OF_YEAR);
    }
    if needs_auto_mileage {
        prelude.push_str("let autoMileage = false\n");
        prelude.push_str("function _initMileageUnit() {\n");
        prelude.push_str("  try {\n");
        prelude.push_str("    if (typeof hmSetting !== 'undefined' && hmSetting.getMileageUnit) autoMileage = hmSetting.getMileageUnit() === 1\n");
        prelude.push_str("  } catch (e) { autoMileage = false }\n");
        prelude.push_str("}\n");
    }
    prelude.push_str("let timeSensor = null\nlet tick = null\nlet tickTimer = null\n");
    if !weather_widgets.is_empty() || !weather_icons.is_empty() {
        prelude.push_str("let weatherSensor = null\nlet weatherTick = null\nlet _wData = null\nlet _wForecast = { data: [] }\nlet _wTide = { data: [] }\nlet _lastWeatherIcon = {}\n");
        prelude.push_str("function _getWeather() {\n");
        prelude.push_str("  if (!weatherSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.WEATHER) {\n");
        prelude.push_str("    try { weatherSensor = hmSensor.createSensor(hmSensor.id.WEATHER) } catch (e) { weatherSensor = null }\n");
        prelude.push_str("  }\n  return weatherSensor\n}\n");
        for (index, names, _) in &weather_icons {
            prelude.push_str(&format!(
                "const weatherIcons_{index} = [{names}]\n"
            ));
        }
    }
    if !sleep_target_indexes.is_empty() {
        prelude.push_str("function _sleepTargetText() {\n");
        prelude.push_str("  let minutes = 0\n");
        prelude.push_str("  try { if (typeof hmSetting !== 'undefined' && hmSetting.getSleepTarget) minutes = hmSetting.getSleepTarget() || 0 } catch (e) { minutes = 0 }\n");
        prelude.push_str("  if (!minutes || minutes < 0) return '-- h'\n");
        prelude.push_str("  const hours = Math.floor(minutes / 60)\n  const rest = minutes % 60\n");
        prelude.push_str("  return rest ? (hours + ' h ' + rest + ' min') : (hours + ' h')\n}\n");
    }
    prelude.push_str("let stepSensor = null\nlet stepTick = null\n");
    prelude.push_str("let distanceSensor = null\nlet distanceTick = null\n");
    if !battery_indexes.is_empty() {
        prelude.push_str("let batterySensor = null\nlet batteryTick = null\n");
    }
    if !target_widgets.is_empty() {
        prelude.push_str("let targetTick = null\nlet targetSensors = null\n");
    }
    prelude.push_str("const refs = []\n\n");
    prelude.push_str("function _getTime() {\n");
    prelude.push_str("  if (!timeSensor) timeSensor = hmSensor.createSensor(hmSensor.id.TIME)\n");
    prelude.push_str("  return timeSensor\n}\n");
    if weather_widgets
        .iter()
        .any(|(_, field, _)| field.source == DataSource::SunCurrent)
    {
        // Czas do najbliższego wschodu albo zachodu. Klasyczne API nie ma
        // `Date`, więc liczymy minuty z czujnika TIME i tablicy tideData.
        prelude.push_str(concat!(
            "function _sunCurrentText() {\n",
            "  const nowSensor = _getTime()\n",
            "  if (!nowSensor) return '--:--'\n",
            "  const now = ((nowSensor.hour | 0) * 60) + (nowSensor.minute | 0)\n",
            "  const days = _wTide && _wTide.data\n",
            "  if (!days || !days.length) return '--:--'\n",
            "  let next = -1\n",
            "  for (let d = 0; d < 2 && next < 0; d++) {\n",
            "    const day = days[d]\n",
            "    if (!day) continue\n",
            "    const events = [day.sunrise, day.sunset]\n",
            "    for (let i = 0; i < events.length; i++) {\n",
            "      const e = events[i]\n",
            "      if (!e || e.hour === undefined || e.minute === undefined) continue\n",
            "      const at = d * 1440 + (e.hour | 0) * 60 + (e.minute | 0)\n",
            "      if (at >= now && (next < 0 || at < next)) next = at\n",
            "    }\n  }\n",
            "  if (next < 0) return '--:--'\n",
            "  const left = next - now\n",
            "  return String(Math.floor(left / 60)) + ':' + String(left % 60).padStart(2, '0')\n",
            "}\n"
        ));
    }
    if !progress_widgets.is_empty() {
        prelude.push_str("let progressTick = null\nlet progressTickEvent = null\nlet progressSensors = null\n");
        prelude.push_str("function _createProgSensor(id) {\n");
        prelude.push_str("  if (!hmSensor || !hmSensor.createSensor || !hmSensor.id || !hmSensor.id[id]) return null\n");
        prelude.push_str("  try { return hmSensor.createSensor(hmSensor.id[id]) } catch (e) { return null }\n}\n");
        for (index, w) in &progress_widgets {
            prelude.push_str(&progress_value_fn(*index, w));
            prelude.push_str(&progress_apply_fn(*index, w));
        }
    }
    // Brak czujnika kroków nie może położyć całej tarczy — izolujemy jego
    // utworzenie i pozostawiamy tekst początkowy („0"), gdy API go nie udostępnia.
    prelude.push_str("function _getStep() {\n");
    prelude.push_str("  if (!stepSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.STEP) {\n");
    prelude.push_str("    try { stepSensor = hmSensor.createSensor(hmSensor.id.STEP) } catch (e) { stepSensor = null }\n");
    prelude.push_str("  }\n");
    prelude.push_str("  return stepSensor\n}\n");
    prelude.push_str("function _getDistance() {\n");
    prelude.push_str("  if (!distanceSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.DISTANCE) {\n");
    prelude.push_str("    try { distanceSensor = hmSensor.createSensor(hmSensor.id.DISTANCE) } catch (e) { distanceSensor = null }\n");
    prelude.push_str("  }\n");
    prelude.push_str("  return distanceSensor\n}\n");

    if !battery_indexes.is_empty() {
        prelude.push_str("function _getBattery() {\n");
        prelude.push_str("  if (!batterySensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.BATTERY) {\n");
        prelude.push_str("    try { batterySensor = hmSensor.createSensor(hmSensor.id.BATTERY) } catch (e) { batterySensor = null }\n");
        prelude.push_str("  }\n  return batterySensor\n}\n");
        prelude.push_str("const _batteryText = () => {\n");
        prelude.push_str("  const _battery = _getBattery()\n");
        prelude.push_str("  return _battery ? String(_battery.current) + '%' : '100%'\n}\n");
    }

    if needs_world_clock {
        prelude.push_str("let worldClockSensor = null\n");
        prelude.push_str("function _getWorldClock() {\n");
        prelude.push_str("  if (!worldClockSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.WORLD_CLOCK) {\n");
        prelude.push_str("    try {\n");
        prelude.push_str("      worldClockSensor = hmSensor.createSensor(hmSensor.id.WORLD_CLOCK)\n");
        prelude.push_str("      if (worldClockSensor.init) worldClockSensor.init()\n");
        prelude.push_str("    } catch (e) { worldClockSensor = null }\n");
        prelude.push_str("  }\n  return worldClockSensor\n}\n");
    }

    for sensor in &used_health {
        prelude.push_str(&format!("let {} = null\n", sensor.var));
        prelude.push_str(&format!("function {}() {{\n", sensor.getter));
        prelude.push_str(&format!(
            "  if (!{var} && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.{id}) {{\n",
            var = sensor.var,
            id = sensor.id
        ));
        prelude.push_str(&format!(
            "    try {{ {var} = hmSensor.createSensor(hmSensor.id.{id}) }} catch (e) {{ {var} = null }}\n",
            var = sensor.var,
            id = sensor.id
        ));
        prelude.push_str(&format!("  }}\n  return {}\n}}\n", sensor.var));
    }
    if !health_widgets.is_empty() {
        prelude.push_str("let healthTick = null\nlet healthTickEvent = null\nlet healthSensors = null\n");
        if used_health
            .iter()
            .any(|sensor| sensor.id == "SLEEP")
        {
            prelude.push_str("const _fmtDuration = (minutes) => {\n");
            prelude.push_str("  const m = Math.max(0, Math.round(minutes || 0))\n");
            prelude.push_str("  return Math.floor(m / 60) + ':' + String(m % 60).padStart(2, '0')\n");
            prelude.push_str("}\n");
        }
    }
    if health_widgets.iter().any(|(_, sensor, _)| {
        matches!(
            sensor.source,
            DataSource::HeartRateMin | DataSource::HeartRateMax | DataSource::HeartRateAvg
        )
    }) {
        prelude.push_str(concat!(
            "let _hrStats = { min: '--', max: '--', avg: '--' }\n",
            "let _hrStatsMinute = -1\n",
            "function _refreshHrStats() {\n",
            "  const heart = _getHealthHeart()\n",
            "  const values = (heart && heart.today && heart.today.length) ? heart.today : null\n",
            "  const t = _getTime()\n",
            "  const stamp = t ? (t.hour * 60 + t.minute) : -1\n",
            "  if (!values || stamp === _hrStatsMinute) return\n",
            "  _hrStatsMinute = stamp\n",
            "  let min = 0, max = 0, sum = 0, count = 0\n",
            "  for (let i = 0; i < values.length; i++) {\n",
            "    const v = values[i] | 0\n",
            "    if (v <= 0 || v > 250) continue\n",
            "    if (!count || v < min) min = v\n",
            "    if (v > max) max = v\n",
            "    sum += v\n    count++\n  }\n",
            "  _hrStats = count ? { min: String(min), max: String(max), avg: String(Math.round(sum / count)) } : { min: '--', max: '--', avg: '--' }\n",
            "}\n"
        ));
    }
    if !sleep_widgets.is_empty() {
        prelude.push_str(concat!(
            "let sleepSensor = null\nlet sleepTick = null\n",
            "let _sBasic = null\nlet _sStages = []\nlet _sModels = null\nlet _sTotal = 0\nlet _sInfoMinute = -1\n",
            "function _getSleep() {\n",
            "  if (!sleepSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.SLEEP) {\n",
            "    try { sleepSensor = hmSensor.createSensor(hmSensor.id.SLEEP) } catch (e) { sleepSensor = null }\n",
            "  }\n  return sleepSensor\n}\n",
            "function _fmtSleepHM(minutes) {\n",
            "  if (minutes === null || minutes === undefined || isNaN(minutes)) return '--:--'\n",
            "  let m = Math.round(minutes) % 1440\n  if (m < 0) m += 1440\n",
            "  return String(Math.floor(m / 60)).padStart(2, '0') + ':' + String(m % 60).padStart(2, '0')\n}\n",
            "function _sleepStageMinutes(model) {\n",
            "  if (!_sStages.length || model === undefined || model === null) return null\n",
            "  let total = 0\n",
            "  for (let i = 0; i < _sStages.length; i++) {\n",
            "    const stage = _sStages[i]\n",
            "    if (stage && stage.model === model) total += Math.max(0, (stage.stop | 0) - (stage.start | 0))\n",
            "  }\n  return total\n}\n",
            "function _sleepStageText(model, percent) {\n",
            "  const minutes = _sleepStageMinutes(model)\n",
            "  if (minutes === null) return percent ? '--%' : '-- min'\n",
            "  if (!percent) return String(minutes) + ' min'\n",
            "  if (!_sTotal || _sTotal <= 0) return '--%'\n",
            "  return String(Math.round(minutes * 100 / _sTotal)) + '%'\n}\n"
        ));
    }

    // Getery pól celu. `_getStep()` jest już wygenerowany powyżej i współdzielony
    // z licznikiem kroków; pozostałe czujniki tworzymy leniwie i w try/catch,
    // żeby brak STAND/FAT_BURRING na Active 2 nie położył całej tarczy.
    for sensor in &used_targets {
        if sensor.getter == "_getStep" {
            continue;
        }
        prelude.push_str(&format!("let {} = null\n", sensor.var));
        prelude.push_str(&format!("function {}() {{\n", sensor.getter));
        prelude.push_str(&format!(
            "  if (!{var} && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.{id}) {{\n",
            var = sensor.var,
            id = sensor.id
        ));
        prelude.push_str(&format!(
            "    try {{ {var} = hmSensor.createSensor(hmSensor.id.{id}) }} catch (e) {{ {var} = null }}\n",
            var = sensor.var,
            id = sensor.id
        ));
        prelude.push_str(&format!("  }}\n  return {}\n}}\n", sensor.var));
    }

    if let Some(custom) = &project.custom_code {
        prelude.push_str("\n// --- custom code (unowned block) ---\n");
        prelude.push_str(custom);
        prelude.push('\n');
    }

    let mut out = String::new();
    out.push_str("\ntry {\n  (() => {\n");
    out.push_str("    const __$$app$$__ = __$$hmAppManager$$__.currentApp\n");
    out.push_str("    const __$$module$$__ = __$$app$$__.current\n");
    out.push_str("    const h = new DeviceRuntimeCore.WidgetFactory(new DeviceRuntimeCore.HmDomApi(__$$app$$__, __$$module$$__))\n");
    out.push_str("    const {px} = __$$app$$__.__globals__\n");
    out.push_str("    function getApp() {\n");
    out.push_str("      return __$$app$$__.app\n");
    out.push_str("    }\n");
    out.push_str("    function getCurrentPage() {\n");
    out.push_str("      return __$$app$$__.current && __$$app$$__.current.module\n");
    out.push_str("    }\n");
    out.push_str(&prelude);
    out.push_str("    __$$module$$__.module = DeviceRuntimeCore.WatchFace({\n");
    out.push_str("      init_view() {\n");
    // Tarcza zaczyna od pełnego, nieprzezroczystego czarnego prostokąta —
    // nawet gdy ładowanie którejś czcionki/obrazu zawiedzie, ekran nie jest pusty.
    let screen = project.device.profile().design_width;
    out.push_str(&format!(
        "        hmUI.createWidget(hmUI.widget.FILL_RECT, {{ x: 0, y: 0, w: {screen}, h: {screen}, color: 0xFF000000, show_level: hmUI.show_level.ONLY_NORMAL }})\n"
    ));
    if needs_auto_mileage {
        out.push_str("      _initMileageUnit()\n");
    }

    for (index, w) in widgets.iter().enumerate() {
        out.push_str(&format!(
            "      refs[{index}] = {}\n",
            widget_call(w, placements)
        ));
    }

    if !step_indexes.is_empty() {
        // Step zmienia się niezależnie od tyknięcia zegara — nasłuchujemy CHANGE,
        // żeby licznik odświeżał się od razu, a nie raz na minutę.
        out.push_str("      stepTick = () => {\n");
        out.push_str("        const _step = _getStep()\n");
        out.push_str("        if (!_step) return\n");
        out.push_str("        const _steps = String(_step.current)\n");
        for index in &step_indexes {
            out.push_str(&format!(
                "        refs[{index}].setProperty(hmUI.prop.TEXT, _steps)\n"
            ));
        }
        out.push_str("      }\n");
        out.push_str("      const _stepSensor = _getStep()\n");
        out.push_str("      if (_stepSensor && _stepSensor.addEventListener && _stepSensor.event && _stepSensor.event.CHANGE) _stepSensor.addEventListener(_stepSensor.event.CHANGE, stepTick)\n");
        out.push_str("      stepTick()\n");
    }

    if !distance_units.is_empty() {
        // Dystans jest w metrach — przeliczamy go do km albo mil zgodnie
        // z systemem projektu / własnym wyborem widgetu.
        out.push_str("      distanceTick = () => {\n");
        out.push_str("        const _distance = _getDistance()\n");
        out.push_str("        if (!_distance) return\n");
        out.push_str("        const _meters = _distance.current\n");
        for (index, system) in &distance_units {
            out.push_str(&format!(
                "        refs[{index}].setProperty(hmUI.prop.TEXT, {})\n",
                distance_value_js(*system, "_meters")
            ));
        }
        out.push_str("      }\n");
        out.push_str("      const _distanceSensor = _getDistance()\n");
        out.push_str("      if (_distanceSensor && _distanceSensor.addEventListener && _distanceSensor.event && _distanceSensor.event.CHANGE) _distanceSensor.addEventListener(_distanceSensor.event.CHANGE, distanceTick)\n");
        out.push_str("      distanceTick()\n");
    }

    if !battery_indexes.is_empty() {
        // Bateria zmienia się rzadko, ale nasłuchujemy CHANGE, żeby nie czekać
        // na następną minutę. Brak sensora pozostawia tekst początkowy (100%).
        out.push_str("      batteryTick = () => {\n");
        out.push_str("        const _level = _batteryText()\n");
        for index in &battery_indexes {
            out.push_str(&format!(
                "        refs[{index}].setProperty(hmUI.prop.TEXT, _level)\n"
            ));
        }
        out.push_str("      }\n");
        out.push_str("      const _batterySensor = _getBattery()\n");
        out.push_str("      if (_batterySensor && _batterySensor.addEventListener && _batterySensor.event && _batterySensor.event.CHANGE) _batterySensor.addEventListener(_batterySensor.event.CHANGE, batteryTick)\n");
        out.push_str("      batteryTick()\n");
    }

    if !health_widgets.is_empty() {
        // Jeden wspólny tick dla wszystkich widgetów zdrowotnych — aktualizuje
        // je raz na zmianie czujnika oraz przy każdym tyknięciu zegara.
        out.push_str("      healthTick = (fromTick) => {\n");
        // Jeden getter na czujnik — kilka widgetów może czytać ten sam `HEART`.
        // Przy odświeżeniu z timera nie czytamy tętna: aktualizuje je wyłącznie
        // zdarzenie LAST, więc tarcza nie potrzebuje ciągłego pomiaru.
        let mut emitted_getters = std::collections::HashSet::new();
        for (_, sensor, _) in &health_widgets {
            if emitted_getters.insert(sensor.getter) {
                if sensor.id == "HEART" {
                    out.push_str(&format!(
                        "        const {} = fromTick ? null : {}()\n",
                        sensor.read_var, sensor.getter
                    ));
                } else {
                    out.push_str(&format!(
                        "        const {} = {}()\n",
                        sensor.read_var, sensor.getter
                    ));
                }
            }
        }
        if health_widgets
            .iter()
            .any(|(_, sensor, _)| sensor.source == DataSource::Spo2HourAvg)
        {
            out.push_str("        const _spo2Now = _getTime()\n");
            out.push_str("        const _spo2Hour = _spo2Now ? (_spo2Now.hour | 0) : 0\n");
        }
        if health_widgets.iter().any(|(_, sensor, _)| {
            matches!(
                sensor.source,
                DataSource::HeartRateMin | DataSource::HeartRateMax | DataSource::HeartRateAvg
            )
        }) {
            out.push_str("        _refreshHrStats()\n");
        }
        for (index, sensor, show_unit) in &health_widgets {
            let read = match (*show_unit, sensor.unit) {
                (true, Some(unit)) => {
                    format!("String({value}) + ' {unit}'", value = sensor.value)
                }
                _ => sensor.value.to_string(),
            };
            out.push_str(&format!(
                "        if ({var}) refs[{index}].setProperty(hmUI.prop.TEXT, {read})\n",
                var = sensor.read_var
            ));
        }
        out.push_str("      }\n");
        out.push_str("      healthTickEvent = () => healthTick()\n");
        out.push_str("      healthSensors = [\n");
        for sensor in &used_health {
            out.push_str(&format!(
                "        {{ get: {}, event: '{}' }},\n",
                sensor.getter, sensor.event
            ));
        }
        out.push_str("      ]\n");
        out.push_str("      healthSensors.forEach((entry) => {\n");
        out.push_str("        const sensor = entry.get()\n");
        out.push_str("        if (sensor && sensor.addEventListener && sensor.event && sensor.event[entry.event]) sensor.addEventListener(sensor.event[entry.event], healthTickEvent)\n");
        out.push_str("      })\n");
        out.push_str("      healthTick()\n");
    }

    if !sleep_widgets.is_empty() {
        out.push_str("      sleepTick = () => {\n");
        out.push_str("        const s = _getSleep()\n");
        out.push_str("        if (!s) return\n");
        // Dane snu zmieniają się co ~30 min — nie odświeżamy ich co sekundę,
        // gdy tarcza ma widget sekund. Jeden odczyt na minutę wystarczy.
        out.push_str("        const _st = _getTime()\n");
        out.push_str("        const _stamp = _st ? (_st.hour * 60 + _st.minute) : -1\n");
        out.push_str("        if (_stamp === _sInfoMinute) return\n");
        out.push_str("        _sInfoMinute = _stamp\n");
        out.push_str("        try { s.updateInfo() } catch (e) {}\n");
        out.push_str("        try { _sBasic = s.getBasicInfo() || null } catch (e) { _sBasic = null }\n");
        out.push_str("        try { _sModels = s.getSleepStageModel() || null } catch (e) { _sModels = null }\n");
        out.push_str("        try { const stages = s.getSleepStageData(); _sStages = (stages && stages.length) ? stages : [] } catch (e) { _sStages = [] }\n");
        out.push_str("        try { _sTotal = s.getTotalTime() || 0 } catch (e) { _sTotal = 0 }\n");
        for (index, source, mode) in &sleep_widgets {
            out.push_str(&format!(
                "        refs[{index}].setProperty(hmUI.prop.TEXT, {})\n",
                sleep_value(*source, *mode)
            ));
        }
        out.push_str("      }\n");
        out.push_str("      sleepTick()\n");
    }

    if !target_widgets.is_empty() {
        // Wspólny tick dla wszystkich pól celu — czyta `.target`, nie `.current`.
        out.push_str("      targetTick = () => {\n");
        for (index, sensor, show_unit) in &target_widgets {
            let read = match (*show_unit, sensor.unit) {
                (true, Some(unit)) => {
                    format!("String({var}.target) + ' {unit}'", var = sensor.read_var)
                }
                _ => format!("String({}.target)", sensor.read_var),
            };
            out.push_str(&format!(
                "        const {var} = {}()\n",
                sensor.getter,
                var = sensor.read_var
            ));
            out.push_str(&format!(
                "        if ({var}) refs[{index}].setProperty(hmUI.prop.TEXT, {read})\n",
                var = sensor.read_var
            ));
        }
        out.push_str("      }\n");
        out.push_str("      targetSensors = [\n");
        for sensor in &used_targets {
            out.push_str(&format!(
                "        {{ get: {}, event: 'CHANGE' }},\n",
                sensor.getter
            ));
        }
        out.push_str("      ]\n");
        out.push_str("      targetSensors.forEach((entry) => {\n");
        out.push_str("        const sensor = entry.get()\n");
        out.push_str("        if (sensor && sensor.addEventListener && sensor.event && sensor.event[entry.event]) sensor.addEventListener(sensor.event[entry.event], targetTick)\n");
        out.push_str("      })\n");
        out.push_str("      targetTick()\n");
    }

    if !weather_widgets.is_empty() || !weather_icons.is_empty() {
        out.push_str("      weatherTick = () => {\n");
        out.push_str("        const _weather = _getWeather()\n");
        out.push_str("        if (!_weather || !_weather.getForecastWeather) return\n");
        out.push_str("        try { _wData = _weather.getForecastWeather() || null } catch (e) { _wData = null }\n");
        out.push_str("        if (!_wData) return\n");
        out.push_str("        _wForecast = (_wData.forecastData && _wData.forecastData.data && _wData.forecastData.data.length) ? _wData.forecastData : { data: [] }\n");
        out.push_str("        _wTide = (_wData.tideData && _wData.tideData.data && _wData.tideData.data.length) ? _wData.tideData : { data: [] }\n");
        for (index, field, day) in &weather_widgets {
            // Dzień prognozy jest stały dla widgetu — basic Arrays have no Date API
            // na tarczy, więc indeks dnia wstawiamy już w codegenie.
            out.push_str(&format!("        const _wd_{index} = {day}\n"));
            out.push_str(&format!(
                "        refs[{index}].setProperty(hmUI.prop.TEXT, {})\n",
                field.value.replace("$D", &format!("_wd_{index}"))
            ));
        }
        for (index, _, day) in &weather_icons {
            out.push_str(&format!("        const _wd_{index} = {day}\n"));
            // Real firmware smears partially transparent edges when SRC is set
            // again and again (the clock tick calls weatherTick every second).
            // Cache the last index so the icon is replaced only on a real change.
            // Brakujący dzień prognozy nie zmienia ikony — zostaje poprzednia.
            out.push_str(&format!(
                "        if (_wForecast.data[_wd_{index}]) {{\n          const _wi_{index} = Math.max(0, Math.min(28, (_wForecast.data[_wd_{index}].index !== undefined ? _wForecast.data[_wd_{index}].index : 25) | 0))\n          if (_wi_{index} !== _lastWeatherIcon[{index}] && weatherIcons_{index}[_wi_{index}]) {{\n            refs[{index}].setProperty(hmUI.prop.SRC, weatherIcons_{index}[_wi_{index}])\n            _lastWeatherIcon[{index}] = _wi_{index}\n          }}\n        }}\n"
            ));
        }
        out.push_str("      }\n");
        out.push_str("      weatherTick()\n");
    }
    if !sleep_target_indexes.is_empty() {
        for index in &sleep_target_indexes {
            out.push_str(&format!(
                "      refs[{index}].setProperty(hmUI.prop.TEXT, _sleepTargetText())\n"
            ));
        }
    }
    if !progress_widgets.is_empty() {
        let calls: Vec<String> = progress_widgets
            .iter()
            .map(|(index, w)| {
                if progress_of(w).source == ProgressSource::Heart {
                    // Tylko zdarzenie LAST odświeża tętno — nie każdy tick timera.
                    format!("if (!fromTick) _progApply_{index}()")
                } else {
                    format!("_progApply_{index}()")
                }
            })
            .collect();
        out.push_str(&format!(
            "      progressTick = (fromTick) => {{ {} }}\n",
            calls.join("; ")
        ));
        // Jeden listener na unikalny czujnik — kilka łuków/pasków może czytać
        // ten sam HEART. Bez tego na zegarku poziom zostawał 0 z build() i nigdy
        // się nie odświeżał (symulator miał dane od razu, więc bug był niewidoczny).
        let mut seen = std::collections::HashSet::new();
        let mut entries = Vec::new();
        for (index, w) in &progress_widgets {
            let source = progress_of(w).source;
            let (Some(id), Some(event)) = (
                progress_sensor_id(source),
                progress_sensor_event(source),
            ) else {
                continue;
            };
            if seen.insert((id, event)) {
                entries.push(format!(
                    "        {{ get: () => _progSensor_{index}, event: '{event}' }},\n"
                ));
            }
        }
        if entries.is_empty() {
            out.push_str("      progressTick()\n");
        } else {
            out.push_str("      progressSensors = [\n");
            for entry in entries {
                out.push_str(&entry);
            }
            out.push_str("      ]\n");
            out.push_str("      progressTickEvent = () => progressTick()\n");
            out.push_str("      progressSensors.forEach((entry) => {\n");
            out.push_str("        const sensor = entry.get()\n");
            out.push_str("        if (sensor && sensor.addEventListener && sensor.event && sensor.event[entry.event]) sensor.addEventListener(sensor.event[entry.event], progressTickEvent)\n");
            out.push_str("      })\n");
            out.push_str("      progressTick()\n");
        }
    }

    out.push_str("      tick = () => {\n");
    out.push_str("        const t = _getTime()\n");
    if !step_indexes.is_empty() {
        out.push_str("        const _stepNow = _getStep()\n");
    }
    if !distance_units.is_empty() {
        out.push_str("        const _distanceNow = _getDistance()\n");
    }
    if !battery_indexes.is_empty() {
        out.push_str("        const _batteryNow = _getBattery()\n");
    }
    out.push_str("        const h = t.hour\n");
    out.push_str("        const m = t.minute\n");
    out.push_str("        const s = t.second\n");
    for (index, w) in widgets.iter().enumerate() {
        if let Some(line) = update_line(index, w, default_system) {
            out.push_str("        ");
            out.push_str(&line);
            out.push('\n');
        }
    }
    if !battery_indexes.is_empty() {
        out.push_str("        if (batteryTick) batteryTick()\n");
    }
    if !health_widgets.is_empty() {
        out.push_str("        if (healthTick) healthTick(true)\n");
    }
    if !progress_widgets.is_empty() {
        out.push_str("        if (progressTick) progressTick(true)\n");
    }
    if !sleep_widgets.is_empty() {
        out.push_str("        if (sleepTick) sleepTick()\n");
    }
    if !target_widgets.is_empty() {
        out.push_str("        if (targetTick) targetTick()\n");
    }
    if !weather_widgets.is_empty() || !weather_icons.is_empty() {
        out.push_str("        if (weatherTick) weatherTick()\n");
    }
    if !sleep_target_indexes.is_empty() {
        for index in &sleep_target_indexes {
            out.push_str(&format!(
                "        refs[{index}].setProperty(hmUI.prop.TEXT, _sleepTargetText())\n"
            ));
        }
    }
    out.push_str("      }\n");
    // WIDGET_DELEGATE przejmuje cykl pracy tarczy — tak jak w referencyjnej
    // paczce. W AOD zegarek sam budzi tarczę MINUTEEND, w trybie normalnym
    // używamy timera z API tarczy (zwykły setInterval nie jest gwarantowany).
    out.push_str("      tick()\n");
    out.push_str("      hmUI.createWidget(hmUI.widget.WIDGET_DELEGATE, {\n");
    out.push_str("        resume_call: function() {\n");
    if needs_seconds {
        out.push_str("          if (_screenIsAod()) {\n");
        out.push_str("            const _t = _getTime()\n");
        out.push_str("            if (_t && _t.addEventListener && _t.event && _t.event.MINUTEEND) _t.addEventListener(_t.event.MINUTEEND, tick)\n");
        out.push_str("          } else if (!tickTimer) {\n");
        out.push_str("            tickTimer = timer.createTimer(0, 1000, tick)\n");
        out.push_str("          }\n");
    } else {
        out.push_str("          const _t = _getTime()\n");
        out.push_str("          if (_t && _t.addEventListener && _t.event && _t.event.MINUTEEND) _t.addEventListener(_t.event.MINUTEEND, tick)\n");
    }
    out.push_str("          tick()\n");
    if !sleep_target_indexes.is_empty() {
        for index in &sleep_target_indexes {
            out.push_str(&format!(
                "          refs[{index}].setProperty(hmUI.prop.TEXT, _sleepTargetText())\n"
            ));
        }
    }
    out.push_str("        },\n");
    out.push_str("        pause_call: function() {\n");
    out.push_str("          if (tickTimer) { timer.stopTimer(tickTimer); tickTimer = null }\n");
    out.push_str("          const _t = timeSensor\n");
    out.push_str("          if (_t && _t.removeEventListener && _t.event && _t.event.MINUTEEND) _t.removeEventListener(_t.event.MINUTEEND, tick)\n");
    out.push_str("        }\n");
    out.push_str("      })\n");
    out.push_str("    },\n    onDestroy() {\n");
    out.push_str("      if (tickTimer !== null) { timer.stopTimer(tickTimer); tickTimer = null }\n");
    out.push_str("      if (timeSensor && tick && timeSensor.removeEventListener && timeSensor.event && timeSensor.event.MINUTEEND) timeSensor.removeEventListener(timeSensor.event.MINUTEEND, tick)\n");
    if !step_indexes.is_empty() {
        out.push_str("      if (stepSensor && stepTick && stepSensor.removeEventListener && stepSensor.event && stepSensor.event.CHANGE) stepSensor.removeEventListener(stepSensor.event.CHANGE, stepTick)\n");
        out.push_str("      stepSensor = null\n      stepTick = null\n");
    }
    if !distance_units.is_empty() {
        out.push_str("      if (distanceSensor && distanceTick && distanceSensor.removeEventListener && distanceSensor.event && distanceSensor.event.CHANGE) distanceSensor.removeEventListener(distanceSensor.event.CHANGE, distanceTick)\n");
        out.push_str("      distanceSensor = null\n      distanceTick = null\n");
    }
    if !battery_indexes.is_empty() {
        out.push_str("      if (batterySensor && batteryTick && batterySensor.removeEventListener && batterySensor.event && batterySensor.event.CHANGE) batterySensor.removeEventListener(batterySensor.event.CHANGE, batteryTick)\n");
        out.push_str("      batterySensor = null\n      batteryTick = null\n");
    }
    if needs_world_clock {
        out.push_str("      if (worldClockSensor && worldClockSensor.uninit) worldClockSensor.uninit()\n");
        out.push_str("      worldClockSensor = null\n");
    }
    if !health_widgets.is_empty() {
        out.push_str("      if (healthSensors && healthTickEvent) healthSensors.forEach((entry) => {\n");
        out.push_str("        const sensor = entry.get()\n");
        out.push_str("        if (sensor && sensor.removeEventListener && sensor.event && sensor.event[entry.event]) sensor.removeEventListener(sensor.event[entry.event], healthTickEvent)\n");
        out.push_str("      })\n");
        out.push_str("      healthSensors = null\n      healthTickEvent = null\n      healthTick = null\n");
    }
    if !progress_widgets.is_empty() {
        out.push_str("      if (progressSensors && progressTickEvent) progressSensors.forEach((entry) => {\n");
        out.push_str("        const sensor = entry.get()\n");
        out.push_str("        if (sensor && sensor.removeEventListener && sensor.event && sensor.event[entry.event]) sensor.removeEventListener(sensor.event[entry.event], progressTickEvent)\n");
        out.push_str("      })\n");
        out.push_str("      progressSensors = null\n      progressTickEvent = null\n      progressTick = null\n");
    }
    if !sleep_widgets.is_empty() {
        out.push_str("      sleepSensor = null\n      sleepTick = null\n");
    }
    if !target_widgets.is_empty() {
        out.push_str("      if (targetSensors && targetTick) targetSensors.forEach((entry) => {\n");
        out.push_str("        const sensor = entry.get()\n");
        out.push_str("        if (sensor && sensor.removeEventListener && sensor.event && sensor.event[entry.event]) sensor.removeEventListener(sensor.event[entry.event], targetTick)\n");
        out.push_str("      })\n");
        out.push_str("      targetSensors = null\n      targetTick = null\n");
    }
    if !weather_widgets.is_empty() || !weather_icons.is_empty() {
        out.push_str("      weatherSensor = null\n      weatherTick = null\n      _wData = null\n");
    }
    out.push_str("      timeSensor = null\n      tick = null\n    },\n");
    out.push_str("    onInit() {},\n");
    out.push_str("    build() { this.init_view() }\n");
    out.push_str("    });\n");
    out.push_str("  })()\n");
    out.push_str("} catch (e) {\n  console.log('Mini Program Error', e)\n");
    out.push_str("  if (e && e.stack) e.stack.split('\\n').forEach((line) => console.log('error stack', line))\n}\n");
    out
}

fn js_string(value: &str) -> String {
    serde_json::to_string(value).expect("string is serializable")
}

/// Eksport zawsze używa `ONLY_NORMAL`. Flaga `ONAL_AOD` na realnym firmware
/// domalowywała nieprzezroczystą nakładkę AOD — przy małej ramce dawała biały
/// kwadrat, przy ramce pełnego ekranu czarny ekran. AOD obsługuje
/// `WIDGET_DELEGATE` z `hmSetting.getScreenType()` (tak jak w działającej tarczy).
const JS_SHOW_LEVEL: &str = "hmUI.show_level.ONLY_NORMAL";

fn optional_field(out: &mut String, indent: &str, key: &str, value: &Option<String>, raw: bool) {
    if let Some(v) = value {
        let rendered = if raw {
            v.clone()
        } else {
            js_string(v)
        };
        out.push_str(&format!("{indent}{key}: {rendered},\n"));
    }
}

fn initial_text(w: &Widget) -> Option<String> {
    // Temperatura bieżąca jest wiązana przez firmware (`TEXT_FONT` + `type`) —
    // żaden statyczny tekst nie może trafić do wygenerowanego JS.
    if w.kind == WidgetKind::WeatherCurrentText {
        return None;
    }
    match (&w.data_source, &w.time_format) {
        (Some(DataSource::Time), Some(pattern)) => Some(
            time_format::format(pattern, 0, 0, 0).unwrap_or_else(|_| {
                w.text.clone().unwrap_or_else(|| "00:00:00".to_string())
            }),
        ),
        (Some(DataSource::Date), Some(pattern)) => Some(
            crate::date_format::format(pattern, 1, 0, 2026).unwrap_or_else(|_| {
                w.text.clone()
                    .unwrap_or_else(|| crate::date_format::DEFAULT_PATTERN.to_string())
            }),
        ),
        // Czujnik kroków jest dostępny dopiero na zegarku — podgląd i pierwsza
        // klatka tarczy pokazują 0 do czasu pierwszego odczytu sensora.
        (Some(DataSource::Step), _) => Some(w.text.clone().unwrap_or_else(|| "0".to_string())),
        (Some(DataSource::Distance), _) => Some(
            w.text.clone().unwrap_or_else(|| {
                w.measurement_system
                    .unwrap_or(crate::model::MeasurementSystem::Watch)
                    .initial_distance_text()
            }),
        ),
        _ if w.kind.is_health() => w
            .kind
            .health_initial_text(w.show_unit)
            .or_else(|| w.text.clone()),
        _ if w.kind.is_goal() => w
            .kind
            .goal_initial_text(w.show_unit)
            .or_else(|| w.text.clone()),
        _ => w.text.clone(),
    }
}

/// Wyrażenie JS formatujące dystans (metry) zgodnie z wybranym systemem.
/// `Watch` czyta jednostkę ustawioną na zegarku raz przy starcie tarczy —
/// dzięki temu jeden ZIP pokazuje km albo mile bez rozróżniania wersji.
fn distance_value_js(system: crate::model::MeasurementSystem, meters: &str) -> String {
    if system.is_watch() {
        format!(
            "String(({meters} / (autoMileage ? 1609.344 : 1000)).toFixed(2)) + ' ' + (autoMileage ? 'mi' : 'km')"
        )
    } else {
        format!(
            "String(({meters} / {}).toFixed(2)) + ' {}'",
            system.meters_per_unit(),
            system.suffix()
        )
    }
}

/// Pozycja i punkt obrotu pojedynczej wskazówki. Domyślnie jest to cała ramka
/// widgetu z pivotem w jej środku; `hands::prepare_hand_assets` może podać
/// mniejszy KWADRAT wyśrodkowany na tym samym pivocie (po usunięciu przezroczystych
/// marginesów). Kwadrat jest kluczowy: ZeppOS przycina obrócony obraz do `w/h`,
/// więc prostokąt dopasowany do bboxu grafiki obcinałby wskazówkę po obrocie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandPlacement {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub center_x: i32,
    pub center_y: i32,
}

/// Buduje jeden obracany widget `IMG`. ZeppOS nie skaluje zasobu w runtime —
/// wyświetla PNG w jego naturalnym rozmiarze i przycina go do `w/h` widgetu.
/// Dlatego `hands.rs` dopasowuje pliki PNG w projekcie do rozmiaru ramki
/// (`scale_project_hands` przy zapisie, `export_asset_bytes` przy eksporcie),
/// a środek obrotu to środek obrazu.
fn hand_widget_call(
    w: &Widget,
    src: &Option<String>,
    angle_init: &str,
    placements: &HashMap<String, HandPlacement>,
    rotating: bool,
) -> Option<String> {
    let src = src.as_ref()?;
    let placement = placements.get(src.as_str());
    let (x, y, width, height) = match placement {
        Some(p) => (p.x, p.y, p.w, p.h),
        None => (w.x, w.y, w.w, w.h),
    };
    let pad = "        ";
    let mut props = String::new();
    if rotating {
        // Tak jak w referencyjnej tarczy: ramka obracanej wskazówki obejmuje
        // cały ekran, obraz jest w niej pozycjonowany przez `pos_x/pos_y`,
        // a pivitem jest środek tarczy. Mała ramka + względny pivot dawała na
        // realnym firmware biały prostokąt zamiast wskazówek.
        let screen = crate::model::SCREEN_SIZE as i32;
        let middle = screen / 2;
        props.push_str(&format!("{pad}x: 0,\n{pad}y: 0,\n{pad}w: {screen},\n{pad}h: {screen},\n"));
        props.push_str(&format!("{pad}pos_x: {x},\n{pad}pos_y: {y},\n"));
        props.push_str(&format!("{pad}center_x: {middle},\n{pad}center_y: {middle},\n"));
    } else {
        props.push_str(&format!("{pad}x: {x},\n{pad}y: {y},\n{pad}w: {width},\n{pad}h: {height},\n"));
    }
    let flat = crate::export_paths::exported_asset_name(src);
    props.push_str(&format!("{pad}src: {},\n", js_string(&flat)));
    props.push_str(&format!("{pad}auto_scale: false,\n"));
    if w.alpha < 255 {
        props.push_str(&format!("{pad}alpha: {},\n", w.alpha));
    }
    if rotating {
        props.push_str(&format!("{pad}angle: {angle_init},\n"));
    }
    props.push_str(&format!(
        "{pad}show_level: {}\n",
        JS_SHOW_LEVEL
    ));
    Some(format!(
        "hmUI.createWidget(hmUI.widget.IMG, {{\n{}      }})",
        props
    ))
}

/// Zegar analogowy to grupa widgetów `IMG` zapisana w jednym `refs[i]`,
/// dzięki czemu kolejność warstw w edytorze i w tarczy pozostaje 1:1.
fn analog_widget_call(w: &Widget, placements: &HashMap<String, HandPlacement>) -> String {
    let hour = hand_widget_call(w, &w.hour_src, "0", placements, true)
        .unwrap_or_else(|| "null".to_string());
    let minute = hand_widget_call(w, &w.minute_src, "0", placements, true)
        .unwrap_or_else(|| "null".to_string());
    let second = hand_widget_call(w, &w.second_src, "0", placements, true)
        .unwrap_or_else(|| "null".to_string());
    let cap = hand_widget_call(w, &w.cap_src, "0", placements, false)
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\n\
         \x20\x20\x20\x20\x20\x20hour: {hour},\n\
         \x20\x20\x20\x20\x20\x20minute: {minute},\n\
         \x20\x20\x20\x20\x20\x20second: {second},\n\
         \x20\x20\x20\x20\x20\x20cap: {cap}\n\
         \x20\x20\x20\x20\x20}}"
    )
}

fn widget_call(w: &Widget, placements: &HashMap<String, HandPlacement>) -> String {
    if w.kind == WidgetKind::Shortcut {
        let target = w
            .shortcut_target
            .unwrap_or(crate::model::ShortcutTarget::Battery);
        let alpha = if w.alpha < 255 {
            format!("alpha: {}, ", w.alpha)
        } else {
            String::new()
        };
        // Pusty `src` to w pełni przezroczysta nakładka — wzorzec używany przez
        // działające tarcze w amazfitwatchfaces. Nie dodajemy żadnego zasobu do ZIP-u:
        // 1×1 przezroczysty PNG był odrzucany przy instalacji przez QR i w zeus.
        return format!(
            "hmUI.createWidget(hmUI.widget.IMG_CLICK, {{ {alpha}x: {}, y: {}, w: {}, h: {}, src: \"\", type: hmUI.data_type.{}, show_level: {} }})",
            w.x,
            w.y,
            w.w,
            w.h,
            target.js_constant(),
            JS_SHOW_LEVEL
        );
    }
    if w.kind == WidgetKind::AnalogTime {
        return analog_widget_call(w, placements);
    }
    if w.kind == WidgetKind::StatusIcon {
        // IMG_STATUS jest w pełni obsługiwany przez system — pokazuje ikonę tylko
        // wtedy, gdy dany stan jest aktywny. Nie potrzebuje sensora ani timera.
        let status = w.status_type.unwrap_or(StatusType::Alarm);
        let src = w.src.clone().unwrap_or_else(|| {
            crate::status_icons::js_status_path(
                w.status_icon_set
                    .as_deref()
                    .unwrap_or(crate::status_icons::DEFAULT_STATUS_PACK),
                status,
            )
        });
        let flat = crate::export_paths::exported_asset_name(&src);
        let pad = "      ";
        return format!(
            "hmUI.createWidget(hmUI.widget.IMG_STATUS, {{\n\
             {pad}x: {},\n\
             {pad}y: {},\n\
             {pad}w: {},\n\
             {pad}h: {},\n\
             {pad}src: {},\n\
             {pad}type: {},\n\
             {pad}alpha: {},\n\
             {pad}show_level: {}\n\
             {pad}}})",
            w.x,
            w.y,
            w.w,
            w.h,
            js_string(&flat),
            status.js_constant(),
            w.alpha,
            JS_SHOW_LEVEL
        );
    }
    if matches!(w.kind, WidgetKind::ArcProgress | WidgetKind::BarProgress) {
        return progress_widget_call(w);
    }
    let mut props = String::new();
    let pad = "      ";
    props.push_str(&format!("{pad}x: {},\n", w.x));
    props.push_str(&format!("{pad}y: {},\n", w.y));
    props.push_str(&format!("{pad}w: {},\n", w.w));
    props.push_str(&format!("{pad}h: {},\n", w.h));

    match w.kind {
        WidgetKind::Image | WidgetKind::Images | WidgetKind::WeatherIcon => {
            // Plik PNG jest fizycznie przygotowany do w × h (image_fit), więc
            // ZeppOS nie musi nic skalować w runtime. Nazwa jest spłaszczana
            // tak samo jak wpis w ZIP-ie (patrz `export_paths`).
            let flat_src = w
                .src
                .as_deref()
                .map(crate::export_paths::exported_asset_name);
            optional_field(&mut props, pad, "src", &flat_src, false);
        }
        WidgetKind::Text
        | WidgetKind::TimeText
        | WidgetKind::DateText
        | WidgetKind::WeekdayNumberText
        | WidgetKind::WeekNumberText
        | WidgetKind::DayOfYearText
        | WidgetKind::LunarDateText
        | WidgetKind::FestivalText
        | WidgetKind::WorldClockText
        | WidgetKind::StepText
        | WidgetKind::DistanceText
        | WidgetKind::HeartRateText
        | WidgetKind::HeartRateLastText
        | WidgetKind::HeartRateMinText
        | WidgetKind::HeartRateMaxText
        | WidgetKind::HeartRateAvgText
        | WidgetKind::StressText
        | WidgetKind::PaiText
        | WidgetKind::SkinTempText
        | WidgetKind::SleepText
        | WidgetKind::SleepScoreText
        | WidgetKind::SleepDeepText
        | WidgetKind::SleepLightText
        | WidgetKind::SleepRemText
        | WidgetKind::SleepWakeText
        | WidgetKind::SleepStartText
        | WidgetKind::SleepEndText
        | WidgetKind::CalorieText
        | WidgetKind::Spo2Text
        | WidgetKind::Spo2HourAvgText
        | WidgetKind::BatteryText
        | WidgetKind::StepTargetText
        | WidgetKind::CalorieTargetText
        | WidgetKind::StandTargetText
        | WidgetKind::FatBurningTargetText
        | WidgetKind::StandText
        | WidgetKind::FatBurningText
        | WidgetKind::PaiWeeklyText
        | WidgetKind::PaiPrepaiText
        | WidgetKind::SleepTargetText
        | WidgetKind::WeatherConditionText
        | WidgetKind::WeatherCityText
        | WidgetKind::WeatherCurrentText
        | WidgetKind::WeatherHighText
        | WidgetKind::WeatherLowText
        | WidgetKind::SunriseText
        | WidgetKind::SunsetText
        | WidgetKind::SunCurrentText => {
            if w.kind == WidgetKind::WeatherCurrentText {
                // `TEXT_FONT` nie przyjmuje własnej treści — firmware podstawia
                // temperaturę bieżącą i odświeża ją co minutę / przy odświeżeniu ekranu.
                props.push_str(&format!("{pad}type: hmUI.data_type.WEATHER_CURRENT,\n"));
                props.push_str(&format!(
                    "{pad}unit_type: {},\n",
                    if w.show_unit { 1 } else { 0 }
                ));
            } else {
                optional_field(&mut props, pad, "text", &initial_text(w), false);
            }
            if let Some(size) = w.text_size {
                props.push_str(&format!("{pad}text_size: {size},\n"));
            }
            // Pola obecne w 100% działającej tarczy referencyjnej.
            props.push_str(&format!("{pad}char_space: 0,\n"));
            props.push_str(&format!("{pad}line_space: 0,\n"));
            if let Some(font) = &w.font {
                let flat = crate::export_paths::exported_asset_name(&crate::fonts::js_font_path(font));
                optional_field(&mut props, pad, "font", &Some(flat), false);
            }
            optional_field(&mut props, pad, "color", &w.color, true);
            let align = w.align.unwrap_or(Align::Center);
            props.push_str(&format!("{pad}align_h: {},\n", align.as_js()));
            props.push_str(&format!("{pad}align_v: {},\n", w.v_align.as_js()));
            props.push_str(&format!("{pad}text_style: {},\n", w.text_style.as_js()));
        }
        WidgetKind::StatusIcon | WidgetKind::AnalogTime => {
            unreachable!("image widgets are handled above")
        }
        WidgetKind::ArcProgress | WidgetKind::BarProgress => {
            unreachable!("progress widgets are handled above")
        }
        WidgetKind::Shortcut => unreachable!("shortcut widgets are handled above"),
    }

    if w.alpha < 255 {
        props.push_str(&format!("{pad}alpha: {},\n", w.alpha));
    }
    props.push_str(&format!(
        "{pad}show_level: {}\n",
        JS_SHOW_LEVEL
    ));

    format!(
        "hmUI.createWidget(hmUI.widget.{}, {{\n{}    }})",
        match w.kind {
            WidgetKind::Image | WidgetKind::Images | WidgetKind::WeatherIcon => "IMG",
            // Undocumented, ale klasyczny widget hmUI (const 34) używany m.in.
            // przez edytor SashaCX75 dla Active 2. firmware wiąże wartość
            // z `hmUI.data_type.WEATHER_CURRENT` — bez sensora i nasłuchiwania JS.
            WidgetKind::WeatherCurrentText => "TEXT_FONT",
            WidgetKind::Text
            | WidgetKind::TimeText
            | WidgetKind::DateText
            | WidgetKind::WeekdayNumberText
            | WidgetKind::WeekNumberText
            | WidgetKind::DayOfYearText
            | WidgetKind::LunarDateText
            | WidgetKind::FestivalText
            | WidgetKind::WorldClockText
            | WidgetKind::StepText
            | WidgetKind::DistanceText
            | WidgetKind::HeartRateText
            | WidgetKind::HeartRateLastText
            | WidgetKind::HeartRateMinText
            | WidgetKind::HeartRateMaxText
            | WidgetKind::HeartRateAvgText
            | WidgetKind::StressText
            | WidgetKind::PaiText
            | WidgetKind::SkinTempText
            | WidgetKind::SleepText
            | WidgetKind::SleepScoreText
            | WidgetKind::SleepDeepText
            | WidgetKind::SleepLightText
            | WidgetKind::SleepRemText
            | WidgetKind::SleepWakeText
            | WidgetKind::SleepStartText
            | WidgetKind::SleepEndText
            | WidgetKind::CalorieText
            | WidgetKind::Spo2Text
            | WidgetKind::Spo2HourAvgText
            | WidgetKind::BatteryText
            | WidgetKind::StepTargetText
            | WidgetKind::CalorieTargetText
            | WidgetKind::StandTargetText
            | WidgetKind::FatBurningTargetText
            | WidgetKind::StandText
            | WidgetKind::FatBurningText
            | WidgetKind::PaiWeeklyText
            | WidgetKind::PaiPrepaiText
            | WidgetKind::SleepTargetText
            | WidgetKind::WeatherConditionText
            | WidgetKind::WeatherCityText
            | WidgetKind::WeatherHighText
            | WidgetKind::WeatherLowText
            | WidgetKind::SunriseText
            | WidgetKind::SunsetText
            | WidgetKind::SunCurrentText => "TEXT",
            WidgetKind::StatusIcon | WidgetKind::AnalogTime => {
                unreachable!("image widgets are handled above")
            }
            WidgetKind::ArcProgress | WidgetKind::BarProgress => {
                unreachable!("progress widgets are handled above")
            }
            WidgetKind::Shortcut => unreachable!("shortcut widgets are handled above"),
        },
        props
    )
}

fn update_line(
    index: usize,
    w: &Widget,
    default_system: crate::model::MeasurementSystem,
) -> Option<String> {
    if matches!(w.kind, WidgetKind::ArcProgress | WidgetKind::BarProgress) {
        return Some(format!("_progApply_{index}()"));
    }
    if w.kind == WidgetKind::AnalogTime {
        let mut lines = Vec::new();
        if w.hour_src.is_some() {
            lines.push(format!(
                "if (refs[{index}].hour) refs[{index}].hour.setProperty(hmUI.prop.MORE, {{ angle: (h % 12) * 30 + m * 0.5 }})"
            ));
        }
        if w.minute_src.is_some() {
            lines.push(format!(
                "if (refs[{index}].minute) refs[{index}].minute.setProperty(hmUI.prop.MORE, {{ angle: m * 6 + s * 0.1 }})"
            ));
        }
        if w.second_src.is_some() {
            lines.push(format!(
                "if (refs[{index}].second) refs[{index}].second.setProperty(hmUI.prop.MORE, {{ angle: s * 6 }})"
            ));
        }
        return Some(lines.join("\n        "));
    }
    match w.data_source {
        Some(DataSource::Time) => {
            let pattern = w.time_format.as_deref().unwrap_or("HH:MM");
            let expression = time_format::js_expression_localized(pattern, "h", "m", "s")
                .unwrap_or_else(|_| {
                    time_format::js_expression_localized("HH:MM", "h", "m", "s")
                        .expect("valid fallback")
                });
            Some(format!(
                "refs[{index}].setProperty(hmUI.prop.TEXT, {expression})"
            ))
        }
        Some(DataSource::Date) => {
            let date_expr = format!("d{index}");
            let pattern = w
                .time_format
                .as_deref()
                .unwrap_or(crate::date_format::DEFAULT_PATTERN);
            let expression = crate::date_format::js_expression_localized(pattern, &date_expr)
                .unwrap_or_else(|_| {
                    format!("L.months[{date_expr}.m0] + ' ' + ({date_expr}.d)")
                });
            Some(format!(
                "if (t) {{ const {date_expr} = _dateFromT(t); refs[{index}].setProperty(hmUI.prop.TEXT, {expression}) }}"
            ))
        }
        Some(DataSource::WeekDayNumber) => Some(format!(
            "if (t) refs[{index}].setProperty(hmUI.prop.TEXT, String(_weekdayNumber(t)))"
        )),
        Some(DataSource::WeekNumber) => Some(format!(
            "if (t) refs[{index}].setProperty(hmUI.prop.TEXT, String(_isoWeek(_dateFromT(t))))"
        )),
        Some(DataSource::DayOfYear) => Some(format!(
            "if (t) refs[{index}].setProperty(hmUI.prop.TEXT, String(_dayOfYear(_dateFromT(t))))"
        )),
        Some(DataSource::LunarDate) => Some(format!(
            "if (t) refs[{index}].setProperty(hmUI.prop.TEXT, String(t.lunar_month || '--') + '/' + String(t.lunar_day || '--'))"
        )),
        Some(DataSource::Festival) => Some(format!(
            "if (t) refs[{index}].setProperty(hmUI.prop.TEXT, t.solar_festival || t.lunar_festival || t.lunar_solar_term || '--')"
        )),
        Some(DataSource::WorldClock) => Some(format!(
            "const _wc = _getWorldClock(); if (_wc && _wc.getWorldClockCount) _wc.getWorldClockCount(); const _wcInfo = _wc ? (_wc.getWorldClockInfo ? _wc.getWorldClockInfo(0) : (_wc.getWorldClockCountInfo ? _wc.getWorldClockCountInfo(0) : null)) : null; refs[{index}].setProperty(hmUI.prop.TEXT, _wcInfo ? (_wcInfo.city + ' ' + p2(_wcInfo.hour) + ':' + p2(_wcInfo.minute)) : '--')"
        )),
        Some(DataSource::Step) => Some(format!(
            "if (_stepNow) refs[{index}].setProperty(hmUI.prop.TEXT, String(_stepNow.current))"
        )),
        Some(DataSource::Distance) => {
            let system = w.measurement_system.unwrap_or(default_system);
            Some(format!(
                "if (_distanceNow) refs[{index}].setProperty(hmUI.prop.TEXT, {})",
                distance_value_js(system, "_distanceNow.current")
            ))
        }
        Some(DataSource::Battery) => Some(format!(
            "if (_batteryNow) refs[{index}].setProperty(hmUI.prop.TEXT, String(_batteryNow.current) + '%')"
        )),
        // Widgety zdrowotne są aktualizowane przez `healthTick`.
        None | Some(_) => None,
    }
}
