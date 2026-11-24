use std::collections::HashMap;

use serde_json::json;

use crate::locale::WatchfaceLocale;
use crate::model::{Align, DataSource, Project, Widget, WidgetKind};
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

/// Generuje `app.json` zgodny z konfiguracją v3 dla profilu Active 2 Round.
pub fn generate_app_json(project: &Project) -> String {
    let profile = project.device.profile();
    let app = json!({
        "configVersion": "v3",
        "app": {
            "appId": project.metadata.app_id,
            "appName": project.metadata.name,
            "appType": "watchface",
            "version": { "code": 1, "name": project.metadata.version },
            "icon": "icon.png",
            "vender": project.metadata.author,
            "description": project.metadata.description
        },
        "permissions": [],
        "runtime": {
            // Tarcza korzysta z klasycznego API `hmUI` (tak jak oficjalny szablon
            // watchface i tarcze z zewnętrznych edytorów). Wymaga to wersji API 1.x —
            // przy 3.x runtime ładuje moduły `@zos/*` i nie udostępnia globalnego
            // `hmUI`, przez co createWidget jest undefined i ekran pozostaje czarny.
            "apiVersion": {
                "compatible": "1.0.0",
                "target": "1.0.1",
                "minVersion": "1.0.0"
            }
            // Nie ustawiamy `type`: domyślnie 0 = interpreter QuickJS dla plików .js.
            // `type: 2` oznacza bytecode (.bin) i zapobiega załadowaniu naszego JS.
        },
        "targets": {
            profile.target_key: {
                "module": {
                    "watchface": {
                        "path": "watchface/index",
                        "main": 1,
                        "editable": 1,
                        "lockscreen": 1
                    }
                },
                "platforms": [
                    { "name": "Amazfit Active 2 (Round)", "deviceSource": profile.device_source }
                ],
                "designWidth": profile.design_width
            }
        },
        "i18n": {
            "en-US": { "appName": project.metadata.name },
            "pl-PL": { "appName": project.metadata.name }
        },
        "defaultLanguage": "en-US"
    });
    serde_json::to_string_pretty(&app).expect("app json is serializable")
}

