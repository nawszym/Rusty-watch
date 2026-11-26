use std::collections::HashSet;

use crate::model::{DeviceId, Project, WidgetKind, SCREEN_SIZE};
use crate::time_format;

/// Walidacja projektu przed eksportem. Zwraca listę czytelnych błędów.
pub fn validate(project: &Project) -> Vec<String> {
    let mut errors = Vec::new();

    if project.metadata.name.trim().is_empty() {
        errors.push("Nazwa projektu nie może być pusta".to_string());
    }
    if project.metadata.author.trim().is_empty() {
        errors.push("Autor tarczy nie może być pusty".to_string());
    }
    if project.metadata.app_id == 0 {
        errors.push("appId musi być większe od 0".to_string());
    }
    if project.device != DeviceId::Active2Round {
        errors.push("Obsługiwane jest tylko urządzenie active-2-round".to_string());
    }
    if project.widgets.is_empty() {
        errors.push("Projekt nie zawiera żadnych widgetów".to_string());
    }

    let mut ids = HashSet::new();
    let asset_paths: HashSet<&str> = project.assets.iter().map(|a| a.path.as_str()).collect();

    for widget in &project.widgets {
        if widget.name.trim().is_empty() {
            errors.push(format!("Widget '{}' ma pustą nazwę", widget.id));
        }
        if !ids.insert(widget.id.as_str()) {
            errors.push(format!("Zduplikowany id widgetu: '{}'", widget.id));
        }
        if widget.x < 0 || widget.y < 0 {
            errors.push(format!("Widget '{}' ma ujemne współrzędne", widget.id));
        }
        let right = widget.x.saturating_add(widget.w);
        let bottom = widget.y.saturating_add(widget.h);
        if right > SCREEN_SIZE as i32 || bottom > SCREEN_SIZE as i32 {
            errors.push(format!(
                "Widget '{}' wykracza poza ekran 466x466 (prawy: {}, dolny: {})",
                widget.id, right, bottom
            ));
        }
        if widget.w <= 0 || widget.h <= 0 {
            errors.push(format!("Widget '{}' ma nieprawidłowy rozmiar", widget.id));
        }

        match widget.kind {
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
                if widget.text_size.unwrap_or(0) == 0 {
                    errors.push(format!("Widget tekstowy '{}' nie ma rozmiaru czcionki", widget.id));
                }
                if let Some(pattern) = &widget.time_format {
                    let parsed = if widget.kind == WidgetKind::DateText {
                        crate::date_format::parse(pattern).map(|_| ())
                    } else {
                        time_format::parse(pattern).map(|_| ())
                    };
                    if let Err(error) = parsed {
                        errors.push(format!(
                            "Widget '{}' ma nieprawidłowy wzorzec: {error}",
                            widget.id
                        ));
                    }
                }
                if let Some(color) = &widget.color {
                    if !valid_color(color) {
                        errors.push(format!("Widget '{}' ma nieprawidłowy kolor: {color}", widget.id));
                    }
                }
                if let Some(font) = &widget.font {
                    let path = crate::fonts::js_font_path(font);
                    if !asset_paths.contains(path.as_str()) {
                        errors.push(format!(
                            "Widget '{}' odwołuje się do niezainstalowanej czcionki: {font}",
                            widget.id
                        ));
                    }
                }
            }
            WidgetKind::ArcProgress | WidgetKind::BarProgress => {
                match &widget.progress {
                    None => errors.push(format!("Widget postępu '{}' nie ma ustawień", widget.id)),
                    Some(progress) => {
                        if progress.segments == 0 || progress.segments > 24 {
                            errors.push(format!("Widget postępu '{}' ma nieprawidłową liczbę segmentów", widget.id));
                        }
                        if progress.thickness == 0 || progress.thickness > 233 {
                            errors.push(format!("Widget postępu '{}' ma nieprawidłową grubość", widget.id));
                        }
                        if !valid_color(&progress.color) || !valid_color(&progress.track_color) {
                            errors.push(format!("Widget postępu '{}' ma nieprawidłowy kolor", widget.id));
                        }
                    }
                }
            }
            WidgetKind::Shortcut => {
                if widget.shortcut_target.is_none() {
                    errors.push(format!("Strefa klikalna '{}' nie ma celu", widget.id));
                }
            }
            WidgetKind::StatusIcon => {
                if widget.status_icon_set.is_none() || widget.status_type.is_none() {
                    errors.push(format!(
                        "Widget statusu '{}' nie ma przypisanej paczki ikon",
                        widget.id
                    ));
                }
                match &widget.src {
                    None => errors.push(format!("Widget statusu '{}' nie ma src", widget.id)),
                    Some(src) if !asset_paths.contains(src.as_str()) => errors.push(format!(
                        "Widget '{}' odwołuje się do nieistniejącej ikony statusu: {src}",
                        widget.id
                    )),
                    _ => {}
                }
            }
            WidgetKind::WeatherIcon => {
                if widget.weather_icon_set.is_none() {
                    errors.push(format!(
                        "Widget ikony pogody '{}' nie ma przypisanej paczki",
                        widget.id
                    ));
                }
                match &widget.src {
                    None => errors.push(format!("Widget ikony pogody '{}' nie ma src", widget.id)),
                    Some(src) if !asset_paths.contains(src.as_str()) => errors.push(format!(
                        "Widget '{}' odwołuje się do nieistniejącej ikony pogody: {src}",
                        widget.id
                    )),
                    _ => {}
                }
            }
            WidgetKind::Image | WidgetKind::Images => {
                match &widget.src {
                    None => errors.push(format!("Widget obrazka '{}' nie ma src", widget.id)),
                    Some(src) if !asset_paths.contains(src.as_str()) => errors.push(format!(
                        "Widget '{}' odwołuje się do nieistniejącego zasobu: {src}",
                        widget.id
                    )),
                    _ => {}
                }
            }
            WidgetKind::AnalogTime => {
                for src in [
                    &widget.hour_src,
                    &widget.minute_src,
                    &widget.second_src,
                    &widget.cap_src,
                ] {
                    if let Some(src) = src {
                        if !asset_paths.contains(src.as_str()) {
                            errors.push(format!(
                                "Widget '{}' odwołuje się do nieistniejącej wskazówki: {src}",
                                widget.id
                            ));
                        }
                    }
                }
            }
        }
    }

    for asset in &project.assets {
        if asset.path.is_empty() || asset.path.contains("..") || asset.path.starts_with('/') {
            errors.push(format!("Nieprawidłowa ścieżka zasobu: '{}'", asset.path));
        }
    }

    errors
}

fn valid_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("0x") else {
        return false;
    };
    hex.len() == 8 && hex.chars().all(|c| c.is_ascii_hexdigit())
}
