pub mod asset_cache;
pub mod codegen;
pub mod date_format;
pub mod export_paths;
pub mod fonts;
pub mod hands;
pub mod image_fit;
pub mod images;
pub mod layers;
pub mod locale;
pub mod model;
pub mod package;
pub mod preview;
pub mod project;
pub mod qr;
pub mod recent;
pub mod status_icons;
pub mod time_format;
pub mod validate;
pub mod weather;
pub mod widgets;

pub use model::{DeviceId, Project, Widget, WidgetKind, SCREEN_SIZE, SCHEMA_VERSION};
pub use package::{export_zip, export_zip_bytes, inspect_zip, prepare_emulator_workspace, EmulatorWorkspace};
pub use project::{
    create_project, create_project_in, create_project_with, load_project, random_app_id,
    save_project, save_project_as, update_project_metadata, watchface_folder_name, NewProject,
};
pub use recent::RecentProject;
pub use validate::validate;

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use std::io::{Cursor, Read};
    use std::path::Path;
    use zip::ZipArchive;

    #[test]
    fn starter_project_is_valid() {
        let project = Project::starter("Test Face");
        let errors = validate(&project);
        assert!(errors.is_empty(), "errors: {errors:?}");
    }

    #[test]
    fn generated_json_has_active2_device_source() {
        let project = Project::starter("Test");
        let json = codegen::generate_app_json(&project);
        for source in [8913152, 8913153, 8913155, 8913159, 10092800, 10092801, 10092803, 10092807] {
            assert!(json.contains(&format!("\"deviceSource\": {source}")), "missing {source}");
        }
        assert!(json.contains("\"designWidth\": 466"));
        assert!(json.contains("\"lockscreen\": 0"));
        assert!(json.contains("\"configVersion\": \"v2\""));
        // Blok `targets` jest formatem roboczym zeus — nie może trafić do ZIP dla zegarka.
        assert!(!json.contains("\"targets\""));
        assert!(json.contains("\"icon\": \"Preview.png\""));
        assert!(json.contains("\"packageInfo\""));
        assert!(json.contains("\"defaultLanguage\": \"en-US\""));
        // `type: 2` oznacza bytecode (.bin) — zablokowałoby ładowanie naszych plików .js.
        assert!(!json.contains("\"type\":"));
        // Klasyczne API hmUI wymaga wersji 1.x — przy 3.x globalny hmUI jest undefined.
        assert!(json.contains("\"compatible\": \"1.0.0\""));
    }


    #[test]
    fn generated_js_background_is_fully_black() {
        let project = Project::starter("Test");
        let js = codegen::generate_index_js(&project);
        assert!(
            js.contains("hmUI.widget.FILL_RECT, { x: 0, y: 0, w: 466, h: 466, color: 0xFF000000"),
            "tło nie jest w pełni czarne: {js}"
        );
    }

    #[test]
    fn generated_js_uses_watchface_api() {
        let project = Project::starter("Test");
        let js = codegen::generate_index_js(&project);
        // Tarcza używa API hmUI, nie modułu @zos/ui przeznaczonego dla Mini Program.
        assert!(js.contains("hmUI.widget.FILL_RECT"));
        assert!(js.contains("hmUI.createWidget(hmUI.widget.TEXT"));
        assert!(!js.contains("bg.png"), "tło nie jest zasobem: {js}");
        assert!(js.contains("hmSensor.createSensor(hmSensor.id.TIME)"));
        assert!(!js.contains("@zos/ui"));
        assert!(!js.contains("@zos/sensor"));
        assert!(js.contains("const s = t.second"));
        assert!(js.contains("timer.createTimer(0, 1000, tick)"));
        assert!(js.contains("timer.stopTimer(tickTimer)"));
    }

    #[test]
    fn goal_widgets_are_in_catalog_and_export_target_readers() {
        let goals: Vec<_> = widgets::supported_widgets()
            .into_iter()
            .filter(|item| item.category == widgets::CATEGORY_GOALS)
            .map(|item| item.kind)
            .collect();
        assert_eq!(
            goals,
            vec![
                "steps-target",
                "calories-target",
                "stand-target",
                "fat-burning-target",
            ]
        );

        let mut project = Project::starter("Cele");
        for kind in &goals {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        assert!(validate(&project).is_empty());

        let js = codegen::generate_index_js(&project);
        for token in [
            "hmSensor.id.STAND",
            "hmSensor.id.FAT_BURRING",
            "function _getTargetStand()",
            "function _getTargetFatBurning()",
            "String(_tStep.target)",
            "String(_tCalorie.target) + ' kcal'",
            "String(_tStand.target) + ' h'",
            "String(_tFat.target) + ' min'",
            "targetSensors.forEach",
            "sensor.removeEventListener(sensor.event[entry.event], targetTick)",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}: {js}");
        }
        // Cele nie mogą wprowadzać API Mini Program ani obiektu Date.
        assert!(!js.contains("@zos"));
        assert!(!js.contains("new Date"));
        assert!(js.contains("show_level: hmUI.show_level.ONLY_NORMAL"));
    }

    #[test]
    fn goal_widgets_without_sensors_keep_placeholder_and_no_getters() {
        let mut project = Project::starter("Cele");
        let widget = widgets::add_widget(&mut project, "steps-target").unwrap();
        assert_eq!(widget.text.as_deref(), Some("10000"));
        let js = codegen::generate_index_js(&project);
        assert!(!js.contains("hmSensor.id.STAND"));
        assert!(!js.contains("hmSensor.id.FAT_BURRING"));
        assert!(js.contains("text: \"10000\""));
    }

    #[test]
    fn goal_show_unit_checkmark_changes_preview_and_export() {
        let mut project = Project::starter("Cele");
        let widget = widgets::add_widget(&mut project, "stand-target").unwrap();
        assert_eq!(widget.text.as_deref(), Some("12 h"));

        let settings = serde_json::json!({
            "id": widget.id,
            "x": widget.x,
            "y": widget.y,
            "w": widget.w,
            "h": widget.h,
            "textSize": widget.text_size.unwrap_or(40),
            "color": "0xFFFFFFFF",
            "align": "center",
            "showUnit": false,
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert!(!updated.show_unit);
        assert_eq!(updated.text.as_deref(), Some("12"));

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("String(_tStand.target)"));
        assert!(!js.contains("String(_tStand.target) + ' h'"));

        let series = preview::random_series(42, "en");
        let frame = preview::frame_data(0, &series).unwrap();
        let layer = layers::describe(&project)
            .into_iter()
            .find(|layer| layer.kind == "stand-target")
            .expect("warstwa celu");
        assert_eq!(preview::text_for_layer(&layer, &frame, &project, locale::WatchfaceLocale::English), "12");
    }

    #[test]
    fn additional_health_widgets_read_expected_sensor_fields() {
        let goals: Vec<_> = widgets::supported_widgets()
            .into_iter()
            .filter(|item| item.category == widgets::CATEGORY_HEALTH)
            .map(|item| item.kind)
            .collect();
        for kind in ["stand", "fat-burning", "pai-weekly"] {
            assert!(goals.contains(&kind), "missing health widget {kind}");
        }

        let mut project = Project::starter("Zdrowie+");
        for kind in ["pai", "stand", "fat-burning", "pai-weekly"] {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        assert!(validate(&project).is_empty());

        let js = codegen::generate_index_js(&project);
        for token in [
            "function _getHealthStand()",
            "function _getHealthFatBurning()",
            "function _getHealthPaiWeekly()",
            "hmSensor.id.STAND",
            "hmSensor.id.FAT_BURRING",
            "_hStand.current",
            "_hFatBurn.current",
            "_hPaiWeekly.totalpai || 0",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}: {js}");
        }
        // Oba pola PAI czytają ten sam czujnik, ale mają własne gettery i nasłuchy.
        assert!(js.contains("_getHealthPai()"));
        assert!(js.contains("_getHealthPaiWeekly()"));
    }

    #[test]
    fn alpha_is_emitted_only_for_non_opaque_widgets() {
        let mut project = Project::starter("Krycie");
        let id = project.widgets[0].id.clone();
        let js = codegen::generate_index_js(&project);
        assert!(!js.contains("alpha: 255"));

        let settings = serde_json::json!({
            "id": id,
            "x": 23,
            "y": 183,
            "w": 420,
            "h": 100,
            "textSize": 64,
            "color": "0xFFFFFFFF",
            "align": "center",
            "alpha": 128,
        });
        let widget = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(widget.alpha, 128);
        let layer = layers::describe(&project)
            .into_iter()
            .find(|layer| layer.id == id)
            .unwrap();
        assert_eq!(layer.alpha, 128);

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("alpha: 128"), "JS: {js}");
    }

    #[test]
    fn sleep_target_and_weather_widgets_export_expected_js() {
        let mut project = Project::starter("Pogoda");
        for kind in [
            "sleep-target",
            "weather-condition",
            "weather-city",
            "weather-high",
            "weather-low",
            "sunrise",
            "sunset",
            "weather-icon",
        ] {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        // FFI przypisuje paczkę zaraz po dodaniu — tutaj robimy to samo ręcznie.
        let icon = project.widgets.iter_mut().find(|w| w.kind == WidgetKind::WeatherIcon).unwrap();
        icon.weather_icon_set = Some(weather::DEFAULT_WEATHER_PACK.to_string());
        icon.src = Some(weather::js_icon_path(weather::DEFAULT_WEATHER_PACK, 3));
        project.assets.push(model::Asset {
            path: weather::js_icon_path(weather::DEFAULT_WEATHER_PACK, 3),
            role: model::AssetRole::WeatherIcon,
        });
        assert!(validate(&project).is_empty(), "{:?}", validate(&project));

        let js = codegen::generate_index_js(&project);
        for token in [
            "hmSetting.getSleepTarget()",
            "hmSensor.id.WEATHER",
            "getForecastWeather()",
            "_cityName(_wData && _wData.cityName)",
            "_wForecast.data[_wd_",
            "_wTide.data[_wd_",
            "L.weather[",
            "setProperty(hmUI.prop.SRC, weatherIcons_",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}: {js}");
        }
        assert!(!js.contains("@zos/ui"));
        assert!(!js.contains("new Date("));

        // Regression: the 29 icon paths must be plain JS string literals. The old
        // `format!("'{}'", js_string(..))` produced '"weather_x.png"' entries, so
        // setProperty(hmUI.prop.SRC, …) received a path with literal quotes and
        // the weather icon never rendered on the watch or in the simulator.
        let array_line = js
            .lines()
            .find(|line| line.starts_with("const weatherIcons_"))
            .expect("weather icon array is missing");
        let entries = array_line
            .split_once(" = [")
            .unwrap()
            .1
            .trim_end_matches(']')
            .split(", ")
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 29, "{array_line}");
        assert!(
            entries.iter().all(|entry| entry.starts_with('"')
                && entry.ends_with('"')
                && !entry.contains('\'')),
            "quoted icon path in {array_line}"
        );
        let third = format!(
            "\"{}\"",
            export_paths::exported_asset_name(&weather::js_icon_path(
                weather::DEFAULT_WEATHER_PACK,
                3
            ))
        );
        assert_eq!(entries[3], third, "{array_line}");
        assert!(js.contains(&format!("src: {third}")), "initial src differs");

        // Dzień 0: `curAirIconIndex` z czujnika WEATHER jest źródłem bieżącej
        // pogody — tak jak w widżecie pogody i innych tarczach ZeppOS.
        for token in [
            "weather.curAirIconIndex",
            "function _currentWeatherCode()",
            "function _wxNightCode(",
            "let _wxToday = _currentWeatherCode()",
            "L.weather[_wxToday]",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}: {js}");
        }
        assert!(!js.contains("IMG_LEVEL"), "hidden widget must be gone: {js}");
        assert!(!js.contains("_wxIcons"), "hidden icon array must be gone: {js}");
        // Regression: `| 0` zamieniało null/string w indeks 0 (w00 / "Cloudy").
        assert!(!js.contains("index | 0"), "unsafe weather coercion: {js}");
        assert!(!js.contains("index !== undefined ?"), "unsafe index guard: {js}");

        // Regression: a seconds widget calls weatherTick() every second. Setting
        // SRC unconditionally made the real firmware smudge the semi-transparent
        // white edges into a jagged white halo; only replace the icon on change.
        assert!(js.contains("let _lastWeatherIcon = {}"), "icon cache missing");
        assert!(
            js.lines().any(|line| line.trim() == "if (_wxToday !== _lastWeatherIcon[8] && weatherIcons_8[_wxToday]) {"),
            "day-0 icon must use the current weather code: {js}"
        );
        assert!(js.contains("_lastWeatherIcon[8] = _wxToday"), "cache not updated");
    }

    #[test]
    fn weather_day_zero_uses_current_code_other_days_use_forecast() {
        let mut project = Project::starter("Pogoda");
        for kind in ["weather-condition", "weather-icon", "weather-icon"] {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        for (i, icon) in project
            .widgets
            .iter_mut()
            .filter(|w| w.kind == WidgetKind::WeatherIcon)
            .enumerate()
        {
            icon.weather_icon_set = Some(weather::DEFAULT_WEATHER_PACK.to_string());
            icon.weather_day_offset = if i == 0 { 0 } else { 2 };
            icon.src = Some(weather::js_icon_path(weather::DEFAULT_WEATHER_PACK, 3));
        }
        let js = codegen::generate_index_js(&project);
        let cond = project
            .widgets
            .iter()
            .position(|w| w.kind == WidgetKind::WeatherConditionText)
            .unwrap();
        assert!(js.contains(&format!("refs[{cond}].setProperty(hmUI.prop.TEXT, L.weather[_wxToday] || L.weather[25])")));
        // Pierwsza ikona (dziś) używa kodu z firmware'u; druga (dzień 2) prognozy.
        assert!(js.contains("if (_wxToday !== _lastWeatherIcon["));
        assert!(js.contains("const _wi_"));
        assert!(js.contains("_weatherIndex(_wForecast.data[_wd_"));

        // Tylko dni 1…4: parser nadal musi istnieć, ale źródło firmware'u nie.
        let mut forecast_only = Project::starter("Pogoda");
        widgets::add_widget(&mut forecast_only, "weather-icon").unwrap();
        let icon = forecast_only.widgets.iter_mut().find(|w| w.kind == WidgetKind::WeatherIcon).unwrap();
        icon.weather_icon_set = Some(weather::DEFAULT_WEATHER_PACK.to_string());
        icon.weather_day_offset = 2;
        let js = codegen::generate_index_js(&forecast_only);
        assert!(js.contains("function _weatherIndex(value)"));
        assert!(!js.contains("function _currentWeatherCode()"));
        assert!(!js.contains("const _wxIcons"));
        assert!(js.contains("_weatherIndex(_wForecast.data[_wd_"));
    }

    #[test]
    fn weather_preview_frames_use_european_capitals_and_warsaw_last() {
        let caps: Vec<_> = preview::WEATHER_FRAMES
            .iter()
            .take(preview::SOURCE_FRAME_COUNT)
            .map(|w| w.city)
            .collect();
        assert_eq!(
            caps,
            vec!["Berlin", "Paris", "London", "Rome", "Madrid", "Prague", "Warsaw"]
        );
        assert_eq!(preview::WEATHER_FRAMES[6].city, "Warsaw");
        assert_eq!(preview::WEATHER_FRAMES[7].city, "Warsaw");
        assert_eq!(preview::WEATHER_FRAMES[6].index, preview::WEATHER_FRAMES[7].index);
    }

    #[test]
    fn city_names_follow_watchface_language() {
        use locale::WatchfaceLocale;
        assert_eq!(WatchfaceLocale::Polish.city_name("Warsaw"), "Warszawa");
        assert_eq!(WatchfaceLocale::English.city_name("Warszawa"), "Warsaw");
        assert_eq!(WatchfaceLocale::English.city_name("Paris"), "Paris");
        assert_eq!(WatchfaceLocale::Polish.city_name("Paris"), "Paryż");
        assert_eq!(WatchfaceLocale::Polish.weather_names()[3], "Słonecznie");
        assert_eq!(WatchfaceLocale::English.weather_names()[3], "Sunny");
        let js = WatchfaceLocale::js_detection();
        assert!(js.contains("weather: ["));
        assert!(js.contains("'warszawa': 'Warsaw'"));
        assert!(js.contains("'warsaw': 'Warszawa'"));
        assert!(js.contains("function _cityName"));
    }

    #[test]
    fn default_weather_preview_is_cloudy_warsaw_with_full_data() {
        use locale::WatchfaceLocale;
        let state = preview::WeatherPreviewState::from(preview::DEFAULT_PREVIEW_WEATHER);
        assert_eq!(state.city, "Warszawa");
        assert_eq!(state.index, 0);
        assert_eq!(state.high, 22);
        assert_eq!(state.low, 16);
        assert_eq!(preview::weather_text("weather-city", &state, WatchfaceLocale::Polish), "Warszawa");
        assert_eq!(preview::weather_text("weather-city", &state, WatchfaceLocale::English), "Warsaw");
        assert_eq!(preview::weather_text("weather-condition", &state, WatchfaceLocale::Polish), "Pochmurno");
        assert_eq!(preview::weather_text("weather-high", &state, WatchfaceLocale::Polish), "22°");
        assert_eq!(preview::weather_text("sunrise", &state, WatchfaceLocale::Polish), "05:13");

        // Custom Data nadpisuje tylko wysłane pola; reszta wraca do Warszawy.
        let overrides = serde_json::json!({ "index": 3, "high": -5 });
        let changed = state.clone().with_overrides(&overrides);
        assert_eq!(changed.index, 3);
        assert_eq!(changed.high, -5);
        assert_eq!(changed.low, 16);
        assert_eq!(preview::weather_text("weather-condition", &changed, WatchfaceLocale::English), "Sunny");

        // Ikona nie niesie tekstu — regression: fallback "--" rysowany w ramce.
        assert_eq!(preview::weather_text("weather-icon", &state, WatchfaceLocale::Polish), "");

        // Jedno pole HH:MM zastępuje dwie osobne wartości; błędny wpis jest ignorowany.
        let times = serde_json::json!({ "sunrise": "06:42", "sunset": "21:05" });
        let times = state.clone().with_overrides(&times);
        assert_eq!(preview::weather_text("sunrise", &times, WatchfaceLocale::Polish), "06:42");
        assert_eq!(preview::weather_text("sunset", &times, WatchfaceLocale::Polish), "21:05");
        let invalid = serde_json::json!({ "sunrise": "nope", "sunset": "25:99" });
        let invalid = state.with_overrides(&invalid);
        assert_eq!(preview::weather_text("sunrise", &invalid, WatchfaceLocale::Polish), "05:13");
        assert_eq!(preview::weather_text("sunset", &invalid, WatchfaceLocale::Polish), "20:40");
    }

    #[test]
    fn bootstrap_matches_reference_package() {
        let app_js = codegen::generate_app_js();
        for token in [
            "getApp()",
            "getCurrentPage()",
            "HmUtils.getPx(454)",
            "gettextFactory(languageTable",
            "onCreate(options)",
            "DeviceRuntimeCore.App",
        ] {
            assert!(app_js.contains(token), "app.js lacks {token}: {app_js}");
        }

        let project = Project::starter("Bootstrap");
        let js = codegen::generate_index_js(&project);
        // Cały kod helperów musi być wewnątrz try/IIFE — firmware nie wybacza
        // wyjątków na poziomie modułu, symulator je pomija.
        let try_at = js.find("try {").expect("try block");
        let module_at = js.find("DeviceRuntimeCore.WatchFace").expect("module");
        for token in ["const IS_POLISH", "const L =", "function _getTime()"] {
            let at = js.find(token).unwrap_or(usize::MAX);
            assert!(try_at < at && at < module_at, "{token} outside IIFE");
        }
        assert!(js.contains("function getApp()"));
        assert!(js.contains("function getCurrentPage()"));
    }

    #[test]
    fn generated_js_respects_time_format() {
        let mut project = Project::starter("Test");
        project.widgets[0].time_format = Some("h:mm A".to_string());
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("(h % 12) || 12"));
        assert!(js.contains("(h < 12) ? L.amUp : L.pmUp"));
        assert!(!js.contains("timer.createTimer(0, 1000, tick)"));
        assert!(js.contains("MINUTEEND"));
    }

    #[test]
    fn hidden_widget_stays_in_project_but_is_not_exported() {
        let mut project = Project::starter("Widoczność");
        let mut extra = project.widgets[0].clone();
        extra.id = "hidden".to_string();
        extra.name = "Hidden".to_string();
        extra.kind = model::WidgetKind::Text;
        extra.data_source = None;
        extra.time_format = None;
        extra.text = Some("UNIKATOWY_TEKST".to_string());
        extra.visible = false;
        project.widgets.push(extra);

        let js = codegen::generate_index_js(&project);
        assert!(!js.contains("UNIKATOWY_TEKST"));
        // Tylko widoczny widget czasu; ukryta warstwa nie trafia do generatora JS.
        assert_eq!(layers::ordered_widgets(&project).len(), 1);

        let described = layers::describe(&project);
        let hidden = described.iter().find(|layer| layer.id == "hidden").unwrap();
        assert!(!hidden.visible, "ukryta warstwa musi być widoczna na liście");
        let shown = described.iter().find(|layer| layer.id == "time").unwrap();
        assert!(shown.visible);

        layers::set_widget_visibility(&mut project, "hidden", true).unwrap();
        assert!(codegen::generate_index_js(&project).contains("UNIKATOWY_TEKST"));
        assert_eq!(layers::ordered_widgets(&project).len(), 2);
    }

    #[test]
    fn widget_without_visible_field_defaults_to_visible() {
        let project = Project::starter("Test");
        let mut value = serde_json::to_value(&project.widgets[0]).unwrap();
        value.as_object_mut().unwrap().remove("visible");
        let widget: model::Widget = serde_json::from_value(value).unwrap();
        assert!(widget.visible);
    }

    #[test]
    fn generated_js_detects_polish_language() {
        let mut project = Project::starter("Język");
        project.widgets[0].time_format = Some("hh A".to_string());
        let mut date = project.widgets[0].clone();
        date.id = "date".to_string();
        date.data_source = Some(model::DataSource::Date);
        project.widgets.push(date);
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("DeviceRuntimeCore.HmUtils.getLanguage()"));
        assert!(js.contains("hmSetting.getLanguage()"));
        assert!(js.contains("const IS_POLISH = _watchLanguage() === 9"));
        assert!(js.find("try {").unwrap() < js.find("const IS_POLISH").unwrap());
        assert!(js.contains("'Paź'"));
        assert!(js.contains("amUp: 'PRZEDP.'"));
        assert!(js.contains("? L.amUp : L.pmUp"));
        assert!(js.contains("L.months[d"));
    }

    #[test]
    fn generated_js_hides_disabled_optional_hands() {
        let mut project = Project::starter("Analog");
        project.widgets[0].kind = model::WidgetKind::AnalogTime;
        project.widgets[0].data_source = None;
        project.widgets[0].time_format = None;
        project.widgets[0].hour_src = Some("hands/Classic/hour.png".to_string());
        project.widgets[0].minute_src = Some("hands/Classic/minute.png".to_string());
        project.widgets[0].second_src = Some("hands/Classic/second.png".to_string());
        project.widgets[0].cap_src = Some("hands/Classic/cap.png".to_string());

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("second: "));
        assert!(js.contains("cap: "));
        assert!(js.contains("timer.createTimer(0, 1000, tick)"));

        project.widgets[0].hour_src = None;
        project.widgets[0].minute_src = None;
        project.widgets[0].second_src = None;
        project.widgets[0].cap_src = None;
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("hour: null"));
        assert!(js.contains("minute: null"));
        assert!(js.contains("second: null"));
        assert!(js.contains("cap: null"));
        assert!(!js.contains("(h % 12) * 30 + m * 0.5"));
        assert!(!js.contains("m * 6 + s * 0.1"));
        assert!(!js.contains("timer.createTimer(0, 1000, tick)"));
        assert!(js.contains("MINUTEEND"));
    }

    #[test]
    fn project_roundtrip_preserves_widgets() {
        let project = Project::starter("Roundtrip");
        let json = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.widgets.len(), 1);
        assert_eq!(loaded.widgets[0].kind, WidgetKind::TimeText);
        assert_eq!(loaded.widgets[0].time_format.as_deref(), Some("HH:MM:SS"));
    }

    #[test]
    fn export_bytes_contains_expected_entries() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        create_project(&dir, "Zip Test").unwrap();
        let file = project::project_file(&dir);
        let bytes = export_zip_bytes(&file).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert!(archive.by_name("app.json").is_ok());
        assert!(archive.by_name("app.js").is_ok());
        assert!(archive.by_name("watchface/index.js").is_ok());
        assert!(archive.by_name("assets/Preview.png").is_ok());
        // Tło jest stałym FILL_RECT — w paczce nie ma żadnego zasobu tła.
        assert!(archive.by_name("assets/bg.png").is_err());
        // Struktura płaska — bez katalogu targetu i bez pustych katalogów-duplikatów.
        assert!(archive.by_name("assets/active2round/").is_err());
        let mut icon = archive.by_name("assets/Preview.png").unwrap();
        let mut png = Vec::new();
        icon.read_to_end(&mut png).unwrap();
        let image = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (314, 314));
        // Ikona to oprawiony podgląd, nie stare logo: przezroczyste rogi i brąz.
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        let brown = image.get_pixel(157, 27).0;
        assert!(brown[0] > brown[2], "brak pierścienia logo: {brown:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_directory_entries_have_reference_metadata() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-zipmeta-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        create_project(&dir, "Zip Meta").unwrap();
        let file = project::project_file(&dir);
        let bytes = export_zip_bytes(&file).unwrap();

        let u16at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let u32at = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let eocd = (22..=bytes.len())
            .rev()
            .find(|&i| u32at(i - 22) == 0x0605_4b50)
            .expect("EOCD");
        let eocd = eocd - 22;
        let count = u16at(eocd + 10) as usize;
        let mut at = u32at(eocd + 16) as usize;
        let mut directories = 0;
        for _ in 0..count {
            assert_eq!(u32at(at), 0x0201_4b50);
            // DOS (0), version 20 — tak samo jak w paczce Pythona.
            assert_eq!(u16at(at + 4), 20, "version made by");
            let name_len = u16at(at + 28) as usize;
            let name = &bytes[at + 46..at + 46 + name_len];
            if name.last() == Some(&b'/') {
                directories += 1;
                assert_eq!(u32at(at + 38), 0x41ff_0010, "directory attributes");
            }
            at += 46 + name_len
                + u16at(at + 30) as usize
                + u16at(at + 32) as usize;
        }
        assert_eq!(directories, 3);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn exported_asset_paths_match_js_and_are_flat_and_masked() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-flat-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        create_project(&dir, "Flat Test").unwrap();
        let file = project::project_file(&dir);
        let bytes = export_zip_bytes(&file).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();

        // Zasoby leżą bezpośrednio w `assets/`; jedynym dozwolonym podfolderem
        // jest `fonts/`, którego wymaga firmware (tak jak w referencyjnej tarczy).
        let names: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_string())
            .collect();
        let nested: Vec<&String> = names
            .iter()
            .filter(|name| {
                name.starts_with("assets/")
                    && name.matches('/').count() > 1
                    && !name.starts_with("assets/fonts/")
            })
            .collect();
        assert!(nested.is_empty(), "nested assets: {nested:?}");

        // Każda ścieżka z JS musi mieć dokładnie odpowiadający wpis w ZIP-ie.
        let mut js = String::new();
        archive
            .by_name("watchface/index.js")
            .unwrap()
            .read_to_string(&mut js)
            .unwrap();
        for marker in ["src: \"", "font: \""] {
            let mut rest = js.as_str();
            while let Some(at) = rest.find(marker) {
                let value = &rest[at + marker.len()..];
                let end = value.find('"').expect("unterminated asset path");
                let path = format!("assets/{}", &value[..end]);
                assert!(names.contains(&path), "{path} missing in ZIP");
                rest = &value[end + 1..];
            }
        }

        // Bootstrap zgodny z referencją (realny firmware go wymaga).
        let mut app_js = String::new();
        archive
            .by_name("app.js")
            .unwrap()
            .read_to_string(&mut app_js)
            .unwrap();
        assert!(app_js.contains("DeviceRuntimeCore.App"));
        assert!(app_js.contains("__$$app$$__.__globals__"));
        assert!(js.contains("__$$module$$__.module = DeviceRuntimeCore.WatchFace"));
        assert!(js.contains("hmUI.widget.FILL_RECT"));
        assert!(js.contains("WIDGET_DELEGATE"));
        assert!(js.contains("resume_call") && js.contains("pause_call"));
        // Flaga AOD w show_level dawała na realnym zegarku czarną nakładkę.
        assert!(!js.contains("ONAL_AOD"), "js: {js}");
        assert!(!js.contains("x: -1000"));
        // Firmware nie ma obiektu `Date` — zakaz dowolnego jego użycia w JS.
        for banned in [
            "new Date",
            "Date.UTC",
            ".getDate(",
            ".getMonth(",
            ".getDay(",
            ".getFullYear(",
            "getUTCDay",
            "setUTCDate",
        ] {
            assert!(!js.contains(banned), "banned {banned} in js: {js}");
        }

        // Ikona 314x314 jest wycinana do okręgu: rogi przezroczyste, środek nie.
        let mut png = Vec::new();
        archive
            .by_name("assets/Preview.png")
            .unwrap()
            .read_to_end(&mut png)
            .unwrap();
        let image = image::load_from_memory(&png).unwrap();
        assert_eq!(image.dimensions(), (314, 314));
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        assert!(image.get_pixel(157, 157).0[3] > 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn new_project_metadata_is_used_on_the_watch() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-meta-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let input = NewProject {
            name: "My Face".to_string(),
            author: "Jane Doe".to_string(),
            app_id: 12_345_678,
        };
        create_project_with(&dir, &input).unwrap();
        let file = project::project_file(&dir);
        let project = load_project(&file).unwrap();
        assert_eq!(project.metadata.name, "My Face");
        assert_eq!(project.metadata.author, "Jane Doe");
        assert_eq!(project.metadata.app_id, 12_345_678);

        let json = codegen::generate_app_json(&project);
        assert!(json.contains("\"appId\": 12345678"));
        assert!(json.contains("\"appName\": \"My Face\""));
        assert!(json.contains("\"vender\": \"zepp\""));
        assert!(!json.contains("pl-PL"));
        assert!(json.contains("\"description\": \"\""));

        let bytes = export_zip_bytes(&file).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut app_json = String::new();
        archive
            .by_name("app.json")
            .unwrap()
            .read_to_string(&mut app_json)
            .unwrap();
        assert!(app_json.contains("12345678"));
        assert!(app_json.contains("My Face"));
        assert!(app_json.contains("\"vender\": \"zepp\""));
        assert!(!app_json.contains("Jane Doe"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn random_app_ids_are_valid() {
        for _ in 0..50 {
            let id = random_app_id();
            assert!((10_000_000..100_000_000).contains(&id));
        }
    }

    #[test]
    fn new_project_rejects_invalid_input() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-invalid-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(create_project_with(
            &dir,
            &NewProject {
                name: "".to_string(),
                author: "Jane".to_string(),
                app_id: 1,
            }
        )
        .is_err());
        assert!(create_project_with(
            &dir,
            &NewProject {
                name: "Face".to_string(),
                author: "".to_string(),
                app_id: 1,
            }
        )
        .is_err());
        assert!(create_project_with(
            &dir,
            &NewProject {
                name: "Face".to_string(),
                author: "Jane".to_string(),
                app_id: 0,
            }
        )
        .is_err());
        assert!(!dir.exists());
    }

    #[test]
    fn create_project_in_makes_watchface_folder() {
        let root = std::env::temp_dir().join(format!(
            "rusty-watch-folder-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let input = NewProject {
            name: "My: Face? /".to_string(),
            author: "Jane".to_string(),
            app_id: 22_222_222,
        };
        assert_eq!(watchface_folder_name(&input.name), "My_ Face_ _");

        let (dir, project) = create_project_in(&root, &input).unwrap();
        assert_eq!(dir.file_name().unwrap(), "My_ Face_ _");
        assert!(dir.join("project.json").exists());
        assert!(dir.join("assets/icon.png").exists());
        assert_eq!(project.metadata.name, "My: Face? /");
        assert!(create_project_in(&root, &input).is_err());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn save_project_as_copies_project_and_assets() {
        let root = std::env::temp_dir().join(format!(
            "rusty-watch-saveas-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let source_dir = root.join("original");
        create_project(&source_dir, "Original").unwrap();
        let source_file = project::project_file(&source_dir);

        let input = NewProject {
            name: "Copy".to_string(),
            author: "Jane Doe".to_string(),
            app_id: 33_333_333,
        };
        let (file, project) = save_project_as(&source_file, &root, &input).unwrap();
        assert_eq!(file, root.join("Copy/project.json"));
        assert_eq!(project.metadata.author, "Jane Doe");
        assert_eq!(project.metadata.app_id, 33_333_333);
        assert!(root.join("Copy/assets/icon.png").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn update_metadata_keeps_name_and_changes_author_and_version() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-metadata-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        create_project(&dir, "Stable Name").unwrap();
        let file = project::project_file(&dir);

        let project = update_project_metadata(&file, "  Jane Doe  ", " 1.2.3 ").unwrap();
        assert_eq!(project.metadata.name, "Stable Name");
        assert_eq!(project.metadata.author, "Jane Doe");
        assert_eq!(project.metadata.version, "1.2.3");

        let reloaded = load_project(&file).unwrap();
        assert_eq!(reloaded.metadata.name, "Stable Name");
        assert_eq!(reloaded.metadata.author, "Jane Doe");
        assert_eq!(reloaded.metadata.version, "1.2.3");
        assert!(codegen::generate_app_json(&reloaded).contains("\"name\": \"1.2.3\""));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn update_metadata_rejects_invalid_author_or_version() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-metadata-invalid-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        create_project(&dir, "Face").unwrap();
        let file = project::project_file(&dir);

        assert!(update_project_metadata(&file, "", "1.0.0").is_err());
        assert!(update_project_metadata(&file, "Jane", "").is_err());
        assert!(update_project_metadata(&file, "Jane", "1.0 beta").is_err());
        assert!(update_project_metadata(&file, "Jane", &"x".repeat(33)).is_err());
        assert_eq!(load_project(&file).unwrap().metadata.version, "0.1.0");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_layer_order_contains_only_widgets() {
        let project = Project::starter("Layers");
        assert_eq!(layers::order(&project), vec!["time"]);
        let layers = layers::describe(&project);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].kind, "time");
    }

    #[test]
    fn moving_a_widget_updates_z_and_generated_js_order() {
        let mut project = Project::starter("Layers");
        widgets::add_widget(&mut project, "time").unwrap();
        let js_before = codegen::generate_index_js(&project);
        assert_eq!(js_before.matches("hmUI.createWidget(hmUI.widget.TEXT").count(), 2);

        let order = layers::move_layer(&mut project, "time-2", -1).unwrap();
        assert_eq!(order, vec!["time-2", "time"]);
        assert_eq!(
            project.widgets.iter().find(|w| w.id == "time-2").unwrap().z,
            0
        );
        assert_eq!(project.layer_order, vec!["time-2", "time"]);

        let drawn: Vec<String> = layers::ordered_widgets(&project)
            .into_iter()
            .map(|w| w.id)
            .collect();
        assert_eq!(drawn, vec!["time-2", "time"]);
    }

    #[test]
    fn reorder_is_persisted_in_project_json() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-layers-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        create_project(&dir, "Layer Face").unwrap();
        let file = project::project_file(&dir);

        let mut project = load_project(&file).unwrap();
        widgets::add_widget(&mut project, "time").unwrap();
        layers::move_layer(&mut project, "time-2", -1).unwrap();
        save_project(&file, &project).unwrap();

        let reloaded = load_project(&file).unwrap();
        assert_eq!(
            layers::order(&reloaded),
            vec!["time-2", "time"]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bottom_widget_cannot_move_lower() {
        let mut project = Project::starter("Locked");
        assert!(layers::move_layer(&mut project, "time", -1).is_err());
        assert_eq!(layers::order(&project), vec!["time"]);
    }

    #[test]
    fn add_and_remove_widget_updates_layers() {
        let mut project = Project::starter("Widgets");
        let second = widgets::add_widget(&mut project, "time").unwrap();
        let third = widgets::add_widget(&mut project, "time").unwrap();
        assert_eq!(second.id, "time-2");
        assert_eq!(third.id, "time-3");
        assert_eq!(
            layers::order(&project),
            vec!["time", "time-2", "time-3"]
        );

        layers::remove_widget(&mut project, "time-2").unwrap();
        assert_eq!(layers::order(&project), vec!["time", "time-3"]);
        layers::remove_widget(&mut project, "time-3").unwrap();
        layers::remove_widget(&mut project, "time").unwrap_err(); // ostatni widget chroniony
        assert_eq!(layers::order(&project), vec!["time"]);
    }

    #[test]
    fn add_date_widget_creates_localized_date_text() {
        let mut project = Project::starter("Date Face");
        let date = widgets::add_widget(&mut project, "date").unwrap();
        assert_eq!(date.id, "date");
        assert_eq!(date.kind, WidgetKind::DateText);
        assert_eq!(date.data_source, Some(model::DataSource::Date));
        assert_eq!(
            date.time_format.as_deref(),
            Some(crate::date_format::DEFAULT_PATTERN)
        );
        assert_eq!(layers::order(&project), vec!["time", "date"]);
        assert_eq!(layers::describe(&project).last().unwrap().kind, "date");

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("L.months[d"), "generated JS: {js}");
        assert!(js.contains(".m0]"));

        // Zmiana wzorca daty trafia zarówno do podglądu (przez `date_format`),
        // jak i do wygenerowanego JS.
        let settings = serde_json::json!({
            "id": "date",
            "x": 23, "y": 250, "w": 420, "h": 80,
            "textSize": 48, "color": "0xFFFFFFFF", "align": "center",
            "timeFormat": "DD.MM.YYYY"
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.time_format.as_deref(), Some("DD.MM.YYYY"));
        assert_eq!(updated.text.as_deref(), Some("01.01.2026"));
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("p2((d1.d))"), "generated JS: {js}");
        assert!(js.contains("p2((d1.m))"));
    }

    #[test]
    fn add_user_text_widget_creates_static_text() {
        let mut project = Project::starter("Text Face");
        let text = widgets::add_widget(&mut project, "text").unwrap();
        assert_eq!(text.id, "text");
        assert_eq!(text.kind, WidgetKind::Text);
        assert_eq!(text.data_source, None);
        assert_eq!(text.time_format, None);
        assert_eq!(text.text.as_deref(), Some("Text"));
        assert_eq!(layers::order(&project), vec!["time", "text"]);
        assert_eq!(layers::describe(&project).last().unwrap().kind, "text");

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("text: \"Text\""), "generated JS: {js}");

        // Treść wpisana w inspektorze trafia do projektu i do wygenerowanego JS.
        // Pole `timeFormat` jest dla tego typu ignorowane.
        let settings = serde_json::json!({
            "id": "text",
            "x": 23, "y": 350, "w": 420, "h": 60,
            "textSize": 32, "color": "0xFFFFFFFF", "align": "center",
            "timeFormat": "HH:MM:SS", "text": "Witaj, świecie!"
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.text.as_deref(), Some("Witaj, świecie!"));
        assert_eq!(updated.time_format, None);
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("text: \"Witaj, świecie!\""), "generated JS: {js}");
    }

    #[test]
    fn add_step_widget_reads_step_sensor() {
        assert!(widgets::supported_widgets()
            .iter()
            .any(|item| item.kind == "steps" && item.category == "distance"));

        let mut project = Project::starter("Step Face");
        let steps = widgets::add_widget(&mut project, "steps").unwrap();
        assert_eq!(steps.id, "steps");
        assert_eq!(steps.kind, WidgetKind::StepText);
        assert_eq!(steps.data_source, Some(model::DataSource::Step));
        assert_eq!(steps.text.as_deref(), Some("0"));
        assert_eq!(steps.time_format, None);
        assert_eq!(layers::order(&project), vec!["time", "steps"]);
        assert_eq!(layers::describe(&project).last().unwrap().kind, "steps");
        assert!(validate(&project).is_empty());

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("hmSensor.createSensor(hmSensor.id.STEP)"), "JS: {js}");
        assert!(js.contains("_stepSensor.event.CHANGE"), "JS: {js}");
        assert!(js.contains("refs[1].setProperty(hmUI.prop.TEXT, _steps)"), "JS: {js}");
        assert!(js.contains("removeEventListener(stepSensor.event.CHANGE, stepTick)"), "JS: {js}");
        assert!(js.contains("text: \"0\""), "JS: {js}");

        // Inspektor nie wysyła wzorca dla kroków — a nawet gdyby wysłał,
        // nie może nadpisać wartości z czujnika.
        let settings = serde_json::json!({
            "id": "steps",
            "x": 23, "y": 320, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
            "timeFormat": "HH:MM:SS"
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.time_format, None);
        assert_eq!(updated.text.as_deref(), Some("0"));
    }

    #[test]
    fn add_distance_widget_uses_project_unit_and_widget_override() {
        assert!(widgets::supported_widgets()
            .iter()
            .any(|item| item.kind == "distance" && item.category == "distance"));

        let mut project = Project::starter("Distance Face");
        let distance = widgets::add_widget(&mut project, "distance").unwrap();
        assert_eq!(distance.id, "distance");
        assert_eq!(distance.kind, WidgetKind::DistanceText);
        assert_eq!(distance.data_source, Some(model::DataSource::Distance));
        assert_eq!(distance.text.as_deref(), Some("0.00 km"));
        assert_eq!(distance.measurement_system, None);
        assert_eq!(layers::describe(&project).last().unwrap().kind, "distance");
        assert!(validate(&project).is_empty());

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("hmSensor.createSensor(hmSensor.id.DISTANCE)"), "JS: {js}");
        assert!(js.contains("addEventListener(_distanceSensor.event.CHANGE, distanceTick)"), "JS: {js}");
        // Domyślnie jednostka pochodzi z zegarka (hmSetting.getMileageUnit()).
        assert!(js.contains("hmSetting.getMileageUnit() === 1"), "JS: {js}");
        assert!(js.contains("(_meters / (autoMileage ? 1609.344 : 1000)).toFixed(2)) + ' ' + (autoMileage ? 'mi' : 'km')"), "JS: {js}");
        assert!(js.contains("removeEventListener(distanceSensor.event.CHANGE, distanceTick)"), "JS: {js}");

        // Ustawienie aplikacji zmienia domyślną jednostkę całego projektu.
        project::set_measurement_system(&mut project, model::MeasurementSystem::Imperial);
        let widget = project.widgets.iter().find(|w| w.id == "distance").unwrap();
        assert_eq!(widget.text.as_deref(), Some("0.00 mi"));
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("(_meters / 1609.344).toFixed(2)) + ' mi'"), "JS: {js}");

        // Własny system widgetu nadpisuje domyślny z projektu.
        let settings = serde_json::json!({
            "id": "distance",
            "x": 23, "y": 260, "w": 420, "h": 60,
            "textSize": 32, "color": "0xFFFFFFFF", "align": "center",
            "measurementSystem": "metric"
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.measurement_system, Some(model::MeasurementSystem::Metric));
        assert_eq!(updated.text.as_deref(), Some("0.00 km"));
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("(_meters / 1000).toFixed(2)) + ' km'"), "JS: {js}");

        // Pusty string wraca do systemu projektu.
        let settings = serde_json::json!({
            "id": "distance",
            "x": 23, "y": 260, "w": 420, "h": 60,
            "textSize": 32, "color": "0xFFFFFFFF", "align": "center",
            "measurementSystem": ""
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.measurement_system, None);
        assert_eq!(updated.text.as_deref(), Some("0.00 mi"));
    }

    #[test]
    fn date_widgets_live_in_date_category_and_generate_js() {
        let cases = [
            ("date", "date", "_dateFromT(t)"),
            ("weekday-number", "weekday-number", "_weekdayNumber(t)"),
            ("week-number", "week-number", "_isoWeek(_dateFromT(t))"),
            ("day-of-year", "day-of-year", "_dayOfYear(_dateFromT(t))"),
            ("lunar-date", "lunar-date", "lunar_month"),
            ("festival", "festival", "solar_festival"),
            ("world-clock", "world-clock", "WORLD_CLOCK"),
        ];
        for (kind, layer_kind, needle) in cases {
            assert!(
                widgets::supported_widgets()
                    .iter()
                    .any(|item| item.kind == kind && item.category == "date"),
                "kind={kind}"
            );
            let mut project = Project::starter("Dates");
            let widget = widgets::add_widget(&mut project, kind).unwrap();
            if kind != "date" {
                assert_eq!(widget.time_format, None, "kind={kind}");
            }
            assert_eq!(
                layers::describe(&project).last().unwrap().kind,
                layer_kind,
                "kind={kind}"
            );
            assert!(validate(&project).is_empty(), "kind={kind}");
            let js = codegen::generate_index_js(&project);
            assert!(js.contains(needle), "kind={kind} JS: {js}");
        }

        // Algorytmy podglądu i JS muszą dawać te same wartości.
        assert_eq!(date_format::iso_week(3, 11, 2026), 45);
        // 2026-11-03 to wtorek: poniedziałek jako pierwszy dzień daje 2, niedziela daje 3.
        assert_eq!(
            date_format::weekday_number(3, 11, 2026, model::FirstDayOfWeek::Monday),
            2
        );
        assert_eq!(
            date_format::weekday_number(3, 11, 2026, model::FirstDayOfWeek::Sunday),
            3
        );
        assert_eq!(date_format::day_of_year(3, 11, 2026), 307);
    }

    #[test]
    fn date_widgets_never_generate_date_object() {
        let mut project = Project::starter("No Date");
        for kind in ["date", "weekday-number", "week-number", "day-of-year"] {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        let js = codegen::generate_index_js(&project);
        for banned in [
            "new Date",
            "Date.UTC",
            ".getDate(",
            ".getMonth(",
            ".getDay(",
            ".getFullYear(",
            "getUTCDay",
            "setUTCDate",
        ] {
            assert!(!js.contains(banned), "banned {banned} in js: {js}");
        }
        // Pola liczone z czujnika TIME, bez obiektu Date.
        assert!(js.contains("const _dateFromT = (t) => {"), "{js}");
        assert!(js.contains("w: _weekdayIndex("), "{js}");
        assert!(js.contains("L.months[d") && js.contains(".m0]"), "{js}");
        assert!(js.contains("function _isoWeek(d)"), "{js}");
        assert!(js.contains("function _dayOfYear(d)"), "{js}");
    }

    #[test]
    fn health_widgets_generate_sensor_js() {
        let cases = [
            ("heart", "HEART", "-- bpm", "heart"),
            ("stress", "STRESS", "--", "stress"),
            ("pai", "PAI", "--", "pai"),
            ("skin-temp", "BODY_TEMP", "-- °C", "skin-temp"),
            ("calories", "CALORIE", "-- kcal", "calories"),
            ("spo2", "SPO2", "--%", "spo2"),
        ];
        for (kind, _, _, _) in cases {
            assert!(
                widgets::supported_widgets()
                    .iter()
                    .any(|item| item.kind == kind && item.category == "health"),
                "kind={kind}"
            );
        }
        for (kind, sensor_id, initial, layer_kind) in cases {
            let mut project = Project::starter("Health Face");
            let widget = widgets::add_widget(&mut project, kind).unwrap();
            assert_eq!(widget.text.as_deref(), Some(initial), "kind={kind}");
            assert_eq!(widget.time_format, None, "kind={kind}");
            assert_eq!(
                layers::describe(&project).last().unwrap().kind,
                layer_kind,
                "kind={kind}"
            );
            assert!(validate(&project).is_empty(), "kind={kind}");

            let js = codegen::generate_index_js(&project);
            assert!(
                js.contains(&format!("hmSensor.id.{sensor_id}")),
                "kind={kind} JS: {js}"
            );
            assert!(js.contains("healthTick"), "kind={kind} JS: {js}");
            assert!(js.contains("healthSensors"), "kind={kind} JS: {js}");
            assert!(
                js.contains("removeEventListener"),
                "kind={kind} JS: {js}"
            );
        }
    }

    #[test]
    fn sleep_widgets_live_in_sleep_category_and_generate_js() {
        let expected = vec![
            "sleep",
            "sleep-score",
            "sleep-deep",
            "sleep-light",
            "sleep-rem",
            "sleep-wake",
            "sleep-start",
            "sleep-end",
            "sleep-target",
        ];
        let actual: Vec<_> = widgets::supported_widgets()
            .into_iter()
            .filter(|item| item.category == widgets::CATEGORY_SLEEP)
            .map(|item| item.kind)
            .collect();
        assert_eq!(actual, expected);

        let mut project = Project::starter("Sleep");
        for kind in expected {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        assert!(validate(&project).is_empty());
        let js = codegen::generate_index_js(&project);
        for token in [
            "hmSensor.id.SLEEP",
            "function _getSleep()",
            "s.getBasicInfo()",
            "s.getSleepStageData()",
            "_fmtSleepHM(",
            "_sleepStageText(",
            "sleepTick()",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}");
        }
        assert!(!js.contains("new Date"));
    }

    #[test]
    fn sleep_stage_mode_changes_preview_and_export() {
        let mut project = Project::starter("Sleep");
        let widget = widgets::add_widget(&mut project, "sleep-deep").unwrap();
        assert_eq!(widget.text.as_deref(), Some("-- min"));
        let settings = serde_json::json!({
            "id": widget.id,
            "x": 23, "y": 150, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
            "sleepStageMode": "percent",
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.sleep_stage_mode, crate::model::SleepStageMode::Percent);
        assert_eq!(updated.text.as_deref(), Some("--%"));
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("_sleepStageText(_sModels && _sModels.DEEP_STAGE, true)"));
    }

    #[test]
    fn heart_stats_read_today_array_without_redeclaring_getter() {
        let mut project = Project::starter("HrStats");
        for kind in ["heart", "heart-min", "heart-max", "heart-avg"] {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        let js = codegen::generate_index_js(&project);
        for token in [
            "_refreshHrStats()",
            "heart.today",
            "_hrStats.min",
            "_hrStats.max",
            "_hrStats.avg",
            "if (v <= 0 || v > 250) continue",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}");
        }
        let declarations = js
            .matches("const _hHeart = fromTick ? null : _getHealthHeart()")
            .count();
        assert_eq!(declarations, 1, "getter must be emitted once: {js}");
        assert!(!js.contains("new Date"));
    }

    #[test]
    fn weather_day_offset_controls_forecast_day() {
        let mut project = Project::starter("Forecast");
        let widget = widgets::add_widget(&mut project, "weather-high").unwrap();
        let settings = serde_json::json!({
            "id": widget.id,
            "x": 23, "y": 150, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
            "weatherDayOffset": 2,
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.weather_day_offset, 2);
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("const _wd_"), "day constant missing: {js}");
        assert!(js.contains("_wForecast.data[_wd_"), "day not used: {js}");
        assert!(!js.contains("_wForecast.data[0].high"));

        let state = preview::WeatherPreviewState::from(preview::DEFAULT_PREVIEW_WEATHER)
            .with_overrides(&serde_json::json!({ "dayOffset": 2 }));
        assert_eq!(state.day_offset, 2);
        assert_eq!(state.resolved_day().unwrap().index, 6);
        assert_eq!(preview::weather_text("weather-high", &state, crate::locale::WatchfaceLocale::English), "23°");
    }

    #[test]
    fn sensor_fields_package_one_are_in_catalog_and_export() {
        let cases = [
            ("heart", "health", "heart", "HEART", "-- bpm"),
            ("spo2-hour-avg", "health", "spo2-hour-avg", "SPO2", "--%"),
            ("pai-prepai", "health", "pai-prepai", "PAI", "--"),
            ("sleep-wake", "sleep", "sleep-wake", "SLEEP", "-- min"),
            ("sun-current", "weather", "sun-current", "WEATHER", "--:--"),
        ];
        for (kind, category, layer_kind, sensor_id, initial) in cases {
            let entry = widgets::supported_widgets()
                .into_iter()
                .find(|item| item.kind == kind)
                .unwrap_or_else(|| panic!("missing catalog entry {kind}"));
            assert_eq!(entry.category, category, "kind={kind}");
            assert!(!entry.has_pattern, "kind={kind}");

            let mut project = Project::starter("Package One");
            let widget = widgets::add_widget(&mut project, kind).unwrap();
            assert_eq!(widget.text.as_deref(), Some(initial), "kind={kind}");
            assert_eq!(widget.time_format, None, "kind={kind}");
            assert_eq!(
                layers::describe(&project).last().unwrap().kind,
                layer_kind,
                "kind={kind}"
            );
            assert!(validate(&project).is_empty(), "kind={kind}");
            let js = codegen::generate_index_js(&project);
            assert!(js.contains(&format!("hmSensor.id.{sensor_id}")), "kind={kind}");
            assert!(!js.contains("new Date"), "kind={kind}");
        }

        let mut project = Project::starter("Sources");
        for kind in [
            "heart",
            "spo2-hour-avg",
            "pai-prepai",
            "sleep-wake",
            "sun-current",
        ] {
            widgets::add_widget(&mut project, kind).unwrap();
        }
        let js = codegen::generate_index_js(&project);
        for token in [
            "_hHeart.last",
            "_spo2Hour",
            "hourAvgofDay",
            "prepai0",
            "WAKE_STAGE",
            "function _sunCurrentText()",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}");
        }
        // Kilka widgetów czyta ten sam czujnik — getter może być wyemitowany tylko raz.
        assert_eq!(
            js.matches("const _hHeart = fromTick ? null : _getHealthHeart()")
                .count(),
            1
        );
        assert_eq!(js.matches("const _hPai = _getHealthPai()").count(), 1);
        assert!(!js.contains("@zos/ui"));
    }

    #[test]
    fn heart_last_duplicate_is_not_in_catalog() {
        assert!(!widgets::supported_widgets()
            .iter()
            .any(|entry| entry.kind == "heart-last"));
        assert!(widgets::add_widget(&mut Project::starter("Heart"), "heart-last").is_err());

        let mut project = Project::starter("Heart");
        widgets::add_widget(&mut project, "heart").unwrap();
        let js = codegen::generate_index_js(&project);
        // Zwykły `heart` czyta HEART.last — nie ma potrzeby drugiego widgetu.
        assert!(js.contains("_hHeart.last"));
        assert_eq!(js.matches("event: 'LAST'").count(), 1);
        assert_eq!(js.matches("event:LAST").count(), 0);
    }

    #[test]
    fn weather_current_uses_firmware_bound_text_font() {
        let mut project = Project::starter("Current temp");
        let widget = widgets::add_widget(&mut project, "weather-current").unwrap();
        // Tekst w projekcie jest tylko wartością zastęczą dla podglądu PC —
        // nie trafia do JS, bo firmware podstawia prawdziwą temperaturę.
        assert_eq!(widget.text.as_deref(), Some("22°"));
        let settings = serde_json::json!({
            "id": widget.id,
            "x": 23, "y": 150, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
            "text": "18°",
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.text.as_deref(), Some("22°"), "text content must not be editable");

        let js = codegen::generate_index_js(&project);
        for token in [
            "hmUI.widget.TEXT_FONT",
            "type: hmUI.data_type.WEATHER_CURRENT",
            "unit_type: 1",
            "text_size: 40",
            "align_h: hmUI.align.CENTER_H",
            "show_level: hmUI.show_level.ONLY_NORMAL",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}: {js}");
        }
        assert!(!js.contains("22°"), "no static text may be exported: {js}");
        assert!(!js.contains("hmSensor.id.WEATHER"));
        assert!(!js.contains("@zos/ui"));

        let no_unit = serde_json::json!({
            "id": widget.id,
            "x": 23, "y": 150, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
            "showUnit": false,
        });
        widgets::update_widget(&mut project, &no_unit).unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("unit_type: 0"), "showUnit must map to unit_type: {js}");

        let state = preview::WeatherPreviewState::from(preview::DEFAULT_PREVIEW_WEATHER);
        assert_eq!(state.current, 22);
        assert_eq!(
            preview::weather_text("weather-current", &state, crate::locale::WatchfaceLocale::English),
            "22°"
        );
        let overridden = state.with_overrides(&serde_json::json!({ "current": 18 }));
        assert_eq!(
            preview::weather_text("weather-current", &overridden, crate::locale::WatchfaceLocale::English),
            "18°"
        );
        assert!(validate(&project).is_empty());
    }

    #[test]
    fn sun_current_preview_matches_watch_minutes() {
        let base = preview::DEFAULT_PREVIEW_WEATHER;
        let forecast = preview::forecast_days(base);
        let state = preview::WeatherPreviewState::from(base)
            .with_overrides(&serde_json::json!({ "nowMinute": 1297 }));
        assert_eq!(state.now_minute, 1297);
        let text = preview::weather_text("sun-current", &state, crate::locale::WatchfaceLocale::English);
        let parts: Vec<_> = text.split(':').collect();
        assert_eq!(parts.len(), 2, "unexpected text: {text}");
        assert!(parts[0].parse::<u32>().is_ok() && parts[1].parse::<u32>().is_ok());
        assert_eq!(preview::sun_current_minutes(&forecast, 1297).is_some(), true);
        assert_eq!(preview::sun_current_minutes(&forecast, 1440), None);
        let missing = preview::WeatherPreviewState {
            forecast: Vec::new(),
            ..state
        };
        assert_eq!(
            preview::weather_text("sun-current", &missing, crate::locale::WatchfaceLocale::English),
            "--:--"
        );
    }

    #[test]
    fn sleep_wake_supports_duration_and_percent() {
        let mut project = Project::starter("Wake");
        let widget = widgets::add_widget(&mut project, "sleep-wake").unwrap();
        assert_eq!(widget.text.as_deref(), Some("-- min"));
        let settings = serde_json::json!({
            "id": widget.id,
            "x": 23, "y": 150, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
            "sleepStageMode": "percent",
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.sleep_stage_mode, crate::model::SleepStageMode::Percent);
        assert_eq!(updated.text.as_deref(), Some("--%"));
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("_sleepStageText(_sModels && _sModels.WAKE_STAGE, true)"));
    }

    #[test]
    fn progress_widgets_generate_native_arc_and_bar() {
        for kind in ["arc-progress", "bar-progress"] {
            let entry = widgets::supported_widgets()
                .into_iter()
                .find(|item| item.kind == kind)
                .unwrap_or_else(|| panic!("missing {kind}"));
            assert_eq!(entry.category, widgets::CATEGORY_PROGRESS);
        }
        let mut project = Project::starter("Progress");
        let arc = widgets::add_widget(&mut project, "arc-progress").unwrap();
        let bar = widgets::add_widget(&mut project, "bar-progress").unwrap();
        assert!(arc.progress.is_some() && bar.progress.is_some());
        assert!(validate(&project).is_empty());

        let arc_settings = serde_json::json!({
            "id": arc.id,
            "x": 33, "y": 33, "w": 400, "h": 400,
            "alpha": 255,
            "progress": {
                "source": "steps", "min": 0, "max": 10000,
                "color": "0xFF00FF00", "trackColor": "0xFF333333",
                "thickness": 20, "startAngle": -135, "endAngle": 135,
                "corner": "round", "segments": 4, "gap": 6, "showTrack": true
            }
        });
        let updated = widgets::update_widget(&mut project, &arc_settings).unwrap();
        let p = updated.progress.unwrap();
        assert_eq!(p.segments, 4);
        assert_eq!(p.gap, 6);
        assert_eq!(p.source, crate::model::ProgressSource::Steps);

        let js = codegen::generate_index_js(&project);
        for token in [
            "hmUI.widget.ARC_PROGRESS",
            "hmUI.widget.FILL_RECT",
            "corner_flag: 0",
            "_progApply_",
            "_createProgSensor('STEP')",
            "view.fill.setProperty(hmUI.prop.MORE",
            "Math.round(420 * v)",
            "hmUI.show_level.ONLY_NORMAL",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}");
        }
        // Regresja: wcześniej generowało się `Math.round(w * v)` bez zdefiniowanego `w`.
        assert!(!js.contains("Math.round(w * v)"));
        assert!(!js.contains("view.segments"));
        assert!(!js.contains("@zos/ui"));
        assert!(!js.contains("new Date"));
    }

    #[test]
    fn progress_defaults_are_full_circle_and_rounded() {
        let mut project = Project::starter("Defaults");
        let arc = widgets::add_widget(&mut project, "arc-progress").unwrap();
        let bar = widgets::add_widget(&mut project, "bar-progress").unwrap();
        let defaults = arc.progress.as_ref().unwrap();
        assert_eq!(defaults.start_angle, 0);
        assert_eq!(defaults.end_angle, 360);
        assert_eq!(defaults.corner, crate::model::ProgressCorner::Round);
        assert_eq!(bar.progress.as_ref().unwrap().orientation, crate::model::ProgressOrientation::Horizontal);
        assert!(validate(&project).is_empty());

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("start_angle: 0, end_angle: 360"), "default arc must be a full circle: {js}");
        assert!(js.contains("corner_flag: 0"));

        let flat = serde_json::json!({
            "id": bar.id,
            "x": 23, "y": 203, "w": 420, "h": 60,
            "progress": { "corner": "flat" },
        });
        widgets::update_widget(&mut project, &flat).unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("radius: 0"), "flat bar must have no corner radius: {js}");
        let flat_arc = serde_json::json!({
            "id": arc.id,
            "x": 33, "y": 33, "w": 400, "h": 400,
            "progress": { "corner": "flat" },
        });
        widgets::update_widget(&mut project, &flat_arc).unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("corner_flag: 3"), "flat arc must disable rounded ends: {js}");
    }

    #[test]
    fn progress_sensors_are_listened_and_ticked() {
        let mut project = Project::starter("ProgressRefresh");
        let arc = widgets::add_widget(&mut project, "arc-progress").unwrap();
        let settings = serde_json::json!({
            "id": arc.id,
            "x": 33, "y": 33, "w": 400, "h": 400,
            "progress": { "source": "heart", "min": 0, "max": 180 },
        });
        widgets::update_widget(&mut project, &settings).unwrap();
        let js = codegen::generate_index_js(&project);
        // HEART czyta ostatni pomiar i odświeża się wyłącznie zdarzeniem LAST —
        // nie uruchamiamy ciągłego pomiaru (CURRENT) ani odczytu co tick.
        assert!(js.contains("_createProgSensor('HEART')"), "JS: {js}");
        assert!(js.contains("s.last || 0"), "JS: {js}");
        assert!(js.contains("const lo = 0, hi = 165"), "JS: {js}");
        assert!(js.contains("event: 'LAST'"), "JS: {js}");
        assert!(!js.contains("event: 'CURRENT'"), "JS: {js}");
        assert!(
            js.contains("sensor.addEventListener(sensor.event[entry.event], progressTickEvent)"),
            "JS: {js}"
        );
        assert!(js.contains("if (progressTick) progressTick(true)"), "JS: {js}");
        assert!(
            js.contains("sensor.removeEventListener(sensor.event[entry.event], progressTickEvent)"),
            "JS: {js}"
        );
        assert!(!js.contains("event: 'CHANGE'"), "JS: {js}");
    }

    #[test]
    fn progress_change_sources_and_fixed_source() {
        let mut project = Project::starter("ProgressEvents");
        let bar = widgets::add_widget(&mut project, "bar-progress").unwrap();
        let fixed = widgets::add_widget(&mut project, "arc-progress").unwrap();
        widgets::update_widget(
            &mut project,
            &serde_json::json!({
                "id": bar.id,
                "x": 23, "y": 203, "w": 420, "h": 60,
                "progress": { "source": "steps" },
            }),
        )
        .unwrap();
        widgets::update_widget(
            &mut project,
            &serde_json::json!({
                "id": fixed.id,
                "x": 33, "y": 33, "w": 400, "h": 400,
                "progress": { "source": "fixed", "value": 42 },
            }),
        )
        .unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("_createProgSensor('STEP')"), "JS: {js}");
        assert!(js.contains("event: 'CHANGE'"), "JS: {js}");
        // Fixed nie tworzy czujnika ani listenera.
        assert_eq!(js.matches("_createProgSensor('").count(), 1, "JS: {js}");
        assert_eq!(js.matches("get: () => _progSensor_").count(), 1, "JS: {js}");
    }

    #[test]
    fn heart_text_uses_last_measurement_without_tick_polling() {
        let mut project = Project::starter("HeartLast");
        widgets::add_widget(&mut project, "heart").unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("_hHeart.last ? String(_hHeart.last)"), "JS: {js}");
        assert!(!js.contains("_hHeart.current ? String(_hHeart.current)"), "JS: {js}");
        assert!(js.contains("event: 'LAST'"), "JS: {js}");
        assert!(!js.contains("event: 'CURRENT'"), "JS: {js}");
        assert!(
            js.contains("const _hHeart = fromTick ? null : _getHealthHeart()"),
            "JS: {js}"
        );
        assert!(js.contains("healthTick(true)"), "JS: {js}");
    }

    #[test]
    fn progress_goal_sources_follow_watch_target() {
        let mut project = Project::starter("Goals");
        let bar = widgets::add_widget(&mut project, "bar-progress").unwrap();
        widgets::update_widget(
            &mut project,
            &serde_json::json!({
                "id": bar.id,
                "x": 23, "y": 203, "w": 420, "h": 60,
                "progress": { "source": "steps", "max": 100 },
            }),
        )
        .unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(
            js.contains("const lo = 0, hi = (s && s.target ? s.target : 100)"),
            "goal from watch must win over stored max: {js}"
        );
    }

    #[test]
    fn progress_sleep_source_reads_fresh_data() {
        let mut project = Project::starter("ProgressSleep");
        let arc = widgets::add_widget(&mut project, "arc-progress").unwrap();
        widgets::update_widget(
            &mut project,
            &serde_json::json!({
                "id": arc.id,
                "x": 33, "y": 33, "w": 400, "h": 400,
                "progress": { "source": "sleep" },
            }),
        )
        .unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("_createProgSensor('SLEEP')"), "JS: {js}");
        assert!(js.contains("s.updateInfo()"), "JS: {js}");
    }

    #[test]
    fn arc_thickness_keeps_center_radius_fixed() {
        let mut project = Project::starter("Thickness");
        let widget = widgets::add_widget(&mut project, "arc-progress").unwrap();
        let base = serde_json::json!({
            "id": widget.id,
            "x": 33, "y": 33, "w": 400, "h": 400,
            "progress": { "thickness": 16 },
        });
        widgets::update_widget(&mut project, &base).unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("radius: 190"), "center radius for 400px frame: {js}");
        assert!(js.contains("line_width: 16"));

        // Grubsza linia przesuwa środek do wewnątrz, ale krawędź zewnętrzna
        // pozostaje w ramce (2 px marginesu) — niebieski obrys jej nie tnie.
        let thick = serde_json::json!({
            "id": widget.id,
            "x": 33, "y": 33, "w": 400, "h": 400,
            "progress": { "thickness": 40 },
        });
        widgets::update_widget(&mut project, &thick).unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("radius: 190"), "center must stay fixed: {js}");
        assert!(js.contains("line_width: 40"));
        assert!(validate(&project).is_empty());
    }

    #[test]
    fn shortcut_exports_transparent_click_zone_without_asset() {
        let entry = widgets::supported_widgets()
            .into_iter()
            .find(|item| item.kind == "shortcut")
            .unwrap();
        assert_eq!(entry.category, widgets::CATEGORY_SHORTCUT);

        let mut project = Project::starter("Shortcut");
        let widget = widgets::add_widget(&mut project, "shortcut").unwrap();
        assert_eq!(widget.shortcut_target, Some(crate::model::ShortcutTarget::Battery));
        assert!(validate(&project).is_empty());

        let settings = serde_json::json!({
            "id": widget.id,
            "x": 173, "y": 203, "w": 120, "h": 60,
            "shortcutTarget": "heart",
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.shortcut_target, Some(crate::model::ShortcutTarget::Heart));
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("hmUI.widget.IMG_CLICK"), "JS: {js}");
        assert!(js.contains("type: hmUI.data_type.HEART"), "JS: {js}");
        // Pusty src = w pełni przezroczysta nakładka; żaden zasób nie trafia do ZIP-u
        // (popuprawny 1×1 PNG był odrzucany przy instalacji przez QR i w zeus).
        assert!(js.contains("src: \"\""), "JS: {js}");
        assert!(!js.contains("shortcut_hit.png"), "JS: {js}");
        assert!(!js.contains("@zos/ui"));
    }

    #[test]
    fn shortcuts_are_created_last_and_keep_their_layer_order() {
        let mut project = Project::starter("Shortcuts");
        let bottom = widgets::add_widget(&mut project, "shortcut").unwrap();
        widgets::add_widget(&mut project, "heart").unwrap();
        let top = widgets::add_widget(&mut project, "shortcut").unwrap();
        // Wyższa warstwa nad shortcutem nie może przejąć dotyku — przesuwamy
        // pierwszą strefę na spód, żeby udowodnić niezależność od kolejności warstw.
        let pos = layers::order(&project)
            .iter()
            .position(|id| id == &bottom.id)
            .unwrap() as i32;
        layers::move_layer(&mut project, &bottom.id, -pos).unwrap();
        assert_eq!(layers::order(&project)[0], bottom.id);

        let js = codegen::generate_index_js(&project);
        let created: Vec<(String, usize)> = js
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("refs[")?;
                let index = rest.split(']').next()?.parse().ok()?;
                let widget = line
                    .split("hmUI.widget.")
                    .nth(1)?
                    .split(|c| c == ',' || c == ' ')
                    .next()?
                    .to_string();
                Some((widget, index))
            })
            .collect();
        let first_click = created.iter().position(|(kind, _)| kind == "IMG_CLICK").unwrap();
        let last_other = created.iter().rposition(|(kind, _)| kind != "IMG_CLICK").unwrap();
        assert!(first_click > last_other, "a widget is created above a shortcut: {created:?}");

        let click_refs: Vec<usize> = created
            .iter()
            .filter(|(kind, _)| kind == "IMG_CLICK")
            .map(|(_, index)| *index)
            .collect();
        let expected: Vec<usize> = layers::ordered_widgets(&project)
            .iter()
            .enumerate()
            .filter(|(_, w)| w.kind == WidgetKind::Shortcut)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(click_refs, expected);
        assert!(click_refs.contains(&0));
        assert!(click_refs.iter().any(|&index| index != 0));
        assert!(layers::ordered_widgets(&project)
            .get(click_refs[1])
            .is_some_and(|w| w.id == top.id));
    }

    #[test]
    fn shortcut_overlay_marks_click_zones_in_preview_frame() {
        let mut project = Project::starter("ShortcutOverlay");
        widgets::add_widget(&mut project, "shortcut").unwrap();
        let series = preview::random_series(1234, "en");
        let frame = preview::frame_data(0, &series).unwrap();
        let mut image = preview::render_frame(std::path::Path::new("."), &project, &frame, &series);
        let before_image = image.clone();
        preview::draw_shortcut_overlays(&mut image, &project);
        let changed = before_image
            .pixels()
            .zip(image.pixels())
            .filter(|(a, b)| a.0 != b.0)
            .count();
        assert!(changed > 0, "click zone overlay was not drawn");
    }

    #[test]
    fn progress_bar_allows_one_pixel_frame() {
        let input: widgets::FrameInput = serde_json::from_value(serde_json::json!({
            "x": 10, "y": 10, "w": 0, "h": 0, "textSize": 40,
            "handle": "move", "dx": 0, "dy": 0, "autoScale": false, "minSize": 1
        }))
        .unwrap();
        let frame = widgets::frame_geometry(&input);
        assert_eq!((frame.w, frame.h), (1, 1));

        let standard: widgets::FrameInput = serde_json::from_value(serde_json::json!({
            "x": 10, "y": 10, "w": 0, "h": 0, "textSize": 40,
            "handle": "move", "dx": 0, "dy": 0, "autoScale": false
        }))
        .unwrap();
        let frame = widgets::frame_geometry(&standard);
        assert_eq!((frame.w, frame.h), (widgets::MIN_FRAME_SIZE, widgets::MIN_FRAME_SIZE));
    }

    #[test]
    fn progress_colours_reach_export_and_preview_frames() {
        let mut project = Project::starter("Colours");
        let bar = widgets::add_widget(&mut project, "bar-progress").unwrap();
        let settings = serde_json::json!({
            "id": bar.id,
            "x": 183, "y": 220, "w": 100, "h": 20,
            "progress": {
                "color": "0xFF112233",
                "trackColor": "0xFF445566",
                "corner": "flat",
                "showTrack": true
            }
        });
        widgets::update_widget(&mut project, &settings).unwrap();
        assert!(validate(&project).is_empty());

        // JS na zegarku musi dostać oba kolory.
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("color: 0xFF112233"), "fill colour missing: {js}");
        assert!(js.contains("color: 0xFF445566"), "track colour missing: {js}");

        // Klatki PNG/GIF używają tych samych kolorów.
        let series = preview::random_series(1234, "en");
        let full = preview::frame_data(0, &series).unwrap();
        let image = preview::render_frame(std::path::Path::new("."), &project, &full, &series);
        assert_eq!(image.get_pixel(188, 230).0, [17, 34, 51, 255]);

        let low = preview::frame_data(6, &series).unwrap();
        let image = preview::render_frame(std::path::Path::new("."), &project, &low, &series);
        assert_eq!(image.get_pixel(270, 230).0, [68, 85, 102, 255]);
    }

    #[test]
    fn square_bar_roundness_becomes_a_circle() {
        let mut project = Project::starter("SquareBar");
        let widget = widgets::add_widget(&mut project, "bar-progress").unwrap();
        let settings = serde_json::json!({
            "id": widget.id,
            "x": 100, "y": 100, "w": 150, "h": 150,
            "progress": { "corner": "round", "radius": 100 },
        });
        widgets::update_widget(&mut project, &settings).unwrap();
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("radius: 75"), "150px square must clamp radius to 75: {js}");
        assert!(validate(&project).is_empty());
    }

    #[test]
    fn health_widget_can_hide_its_unit() {
        let mut project = Project::starter("Units");
        let heart = widgets::add_widget(&mut project, "heart").unwrap();
        assert!(heart.show_unit);
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("+ ' bpm'"), "JS: {js}");

        let settings = serde_json::json!({
            "id": heart.id,
            "x": 23, "y": 150, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center",
            "showUnit": false
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert!(!updated.show_unit);
        let js = codegen::generate_index_js(&project);
        assert!(!js.contains("' bpm'"), "JS: {js}");
        assert!(js.contains("if (_hHeart)"), "JS: {js}");
        assert_eq!(
            layers::describe(&project).last().unwrap().show_unit,
            false
        );

        // Brak klucza zachowuje poprzedni wybór.
        let settings = serde_json::json!({
            "id": heart.id,
            "x": 23, "y": 150, "w": 420, "h": 60,
            "textSize": 40, "color": "0xFFFFFFFF", "align": "center"
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert!(!updated.show_unit);
        assert!(validate(&project).is_empty());
    }

    #[test]
    fn distance_widget_follows_watch_unit_by_default() {
        let mut project = Project::starter("Watch Unit");
        assert_eq!(project.measurement_system, model::MeasurementSystem::Watch);
        let distance = widgets::add_widget(&mut project, "distance").unwrap();
        assert_eq!(distance.text.as_deref(), Some("0.00 km"));
        assert_eq!(
            layers::describe(&project).last().unwrap().measurement_system,
            None
        );

        let js = codegen::generate_index_js(&project);
        assert!(js.contains("function _initMileageUnit()"), "JS: {js}");
        assert!(js.contains("_initMileageUnit()"), "JS: {js}");
        assert!(js.contains("hmSetting.getMileageUnit() === 1"), "JS: {js}");
        assert!(js.contains("autoMileage ? 'mi' : 'km'"), "JS: {js}");

        // „Zgodnie z zegarkiem" działa też jako jawne nadpisanie widgetu.
        let settings = serde_json::json!({
            "id": "distance",
            "x": 23, "y": 260, "w": 420, "h": 60,
            "textSize": 32, "color": "0xFFFFFFFF", "align": "center",
            "measurementSystem": "watch"
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(
            updated.measurement_system,
            Some(model::MeasurementSystem::Watch)
        );
        assert_eq!(
            layers::describe(&project).last().unwrap().measurement_system.as_deref(),
            Some("watch")
        );

        // Wyłączenie nadpisania wraca do systemu projektu (imperial) bez odczytu z zegarka.
        let settings = serde_json::json!({
            "id": "distance",
            "x": 23, "y": 260, "w": 420, "h": 60,
            "textSize": 32, "color": "0xFFFFFFFF", "align": "center",
            "measurementSystem": ""
        });
        project::set_measurement_system(&mut project, model::MeasurementSystem::Imperial);
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.measurement_system, None);
        assert_eq!(updated.text.as_deref(), Some("0.00 mi"));
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("(_meters / 1609.344).toFixed(2)) + ' mi'"), "JS: {js}");
        assert!(!js.contains("autoMileage"), "JS: {js}");
    }

    #[test]
    fn update_widget_changes_settings_of_selected_widget() {
        let mut project = Project::starter("Inspector");
        widgets::add_widget(&mut project, "time").unwrap();
        let settings = serde_json::json!({
            "id": "time-2",
            "x": 30,
            "y": 200,
            "w": 300,
            "h": 80,
            "textSize": 42,
            "color": "0xFF00FF00",
            "align": "left",
            "timeFormat": "HH:MM"
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert_eq!(updated.x, 30);
        assert_eq!(updated.text_size, Some(42));
        assert_eq!(updated.time_format.as_deref(), Some("HH:MM"));
        assert_eq!(updated.text.as_deref(), Some("00:00"));
        assert_eq!(project.widgets[0].x, 23); // oryginalny widget nietknięty

        let mut bad = settings.clone();
        bad["x"] = serde_json::json!(400);
        bad["w"] = serde_json::json!(200);
        assert!(widgets::update_widget(&mut project, &bad).is_err());
    }

    #[test]
    fn frame_fields_default_and_survive_roundtrip() {
        let project = Project::starter("Ramka");
        assert!(!project.widgets[0].auto_scale);
        assert_eq!(project.widgets[0].text_style, model::TextStyle::Ellipsis);
        assert_eq!(project.widgets[0].v_align, model::VAlign::Center);

        let json = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.widgets[0].text_style, model::TextStyle::Ellipsis);
        assert!(codegen::generate_index_js(&loaded).contains("text_style: hmUI.text_style.ELLIPSIS"));

        // Stary projekt bez nowych pól też się wczyta.
        let legacy = json.replace(",\"autoScale\":false", "").replace(",\"textStyle\":\"ellipsis\"", "");
        let legacy: Project = serde_json::from_str(&legacy).unwrap();
        assert!(!legacy.widgets[0].auto_scale);
        assert_eq!(legacy.widgets[0].text_style, model::TextStyle::Ellipsis);
    }

    #[test]
    fn update_widget_sets_frame_mode_and_text_fit() {
        let mut project = Project::starter("Ramka");
        let settings = serde_json::json!({
            "id": "time",
            "x": 23,
            "y": 183,
            "w": 420,
            "h": 100,
            "textSize": 64,
            "color": "0xFFFFFFFF",
            "align": "center",
            "timeFormat": "HH:MM:SS",
            "autoScale": true,
            "textStyle": "slide",
            "vAlign": "bottom",
        });
        let updated = widgets::update_widget(&mut project, &settings).unwrap();
        assert!(updated.auto_scale);
        assert_eq!(updated.text_style, model::TextStyle::None);
        assert_eq!(updated.v_align, model::VAlign::Bottom);
        let js = codegen::generate_index_js(&project);
        assert!(js.contains("text_style: hmUI.text_style.NONE"));
        assert!(js.contains("align_v: hmUI.align.BOTTOM"));

        let described = layers::describe(&project);
        assert_eq!(described[0].auto_scale, true);
        assert_eq!(described[0].text_style, "none");
        assert_eq!(described[0].v_align, "bottom");

        let mut invalid = settings.clone();
        invalid["textStyle"] = serde_json::json!("bogus");
        assert!(widgets::update_widget(&mut project, &invalid).is_err());
    }

    #[test]
    fn frame_geometry_respects_screen_and_minimum_size() {
        use widgets::{frame_geometry, FrameInput};

        let move_input = FrameInput {
            x: 23,
            y: 183,
            w: 420,
            h: 100,
            text_size: 64,
            handle: "move".to_string(),
            dx: -9999,
            dy: -9999,
            auto_scale: false,
            snap: false,
            align_x: vec![],
            align_y: vec![],
            square: false,
        min_size: crate::widgets::MIN_FRAME_SIZE,
        max_size: 466,
        };
        let moved = frame_geometry(&move_input);
        assert_eq!((moved.x, moved.y), (0, 0));
        assert_eq!((moved.w, moved.h), (420, 100));

        let resize = FrameInput {
            handle: "nw".to_string(),
            dx: -500,
            dy: -500,
            min_size: crate::widgets::MIN_FRAME_SIZE,
            max_size: 466,
            ..move_input
        };
        let resized = frame_geometry(&resize);
        assert_eq!((resized.x, resized.y), (0, 0));
        // Lewy/górny brzeg dociera do zera, prawy i dolny zostają na miejscu.
        assert_eq!((resized.w, resized.h), (443, 283));

        let tiny = FrameInput {
            x: 20,
            y: 20,
            w: 100,
            h: 100,
            text_size: 16,
            handle: "se".to_string(),
            dx: -500,
            dy: -500,
            auto_scale: true,
            snap: false,
            align_x: vec![],
            align_y: vec![],
            square: false,
        min_size: crate::widgets::MIN_FRAME_SIZE,
        max_size: 466,
        };
        let tiny = frame_geometry(&tiny);
        assert_eq!((tiny.w, tiny.h), (widgets::MIN_FRAME_SIZE, widgets::MIN_FRAME_SIZE));
        assert_eq!(tiny.text_size, widgets::MIN_TEXT_SIZE);
    }

    #[test]
    fn frame_geometry_snaps_edges_to_grid() {
        use widgets::{frame_geometry, FrameInput, SNAP_GRID};

        let base = FrameInput {
            x: 111,
            y: 139,
            w: 274,
            h: 134,
            text_size: 64,
            handle: "move".to_string(),
            dx: 0,
            dy: 0,
            auto_scale: false,
            snap: true,
            align_x: vec![],
            align_y: vec![],
            square: false,
        min_size: crate::widgets::MIN_FRAME_SIZE,
        max_size: 466,
        };

        // Przesunięcie: x=111 → 112 (7*16), y=139 → 144 (9*16).
        let moved = frame_geometry(&base);
        assert_eq!((moved.x, moved.y), (112, 144));

        // Bez przyciągania wartości zostają nietknięte.
        let free = frame_geometry(&FrameInput { snap: false, ..base.clone() });
        assert_eq!((free.x, free.y), (111, 139));

        // Zachód: lewa krawędź 111 → 112, prawa (385) zostaje na miejscu.
        let west = frame_geometry(&FrameInput {
            handle: "nw".to_string(),
            dx: -1,
            dy: 0,
            ..base.clone()
        });
        assert_eq!(west.x % SNAP_GRID, 0);
        assert_eq!(west.x + west.w, 111 + 274);

        // Wschód: prawa krawędź (385) → 384, lewa zostaje na miejscu.
        let east = frame_geometry(&FrameInput {
            handle: "se".to_string(),
            dx: -1,
            dy: 0,
            ..base.clone()
        });
        assert_eq!((east.x + east.w) % SNAP_GRID, 0);
        assert_eq!(east.x, 111);

        // Skala podąża za przyciągniętą szerokością, nie za surową (dx=60 → 209px).
        let scaled = frame_geometry(&FrameInput {
            handle: "se".to_string(),
            dx: -60,
            dy: 0,
            auto_scale: true,
            ..base
        });
        assert_eq!(scaled.w, 209);
        let expected = (64i64 * scaled.w as i64 + 274 / 2) / 274;
        assert_eq!(scaled.text_size, expected as u32);
        assert!(scaled.text_size < 64);
    }

    #[test]
    fn frame_geometry_aligns_to_canvas_and_other_widgets() {
        use widgets::{frame_geometry, FrameInput};

        let mut input = FrameInput {
            x: 100,
            y: 100,
            w: 100,
            h: 80,
            text_size: 32,
            handle: "move".to_string(),
            dx: 29,
            dy: -38,
            auto_scale: false,
            snap: false,
            align_x: vec![233], // środek płótna
            align_y: vec![64],  // góra innego widgetu
            square: false,
        min_size: crate::widgets::MIN_FRAME_SIZE,
        max_size: 466,
        };
        let aligned = frame_geometry(&input);
        // Prawa krawędź 229 → 233 (środek), góra 62 → 64.
        assert_eq!((aligned.x, aligned.y), (133, 64));
        assert_eq!(aligned.guide_x, Some(233));
        assert_eq!(aligned.guide_y, Some(64));

        // Poza zasięgiem prowadnicy nie ma wyrównania.
        input.align_x = vec![400];
        input.align_y = vec![400];
        let free = frame_geometry(&input);
        assert_eq!(free.guide_x, None);
        assert_eq!(free.guide_y, None);
        assert_eq!((free.x, free.y), (129, 62));

        // Wyrównanie wygrywa z siatką, gdy obie są blisko.
        input.snap = true;
        input.align_x = vec![233];
        input.align_y = vec![64];
        let both = frame_geometry(&input);
        assert_eq!(both.guide_x, Some(233));
        assert_eq!(both.guide_y, Some(64));
    }

    #[test]
    fn frame_geometry_aligns_widget_center_to_canvas_center_even_with_grid_on() {
        use widgets::{frame_geometry, FrameInput};

        // Szeroki widget: środek musi trafić w 233. Z włączoną siatką x=20
        // zostało by przyciągnięte do 16 (środek 226) i prowadnica by nie zadziałała.
        let input = FrameInput {
            x: 20,
            y: 100,
            w: 420,
            h: 100,
            text_size: 64,
            handle: "move".to_string(),
            dx: 0,
            dy: 0,
            auto_scale: false,
            snap: true,
            align_x: vec![233],
            align_y: vec![],
            square: false,
        min_size: crate::widgets::MIN_FRAME_SIZE,
        max_size: 466,
        };
        let aligned = frame_geometry(&input);
        assert_eq!(aligned.x, 23);
        assert_eq!(aligned.guide_x, Some(233));
    }

    #[test]
    fn frame_geometry_aligns_widget_center_to_other_widget_center_with_grid_on() {
        use widgets::{frame_geometry, FrameInput};

        // Środek innego widgetu w x=150 (nie jest to środek tarczy).
        let input = FrameInput {
            x: 105,
            y: 40,
            w: 100,
            h: 60,
            text_size: 32,
            handle: "move".to_string(),
            dx: 0,
            dy: 0,
            auto_scale: false,
            snap: true,
            align_x: vec![150],
            align_y: vec![],
            square: false,
        min_size: crate::widgets::MIN_FRAME_SIZE,
        max_size: 466,
        };
        let aligned = frame_geometry(&input);
        assert_eq!(aligned.x, 100);
        assert_eq!(aligned.guide_x, Some(150));
    }

    #[test]
    fn emulator_workspace_contains_installable_project() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rw-emulator-{unique}"));
        let source = root.join("source");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        let project = Project::starter("Emulator");
        let project_file = source.join("project.json");
        std::fs::write(&project_file, serde_json::to_vec_pretty(&project).unwrap()).unwrap();
        std::fs::write(source.join("assets/icon.png"), b"fake-png").unwrap();

        let workspace = prepare_emulator_workspace(&project_file, &root).unwrap();
        assert_eq!(workspace.app_dir, workspace.dir.join("app"));
        assert!(workspace.zip.is_file());
        assert!(workspace.app_dir.join("app.json").is_file());
        assert!(workspace.app_dir.join("app.js").is_file());
        assert!(workspace.app_dir.join("watchface/index.js").is_file());
        assert!(workspace
            .app_dir
            .join("assets/active2round/Preview.png")
            .is_file());
        let app_json = std::fs::read_to_string(workspace.app_dir.join("app.json")).unwrap();
        assert!(app_json.contains("8913153"));
        // Tylko kopia dla zeus dostaje v3 i `targets`; ZIP dla zegarka zostaje v2.
        assert!(app_json.contains("\"configVersion\": \"v3\""));
        assert!(app_json.contains("\"targets\""));
        // Log jest tworzony przez zeus, nie przez generator.
        assert!(!workspace.log.exists());

        // Ponowne przygotowanie czyści poprzedni katalog (brak narastających kopii).
        std::fs::write(workspace.app_dir.join("stale.txt"), b"old").unwrap();
        let again = prepare_emulator_workspace(&project_file, &root).unwrap();
        assert_eq!(again.app_dir, workspace.app_dir);
        assert!(!again.app_dir.join("stale.txt").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validator_catches_out_of_bounds_widget() {
        let mut project = Project::starter("Bad");
        project.widgets[0].x = 400;
        project.widgets[0].w = 200;
        let errors = validate(&project);
        assert!(errors.iter().any(|e| e.contains("wykracza")));
    }

    #[test]
    fn default_font_is_installed_attached_and_exported() {
        let root = std::env::temp_dir().join(format!("rusty-fonts-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets/Fonts");
        fonts::ensure_default(&root, &bundled).unwrap();

        let list = fonts::list(&root).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].is_default);
        assert_eq!(list[0].family, "Inter");

        let project_dir = root.join("project");
        let (dir, _) = create_project_in(&root, &NewProject {
            name: "project".into(),
            author: "Dev".into(),
            app_id: 42,
        }).unwrap();
        assert_eq!(dir, project_dir);
        let file = dir.join("project.json");
        let mut project = load_project(&file).unwrap();
        fonts::apply_default_font(&mut project, &dir, &root).unwrap();
        save_project(&file, &project).unwrap();

        assert_eq!(project.widgets[0].font.as_deref(), Some(fonts::DEFAULT_FONT_FILE));
        assert!(dir.join("assets/fonts/Inter-Regular.ttf").is_file());
        assert!(validate(&project).is_empty());

        let js = codegen::generate_index_js(&project);
        let flat_font = export_paths::exported_asset_name("fonts/Inter-Regular.ttf");
        // Jak w działającej referencji: TTF jest przypisany do widgetu.
        assert!(js.contains(&format!("font: \"{flat_font}\"")), "{js}");
        assert!(js.contains("char_space: 0"));
        assert!(js.contains("line_space: 0"));
        // Referencyjny bootstrap: moduł tarczy jest przypisany, a tło rysowane jako pierwsze.
        assert!(js.contains("__$$module$$__.module = DeviceRuntimeCore.WatchFace"));
        assert!(js.contains("hmUI.widget.FILL_RECT"));
        assert!(!js.contains("x: -1000"), "js: {js}");

        let output = root.join("out.zip");
        export_zip(&file, &output).unwrap();
        let archive = std::fs::File::open(&output).unwrap();
        let mut zip = ZipArchive::new(archive).unwrap();
        assert!(zip
            .by_name(&format!("assets/{flat_font}"))
            .is_ok());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn default_font_is_added_only_to_widgets_without_font() {
        let root = std::env::temp_dir().join(format!("rw-font-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets/Fonts");
        fonts::ensure_default(&root, &bundled).unwrap();
        let (dir, _) = create_project_in(&root, &NewProject {
            name: "FontFace".into(),
            author: "Dev".into(),
            app_id: 43,
        }).unwrap();
        let file = dir.join("project.json");
        let mut project = load_project(&file).unwrap();
        widgets::add_widget(&mut project, "date").unwrap();
        project.widgets[0].font = Some("Custom.ttf".to_string());
        save_project(&file, &project).unwrap();

        let mut project = load_project(&file).unwrap();
        fonts::apply_default_font_to_missing(&mut project, &dir, &root).unwrap();
        save_project(&file, &project).unwrap();

        assert_eq!(project.widgets[0].font.as_deref(), Some("Custom.ttf"));
        assert!(project.widgets[1..]
            .iter()
            .all(|w| w.font.as_deref() == Some(fonts::DEFAULT_FONT_FILE)));
        assert!(dir.join("assets/fonts/Inter-Regular.ttf").is_file());
        let js = codegen::generate_index_js(&project);
        let flat_font = export_paths::exported_asset_name("fonts/Inter-Regular.ttf");
        assert!(js.contains(&format!("font: \"{flat_font}\"")), "{js}");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn new_project_and_new_widgets_use_watch_system_font() {
        let root = std::env::temp_dir().join(format!("rw-sysfont-default-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (dir, _) = create_project_in(&root, &NewProject {
            name: "SysFont".into(),
            author: "Dev".into(),
            app_id: 44,
        }).unwrap();
        let file = dir.join("project.json");
        let mut project = load_project(&file).unwrap();
        assert!(project.widgets.iter().all(|w| w.font.is_none()));
        let added = widgets::add_widget(&mut project, "date").unwrap();
        assert!(added.font.is_none());

        let js = codegen::generate_index_js(&project);
        assert!(!js.contains("font:"), "system font must not emit a font property");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
