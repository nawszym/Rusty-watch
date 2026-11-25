use std::collections::HashMap;

use serde_json::json;

use crate::locale::WatchfaceLocale;
use crate::model::{Align, DataSource, Project, StatusType, Widget, WidgetKind};
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
        event: "CURRENT",
        read_var: "_hHeart",
        value: "_hHeart.current ? String(_hHeart.current) : '--'",
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
];

fn health_sensor(source: Option<DataSource>) -> Option<&'static HealthSensor> {
    let source = source?;
    HEALTH_SENSORS.iter().find(|sensor| sensor.source == source)
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
    let health_widgets: Vec<(usize, &'static HealthSensor, bool)> = widgets
        .iter()
        .enumerate()
        .filter_map(|(index, w)| {
            health_sensor(w.data_source).map(|sensor| (index, sensor, w.show_unit))
        })
        .collect();
    let mut used_health: Vec<&'static HealthSensor> = Vec::new();
    for (_, sensor, _) in &health_widgets {
        if !used_health.iter().any(|s| s.id == sensor.id) {
            used_health.push(sensor);
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
    prelude.push_str("let stepSensor = null\nlet stepTick = null\n");
    prelude.push_str("let distanceSensor = null\nlet distanceTick = null\n");
    if !battery_indexes.is_empty() {
        prelude.push_str("let batterySensor = null\nlet batteryTick = null\n");
    }
    prelude.push_str("const refs = []\n\n");
    prelude.push_str("function _getTime() {\n");
    prelude.push_str("  if (!timeSensor) timeSensor = hmSensor.createSensor(hmSensor.id.TIME)\n");
    prelude.push_str("  return timeSensor\n}\n");
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
        prelude.push_str("let healthTick = null\nlet healthSensors = null\n");
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
    // Referencyjna tarcza zaczyna od pełnego, nieprzezroczystego prostokąta —
    // nawet gdy ładowanie którejś czcionki/obrazu zawiedzie, ekran nie jest czarny.
    let screen = project.device.profile().design_width;
    out.push_str(&format!(
        "        hmUI.createWidget(hmUI.widget.FILL_RECT, {{ x: 0, y: 0, w: {screen}, h: {screen}, color: 0xFF05070D, show_level: hmUI.show_level.ONLY_NORMAL }})\n"
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
        out.push_str("      healthTick = () => {\n");
        for (index, sensor, show_unit) in &health_widgets {
            out.push_str(&format!(
                "        const {} = {}()\n",
                sensor.read_var, sensor.getter
            ));
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
        out.push_str("        if (sensor && sensor.addEventListener && sensor.event && sensor.event[entry.event]) sensor.addEventListener(sensor.event[entry.event], healthTick)\n");
        out.push_str("      })\n");
        out.push_str("      healthTick()\n");
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
        out.push_str("        if (healthTick) healthTick()\n");
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
        out.push_str("      if (healthSensors && healthTick) healthSensors.forEach((entry) => {\n");
        out.push_str("        const sensor = entry.get()\n");
        out.push_str("        if (sensor && sensor.removeEventListener && sensor.event && sensor.event[entry.event]) sensor.removeEventListener(sensor.event[entry.event], healthTick)\n");
        out.push_str("      })\n");
        out.push_str("      healthSensors = null\n      healthTick = null\n");
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
             {pad}show_level: {}\n\
             {pad}}})",
            w.x,
            w.y,
            w.w,
            w.h,
            js_string(&flat),
            status.js_constant(),
            JS_SHOW_LEVEL
        );
    }
    let mut props = String::new();
    let pad = "      ";
    props.push_str(&format!("{pad}x: {},\n", w.x));
    props.push_str(&format!("{pad}y: {},\n", w.y));
    props.push_str(&format!("{pad}w: {},\n", w.w));
    props.push_str(&format!("{pad}h: {},\n", w.h));

    match w.kind {
        WidgetKind::Image | WidgetKind::Images => {
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
        | WidgetKind::StressText
        | WidgetKind::PaiText
        | WidgetKind::SkinTempText
        | WidgetKind::SleepText
        | WidgetKind::CalorieText
        | WidgetKind::Spo2Text
        | WidgetKind::BatteryText => {
            optional_field(&mut props, pad, "text", &initial_text(w), false);
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
    }

    props.push_str(&format!(
        "{pad}show_level: {}\n",
        JS_SHOW_LEVEL
    ));

    format!(
        "hmUI.createWidget(hmUI.widget.{}, {{\n{}    }})",
        match w.kind {
            WidgetKind::Image | WidgetKind::Images => "IMG",
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
            | WidgetKind::StressText
            | WidgetKind::PaiText
            | WidgetKind::SkinTempText
            | WidgetKind::SleepText
            | WidgetKind::CalorieText
            | WidgetKind::Spo2Text
            | WidgetKind::BatteryText => "TEXT",
            WidgetKind::StatusIcon | WidgetKind::AnalogTime => {
                unreachable!("image widgets are handled above")
            }
        },
        props
    )
}

fn update_line(
    index: usize,
    w: &Widget,
    default_system: crate::model::MeasurementSystem,
) -> Option<String> {
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
