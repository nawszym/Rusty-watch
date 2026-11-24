use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::model::{
    Align, AssetRole, DataSource, Project, ShowLevel, Widget, WidgetKind, BACKGROUND_LAYER_ID,
    SCREEN_SIZE,
};

/// Czy projekt ma zasób tła, który można pokazać jako warstwę.
fn has_background(project: &Project) -> bool {
    project
        .assets
        .iter()
        .any(|asset| asset.role == AssetRole::Background)
}

/// Kolejność rysowania od spodu do góry. Domyślnie: tło, potem widgety wg `z`
/// (stabilne sortowanie zachowuje kolejność z pliku przy równym `z`).
/// Zapamiętana kolejność jest używana tylko wtedy, gdy opisuje istniejące warstwy.
pub fn order(project: &Project) -> Vec<String> {
    let mut valid: HashSet<String> = project.widgets.iter().map(|w| w.id.clone()).collect();
    if has_background(project) {
        valid.insert(BACKGROUND_LAYER_ID.to_string());
    }

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

    if has_background(project) && !result.iter().any(|id| id == BACKGROUND_LAYER_ID) {
        result.insert(0, BACKGROUND_LAYER_ID.to_string());
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
/// Tło jest zawsze najniższą warstwą i nie daje się przesunąć ani usunąć.
pub fn move_layer(project: &mut Project, id: &str, delta: i32) -> Result<Vec<String>> {
    if id == BACKGROUND_LAYER_ID {
        bail!("Tło jest zablokowane na najniższej warstwie");
    }
    let mut layers = normalize(project);
    let from = layers
        .iter()
        .position(|item| item == id)
        .with_context(|| format!("warstwa nie istnieje: {id}"))?;
    let to = from as i32 + delta;
    // Indeks 0 zarezerwowany dla tła — widget nie może zejść poniżej niego.
    let lowest = if has_background(project) { 1 } else { 0 };
    if to < lowest || to >= layers.len() as i32 {
        bail!("Nie można przesunąć warstwy dalej");
    }
    let layer = layers.remove(from);
    layers.insert(to as usize, layer);
    apply_order(project, &layers);
    Ok(layers)
}

/// Usuwa widget z projektu. Tło i ostatni widget są chronione przed usunięciem.
pub fn remove_widget(project: &mut Project, id: &str) -> Result<Vec<String>> {
    if id == BACKGROUND_LAYER_ID {
        bail!("Tło jest zablokowane na najniższej warstwie");
    }
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
    /// Tylko dla warstwy `background`: wybrany plik z magazynu obrazków.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Tylko dla warstwy `background`: czy `bg.png` jest automatycznie przycinany do 466×466.
    pub background_auto_scale: bool,
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
}

fn widget_kind(widget: &Widget) -> &'static str {
    if widget.kind == WidgetKind::AnalogTime {
        return "analog-time";
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
    let background_path = project
        .assets
        .iter()
        .find(|asset| asset.role == AssetRole::Background)
        .map(|asset| asset.path.clone());

    order(project)
        .into_iter()
        .filter_map(|id| {
            if id == BACKGROUND_LAYER_ID {
                return background_path.as_ref().map(|path| LayerView {
                    id: id.clone(),
                    name: "Background".to_string(),
                    kind: "background".to_string(),
                    x: 0,
                    y: 0,
                    w: SCREEN_SIZE as i32,
                    h: SCREEN_SIZE as i32,
                    text: None,
                    time_format: None,
                    text_size: None,
                    font: None,
                    color: None,
                    align: None,
                    v_align: "center".to_string(),
                    src: Some(path.clone()),
                    image_fit: crate::image_fit::FitMode::Cover.as_str().to_string(),
                    auto_scale: false,
                    text_style: "ellipsis".to_string(),
                    visible: true,
                    background_image: project.background.as_ref().map(|b| b.image.clone()),
                    background_auto_scale: project
                        .background
                        .as_ref()
                        .map(|b| b.auto_scale)
                        .unwrap_or(true),
                    measurement_system: None,
                    show_unit: true,
                    hand_set: None,
                    hour_src: None,
                    minute_src: None,
                    second_src: None,
                    cap_src: None,
                });
            }
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
                background_image: None,
                background_auto_scale: false,
                measurement_system: w.measurement_system.map(|system| system.as_str().to_string()),
                show_unit: w.show_unit,
                hand_set: w.hand_set.clone(),
                hour_src: w.hour_src.clone(),
                minute_src: w.minute_src.clone(),
                second_src: w.second_src.clone(),
                cap_src: w.cap_src.clone(),
            })
        })
        .collect()
}

/// Syntetyczny widget IMAGE dla zasobu tła — używany przez generator JS.
pub fn background_widget(project: &Project) -> Option<Widget> {
    let src = project
        .assets
        .iter()
        .find(|asset| asset.role == AssetRole::Background)?
        .path
        .clone();
    Some(Widget {
        id: BACKGROUND_LAYER_ID.to_string(),
        name: "Background".to_string(),
        kind: WidgetKind::Image,
        x: 0,
        y: 0,
        w: SCREEN_SIZE as i32,
        h: SCREEN_SIZE as i32,
        z: 0,
        text: None,
        text_size: None,
        font: None,
        time_format: None,
        color: None,
        src: Some(src),
        image_fit: crate::image_fit::FitMode::Cover,
        hand_set: None,
        hour_src: None,
        minute_src: None,
        second_src: None,
        cap_src: None,
        data_source: None,
        align: None,
        v_align: crate::model::VAlign::Center,
        show_level: ShowLevel::Always,
        auto_scale: false,
        text_style: crate::model::TextStyle::Ellipsis,
        visible: true,
        measurement_system: None,
        show_unit: true,
    })
}

/// Widgety w kolejności rysowania — jedyne źródło prawdy dla kodu JS.
/// Ukryte warstwy pozostają w projekcie (i w panelu Warstwy), ale nie trafiają
/// do wygenerowanej tarczy. Podgląd edytora pomija je na podstawie `LayerView::visible`.
pub fn ordered_widgets(project: &Project) -> Vec<Widget> {
    let background = background_widget(project);
    order(project)
        .into_iter()
        .filter_map(|id| {
            if id == BACKGROUND_LAYER_ID {
                background.clone()
            } else {
                project.widgets.iter().find(|w| w.id == id).cloned()
            }
        })
        .filter(|widget| widget.visible)
        .collect()
}

/// Zmienia widoczność widgetu. Tło jest zawsze widoczne i nie daje się ukryć.
pub fn set_widget_visibility(
    project: &mut Project,
    id: &str,
    visible: bool,
) -> Result<()> {
    if id == BACKGROUND_LAYER_ID {
        bail!("Tło jest zablokowane na najniższej warstwie");
    }
    let widget = project
        .widgets
        .iter_mut()
        .find(|widget| widget.id == id)
        .with_context(|| format!("warstwa nie istnieje: {id}"))?;
    widget.visible = visible;
    Ok(())
}
