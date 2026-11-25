use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::model::{Align, DataSource, Project, Widget, WidgetKind};

/// Kolejność rysowania od spodu do góry (stabilne sortowanie zachowuje
/// kolejność z pliku przy równym `z`). Tło nie jest warstwą — jest stałym
/// `FILL_RECT` wyemitowanym przed wszystkimi widgetami, tak jak w tarczy
/// referencyjnej.
pub fn order(project: &Project) -> Vec<String> {
    let valid: HashSet<String> = project.widgets.iter().map(|w| w.id.clone()).collect();

    let mut result: Vec<String> = Vec::new();
    for id in &project.layer_order {
        if valid.contains(id) && !result.contains(id) {
            result.push(id.clone());
        }
    }

    let mut widgets: Vec<&Widget> = project.widgets.iter().collect();
    widgets.sort_by_key(|w| w.z);
    for widget in widgets {
        if !result.contains(&widget.id) {
            result.push(widget.id.clone());
        }
    }
    result
}

fn apply_order(project: &mut Project, layers: &[String]) {
    project.layer_order = layers.to_vec();
    for (z, id) in layers.iter().enumerate() {
        if let Some(widget) = project.widgets.iter_mut().find(|w| &w.id == id) {
            widget.z = z as u32;
        }
    }
}

/// Uzupełnia brakującą kolejność i synchronizuje `z` widgetów z kolejnością warstw.
pub fn normalize(project: &mut Project) -> Vec<String> {
    let layers = order(project);
    apply_order(project, &layers);
    layers
}

/// Przesuwa warstwę o `delta` pozycji: `+1` bliżej góry, `-1` bliżej spodu.
pub fn move_layer(project: &mut Project, id: &str, delta: i32) -> Result<Vec<String>> {
    let mut layers = normalize(project);
    let from = layers
        .iter()
        .position(|item| item == id)
        .with_context(|| format!("warstwa nie istnieje: {id}"))?;
    let to = from as i32 + delta;
    if to < 0 || to >= layers.len() as i32 {
        bail!("Nie można przesunąć warstwy dalej");
    }
    let layer = layers.remove(from);
    layers.insert(to as usize, layer);
    apply_order(project, &layers);
    Ok(layers)
}

/// Usuwa widget z projektu. Ostatni widget jest chroniony przed usunięciem.
pub fn remove_widget(project: &mut Project, id: &str) -> Result<Vec<String>> {
    let mut layers = normalize(project);
    let at = layers
        .iter()
        .position(|item| item == id)
        .with_context(|| format!("warstwa nie istnieje: {id}"))?;
    if project.widgets.len() <= 1 {
        bail!("Projekt musi zawierać przynajmniej jeden widget");
    }
    layers.remove(at);
    project.widgets.retain(|w| w.id != id);
    apply_order(project, &layers);
    Ok(layers)
}

/// Opis warstwy dla edytora — zawiera wszystko, czego podgląd potrzebuje do rysowania.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_size: Option<u32>,
    /// Nazwa pliku czcionki (bez ścieżki), `null` = czcionka systemowa.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// `top`, `center` albo `bottom`.
    pub v_align: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// Tylko dla widgetu `images`: `cover`, `contain` albo `stretch`.
    pub image_fit: String,
    /// Czy ramka jest w trybie skalowania tekstu (podwójne kliknięcie w edytorze).
    pub auto_scale: bool,
    /// Sposób dopasowania tekstu: `ellipsis`, `none`, `wrap`.
    pub text_style: String,
    /// Ukryta warstwa jest widoczna na liście, ale nie jest rysowana w podglądzie ani w JS.
    pub visible: bool,
    /// Tylko dla `distance`: własny system jednostek (`null` = domyślny z projektu).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measurement_system: Option<String>,
    /// Tylko dla widgetów zdrowotnych: czy pokazywać jednostkę (`bpm`, `°C`, `kcal`, `%`).
    pub show_unit: bool,
    /// Tylko dla `analog-time`: wybrany zestaw wskazówek i ścieżki jego plików.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hand_set: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hour_src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minute_src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_src: Option<String>,
    /// Tylko dla `status-*`: wybrana paczka z `%APPDATA%\RustyWatch\Status`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_icon_set: Option<String>,
    /// Tylko dla `status-*`: `alarm`, `dnd`, `bluetooth` albo `lock`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_type: Option<String>,
}