pub fn generate_app_js() -> &'static str {
    "App({\n  globalData: {},\n  onCreate() {},\n  onDestroy() {}\n})\n"
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

    let mut out = String::new();
    out.push_str("// Generated by Rusty Watch v0.1.0 — do not edit by hand.\n");
    out.push_str("// Watchface API ZeppOS: hmUI / hmSensor / timer.\n\n");
    // Język tarczy jest wykrywany w runtime — ten sam ZIP działa po polsku i angielsku.
    out.push_str(WatchfaceLocale::js_detection());
    out.push_str("const p2 = (value) => String(value).padStart(2, '0')\n");
    if needs_weekday_number {
        out.push_str(crate::date_format::JS_WEEKDAY_INDEX);
        out.push_str(&crate::date_format::js_weekday_number(
            project.first_day_of_week,
        ));
    }
    if needs_iso_week {
        out.push_str(crate::date_format::JS_ISO_WEEK);
    }
    if needs_day_of_year {
        out.push_str(crate::date_format::JS_DAY_OF_YEAR);
    }
    if needs_auto_mileage {
        out.push_str("let autoMileage = false\n");
        out.push_str("function _initMileageUnit() {\n");
        out.push_str("  try {\n");
        out.push_str("    if (typeof hmSetting !== 'undefined' && hmSetting.getMileageUnit) autoMileage = hmSetting.getMileageUnit() === 1\n");
        out.push_str("  } catch (e) { autoMileage = false }\n");
        out.push_str("}\n");
    }
    out.push_str("let timeSensor = null\nlet tick = null\nlet tickTimer = null\n");
    out.push_str("let stepSensor = null\nlet stepTick = null\n");
    out.push_str("let distanceSensor = null\nlet distanceTick = null\nconst refs = []\n\n");
    out.push_str("function _getTime() {\n");
    out.push_str("  if (!timeSensor) timeSensor = hmSensor.createSensor(hmSensor.id.TIME)\n");
    out.push_str("  return timeSensor\n}\n");
    // Brak czujnika kroków nie może położyć całej tarczy — izolujemy jego
    // utworzenie i pozostawiamy tekst początkowy („0"), gdy API go nie udostępnia.
    out.push_str("function _getStep() {\n");
    out.push_str("  if (!stepSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.STEP) {\n");
    out.push_str("    try { stepSensor = hmSensor.createSensor(hmSensor.id.STEP) } catch (e) { stepSensor = null }\n");
    out.push_str("  }\n");
    out.push_str("  return stepSensor\n}\n");
    out.push_str("function _getDistance() {\n");
    out.push_str("  if (!distanceSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.DISTANCE) {\n");
    out.push_str("    try { distanceSensor = hmSensor.createSensor(hmSensor.id.DISTANCE) } catch (e) { distanceSensor = null }\n");
    out.push_str("  }\n");
    out.push_str("  return distanceSensor\n}\n");

    if needs_world_clock {
        out.push_str("let worldClockSensor = null\n");
        out.push_str("function _getWorldClock() {\n");
        out.push_str("  if (!worldClockSensor && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.WORLD_CLOCK) {\n");
        out.push_str("    try {\n");
        out.push_str("      worldClockSensor = hmSensor.createSensor(hmSensor.id.WORLD_CLOCK)\n");
        out.push_str("      if (worldClockSensor.init) worldClockSensor.init()\n");
        out.push_str("    } catch (e) { worldClockSensor = null }\n");
        out.push_str("  }\n  return worldClockSensor\n}\n");
    }

    for sensor in &used_health {
        out.push_str(&format!("let {} = null\n", sensor.var));
        out.push_str(&format!("function {}() {{\n", sensor.getter));
        out.push_str(&format!(
            "  if (!{var} && hmSensor && hmSensor.createSensor && hmSensor.id && hmSensor.id.{id}) {{\n",
            var = sensor.var,
            id = sensor.id
        ));
        out.push_str(&format!(
            "    try {{ {var} = hmSensor.createSensor(hmSensor.id.{id}) }} catch (e) {{ {var} = null }}\n",
            var = sensor.var,
            id = sensor.id
        ));
        out.push_str(&format!("  }}\n  return {}\n}}\n", sensor.var));
    }
    if !health_widgets.is_empty() {
        out.push_str("let healthTick = null\nlet healthSensors = null\n");
        if used_health
            .iter()
            .any(|sensor| sensor.id == "SLEEP")
        {
            out.push_str("const _fmtDuration = (minutes) => {\n");
            out.push_str("  const m = Math.max(0, Math.round(minutes || 0))\n");
            out.push_str("  return Math.floor(m / 60) + ':' + String(m % 60).padStart(2, '0')\n");
            out.push_str("}\n");
        }
    }

    if let Some(custom) = &project.custom_code {
        out.push_str("\n// --- custom code (unowned block) ---\n");
        out.push_str(custom);
        out.push('\n');
    }

    out.push_str("\ntry {\n  WatchFace({\n    onInit() {},\n    build() {\n");

    // ZeppOS rasteryzuje glify własnej czcionki leniwie — bez wstępnego podgrzania
    // znaki pojawiające się dopiero po aktualizacji czasu bywają puste.
    out.push_str(&font_cache_js(&widgets));
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
    if !health_widgets.is_empty() {
        out.push_str("        if (healthTick) healthTick()\n");
    }
    out.push_str("      }\n");
    if needs_seconds {
        // Timer z API tarczy — zwykły setInterval nie jest gwarantowany w runtime zegarka.
        out.push_str("      tickTimer = timer.createTimer(0, 1000, tick)\n");
    } else {
        out.push_str("      const _t = _getTime()\n");
        out.push_str("      if (_t.addEventListener && _t.event && _t.event.MINUTEEND) _t.addEventListener(_t.event.MINUTEEND, tick)\n");
    }
    out.push_str("      tick()\n    },\n    onDestroy() {\n");
    out.push_str("      if (tickTimer !== null) { timer.stopTimer(tickTimer); tickTimer = null }\n");
    if !needs_seconds {
        out.push_str("      if (timeSensor && tick && timeSensor.removeEventListener && timeSensor.event && timeSensor.event.MINUTEEND) timeSensor.removeEventListener(timeSensor.event.MINUTEEND, tick)\n");
    }
    if !step_indexes.is_empty() {
        out.push_str("      if (stepSensor && stepTick && stepSensor.removeEventListener && stepSensor.event && stepSensor.event.CHANGE) stepSensor.removeEventListener(stepSensor.event.CHANGE, stepTick)\n");
        out.push_str("      stepSensor = null\n      stepTick = null\n");
    }
    if !distance_units.is_empty() {
        out.push_str("      if (distanceSensor && distanceTick && distanceSensor.removeEventListener && distanceSensor.event && distanceSensor.event.CHANGE) distanceSensor.removeEventListener(distanceSensor.event.CHANGE, distanceTick)\n");
        out.push_str("      distanceSensor = null\n      distanceTick = null\n");
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
    out.push_str("      timeSensor = null\n      tick = null\n    }\n  })\n");
    out.push_str("} catch (e) {\n  console.log('Mini Program Error', e)\n");
    out.push_str("  if (e && e.stack) e.stack.split('\\n').forEach((line) => console.log('error stack', line))\n}\n");
    out
}