fn widget_kind(widget: &Widget) -> &'static str {
    if widget.kind == WidgetKind::AnalogTime {
        return "analog-time";
    }
    if widget.kind == WidgetKind::StatusIcon {
        return match widget.status_type {
            Some(crate::model::StatusType::Alarm) => "status-alarm",
            Some(crate::model::StatusType::Dnd) => "status-dnd",
            Some(crate::model::StatusType::Bluetooth) => "status-bluetooth",
            Some(crate::model::StatusType::Lock) => "status-lock",
            None => "status-alarm",
        };
    }
    match widget.data_source {
        Some(DataSource::Time) => "time",
        Some(DataSource::Date) => "date",
        Some(DataSource::WeekDayNumber) => "weekday-number",
        Some(DataSource::WeekNumber) => "week-number",
        Some(DataSource::DayOfYear) => "day-of-year",
        Some(DataSource::LunarDate) => "lunar-date",
        Some(DataSource::Festival) => "festival",
        Some(DataSource::WorldClock) => "world-clock",
        Some(DataSource::Step) => "steps",
        Some(DataSource::Distance) => "distance",
        Some(DataSource::HeartRate) => "heart",
        Some(DataSource::Stress) => "stress",
        Some(DataSource::Pai) => "pai",
        Some(DataSource::SkinTemp) => "skin-temp",
        Some(DataSource::Sleep) => "sleep",
        Some(DataSource::Calorie) => "calories",
        Some(DataSource::Spo2) => "spo2",
        Some(DataSource::Battery) => "battery",
        _ => match widget.kind {
            WidgetKind::Image => "image",
            WidgetKind::Images => "images",
            _ => "text",
        },
    }
}

fn align_name(align: Align) -> &'static str {
    match align {
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
    }
}

/// Warstwy w kolejności rysowania (od spodu) do wyświetlenia w panelu Layers.
pub fn describe(project: &Project) -> Vec<LayerView> {
    order(project)
        .into_iter()
        .filter_map(|id| {
            project.widgets.iter().find(|w| w.id == id).map(|w| LayerView {
                id: w.id.clone(),
                name: w.name.clone(),
                kind: widget_kind(w).to_string(),
                x: w.x,
                y: w.y,
                w: w.w,
                h: w.h,
                text: w.text.clone(),
                time_format: w.time_format.clone(),
                text_size: w.text_size,
                font: w.font.clone(),
                color: w.color.clone(),
                align: w.align.map(|a| align_name(a).to_string()),
                v_align: w.v_align.as_str().to_string(),
                src: w.src.clone(),
                image_fit: w.image_fit.as_str().to_string(),
                auto_scale: w.auto_scale,
                text_style: w.text_style.as_str().to_string(),
                visible: w.visible,
                measurement_system: w.measurement_system.map(|system| system.as_str().to_string()),
                show_unit: w.show_unit,
                hand_set: w.hand_set.clone(),
                hour_src: w.hour_src.clone(),
                minute_src: w.minute_src.clone(),
                second_src: w.second_src.clone(),
                cap_src: w.cap_src.clone(),
                status_icon_set: w.status_icon_set.clone(),
                status_type: w.status_type.map(|status| status.as_str().to_string()),
            })
        })
        .collect()
}

/// Widgety w kolejności rysowania — jedyne źródło prawdy dla kodu JS.
/// Ukryte warstwy pozostają w projekcie (i w panelu Warstwy), ale nie trafiają
/// do wygenerowanej tarczy. Podgląd edytora pomija je na podstawie `LayerView::visible`.
pub fn ordered_widgets(project: &Project) -> Vec<Widget> {
    order(project)
        .into_iter()
        .filter_map(|id| project.widgets.iter().find(|w| w.id == id).cloned())
        .filter(|widget| widget.visible)
        .collect()
}

/// Zmienia widoczność widgetu.
pub fn set_widget_visibility(
    project: &mut Project,
    id: &str,
    visible: bool,
) -> Result<()> {
    let widget = project
        .widgets
        .iter_mut()
        .find(|widget| widget.id == id)
        .with_context(|| format!("warstwa nie istnieje: {id}"))?;
    widget.visible = visible;
    Ok(())
}