fn js_string(value: &str) -> String {
    serde_json::to_string(value).expect("string is serializable")
}

/// Generuje niewidoczne widgety „podgrzewające” glify własnych czcionek.
/// `SashaCX75` opisuje ten trik w dokumentacji edytora tarcz: bez wstępnego
/// załadowania tylko znaki obecne przy starcie tarczy są potem rysowane poprawnie.
fn font_cache_js(widgets: &[Widget]) -> String {
    let mut pairs: Vec<(String, u32)> = Vec::new();
    for w in widgets {
        if let (Some(font), Some(size)) = (&w.font, w.text_size) {
            if !pairs.iter().any(|(f, s)| f == font && *s == size) {
                pairs.push((font.clone(), size));
            }
        }
    }
    if pairs.is_empty() {
        return String::new();
    }
    let mut chars: String = (32u8..=126).map(char::from).collect();
    chars.push_str("ĄĆĘŁŃÓŚŹŻąćęłńóśźż°€");
    let text = js_string(&chars);
    let mut out = String::from("      // Podgrzanie glifów własnych czcionek (poza ekranem).\n");
    for (_i, (font, size)) in pairs.iter().enumerate() {
        let path = js_string(&crate::fonts::js_font_path(font));
        out.push_str(&format!(
            "      hmUI.createWidget(hmUI.widget.TEXT, {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20x: -1000,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20y: -1000,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20w: 466,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20h: 100,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20text_size: {size},\n\
             \x20\x20\x20\x20\x20\x20\x20\x20font: {path},\n\
             \x20\x20\x20\x20\x20\x20\x20\x20text: {text},\n\
             \x20\x20\x20\x20\x20\x20\x20\x20text_style: hmUI.text_style.NONE,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20show_level: hmUI.show_level.ONLY_NORMAL\n\
             \x20\x20\x20\x20\x20\x20}})\n"
        ));
    }
    out
}

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
) -> Option<String> {
    let src = src.as_ref()?;
    let placement = placements.get(src.as_str());
    let (x, y, width, height, center_x, center_y) = match placement {
        Some(p) => (p.x, p.y, p.w, p.h, p.center_x, p.center_y),
        None => (w.x, w.y, w.w, w.h, w.w / 2, w.h / 2),
    };
    let pad = "        ";
    let mut props = String::new();
    props.push_str(&format!("{pad}x: {x},\n"));
    props.push_str(&format!("{pad}y: {y},\n"));
    props.push_str(&format!("{pad}w: {width},\n"));
    props.push_str(&format!("{pad}h: {height},\n"));
    props.push_str(&format!("{pad}src: {},\n", js_string(src)));
    props.push_str(&format!("{pad}center_x: {center_x},\n"));
    props.push_str(&format!("{pad}center_y: {center_y},\n"));
    props.push_str(&format!("{pad}angle: {angle_init},\n"));
    props.push_str(&format!(
        "{pad}show_level: {}\n",
        w.show_level.as_js()
    ));
    Some(format!(
        "hmUI.createWidget(hmUI.widget.IMG, {{\n{}      }})",
        props
    ))
}

/// Zegar analogowy to grupa widgetów `IMG` zapisana w jednym `refs[i]`,
/// dzięki czemu kolejność warstw w edytorze i w tarczy pozostaje 1:1.
fn analog_widget_call(w: &Widget, placements: &HashMap<String, HandPlacement>) -> String {
    let hour = hand_widget_call(w, &w.hour_src, "0", placements).unwrap_or_else(|| "null".to_string());
    let minute =
        hand_widget_call(w, &w.minute_src, "0", placements).unwrap_or_else(|| "null".to_string());
    let second =
        hand_widget_call(w, &w.second_src, "0", placements).unwrap_or_else(|| "null".to_string());
    let cap = hand_widget_call(w, &w.cap_src, "0", placements).unwrap_or_else(|| "null".to_string());
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
    let mut props = String::new();
    let pad = "      ";
    props.push_str(&format!("{pad}x: {},\n", w.x));
    props.push_str(&format!("{pad}y: {},\n", w.y));
    props.push_str(&format!("{pad}w: {},\n", w.w));
    props.push_str(&format!("{pad}h: {},\n", w.h));

    match w.kind {
        WidgetKind::Image | WidgetKind::Images => {
            // Plik PNG jest fizycznie przygotowany do w × h (image_fit), więc
            // ZeppOS nie musi nic skalować w runtime.
            optional_field(&mut props, pad, "src", &w.src, false);
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
        | WidgetKind::Spo2Text => {
            optional_field(&mut props, pad, "text", &initial_text(w), false);
            if let Some(size) = w.text_size {
                props.push_str(&format!("{pad}text_size: {size},\n"));
            }
            // Własna czcionka z `%APPDATA%\RustyWatch\Fonts` — ten sam plik trafia
            // do `assets/active2round/fonts/` w pakiecie. `None` = systemowa.
            if let Some(font) = &w.font {
                optional_field(
                    &mut props,
                    pad,
                    "font",
                    &Some(crate::fonts::js_font_path(font)),
                    false,
                );
            }
            optional_field(&mut props, pad, "color", &w.color, true);
            let align = w.align.unwrap_or(Align::Center);
            props.push_str(&format!("{pad}align_h: {},\n", align.as_js()));
            props.push_str(&format!("{pad}align_v: {},\n", w.v_align.as_js()));
            props.push_str(&format!("{pad}text_style: {},\n", w.text_style.as_js()));
        }
        WidgetKind::AnalogTime => unreachable!("analog widget is handled above"),
    }

    props.push_str(&format!(
        "{pad}show_level: {}\n",
        w.show_level.as_js()
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
            | WidgetKind::Spo2Text => "TEXT",
            WidgetKind::AnalogTime => unreachable!("analog widget is handled above"),
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
                    format!("L.months[{date_expr}.getMonth()] + ' ' + ({date_expr}.getDate())")
                });
            Some(format!(
                "const {date_expr} = new Date(); refs[{index}].setProperty(hmUI.prop.TEXT, {expression})"
            ))
        }
        Some(DataSource::WeekDayNumber) => Some(format!(
            "if (t) refs[{index}].setProperty(hmUI.prop.TEXT, String(_weekdayNumber(t)))"
        )),
        Some(DataSource::WeekNumber) => Some(format!(
            "refs[{index}].setProperty(hmUI.prop.TEXT, String(_isoWeek()))"
        )),
        Some(DataSource::DayOfYear) => Some(format!(
            "refs[{index}].setProperty(hmUI.prop.TEXT, String(_dayOfYear()))"
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
        Some(DataSource::Battery) => {
            Some(format!("// TODO: connect {} sensor for '{}'", "data", w.name))
        }
        // Widgety zdrowotne są aktualizowane przez `healthTick`.
        None | Some(_) => None,
    }
}
