//! C ABI dla natywnego shella WinUI 3.
//! Wszystkie funkcje zwracają ciąg UTF-8 (JSON) alokowany przez Rust;
//! wywołujący musi zwolnić go przez `rw_free_string`.

use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

unsafe fn read_input(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

fn string_result<T>(result: std::result::Result<T, String>) -> Result<T> {
    result.map_err(anyhow::Error::msg)
}

fn respond(result: Result<Value>) -> *mut c_char {
    let payload = match result {
        Ok(data) => json!({ "ok": true, "data": data }),
        Err(error) => json!({ "ok": false, "error": error.to_string() }),
    };
    CString::new(payload.to_string())
        .expect("json cannot contain a raw NUL byte")
        .into_raw()
}

#[no_mangle]
pub extern "C" fn rw_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}

#[no_mangle]
pub extern "C" fn rw_version() -> *mut c_char {
    respond(Ok(json!({
        "name": "Rusty Watch",
        "version": env!("CARGO_PKG_VERSION"),
        "device": "active-2-round"
    })))
}

/// Generuje kod QR (PNG jako base64) dla lokalnego URL-a instalacji tarczy.
#[no_mangle]
pub extern "C" fn rw_generate_qr(text: *const c_char, size: u32) -> *mut c_char {
    use base64::Engine;

    let result = (|| -> Result<Value> {
        let text = unsafe { read_input(text) };
        if text.trim().is_empty() {
            anyhow::bail!("pusty tekst QR");
        }
        let png = rusty_core::qr::qr_png_bytes(&text, size)?;
        Ok(json!({
            "png": base64::engine::general_purpose::STANDARD.encode(png)
        }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_create_project(
    path: *const c_char,
    name: *const c_char,
    author: *const c_char,
    app_id: u32,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let path = unsafe { read_input(path) };
        let name = unsafe { read_input(name) };
        let author = unsafe { read_input(author) };
        let dir = Path::new(&path);
        let input = rusty_core::NewProject {
            name,
            author,
            app_id,
        };
        let project = rusty_core::create_project_with(dir, &input)?;
        Ok(json!({
            "file": rusty_core::project::project_file(dir).to_string_lossy(),
            "name": project.metadata.name,
            "author": project.metadata.author,
            "appId": project.metadata.app_id,
            "widgets": project.widgets.len()
        }))
    })();
    respond(result)
}

/// Tworzy tarczy w nowym folderze `<destination>/<nazwa tarczy>`.
#[no_mangle]
pub extern "C" fn rw_create_project_in(
    destination: *const c_char,
    name: *const c_char,
    author: *const c_char,
    app_id: u32,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let destination = unsafe { read_input(destination) };
        let name = unsafe { read_input(name) };
        let author = unsafe { read_input(author) };
        let input = rusty_core::NewProject {
            name,
            author,
            app_id,
        };
        let dir = Path::new(&destination);
        let (folder, project) = rusty_core::create_project_in(dir, &input)?;
        Ok(json!({
            "file": rusty_core::project::project_file(&folder).to_string_lossy(),
            "folder": folder.to_string_lossy(),
            "name": project.metadata.name,
            "author": project.metadata.author,
            "appId": project.metadata.app_id,
            "widgets": project.widgets.len()
        }))
    })();
    respond(result)
}

/// Save as... — nowy folder z pełną kopią projektu i zasobów.
#[no_mangle]
pub extern "C" fn rw_save_project_as(
    source: *const c_char,
    destination: *const c_char,
    name: *const c_char,
    author: *const c_char,
    app_id: u32,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let source = unsafe { read_input(source) };
        let destination = unsafe { read_input(destination) };
        let name = unsafe { read_input(name) };
        let author = unsafe { read_input(author) };
        let input = rusty_core::NewProject {
            name,
            author,
            app_id,
        };
        let (file, project) = rusty_core::save_project_as(
            Path::new(&source),
            Path::new(&destination),
            &input,
        )?;
        Ok(json!({
            "file": file.to_string_lossy(),
            "name": project.metadata.name,
            "author": project.metadata.author,
            "appId": project.metadata.app_id
        }))
    })();
    respond(result)
}

/// Losowy App ID dla okna tworzenia tarczy (unikalny w Zepp).
#[no_mangle]
pub extern "C" fn rw_random_app_id() -> *mut c_char {
    respond(Ok(json!({ "appId": rusty_core::random_app_id() })))
}

#[no_mangle]
pub extern "C" fn rw_load_project(path: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let path = unsafe { read_input(path) };
        let project = rusty_core::load_project(Path::new(&path))?;
        Ok(serde_json::to_value(project)?)
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_save_project(path: *const c_char, json_data: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let path = unsafe { read_input(path) };
        let raw = unsafe { read_input(json_data) };
        let project: rusty_core::Project =
            serde_json::from_str(&raw).context("invalid project JSON")?;
        let errors = rusty_core::validate(&project);
        if !errors.is_empty() {
            anyhow::bail!("{}", errors.join("; "));
        }
        rusty_core::save_project(Path::new(&path), &project)?;
        Ok(json!({ "path": path }))
    })();
    respond(result)
}

// --- Edytor: warstwy -------------------------------------------------------

/// Warstwy w kolejności rysowania (od spodu) — podgląd i panel Layers.
#[no_mangle]
pub extern "C" fn rw_editor_layers(project_file: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let project = rusty_core::load_project(Path::new(&project_file))?;
        Ok(json!(rusty_core::layers::describe(&project)))
    })();
    respond(result)
}

/// Przesuwa warstwę: delta +1 = wyżej (blisko góry), -1 = niżej.
#[no_mangle]
pub extern "C" fn rw_move_layer(
    project_file: *const c_char,
    id: *const c_char,
    delta: i32,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let id = unsafe { read_input(id) };
        let mut project = rusty_core::load_project(Path::new(&project_file))?;
        let layers = rusty_core::layers::move_layer(&mut project, &id, delta)?;
        rusty_core::save_project(Path::new(&project_file), &project)?;
        Ok(json!({
            "order": layers,
            "layers": rusty_core::layers::describe(&project)
        }))
    })();
    respond(result)
}

/// Typy widgetów obsługiwane przez edytor — katalog dla zakładki Widgets i flyouta.
#[no_mangle]
pub extern "C" fn rw_supported_widgets() -> *mut c_char {
    respond(Ok(json!(rusty_core::widgets::supported_widgets())))
}

/// Oblicza geometrię ramki w trakcie przeciągania (bez zapisu projektu).
/// `input` to JSON z polami x, y, w, h, textSize, handle, dx, dy, autoScale, snap,
/// `alignX`/`alignY` (kandydaci do wyrównania) i opcjonalnymi prowadnicami w odpowiedzi.
#[no_mangle]
pub extern "C" fn rw_frame_geometry(input: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let raw = unsafe { read_input(input) };
        let frame: rusty_core::widgets::FrameInput =
            serde_json::from_str(&raw).context("invalid frame JSON")?;
        Ok(json!(rusty_core::widgets::frame_geometry(&frame)))
    })();
    respond(result)
}

/// Dodaje widget z katalogu Widgets. Tło pozostaje najniższą warstwą.
#[no_mangle]
pub extern "C" fn rw_add_widget(
    project_file: *const c_char,
    kind: *const c_char,
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
    bundled_status_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let kind = unsafe { read_input(kind) };
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        let bundled_status = unsafe { read_input(bundled_status_dir) };
        let mut project = rusty_core::load_project(Path::new(&project_file))?;
        let widget = rusty_core::widgets::add_widget(&mut project, &kind)?;
        let dir = Path::new(&project_file)
            .parent()
            .context("project file has no parent")?;
        if widget.kind == rusty_core::model::WidgetKind::AnalogTime {
            string_result(rusty_core::hands::ensure_default(
                Path::new(&root),
                Path::new(&bundled),
            ))?;
            string_result(rusty_core::hands::attach_to_project(
                &mut project,
                dir,
                Path::new(&root),
                &widget.id,
                rusty_core::hands::DEFAULT_HAND_SET,
            ))?;
        }
        if widget.kind == rusty_core::model::WidgetKind::StatusIcon {
            string_result(rusty_core::status_icons::ensure_default(
                Path::new(&root),
                Path::new(&bundled_status),
            ))?;
            string_result(rusty_core::status_icons::attach_to_project(
                &mut project,
                dir,
                Path::new(&root),
                &widget.id,
                rusty_core::status_icons::DEFAULT_STATUS_PACK,
            ))?;
            string_result(rusty_core::status_icons::scale_project_icons(
                &project,
                dir,
                Path::new(&root),
                &widget.id,
            ))?;
        }
        if widget.kind == rusty_core::model::WidgetKind::WeatherIcon {
            string_result(rusty_core::weather::attach_to_project(
                &mut project,
                dir,
                Path::new(&root),
                &widget.id,
                rusty_core::weather::DEFAULT_WEATHER_PACK,
            ))?;
        }
        rusty_core::save_project(Path::new(&project_file), &project)?;
        Ok(json!({
            "id": widget.id,
            "layers": rusty_core::layers::describe(&project)
        }))
    })();
    respond(result)
}

/// Zapisuje ustawienia wybranego widgetu z inspektora edytora.
#[no_mangle]
pub extern "C" fn rw_set_widget(
    project_file: *const c_char,
    settings_json: *const c_char,
    app_data_root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let raw = unsafe { read_input(settings_json) };
        let app_data_root = unsafe { read_input(app_data_root) };
        let settings: Value = serde_json::from_str(&raw).context("invalid widget JSON")?;
        // Interaktywne zmiany w edytorze (przeciąganie ramki, suwaki) zapisują
        // tylko geometrię. Skalowanie wszech zasobów jest kolejkowane w tle przez
        // UI i wykonywane `rw_rescale_widget_assets` — inaczej 29 ikon pogody
        // blokowało wątek UI przy każdym ruchu myszy.
        let defer_assets = settings
            .get("deferAssets")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut project = rusty_core::load_project(Path::new(&project_file))?;
        let widget = rusty_core::widgets::update_widget(&mut project, &settings)?;
        let dir = Path::new(&project_file)
            .parent()
            .context("project file has no parent")?;
        // ZeppOS nie skaluje IMG w runtime — po zmianie rozmiaru ramki wskazówki
        // muszą mieć fizycznie ten sam rozmiar co pliki PNG w projekcie.
        if !defer_assets && widget.kind == rusty_core::model::WidgetKind::AnalogTime {
            string_result(rusty_core::hands::scale_project_hands(
                &project,
                dir,
                Path::new(&app_data_root),
                &widget.id,
            ))?;
        }
        if !defer_assets && widget.kind == rusty_core::model::WidgetKind::Images {
            string_result(rusty_core::images::scale_widget_image(
                &project,
                dir,
                Path::new(&app_data_root),
                &widget.id,
            ))?;
        }
        if !defer_assets && widget.kind == rusty_core::model::WidgetKind::StatusIcon {
            string_result(rusty_core::status_icons::scale_project_icons(
                &project,
                dir,
                Path::new(&app_data_root),
                &widget.id,
            ))?;
        }
        if !defer_assets && widget.kind == rusty_core::model::WidgetKind::WeatherIcon {
            // Czerwona ramka skaluje WSZYSTKIE 29 ikon, nie tylko widoczną.
            string_result(rusty_core::weather::scale_project_icons(
                &project,
                dir,
                Path::new(&app_data_root),
                &widget.id,
            ))?;
        }
        if !defer_assets {
            rusty_core::images::prune_unused(&mut project, dir);
            rusty_core::status_icons::prune_unused(&mut project, dir);
            rusty_core::weather::prune_unused(&mut project, dir);
        }
        if let Some(font) = &widget.font {
            string_result(rusty_core::fonts::attach_to_project(
                &mut project,
                dir,
                Path::new(&app_data_root),
                font,
            ))?;
        }
        string_result(rusty_core::fonts::prune_unused(&mut project, dir))?;
        rusty_core::save_project(Path::new(&project_file), &project)?;
        Ok(json!({
            "id": widget.id,
            "text": widget.text.clone().unwrap_or_default(),
            "layers": rusty_core::layers::describe(&project)
        }))
    })();
    respond(result)
}

/// Skaluje wszystkie zasoby graficzne jednego widgetu. Wywoływane z wątku tła —
/// nie ładuje ani nie zapisuje projektu, więc nie blokuje wątku UI.
#[no_mangle]
pub extern "C" fn rw_rescale_widget_assets(
    project_file: *const c_char,
    widget_id: *const c_char,
    app_data_root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let widget_id = unsafe { read_input(widget_id) };
        let app_data_root = unsafe { read_input(app_data_root) };
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let project = rusty_core::load_project(path)?;
        let widget = project
            .widgets
            .iter()
            .find(|w| w.id == widget_id)
            .context("warstwa nie istnieje")?;
        match widget.kind {
            rusty_core::model::WidgetKind::AnalogTime => {
                string_result(rusty_core::hands::scale_project_hands(
                    &project,
                    dir,
                    Path::new(&app_data_root),
                    &widget.id,
                ))?;
            }
            rusty_core::model::WidgetKind::Images => {
                string_result(rusty_core::images::scale_widget_image(
                    &project,
                    dir,
                    Path::new(&app_data_root),
                    &widget.id,
                ))?;
            }
            rusty_core::model::WidgetKind::StatusIcon => {
                string_result(rusty_core::status_icons::scale_project_icons(
                    &project,
                    dir,
                    Path::new(&app_data_root),
                    &widget.id,
                ))?;
            }
            rusty_core::model::WidgetKind::WeatherIcon => {
                string_result(rusty_core::weather::scale_project_icons(
                    &project,
                    dir,
                    Path::new(&app_data_root),
                    &widget.id,
                ))?;
            }
            _ => {}
        }
        Ok(json!({ "scaled": true, "widgetId": widget_id }))
    })();
    respond(result)
}

/// Tekst i ścieżka ikony dla podglądu pogody na żywo. Rust jest jedynym
/// źródłem logiki: domyślnie zwraca pochmurną Warszawę, brakujące pola
/// Custom Data są uzupełniane tym wzorcem. Ikona wskazuje plik z magazynu
/// (%APPDATA%), który UI skaluje lokalnie tylko dla aktualnie widocznej warstwy.
#[no_mangle]
pub extern "C" fn rw_format_weather_field(
    kind: *const c_char,
    overrides_json: *const c_char,
    locale: *const c_char,
    app_data_root: *const c_char,
    icon_set: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let kind = unsafe { read_input(kind) };
        let overrides_json = unsafe { read_input(overrides_json) };
        let locale_code = unsafe { read_input(locale) };
        let app_data_root = unsafe { read_input(app_data_root) };
        let icon_set = unsafe { read_input(icon_set) };
        let locale = rusty_core::locale::WatchfaceLocale::from_code(&locale_code);
        let overrides: Value = if overrides_json.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&overrides_json).context("invalid weather overrides JSON")?
        };
        let state = rusty_core::preview::WeatherPreviewState::from(
            rusty_core::preview::DEFAULT_PREVIEW_WEATHER,
        )
        .with_overrides(&overrides);
        let text = rusty_core::preview::weather_text(&kind, &state, locale);
        let icon_path = if kind == "weather-icon" {
            let set = if icon_set.trim().is_empty() {
                rusty_core::weather::DEFAULT_WEATHER_PACK
            } else {
                icon_set.trim()
            };
            let index = state.resolved_day().map(|day| day.index).unwrap_or(state.index);
            let path = rusty_core::weather::pack_dir(Path::new(&app_data_root), set)
                .join(format!("w{index:02}.png"));
            // Nigdy nie podmieniamy ikony na ikonę innej paczki — edytor pokazałby
            // niepasujący wzór pod wybraną nazwą. Brak pliku = pusty `iconPath`.
            if path.is_file() {
                path.to_string_lossy().to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        Ok(json!({ "text": text, "iconPath": icon_path }))
    })();
    respond(result)
}

/// Lista warunków pogodowych do rozwijanej listy w Custom Data. Nazwy pochodzą
/// z tej samej tabeli `locale.rs`, której używa podgląd i generowany JS — dzięki
/// temu etykieta w edytorze i tekst na tarczy zawsze się zgadzają.
#[no_mangle]
pub extern "C" fn rw_weather_conditions(locale: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let locale_code = unsafe { read_input(locale) };
        let locale = rusty_core::locale::WatchfaceLocale::from_code(&locale_code);
        let conditions = locale
            .weather_names()
            .iter()
            .enumerate()
            .map(|(code, name)| json!({ "code": code, "name": name }))
            .collect::<Vec<_>>();
        Ok(json!({ "conditions": conditions }))
    })();
    respond(result)
}

/// Predefiniowana data i czas podglądu edytora (Live Preview OFF). Jedyny
/// źródło tych wartości to `preview.rs`, więc edytor i eksportujący podgląd
/// pozostają spójne bez duplikowania stałych w C#.
#[no_mangle]
pub extern "C" fn rw_preview_default_datetime() -> *mut c_char {
    respond(Ok(json!(rusty_core::preview::DEFAULT_PREVIEW_DATETIME)))
}

/// Ustawia domyślny system jednostek projektu (ustawienie aplikacji).
#[no_mangle]
pub extern "C" fn rw_set_measurement_system(
    project_file: *const c_char,
    system: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let system = unsafe { read_input(system) };
        let parsed = rusty_core::model::MeasurementSystem::parse(&system)
            .ok_or_else(|| anyhow::anyhow!("Nieprawidłowy system jednostek: {system}"))?;
        let path = Path::new(&project_file);
        let mut project = rusty_core::load_project(path)?;
        rusty_core::project::set_measurement_system(&mut project, parsed);
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

/// Ustawia pierwszy dzień tygodnia w projekcie (ustawienie aplikacji).
#[no_mangle]
pub extern "C" fn rw_set_first_day_of_week(
    project_file: *const c_char,
    first_day: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let first_day = unsafe { read_input(first_day) };
        let parsed = rusty_core::model::FirstDayOfWeek::parse(&first_day)
            .ok_or_else(|| anyhow::anyhow!("Nieprawidłowy pierwszy dzień tygodnia: {first_day}"))?;
        let path = Path::new(&project_file);
        let mut project = rusty_core::load_project(path)?;
        rusty_core::project::set_first_day_of_week(&mut project, parsed);
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

/// Pokazuje lub ukrywa warstwę w podglądzie i w eksportowanej tarczy.
#[no_mangle]
pub extern "C" fn rw_set_widget_visibility(
    project_file: *const c_char,
    id: *const c_char,
    visible: bool,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let id = unsafe { read_input(id) };
        let mut project = rusty_core::load_project(Path::new(&project_file))?;
        rusty_core::layers::set_widget_visibility(&mut project, &id, visible)?;
        rusty_core::save_project(Path::new(&project_file), &project)?;
        Ok(json!({
            "id": id,
            "visible": visible,
            "layers": rusty_core::layers::describe(&project)
        }))
    })();
    respond(result)
}

// --- Pierwsze uruchomienie: pełna struktura katalogu danych -----------------

/// Kopiuje wbudowane zasoby do `%APPDATA%\RustyWatch` przy starcie aplikacji.
/// Katalogi języków są seedowane po stronie C# (pliki JSON czytane w runtime).
#[no_mangle]
pub extern "C" fn rw_ensure_defaults(
    app_data_root: *const c_char,
    bundled_fonts_dir: *const c_char,
    bundled_hands_dir: *const c_char,
    bundled_status_dir: *const c_char,
    bundled_weather_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let bundled_fonts = unsafe { read_input(bundled_fonts_dir) };
        let bundled_hands = unsafe { read_input(bundled_hands_dir) };
        let bundled_status = unsafe { read_input(bundled_status_dir) };
        let bundled_weather = unsafe { read_input(bundled_weather_dir) };
        string_result(rusty_core::fonts::ensure_default(
            Path::new(&root),
            Path::new(&bundled_fonts),
        ))?;
        string_result(rusty_core::hands::ensure_default(
            Path::new(&root),
            Path::new(&bundled_hands),
        ))?;
        string_result(rusty_core::status_icons::ensure_default(
            Path::new(&root),
            Path::new(&bundled_status),
        ))?;
        string_result(rusty_core::weather::ensure_default(
            Path::new(&root),
            Path::new(&bundled_weather),
        ))?;
        Ok(json!({ "seeded": true }))
    })();
    respond(result)
}

// --- Zasoby: czcionki (wspólny magazyn w %APPDATA%\RustyWatch\Fonts) --------

/// Lista zainstalowanych czcionek. `bundledDir` to katalog `Assets/Fonts` przy .exe.
#[no_mangle]
pub extern "C" fn rw_fonts_list(
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        let bundled = Path::new(&bundled);
        string_result(rusty_core::fonts::ensure_default(Path::new(&root), bundled))?;
        let fonts = string_result(rusty_core::fonts::list(Path::new(&root)))?;
        Ok(json!({ "fonts": fonts }))
    })();
    respond(result)
}

/// Dodaje czcionkę z dowolnej ścieżki do magazynu aplikacji.
#[no_mangle]
pub extern "C" fn rw_fonts_add(
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
    source_path: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        let source = unsafe { read_input(source_path) };
        string_result(rusty_core::fonts::ensure_default(Path::new(&root), Path::new(&bundled)))?;
        let font = string_result(rusty_core::fonts::add(Path::new(&root), Path::new(&source)))?;
        Ok(json!({ "font": font }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_fonts_remove(
    app_data_root: *const c_char,
    file: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let file = unsafe { read_input(file) };
        string_result(rusty_core::fonts::remove(Path::new(&root), &file))?;
        Ok(json!({ "removed": file }))
    })();
    respond(result)
}

/// Ustawia domyślną czcionkę na wszystkich widgetach nowo utworzonej tarczy.
#[no_mangle]
pub extern "C" fn rw_apply_default_font(
    project_file: *const c_char,
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        string_result(rusty_core::fonts::ensure_default(Path::new(&root), Path::new(&bundled)))?;
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        string_result(rusty_core::fonts::apply_default_font(&mut project, dir, Path::new(&root)))?;
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

/// Uzupełnia domyślną czcionkę tylko w widgetach, które nie mają jeszcze żadnej.
/// Wywoływane przy otwarciu projektu i po dodaniu widgetu, żeby podgląd (TTF z
/// `assets/fonts`) i wygenerowana tarcza używały identycznych metryk tekstu.
#[no_mangle]
pub extern "C" fn rw_apply_default_font_to_missing(
    project_file: *const c_char,
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        string_result(rusty_core::fonts::ensure_default(Path::new(&root), Path::new(&bundled)))?;
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        string_result(rusty_core::fonts::apply_default_font_to_missing(
            &mut project,
            dir,
            Path::new(&root),
        ))?;
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

// --- Zasoby: obrazki IMAGES (%APPDATA%\RustyWatch\Images) -------------------

#[no_mangle]
pub extern "C" fn rw_images_list(app_data_root: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let images = string_result(rusty_core::images::list(Path::new(&root)))?;
        Ok(json!({ "images": images }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_images_add(
    app_data_root: *const c_char,
    source_path: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let source = unsafe { read_input(source_path) };
        let image = string_result(rusty_core::images::add(Path::new(&root), Path::new(&source)))?;
        Ok(json!({ "image": image }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_images_remove(
    app_data_root: *const c_char,
    file: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let file = unsafe { read_input(file) };
        string_result(rusty_core::images::remove(Path::new(&root), &file))?;
        Ok(json!({ "removed": file }))
    })();
    respond(result)
}

/// Podłącza obrazek z magazynu IMAGES do widgetu IMAGES i przygotowuje PNG do ramki.
#[no_mangle]
pub extern "C" fn rw_set_widget_image(
    project_file: *const c_char,
    widget_id: *const c_char,
    file: *const c_char,
    fit: *const c_char,
    app_data_root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let widget_id = unsafe { read_input(widget_id) };
        let file = unsafe { read_input(file) };
        let fit = unsafe { read_input(fit) };
        let root = unsafe { read_input(app_data_root) };
        let fit = rusty_core::image_fit::FitMode::parse(&fit)
            .ok_or_else(|| anyhow::anyhow!("Nieprawidłowy tryb skalowania obrazka: {fit}"))?;
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        let asset_path = string_result(rusty_core::images::set_widget_image(
            &mut project,
            dir,
            Path::new(&root),
            &widget_id,
            &file,
            fit,
        ))?;
        rusty_core::images::prune_unused(&mut project, dir);
        rusty_core::save_project(path, &project)?;
        Ok(json!({
            "path": asset_path,
            "layers": rusty_core::layers::describe(&project)
        }))
    })();
    respond(result)
}

// --- Zasoby: zestawy wskazówek (%APPDATA%\RustyWatch\Hands) -----------------

/// Lista zestawów wskazówek. `bundledDir` to katalog `Assets/Hands` przy .exe.
#[no_mangle]
pub extern "C" fn rw_hands_list(
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        string_result(rusty_core::hands::ensure_default(
            Path::new(&root),
            Path::new(&bundled),
        ))?;
        let hands = string_result(rusty_core::hands::list(Path::new(&root)))?;
        Ok(json!({ "hands": hands }))
    })();
    respond(result)
}

/// Dodaje zestaw wskazówek z wybranego folderu.
#[no_mangle]
pub extern "C" fn rw_hands_add(
    app_data_root: *const c_char,
    folder: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let folder = unsafe { read_input(folder) };
        let hand = string_result(rusty_core::hands::add(
            Path::new(&root),
            Path::new(&folder),
        ))?;
        Ok(json!({ "hand": hand }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_hands_remove(
    app_data_root: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let id = unsafe { read_input(id) };
        string_result(rusty_core::hands::remove(Path::new(&root), &id))?;
        Ok(json!({ "removed": id }))
    })();
    respond(result)
}

/// Przypisuje zestaw wskazówek do wybranego widgetu analogowego (zapisuje projekt).
#[no_mangle]
pub extern "C" fn rw_set_hands(
    project_file: *const c_char,
    widget_id: *const c_char,
    set_id: *const c_char,
    app_data_root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let widget_id = unsafe { read_input(widget_id) };
        let set_id = unsafe { read_input(set_id) };
        let root = unsafe { read_input(app_data_root) };
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        string_result(rusty_core::hands::attach_to_project(
            &mut project,
            dir,
            Path::new(&root),
            &widget_id,
            &set_id,
        ))?;
        // Podmiana zestawu zachowuje ramkę widgetu — nowe pliki muszą mieć
        // fizycznie jej rozmiar, inaczej ZeppOS pokaże tylko środek obrazu.
        string_result(rusty_core::hands::scale_project_hands(
            &project,
            dir,
            Path::new(&root),
            &widget_id,
        ))?;
        rusty_core::hands::prune_unused(&mut project, dir);
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

/// Włącza/wyłącza wskazówki analogowego widgetu.
/// `visibilityJson` przyjmuje `{"hour": true, "minute": true, "second": false, "cap": true}`.
#[no_mangle]
pub extern "C" fn rw_set_hand_visibility(
    project_file: *const c_char,
    widget_id: *const c_char,
    visibility_json: *const c_char,
    app_data_root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let widget_id = unsafe { read_input(widget_id) };
        let raw = unsafe { read_input(visibility_json) };
        let root = unsafe { read_input(app_data_root) };
        let settings: Value = serde_json::from_str(&raw).context("invalid hand visibility JSON")?;
        let hour = settings.get("hour").and_then(Value::as_bool);
        let minute = settings.get("minute").and_then(Value::as_bool);
        let second = settings.get("second").and_then(Value::as_bool);
        let cap = settings.get("cap").and_then(Value::as_bool);

        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        string_result(rusty_core::hands::set_hand_visibility(
            &mut project,
            dir,
            Path::new(&root),
            &widget_id,
            hour,
            minute,
            second,
            cap,
        ))?;
        // Włączenie wskazówki kopiuje oryginał z magazynu — dopasuj go do
        // aktualnego rozmiaru ramki, żeby znów nie został przycięty na tarczy.
        string_result(rusty_core::hands::scale_project_hands(
            &project,
            dir,
            Path::new(&root),
            &widget_id,
        ))?;
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

// --- Zasoby: pakiety ikon STATUS (%APPDATA%\RustyWatch\Status) --------------

/// Lista paczek ikon statusu. Katalog jest skanowany dynamicznie.
#[no_mangle]
pub extern "C" fn rw_status_packs_list(
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        string_result(rusty_core::status_icons::ensure_default(
            Path::new(&root),
            Path::new(&bundled),
        ))?;
        let packs = string_result(rusty_core::status_icons::list(Path::new(&root)))?;
        Ok(json!({ "packs": packs }))
    })();
    respond(result)
}

/// Dodaje paczkę z wybranego folderu (wymagane 4 pliki PNG).
#[no_mangle]
pub extern "C" fn rw_status_packs_add(
    app_data_root: *const c_char,
    folder: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let folder = unsafe { read_input(folder) };
        let pack = string_result(rusty_core::status_icons::add(
            Path::new(&root),
            Path::new(&folder),
        ))?;
        Ok(json!({ "pack": pack }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_status_packs_remove(
    app_data_root: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let id = unsafe { read_input(id) };
        string_result(rusty_core::status_icons::remove(Path::new(&root), &id))?;
        Ok(json!({ "removed": id }))
    })();
    respond(result)
}

/// Przypisuje paczkę do widgetu statusu (zapisuje projekt i skaluje ikony).
#[no_mangle]
pub extern "C" fn rw_set_status_pack(
    project_file: *const c_char,
    widget_id: *const c_char,
    set_id: *const c_char,
    app_data_root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let widget_id = unsafe { read_input(widget_id) };
        let set_id = unsafe { read_input(set_id) };
        let root = unsafe { read_input(app_data_root) };
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        string_result(rusty_core::status_icons::attach_to_project(
            &mut project,
            dir,
            Path::new(&root),
            &widget_id,
            &set_id,
        ))?;
        string_result(rusty_core::status_icons::scale_project_icons(
            &project,
            dir,
            Path::new(&root),
            &widget_id,
        ))?;
        rusty_core::status_icons::prune_unused(&mut project, dir);
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

/// Lista zainstalowanych paczek ikon pogody (system-like, Not Something i paczki użytkownika).
#[no_mangle]
pub extern "C" fn rw_weather_packs_list(
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        string_result(rusty_core::weather::ensure_default(
            Path::new(&root),
            Path::new(&bundled),
        ))?;
        let packs = string_result(rusty_core::weather::list(Path::new(&root)))?;
        Ok(json!({ "packs": packs }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_weather_packs_add(
    app_data_root: *const c_char,
    folder: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let folder = unsafe { read_input(folder) };
        let pack = string_result(rusty_core::weather::add(
            Path::new(&root),
            Path::new(&folder),
        ))?;
        Ok(json!({ "pack": pack }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_weather_packs_remove(
    app_data_root: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let root = unsafe { read_input(app_data_root) };
        let id = unsafe { read_input(id) };
        string_result(rusty_core::weather::remove(Path::new(&root), &id))?;
        Ok(json!({ "removed": id }))
    })();
    respond(result)
}

/// Przypisuje paczkę do widgetu ikony pogody. `attach_to_project` kopiuje i
/// skaluje wszystkie 29 ikon do aktualnej ramki — osobne `scale_project_icons`
/// byłoby drugim, identycznym przetwarzaniem całej paczki. Ta funkcja jest
/// wywoływana w wątku tła; podgląd przełącza się natychmiast z magazynu.
#[no_mangle]
pub extern "C" fn rw_set_weather_pack(
    project_file: *const c_char,
    widget_id: *const c_char,
    set_id: *const c_char,
    app_data_root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let widget_id = unsafe { read_input(widget_id) };
        let set_id = unsafe { read_input(set_id) };
        let root = unsafe { read_input(app_data_root) };
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        string_result(rusty_core::weather::attach_to_project(
            &mut project,
            dir,
            Path::new(&root),
            &widget_id,
            &set_id,
        ))?;
        rusty_core::weather::prune_unused(&mut project, dir);
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

/// Domyślnie przypisuje `Classic` widgetom statusu w starszych projektach.
#[no_mangle]
pub extern "C" fn rw_attach_status_pack_to_missing(
    project_file: *const c_char,
    app_data_root: *const c_char,
    bundled_dir: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let root = unsafe { read_input(app_data_root) };
        let bundled = unsafe { read_input(bundled_dir) };
        string_result(rusty_core::status_icons::ensure_default(
            Path::new(&root),
            Path::new(&bundled),
        ))?;
        let path = Path::new(&project_file);
        let dir = path.parent().context("project file has no parent")?;
        let mut project = rusty_core::load_project(path)?;
        string_result(rusty_core::status_icons::attach_default_to_missing(
            &mut project,
            dir,
            Path::new(&root),
        ))?;
        // Magazyn mógł zostać odświeżony przez `ensure_default` — kopie w starych
        // projektach również muszą dostać nowe pliki, zanim je przeskalujemy.
        string_result(rusty_core::status_icons::refresh_attached(
            Path::new(&root),
            dir,
            &mut project,
        ))?;
        for widget in project
            .widgets
            .iter()
            .filter(|w| w.kind == rusty_core::model::WidgetKind::StatusIcon)
        {
            string_result(rusty_core::status_icons::scale_project_icons(
                &project,
                dir,
                Path::new(&root),
                &widget.id,
            ))?;
        }
        rusty_core::status_icons::prune_unused(&mut project, dir);
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "layers": rusty_core::layers::describe(&project) }))
    })();
    respond(result)
}

/// Usuwa widget — dostępne z poziomu panelu Warstwy. Tła nie da się usunąć.
#[no_mangle]
pub extern "C" fn rw_remove_widget(
    project_file: *const c_char,
    id: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let id = unsafe { read_input(id) };
        let mut project = rusty_core::load_project(Path::new(&project_file))?;
        let layers = rusty_core::layers::remove_widget(&mut project, &id)?;
        let dir = Path::new(&project_file)
            .parent()
            .context("project file has no parent")?;
        // Usunięty widget nie może zostawić osieroconych plików w ZIP-ie.
        let _ = rusty_core::fonts::prune_unused(&mut project, dir);
        rusty_core::hands::prune_unused(&mut project, dir);
        rusty_core::images::prune_unused(&mut project, dir);
        rusty_core::save_project(Path::new(&project_file), &project)?;
        Ok(json!({
            "order": layers,
            "layers": rusty_core::layers::describe(&project)
        }))
    })();
    respond(result)
}

/// Aktualizuje autora i wersję tarczy — nazwa pozostaje bez zmian.
#[no_mangle]
pub extern "C" fn rw_set_project_metadata(
    project_file: *const c_char,
    author: *const c_char,
    version: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let author = unsafe { read_input(author) };
        let version = unsafe { read_input(version) };
        let project =
            rusty_core::update_project_metadata(Path::new(&project_file), &author, &version)?;
        Ok(json!({
            "saved": true,
            "name": project.metadata.name,
            "author": project.metadata.author,
            "version": project.metadata.version,
        }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_validate_project(path: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let path = unsafe { read_input(path) };
        let project = rusty_core::load_project(Path::new(&path))?;
        Ok(json!(rusty_core::validate(&project)))
    })();
    respond(result)
}

/// Losuje całą serię danych dla 8 klatek podglądu (7 oryginalnych + kopia
/// 21:37:40). Jedno wywołanie na eksport — edytor przekazuje zwrócony JSON do
/// każdej klatki, więc dane są spójne.
#[no_mangle]
pub extern "C" fn rw_preview_random_series(locale: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let locale = unsafe { read_input(locale) };
        let locale = if locale.trim().is_empty() {
            "en"
        } else {
            locale.trim()
        };
        let series = rusty_core::preview::random_series(rusty_core::preview::random_seed(), locale);
        Ok(serde_json::to_value(series)?)
    })();
    respond(result)
}

/// Rysuje jedną klatkę do `<project>/preview-cache/frame-NN.png`.
#[no_mangle]
pub extern "C" fn rw_render_preview_frame(
    project: *const c_char,
    index: u32,
    series_json: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project = unsafe { read_input(project) };
        let series_json = unsafe { read_input(series_json) };
        let series: rusty_core::preview::PreviewRandomSeries = serde_json::from_str(&series_json)?;
        let path = Path::new(&project);
        let dir = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Nieprawidłowa ścieżka projektu"))?;
        let loaded = rusty_core::load_project(path)?;
        let index = index as usize;
        let frame = string_result(rusty_core::preview::frame_data(index, &series))?;
        let image = rusty_core::preview::render_frame(dir, &loaded, &frame, &series);
        let cache = dir.join("preview-cache");
        fs::create_dir_all(&cache)?;
        let output = cache.join(format!("frame-{index:02}.png"));
        string_result(rusty_core::preview::write_png(&image, &output))?;
        Ok(json!({ "index": index, "frame": output.to_string_lossy() }))
    })();
    respond(result)
}

/// Składa `preview.gif` (8 klatek, zapętlenie, ~1,7 s na klatkę) i oprawiony
/// statyczny `preview.png`.
#[no_mangle]
pub extern "C" fn rw_encode_preview_gif(
    project: *const c_char,
    series_json: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project = unsafe { read_input(project) };
        let _series_json = unsafe { read_input(series_json) };
        let path = Path::new(&project);
        let dir = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Nieprawidłowa ścieżka projektu"))?;
        let cache = dir.join("preview-cache");
        let mut frames = Vec::with_capacity(rusty_core::preview::FRAME_COUNT);
        for index in 0..rusty_core::preview::FRAME_COUNT {
            let file = cache.join(format!("frame-{index:02}.png"));
            let image = image::open(&file)
                .with_context(|| format!("Brak klatki podglądu: {}", file.display()))?;
            frames.push(image.to_rgba8());
        }
        let gif_path = dir.join("preview.gif");
        let png_path = dir.join("preview.png");
        let model = rusty_core::load_project(Path::new(&project))?;

        // Statyczny PNG korzysta z ostatniej klatki (dokładna kopia 21:37:40),
        // ale zamiast surowej klatki zapisujemy oprawiony podgląd: tarcza
        // pomniejszona w środku, strefy bezpieczne i zewnętrzna warstwa logo.
        // Ten sam plik jest później skalowany do ikony w ZIP-ie.
        let reference = rusty_core::preview::PREVIEW_FRAME_INDEX;
        let branded = rusty_core::preview::compose_branded_preview(
            &frames[reference],
            rusty_core::model::SCREEN_SIZE,
        );
        string_result(rusty_core::preview::write_png(&branded, &png_path))?;

        // Oprawiony podgląd jest PIERWSZĄ klatką GIF-a: surowa kopia 21:37
        // zostaje usunięta, a jej miejsce w animacji zajmuje ten sam obraz co
        // preview.png. GIF ma nadal 8 klatek.
        frames.pop();
        frames.insert(0, branded);
        // Widoczne strefy klikania idą na KOLEJNĄ klatkę (pierwszą surową),
        // nie na oprawiony podgląd, który jest klatką pierwszą.
        if let Some(frame) = frames.get_mut(1) {
            rusty_core::preview::draw_shortcut_overlays(frame, &model);
        }
        string_result(rusty_core::preview::write_gif(&frames, &gif_path))?;
        let _ = fs::remove_dir_all(&cache);
        Ok(json!({
            "gif": gif_path.to_string_lossy(),
            "png": png_path.to_string_lossy(),
            "frames": frames.len(),
            "referenceFrame": 1,
        }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_export_zip(project: *const c_char, output: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project = unsafe { read_input(project) };
        let output = unsafe { read_input(output) };
        let zip = rusty_core::export_zip(Path::new(&project), Path::new(&output))?;
        Ok(json!({ "zip": zip.to_string_lossy() }))
    })();
    respond(result)
}

/// Przygotowuje katalog roboczy z rozpakowanym eksportem dla `zeus install`/`zeus dev`.
#[no_mangle]
pub extern "C" fn rw_prepare_emulator_workspace(
    project: *const c_char,
    root: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project = unsafe { read_input(project) };
        let root = unsafe { read_input(root) };
        let workspace = rusty_core::prepare_emulator_workspace(Path::new(&project), Path::new(&root))?;
        Ok(json!({
            "dir": workspace.dir.to_string_lossy(),
            "appDir": workspace.app_dir.to_string_lossy(),
            "zip": workspace.zip.to_string_lossy(),
            "log": workspace.log.to_string_lossy(),
        }))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_inspect_zip(path: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let path = unsafe { read_input(path) };
        let text = rusty_core::inspect_zip(Path::new(&path))?;
        Ok(json!({ "text": text }))
    })();
    respond(result)
}

// --- Lista ostatnio otwartych projektów (strona HOME) -----------------------

#[no_mangle]
pub extern "C" fn rw_recent_list(store: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let store = unsafe { read_input(store) };
        Ok(json!(rusty_core::recent::load(Path::new(&store))?))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_recent_add(store: *const c_char, project_file: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let store = unsafe { read_input(store) };
        let project_file = unsafe { read_input(project_file) };
        let path = Path::new(&project_file);
        let project = rusty_core::load_project(path)?;
        let items = rusty_core::recent::record(Path::new(&store), path, &project.metadata.name)?;
        Ok(json!(items))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_recent_remove(store: *const c_char, project_file: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let store = unsafe { read_input(store) };
        let project_file = unsafe { read_input(project_file) };
        Ok(json!(rusty_core::recent::remove(
            Path::new(&store),
            Path::new(&project_file)
        )?))
    })();
    respond(result)
}

#[no_mangle]
pub extern "C" fn rw_recent_clear(store: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let store = unsafe { read_input(store) };
        Ok(json!(rusty_core::recent::clear(Path::new(&store))?))
    })();
    respond(result)
}

/// Wczytuje projekt i od razu dopisuje go do listy ostatnich — jedna operacja dla HOME.
#[no_mangle]
pub extern "C" fn rw_open_project(store: *const c_char, project_file: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let store = unsafe { read_input(store) };
        let project_file = unsafe { read_input(project_file) };
        let path = Path::new(&project_file);
        let project = rusty_core::load_project(path)?;
        // Lista ostatnich to tylko wygoda — uszkodzony lub niezapisalny plik nie może
        // zablokować otwarcia samego projektu.
        let store = Path::new(&store);
        let items = rusty_core::recent::record(store, path, &project.metadata.name)
            .unwrap_or_else(|_| rusty_core::recent::load(store).unwrap_or_default());
        Ok(json!({
            "project": serde_json::to_value(project)?,
            "recent": items
        }))
    })();
    respond(result)
}

/// Nazwa miesiąca (0..11) w języku podglądu tarczy — widget daty w edytorze.
#[no_mangle]
pub extern "C" fn rw_month_name(index: i32, locale: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let locale_code = unsafe { read_input(locale) };
        let locale = rusty_core::locale::WatchfaceLocale::from_code(&locale_code);
        let name = locale
            .month_name(index.max(0) as usize)
            .context("month index out of range")?;
        Ok(json!({ "name": name }))
    })();
    respond(result)
}

// --- Edytor: wyświetlacz czasu ---------------------------------------------

fn required_string(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("missing or invalid '{key}'"))
}

fn required_i64(value: &Value, key: &str) -> Result<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .with_context(|| format!("missing or invalid '{key}'"))
}

/// Formatuje czas według wzorca — podgląd w edytorze używa tej samej logiki co generator JS.
#[no_mangle]
pub extern "C" fn rw_format_time(
    pattern: *const c_char,
    hour: i32,
    minute: i32,
    second: i32,
    locale: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let pattern = unsafe { read_input(pattern) };
        let locale_code = unsafe { read_input(locale) };
        let locale = rusty_core::locale::WatchfaceLocale::from_code(&locale_code);
        let text = rusty_core::time_format::format_with_locale(
            &pattern,
            hour.max(0) as u32,
            minute.max(0) as u32,
            second.max(0) as u32,
            locale,
        )?;
        Ok(json!({ "text": text }))
    })();
    respond(result)
}

/// Formatuje datę według wzorca — podgląd edytora używa tej samej logiki co generator JS.
/// `month` jest 0-based (zgodnie z `Date.getMonth()`).
#[no_mangle]
pub extern "C" fn rw_format_date(
    pattern: *const c_char,
    day: i32,
    month: i32,
    year: i32,
    locale: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let pattern = unsafe { read_input(pattern) };
        let locale_code = unsafe { read_input(locale) };
        let locale = rusty_core::locale::WatchfaceLocale::from_code(&locale_code);
        let text = rusty_core::date_format::format_with_locale(
            &pattern,
            day.max(0) as u32,
            month.max(0) as u32,
            year,
            locale,
        )?;
        Ok(json!({ "text": text }))
    })();
    respond(result)
}

/// Podgląd pól daty, które na zegarku pochodzą z `hmSensor.id.TIME` lub są
/// liczone w JS. Rust wykonuje ten sam algorytm co generator, więc podgląd i
/// eksport pozostają 1:1. Dane czujnika na PC nie istnieją — zwracamy `--`.
#[no_mangle]
pub extern "C" fn rw_format_date_field(
    kind: *const c_char,
    day: i32,
    month: i32,
    year: i32,
    first_day: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let kind = unsafe { read_input(kind) };
        let first_day = unsafe { read_input(first_day) };
        let first_day = rusty_core::model::FirstDayOfWeek::parse(&first_day)
            .ok_or_else(|| anyhow::anyhow!("Nieprawidłowy pierwszy dzień tygodnia: {first_day}"))?;
        let day = day.max(0) as u32;
        let month = month.max(0) as u32;
        let text = match kind.as_str() {
            "weekday-number" => {
                rusty_core::date_format::weekday_number(day, month, year, first_day).to_string()
            }
            "week-number" => rusty_core::date_format::iso_week(day, month, year).to_string(),
            "day-of-year" => rusty_core::date_format::day_of_year(day, month, year).to_string(),
            // Pola czujnika TIME nie są dostępne na PC.
            "lunar-date" => "--/--".to_string(),
            "festival" => "--".to_string(),
            "world-clock" => "--:--".to_string(),
            _ => anyhow::bail!("Nieznany typ pola daty: {kind}"),
        };
        Ok(json!({ "text": text }))
    })();
    respond(result)
}

/// Aktualne ustawienia jedynego widgetu czasu w projekcie.
#[no_mangle]
pub extern "C" fn rw_time_widget(project_file: *const c_char) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let project = rusty_core::load_project(Path::new(&project_file))?;
        use rusty_core::model::{Align, DataSource};
        let widget = project
            .widgets
            .iter()
            .find(|w| w.data_source == Some(DataSource::Time))
            .context("projekt nie zawiera widgetu czasu")?;
        let align = match widget.align.unwrap_or(Align::Center) {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        };
        Ok(json!({
            "pattern": widget.time_format.clone().unwrap_or_else(|| "HH:MM:SS".to_string()),
            "textSize": widget.text_size.unwrap_or(64),
            "color": widget.color.clone().unwrap_or_else(|| "0xFFFFFFFF".to_string()),
            "x": widget.x,
            "y": widget.y,
            "w": widget.w,
            "h": widget.h,
            "align": align,
        }))
    })();
    respond(result)
}

/// Zapisuje ustawienia widgetu czasu (i natychmiast przelicza tekst początkowy).
#[no_mangle]
pub extern "C" fn rw_set_time_widget(
    project_file: *const c_char,
    settings_json: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<Value> {
        let project_file = unsafe { read_input(project_file) };
        let raw = unsafe { read_input(settings_json) };
        let settings: Value = serde_json::from_str(&raw).context("invalid time widget JSON")?;

        let pattern = required_string(&settings, "pattern")?;
        rusty_core::time_format::parse(&pattern)?;
        let text_size = required_i64(&settings, "textSize")?;
        if !(8..=160).contains(&text_size) {
            anyhow::bail!("Rozmiar czcionki musi być w zakresie 8–160");
        }
        let color = required_string(&settings, "color")?;
        let hex = color.strip_prefix("0x").context("Kolor musi mieć postać 0xAARRGGBB")?;
        if hex.len() != 8 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            anyhow::bail!("Kolor musi mieć postać 0xAARRGGBB");
        }
        let x = required_i64(&settings, "x")? as i32;
        let y = required_i64(&settings, "y")? as i32;
        let w = required_i64(&settings, "w")? as i32;
        let h = required_i64(&settings, "h")? as i32;
        if w <= 0 || h <= 0 || x < 0 || y < 0 || x + w > 466 || y + h > 466 {
            anyhow::bail!("Widget czasu musi mieścić się w ekranie 466×466");
        }
        let align = match required_string(&settings, "align")?.as_str() {
            "left" => rusty_core::model::Align::Left,
            "right" => rusty_core::model::Align::Right,
            _ => rusty_core::model::Align::Center,
        };

        let path = Path::new(&project_file);
        let mut project = rusty_core::load_project(path)?;
        use rusty_core::model::DataSource;
        let widget = project
            .widgets
            .iter_mut()
            .find(|w| w.data_source == Some(DataSource::Time))
            .context("projekt nie zawiera widgetu czasu")?;
        widget.time_format = Some(pattern.clone());
        widget.text_size = Some(text_size as u32);
        widget.color = Some(color.clone());
        widget.x = x;
        widget.y = y;
        widget.w = w;
        widget.h = h;
        widget.align = Some(align);
        widget.text = Some(rusty_core::time_format::format(&pattern, 0, 0, 0)?);
        let initial_text = widget.text.clone().unwrap_or_default();
        rusty_core::save_project(path, &project)?;
        Ok(json!({ "saved": true, "text": initial_text }))
    })();
    respond(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, ImageEncoder};

    fn take(ptr: *mut c_char) -> Value {
        let json = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
        rw_free_string(ptr);
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn generate_qr_returns_png_base64() {
        use base64::Engine;

        let text = CString::new("zpkd1://192.168.1.10:8123/Test.zip").unwrap();
        let value = take(rw_generate_qr(text.as_ptr(), 256));
        assert_eq!(value["ok"], true, "{value}");
        let png = base64::engine::general_purpose::STANDARD
            .decode(value["data"]["png"].as_str().unwrap())
            .unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn ensure_defaults_seeds_fonts_and_hands() {
        let root = std::env::temp_dir().join(format!(
            "rusty-ffi-defaults-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);

        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../native/RustyWatch/Assets");
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let fonts_arg = CString::new(bundled.join("Fonts").to_string_lossy().as_bytes()).unwrap();
        let hands_arg = CString::new(bundled.join("Hands").to_string_lossy().as_bytes()).unwrap();
        let status_arg = CString::new(bundled.join("Status").to_string_lossy().as_bytes()).unwrap();
        let weather_arg = CString::new(bundled.join("Weather").to_string_lossy().as_bytes()).unwrap();

        let value = take(rw_ensure_defaults(
            root_arg.as_ptr(),
            fonts_arg.as_ptr(),
            hands_arg.as_ptr(),
            status_arg.as_ptr(),
            weather_arg.as_ptr(),
        ));
        assert_eq!(value["ok"], true, "error: {}", value["error"]);
        assert!(root.join("Fonts").join("Inter-Regular.ttf").is_file());
        assert!(root.join("Hands").join("Classic").join("hour.png").is_file());
        assert!(root.join("Hands").join("Classic").join("minute.png").is_file());
        let light = root.join("Hands").join("Light");
        for name in ["hour.png", "minute.png", "second.png", "cap.png"] {
            let file = light.join(name);
            assert!(file.is_file(), "brak wskazówki {name}");
            let image = image::open(&file).unwrap();
            assert_eq!((image.width(), image.height()), (400, 400), "{name}");
        }
        let sets = take(rw_hands_list(root_arg.as_ptr(), hands_arg.as_ptr()));
        assert_eq!(sets["ok"], true, "error: {}", sets["error"]);
        let hands = sets["data"]["hands"].as_array().unwrap();
        assert_eq!(hands.len(), 2);
        assert_eq!(hands[0]["id"], "Classic");
        assert_eq!(hands[1]["id"], "Light");
        assert_eq!(hands[1]["hasSecond"], true);
        assert_eq!(hands[1]["hasCap"], true);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn prepare_emulator_workspace_returns_unpacked_project() {
        let root = std::env::temp_dir().join(format!(
            "rusty-ffi-emulator-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("project");
        let path = CString::new(dir.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Emulator").unwrap();
        let author = CString::new("Tester").unwrap();
        let created = take(rw_create_project(path.as_ptr(), name.as_ptr(), author.as_ptr(), 42));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);

        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let project_arg = CString::new(
            dir.join("project.json")
                .to_string_lossy()
                .as_bytes(),
        )
        .unwrap();
        let value = take(rw_prepare_emulator_workspace(
            project_arg.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(value["ok"], true, "error: {}", value["error"]);
        let data = &value["data"];
        let app_dir = Path::new(data["appDir"].as_str().unwrap());
        assert!(app_dir.join("app.json").is_file());
        assert!(app_dir.join("watchface/index.js").is_file());
        assert!(Path::new(data["zip"].as_str().unwrap()).is_file());
        assert!(data["dir"].as_str().unwrap().contains("app-42"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn create_project_uses_watchface_metadata() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-ffi-create-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = CString::new(dir.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("FFI Face").unwrap();
        let author = CString::new("FFI Author").unwrap();
        let value = take(rw_create_project(path.as_ptr(), name.as_ptr(), author.as_ptr(), 7_654_321));

        assert_eq!(value["ok"], true, "error: {}", value["error"]);
        let data = &value["data"];
        assert_eq!(data["name"], "FFI Face");
        assert_eq!(data["author"], "FFI Author");
        assert_eq!(data["appId"], 7_654_321);

        let project = rusty_core::load_project(&dir.join("project.json")).unwrap();
        assert_eq!(project.metadata.name, "FFI Face");
        assert_eq!(project.metadata.author, "FFI Author");
        assert_eq!(project.metadata.app_id, 7_654_321);

        let random = take(rw_random_app_id());
        let id = random["data"]["appId"].as_u64().unwrap();
        assert!((10_000_000..100_000_000).contains(&id));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_in_and_save_as_use_watchface_folders() {
        let root = std::env::temp_dir().join(format!(
            "rusty-ffi-folders-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let destination = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("First Face").unwrap();
        let author = CString::new("Jane").unwrap();
        let created = take(rw_create_project_in(
            destination.as_ptr(),
            name.as_ptr(),
            author.as_ptr(),
            44_444_444,
        ));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let source = created["data"]["file"].as_str().unwrap();
        assert!(Path::new(source).starts_with(root.join("First Face")));

        let copy_name = CString::new("Second Face").unwrap();
        let source_c = CString::new(source).unwrap();
        let copy = take(rw_save_project_as(
            source_c.as_ptr(),
            destination.as_ptr(),
            copy_name.as_ptr(),
            author.as_ptr(),
            55_555_555,
        ));
        assert_eq!(copy["ok"], true, "error: {}", copy["error"]);
        assert_eq!(copy["data"]["appId"], 55_555_555);
        assert!(root.join("Second Face/project.json").exists());
        assert!(root.join("Second Face/assets/icon.png").exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn date_field_preview_matches_generated_algorithms() {
        let monday = CString::new("monday").unwrap();
        let sunday = CString::new("sunday").unwrap();
        for (kind, expected_monday, expected_sunday) in [
            ("weekday-number", "2", "3"),
            ("week-number", "45", "45"),
            ("day-of-year", "307", "307"),
            ("lunar-date", "--/--", "--/--"),
            ("festival", "--", "--"),
            ("world-clock", "--:--", "--:--"),
        ] {
            let kind_arg = CString::new(kind).unwrap();
            let value = take(rw_format_date_field(kind_arg.as_ptr(), 3, 11, 2026, monday.as_ptr()));
            assert_eq!(value["ok"], true, "kind={kind}");
            assert_eq!(value["data"]["text"], expected_monday, "kind={kind}");
            let value = take(rw_format_date_field(kind_arg.as_ptr(), 3, 11, 2026, sunday.as_ptr()));
            assert_eq!(value["data"]["text"], expected_sunday, "kind={kind}");
        }
    }

    #[test]
    fn first_day_of_week_setting_changes_export() {
        let root = std::env::temp_dir().join(format!("rw-ffi-first-day-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("First day").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 77));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();
        let kind = CString::new("weekday-number").unwrap();
        let added = take(rw_add_widget(
            project_file.as_ptr(),
            kind.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);

        let fresh = rusty_core::load_project(Path::new(file)).unwrap();
        assert_eq!(
            fresh.first_day_of_week,
            rusty_core::model::FirstDayOfWeek::Monday
        );
        assert!(rusty_core::codegen::generate_index_js(&fresh)
            .contains("return ((_w + 6) % 7) + 1"));

        let sunday = CString::new("sunday").unwrap();
        let changed = take(rw_set_first_day_of_week(project_file.as_ptr(), sunday.as_ptr()));
        assert_eq!(changed["ok"], true, "error: {}", changed["error"]);
        let project = rusty_core::load_project(Path::new(file)).unwrap();
        assert_eq!(
            project.first_day_of_week,
            rusty_core::model::FirstDayOfWeek::Sunday
        );
        assert!(rusty_core::codegen::generate_index_js(&project).contains("return _w + 1"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn supported_widgets_lists_time_date_and_text_categories() {
        let value = take(rw_supported_widgets());
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"][0]["kind"], "time");
        assert_eq!(value["data"][0]["category"], "time");
        assert_eq!(value["data"][1]["kind"], "analog-time");
        assert_eq!(value["data"][1]["category"], "time");
        assert_eq!(value["data"][2]["kind"], "date");
        assert_eq!(value["data"][2]["category"], "date");
        for index in 3..=8 {
            assert_eq!(value["data"][index]["category"], "date", "index={index}");
        }
        assert_eq!(value["data"][9]["kind"], "text");
        assert_eq!(value["data"][9]["category"], "text");
    }

    #[test]
    fn only_time_and_date_catalog_entries_have_pattern() {
        let value = take(rw_supported_widgets());
        for item in value["data"].as_array().unwrap() {
            let kind = item["kind"].as_str().unwrap();
            let expected = kind == "time" || kind == "date";
            assert_eq!(
                item["hasPattern"].as_bool().unwrap_or(false),
                expected,
                "hasPattern for {kind}"
            );
        }
    }

    #[test]
    fn frame_geometry_scales_text_and_clamps_to_screen() {
        let scale = CString::new(
            r#"{"x":23,"y":183,"w":420,"h":100,"textSize":64,"handle":"se","dx":-210,"dy":0,"autoScale":true}"#,
        )
        .unwrap();
        let value = take(rw_frame_geometry(scale.as_ptr()));
        assert_eq!(value["ok"], true, "error: {}", value["error"]);
        assert_eq!(value["data"]["w"], 210);
        assert_eq!(value["data"]["textSize"], 32);

        let no_scale = CString::new(
            r#"{"x":23,"y":183,"w":420,"h":100,"textSize":64,"handle":"se","dx":-210,"dy":0,"autoScale":false}"#,
        )
        .unwrap();
        let value = take(rw_frame_geometry(no_scale.as_ptr()));
        assert_eq!(value["data"]["textSize"], 64);

        let move_far = CString::new(
            r#"{"x":23,"y":183,"w":420,"h":100,"textSize":64,"handle":"move","dx":9999,"dy":9999,"autoScale":true}"#,
        )
        .unwrap();
        let value = take(rw_frame_geometry(move_far.as_ptr()));
        assert_eq!(value["data"]["x"], 46);
        assert_eq!(value["data"]["y"], 366);
        assert_eq!(value["data"]["textSize"], 64);
    }

    #[test]
    fn frame_geometry_snap_is_optional_in_json() {
        // Pole `snap` jest opcjonalne — starsi klienci (bez tego pola) działają jak dotąd.
        let free = CString::new(
            r#"{"x":111,"y":139,"w":100,"h":50,"textSize":32,"handle":"move","dx":0,"dy":0,"autoScale":false}"#,
        )
        .unwrap();
        let value = take(rw_frame_geometry(free.as_ptr()));
        assert_eq!(value["data"]["x"], 111);
        assert_eq!(value["data"]["y"], 139);

        let snapped = CString::new(
            r#"{"x":111,"y":139,"w":100,"h":50,"textSize":32,"handle":"move","dx":0,"dy":0,"autoScale":false,"snap":true}"#,
        )
        .unwrap();
        let value = take(rw_frame_geometry(snapped.as_ptr()));
        assert_eq!(value["data"]["x"], 112);
        assert_eq!(value["data"]["y"], 144);
    }

    #[test]
    fn frame_geometry_reports_alignment_guides() {
        let input = CString::new(
            r#"{"x":100,"y":100,"w":100,"h":80,"textSize":32,"handle":"move","dx":29,"dy":-38,"autoScale":false,"alignX":[233],"alignY":[64]}"#,
        )
        .unwrap();
        let value = take(rw_frame_geometry(input.as_ptr()));
        assert_eq!(value["ok"], true, "error: {}", value["error"]);
        assert_eq!(value["data"]["x"], 133);
        assert_eq!(value["data"]["y"], 64);
        assert_eq!(value["data"]["guideX"], 233);
        assert_eq!(value["data"]["guideY"], 64);
    }

    #[test]
    fn widget_add_set_remove_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-ffi-widgets-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let destination = CString::new(dir.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Widget Face").unwrap();
        let author = CString::new("Jane").unwrap();
        let created = take(rw_create_project(destination.as_ptr(), name.as_ptr(), author.as_ptr(), 1_234_567));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let source = created["data"]["file"].as_str().unwrap();
        let file = CString::new(source).unwrap();

        let kind = CString::new("time").unwrap();
        let added = take(rw_add_widget(
            file.as_ptr(),
            kind.as_ptr(),
            destination.as_ptr(),
            destination.as_ptr(),
            destination.as_ptr(),
        ));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);
        assert_eq!(added["data"]["id"], "time-2");
        assert_eq!(added["data"]["layers"].as_array().unwrap().len(), 2);

        let settings = CString::new(
            r#"{"id":"time-2","x":10,"y":20,"w":200,"h":60,"textSize":30,"color":"0xFFFF0000","align":"right","timeFormat":"HH:MM","autoScale":true,"textStyle":"wrap","vAlign":"bottom"}"#,
        ).unwrap();
        let app_root = CString::new(dir.to_string_lossy().as_bytes()).unwrap();
        let updated = take(rw_set_widget(file.as_ptr(), settings.as_ptr(), app_root.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        assert_eq!(updated["data"]["text"], "00:00");
        let layers = updated["data"]["layers"].as_array().unwrap();
        let changed = layers.iter().find(|l| l["id"] == "time-2").unwrap();
        assert_eq!(changed["autoScale"], true);
        assert_eq!(changed["textStyle"], "wrap");
        assert_eq!(changed["vAlign"], "bottom");

        let second = CString::new("time-2").unwrap();
        let removed = take(rw_remove_widget(file.as_ptr(), second.as_ptr()));
        assert_eq!(removed["ok"], true, "error: {}", removed["error"]);
        assert_eq!(removed["data"]["layers"].as_array().unwrap().len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn widget_visibility_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-ffi-visibility-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let destination = CString::new(dir.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Visibility Face").unwrap();
        let author = CString::new("Jane").unwrap();
        let created = take(rw_create_project(destination.as_ptr(), name.as_ptr(), author.as_ptr(), 9_876_543));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = CString::new(created["data"]["file"].as_str().unwrap()).unwrap();
        let id = CString::new("time").unwrap();

        let hidden = take(rw_set_widget_visibility(file.as_ptr(), id.as_ptr(), false));
        assert_eq!(hidden["ok"], true, "error: {}", hidden["error"]);
        assert_eq!(hidden["data"]["visible"], false);
        let layers = hidden["data"]["layers"].as_array().unwrap();
        let widget = layers.iter().find(|layer| layer["id"] == "time").unwrap();
        assert_eq!(widget["visible"], false);

        let shown = take(rw_set_widget_visibility(file.as_ptr(), id.as_ptr(), true));
        assert_eq!(shown["ok"], true, "error: {}", shown["error"]);
        let layers = shown["data"]["layers"].as_array().unwrap();
        let widget = layers.iter().find(|layer| layer["id"] == "time").unwrap();
        assert_eq!(widget["visible"], true);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fonts_store_install_and_protect_default() {
        let root = std::env::temp_dir().join(format!("rusty-ffi-fonts-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets/Fonts");
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let bundled_arg = CString::new(bundled.to_string_lossy().as_bytes()).unwrap();

        let listed = take(rw_fonts_list(root_arg.as_ptr(), bundled_arg.as_ptr()));
        assert_eq!(listed["ok"], true, "error: {}", listed["error"]);
        let fonts = listed["data"]["fonts"].as_array().unwrap();
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0]["family"], "Inter");
        assert_eq!(fonts[0]["isDefault"], true);

        let source = bundled.join("Inter-Regular.ttf");
        let source_arg = CString::new(source.to_string_lossy().as_bytes()).unwrap();
        let added = take(rw_fonts_add(root_arg.as_ptr(), bundled_arg.as_ptr(), source_arg.as_ptr()));
        assert_eq!(added["ok"], false); // domyślna nazwa jest zarezerwowana

        let default_file = CString::new("Inter-Regular.ttf").unwrap();
        let protected = take(rw_fonts_remove(root_arg.as_ptr(), default_file.as_ptr()));
        assert_eq!(protected["ok"], false);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn editor_layers_and_move_layer_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-ffi-layers-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let destination = CString::new(dir.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Layer Face").unwrap();
        let author = CString::new("Jane").unwrap();
        let created = take(rw_create_project(destination.as_ptr(), name.as_ptr(), author.as_ptr(), 1_234_567));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let source = created["data"]["file"].as_str().unwrap();

        let file = CString::new(source).unwrap();
        let listed = take(rw_editor_layers(file.as_ptr()));
        assert_eq!(listed["ok"], true, "error: {}", listed["error"]);
        let layers = listed["data"].as_array().unwrap();
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0]["kind"], "time");

        let kind = CString::new("time").unwrap();
        take(rw_add_widget(
            file.as_ptr(),
            kind.as_ptr(),
            destination.as_ptr(),
            destination.as_ptr(),
            destination.as_ptr(),
        ));
        let id = CString::new("time-2").unwrap();
        let moved = take(rw_move_layer(file.as_ptr(), id.as_ptr(), -1));
        assert_eq!(moved["ok"], true, "error: {}", moved["error"]);
        assert_eq!(moved["data"]["order"][0], "time-2");
        assert_eq!(moved["data"]["order"][1], "time");

        let below_first = take(rw_move_layer(file.as_ptr(), id.as_ptr(), -1));
        assert_eq!(below_first["ok"], false);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn set_project_metadata_keeps_the_name() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-ffi-metadata-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let destination = CString::new(dir.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("FFI Face").unwrap();
        let author = CString::new("Original").unwrap();
        let created = take(rw_create_project(destination.as_ptr(), name.as_ptr(), author.as_ptr(), 1_234_567));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let source = created["data"]["file"].as_str().unwrap();

        let file = CString::new(source).unwrap();
        let new_author = CString::new("New Author").unwrap();
        let version = CString::new("2.0.1").unwrap();
        let value = take(rw_set_project_metadata(file.as_ptr(), new_author.as_ptr(), version.as_ptr()));
        assert_eq!(value["ok"], true, "error: {}", value["error"]);
        assert_eq!(value["data"]["name"], "FFI Face");
        assert_eq!(value["data"]["author"], "New Author");
        assert_eq!(value["data"]["version"], "2.0.1");

        let project = rusty_core::load_project(&dir.join("project.json")).unwrap();
        assert_eq!(project.metadata.name, "FFI Face");
        assert_eq!(project.metadata.author, "New Author");
        assert_eq!(project.metadata.version, "2.0.1");

        let bad = CString::new("").unwrap();
        let rejected = take(rw_set_project_metadata(file.as_ptr(), bad.as_ptr(), version.as_ptr()));
        assert_eq!(rejected["ok"], false);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn analog_widget_and_hand_sets_roundtrip() {
        let root = std::env::temp_dir().join(format!("rw-ffi-hands-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let bundled = root.join("bundled");
        let classic = bundled.join("Classic");
        std::fs::create_dir_all(&classic).unwrap();
        for name in ["hour.png", "minute.png", "second.png", "cap.png"] {
            let image = image::RgbaImage::from_pixel(400, 400, image::Rgba([200, 200, 210, 255]));
            let file = std::fs::File::create(classic.join(name)).unwrap();
            image::codecs::png::PngEncoder::new(file)
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
        }
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let bundled_arg = CString::new(bundled.to_string_lossy().as_bytes()).unwrap();

        let listed = take(rw_hands_list(root_arg.as_ptr(), bundled_arg.as_ptr()));
        assert_eq!(listed["ok"], true, "error: {}", listed["error"]);
        let hands = listed["data"]["hands"].as_array().unwrap();
        assert_eq!(hands.len(), 1);
        assert_eq!(hands[0]["id"], "Classic");

        let name = CString::new("Analog Face").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 88));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();
        let kind = CString::new("analog-time").unwrap();
        let added = take(rw_add_widget(
            project_file.as_ptr(),
            kind.as_ptr(),
            root_arg.as_ptr(),
            bundled_arg.as_ptr(),
            bundled_arg.as_ptr(),
        ));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);
        let widget_id = added["data"]["id"].as_str().unwrap();
        let layer = added["data"]["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["id"] == widget_id)
            .unwrap();
        assert_eq!(layer["kind"], "analog-time");
        assert_eq!(layer["handSet"], "Classic");
        assert_eq!(layer["hourSrc"], "hands/Classic/hour.png");

        // Wyłączanie i wznawianie opcjonalnych wskazówek bez zmiany zestawu.
        let hand_widget_arg = CString::new(widget_id).unwrap();
        let hidden = CString::new(r#"{"hour":false,"minute":false,"second":false,"cap":false}"#).unwrap();
        let invisible = take(rw_set_hand_visibility(
            project_file.as_ptr(),
            hand_widget_arg.as_ptr(),
            hidden.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(invisible["ok"], true, "error: {}", invisible["error"]);
        let hidden_layer = invisible["data"]["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["id"] == widget_id)
            .unwrap();
        assert!(hidden_layer["hourSrc"].is_null());
        assert!(hidden_layer["minuteSrc"].is_null());
        assert!(hidden_layer["secondSrc"].is_null());
        assert!(hidden_layer["capSrc"].is_null());
        let project_path = std::path::Path::new(file);
        let hidden_project = rusty_core::load_project(project_path).unwrap();
        let hidden_js = rusty_core::codegen::generate_index_js(&hidden_project);
        assert!(hidden_js.contains("hour: null"));
        assert!(hidden_js.contains("minute: null"));
        assert!(hidden_js.contains("second: null"));
        assert!(hidden_js.contains("cap: null"));
        // W projekcie jest też cyfrowy widget z sekundami — sprawdzamy konkretnie
        // brak aktualizacji wskazówek, nie globalny timer.
        assert!(!hidden_js.contains("(h % 12) * 30 + m * 0.5"));
        assert!(!hidden_js.contains("m * 6 + s * 0.1"));

        let shown = CString::new(r#"{"hour":true,"minute":true,"second":true,"cap":true}"#).unwrap();
        let visible = take(rw_set_hand_visibility(
            project_file.as_ptr(),
            hand_widget_arg.as_ptr(),
            shown.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(visible["ok"], true, "error: {}", visible["error"]);
        let visible_project = rusty_core::load_project(project_path).unwrap();
        let visible_widget = visible_project.widgets.iter().find(|w| w.id == widget_id).unwrap();
        assert_eq!(visible_widget.hour_src.as_deref(), Some("hands/Classic/hour.png"));
        assert_eq!(visible_widget.minute_src.as_deref(), Some("hands/Classic/minute.png"));
        assert_eq!(visible_widget.second_src.as_deref(), Some("hands/Classic/second.png"));
        assert_eq!(visible_widget.cap_src.as_deref(), Some("hands/Classic/cap.png"));

        // Zmiana zestawu i walidacja eksportu.
        let custom = root.join("My Hands");
        std::fs::create_dir_all(&custom).unwrap();
        for name in ["hour.png", "minute.png"] {
            let image = image::RgbaImage::from_pixel(300, 300, image::Rgba([10, 20, 30, 255]));
            let file = std::fs::File::create(custom.join(name)).unwrap();
            image::codecs::png::PngEncoder::new(file)
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
        }
        let folder = CString::new(custom.to_string_lossy().as_bytes()).unwrap();
        let added_set = take(rw_hands_add(root_arg.as_ptr(), folder.as_ptr()));
        assert_eq!(added_set["ok"], true, "error: {}", added_set["error"]);
        let id = added_set["data"]["hand"]["id"].as_str().unwrap();
        let id_arg = CString::new(id).unwrap();
        let widget_arg = CString::new(widget_id).unwrap();
        let changed = take(rw_set_hands(
            project_file.as_ptr(),
            widget_arg.as_ptr(),
            id_arg.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(changed["ok"], true, "error: {}", changed["error"]);
        let path = std::path::Path::new(file);
        let project = rusty_core::load_project(path).unwrap();
        assert!(rusty_core::validate(&project).is_empty());
        let widget = project.widgets.iter().find(|w| w.id == widget_id).unwrap();
        // Zmiana zestawu nie zmienia ramki — pliki 300×300 są dopasowywane do 400×400.
        assert_eq!((widget.x, widget.y, widget.w, widget.h), (33, 33, 400, 400));
        assert!(widget.second_src.is_none());
        let hand_file = std::path::Path::new(file)
            .parent()
            .unwrap()
            .join("assets")
            .join(widget.hour_src.as_ref().unwrap());
        let image = image::open(&hand_file).unwrap();
        assert_eq!((image.width(), image.height()), (400, 400));

        let js = rusty_core::codegen::generate_index_js(&project);
        assert!(js.contains("hmUI.widget.IMG"));
        assert!(js.contains("(h % 12) * 30 + m * 0.5"));

        let removed = take(rw_hands_remove(root_arg.as_ptr(), id_arg.as_ptr()));
        assert_eq!(removed["ok"], true, "error: {}", removed["error"]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn scaled_analog_widget_resizes_hand_files() {
        let root = std::env::temp_dir().join(format!("rw-ffi-scale-hands-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let classic = root.join("bundled").join("Classic");
        std::fs::create_dir_all(&classic).unwrap();
        for name in ["hour.png", "minute.png", "second.png", "cap.png"] {
            let image = image::RgbaImage::from_pixel(400, 400, image::Rgba([200, 200, 210, 255]));
            let file = std::fs::File::create(classic.join(name)).unwrap();
            image::codecs::png::PngEncoder::new(file)
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
        }
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let bundled_arg = CString::new(root.join("bundled").to_string_lossy().as_bytes()).unwrap();

        let name = CString::new("Scaled Analog").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 89));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();
        let kind = CString::new("analog-time").unwrap();
        let added = take(rw_add_widget(
            project_file.as_ptr(),
            kind.as_ptr(),
            root_arg.as_ptr(),
            bundled_arg.as_ptr(),
            bundled_arg.as_ptr(),
        ));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);
        let widget_id = added["data"]["id"].as_str().unwrap();
        let hand_file = Path::new(file)
            .parent()
            .unwrap()
            .join("assets/hands/Classic/hour.png");
        let original = image::open(&hand_file).unwrap();
        assert_eq!((original.width(), original.height()), (400, 400));

        let settings = CString::new(
            format!(
                r#"{{"id":"{widget_id}","x":146,"y":146,"w":175,"h":175,"autoScale":true}}"#
            ),
        )
        .unwrap();
        let scaled = take(rw_set_widget(
            project_file.as_ptr(),
            settings.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(scaled["ok"], true, "error: {}", scaled["error"]);
        assert_eq!((image::open(&hand_file).unwrap().width(), image::open(&hand_file).unwrap().height()), (175, 175));

        // Ponowne włączenie wskazówki po wyłączeniu zachowuje rozmiar ramki.
        let widget_arg = CString::new(widget_id).unwrap();
        let hidden = CString::new(r#"{"hour":false,"minute":false,"second":false,"cap":false}"#).unwrap();
        let invisible = take(rw_set_hand_visibility(
            project_file.as_ptr(),
            widget_arg.as_ptr(),
            hidden.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(invisible["ok"], true, "error: {}", invisible["error"]);
        let shown = CString::new(r#"{"hour":true,"minute":true,"second":true,"cap":true}"#).unwrap();
        let visible = take(rw_set_hand_visibility(
            project_file.as_ptr(),
            widget_arg.as_ptr(),
            shown.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(visible["ok"], true, "error: {}", visible["error"]);
        assert_eq!((image::open(&hand_file).unwrap().width(), image::open(&hand_file).unwrap().height()), (175, 175));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn switching_hand_set_after_rescaling_keeps_frame() {
        let root = std::env::temp_dir().join(format!("rw-ffi-keep-frame-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../native/RustyWatch/Assets");
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let bundled_arg = CString::new(bundled.join("Hands").to_string_lossy().as_bytes()).unwrap();
        let fonts_arg = CString::new(bundled.join("Fonts").to_string_lossy().as_bytes()).unwrap();
        let status_arg = CString::new(bundled.join("Status").to_string_lossy().as_bytes()).unwrap();
        let weather_arg = CString::new(bundled.join("Weather").to_string_lossy().as_bytes()).unwrap();
        assert_eq!(
            take(rw_ensure_defaults(
                root_arg.as_ptr(),
                fonts_arg.as_ptr(),
                bundled_arg.as_ptr(),
                status_arg.as_ptr(),
                weather_arg.as_ptr()
            ))["ok"],
            true
        );

        let name = CString::new("Keep Frame").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 91));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();
        let kind = CString::new("analog-time").unwrap();
        let added = take(rw_add_widget(
            project_file.as_ptr(),
            kind.as_ptr(),
            root_arg.as_ptr(),
            bundled_arg.as_ptr(),
            bundled_arg.as_ptr(),
        ));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);
        let widget_id = added["data"]["id"].as_str().unwrap();

        // Użytkownik przesuwa i przeskalowuje ramkę po dodaniu widgetu.
        let settings = CString::new(format!(
            r#"{{"id":"{widget_id}","x":150,"y":291,"w":167,"h":167,"autoScale":true}}"#
        ))
        .unwrap();
        let scaled = take(rw_set_widget(
            project_file.as_ptr(),
            settings.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(scaled["ok"], true, "error: {}", scaled["error"]);

        let widget_arg = CString::new(widget_id).unwrap();
        let light = CString::new("Light").unwrap();
        let changed = take(rw_set_hands(
            project_file.as_ptr(),
            widget_arg.as_ptr(),
            light.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(changed["ok"], true, "error: {}", changed["error"]);

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let widget = project.widgets.iter().find(|w| w.id == widget_id).unwrap();
        assert_eq!((widget.x, widget.y, widget.w, widget.h), (150, 291, 167, 167));
        assert_eq!(widget.hand_set.as_deref(), Some("Light"));
        let hand_file = Path::new(file)
            .parent()
            .unwrap()
            .join("assets")
            .join(widget.hour_src.as_ref().unwrap());
        let image = image::open(&hand_file).unwrap();
        assert_eq!((image.width(), image.height()), (167, 167));
        assert!(rusty_core::validate(&project).is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn adding_step_widget_through_ffi_creates_sensor_based_text() {
        let root = std::env::temp_dir().join(format!("rw-ffi-steps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Steps").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 92));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();
        let kind = CString::new("steps").unwrap();
        let added = take(rw_add_widget(
            project_file.as_ptr(),
            kind.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);
        let layers = added["data"]["layers"].as_array().unwrap();
        assert_eq!(layers.last().unwrap()["kind"], "steps");

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let widget = project.widgets.iter().find(|w| w.id == "steps").unwrap();
        assert_eq!(widget.kind, rusty_core::model::WidgetKind::StepText);
        assert_eq!(widget.data_source, Some(rusty_core::model::DataSource::Step));
        let js = rusty_core::codegen::generate_index_js(&project);
        assert!(js.contains("hmSensor.id.STEP"));
        assert!(js.contains("addEventListener(_stepSensor.event.CHANGE, stepTick)"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn distance_widget_uses_app_default_and_own_override() {
        let root = std::env::temp_dir().join(format!("rw-ffi-distance-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Distance").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 93));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();
        let kind = CString::new("distance").unwrap();
        let added = take(rw_add_widget(
            project_file.as_ptr(),
            kind.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);
        let layers = added["data"]["layers"].as_array().unwrap();
        assert_eq!(layers.last().unwrap()["kind"], "distance");
        assert_eq!(layers.last().unwrap()["text"], "0.00 km");

        // Nowe projekty domyślnie czytają jednostkę z zegarka.
        let fresh = rusty_core::load_project(Path::new(file)).unwrap();
        assert_eq!(
            fresh.measurement_system,
            rusty_core::model::MeasurementSystem::Watch
        );
        let fresh_js = rusty_core::codegen::generate_index_js(&fresh);
        assert!(fresh_js.contains("hmSetting.getMileageUnit() === 1"));
        assert!(fresh_js.contains("autoMileage ? 'mi' : 'km'"));

        // „Zgodnie z zegarkiem" działa też jako jawny wybór w ustawieniach.
        let watch = CString::new("watch").unwrap();
        let changed = take(rw_set_measurement_system(project_file.as_ptr(), watch.as_ptr()));
        assert_eq!(changed["ok"], true, "error: {}", changed["error"]);
        let watch_settings = CString::new(
            r#"{"id":"distance","x":23,"y":260,"w":420,"h":60,"textSize":32,"color":"0xFFFFFFFF","align":"center","measurementSystem":"watch"}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(
            project_file.as_ptr(),
            watch_settings.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        let project = rusty_core::load_project(Path::new(file)).unwrap();
        assert_eq!(
            project.measurement_system,
            rusty_core::model::MeasurementSystem::Watch
        );
        let widget = project.widgets.iter().find(|w| w.id == "distance").unwrap();
        assert_eq!(
            widget.measurement_system,
            Some(rusty_core::model::MeasurementSystem::Watch)
        );
        let layers = updated["data"]["layers"].as_array().unwrap();
        assert_eq!(layers.last().unwrap()["measurementSystem"], "watch");

        let imperial = CString::new("imperial").unwrap();
        let changed = take(rw_set_measurement_system(project_file.as_ptr(), imperial.as_ptr()));
        assert_eq!(changed["ok"], true, "error: {}", changed["error"]);
        let layers = changed["data"]["layers"].as_array().unwrap();
        // Widget ma jawne nadpisanie „watch", więc tekst pozostaje metrycznym
        // placeholderem — system projektu go nie zmienia.
        assert_eq!(layers.last().unwrap()["text"], "0.00 km");
        assert_eq!(layers.last().unwrap()["measurementSystem"], "watch");

        let settings = CString::new(
            r#"{"id":"distance","x":23,"y":260,"w":420,"h":60,"textSize":32,"color":"0xFFFFFFFF","align":"center","measurementSystem":"metric"}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(
            project_file.as_ptr(),
            settings.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        let project = rusty_core::load_project(Path::new(file)).unwrap();
        assert_eq!(project.measurement_system, rusty_core::model::MeasurementSystem::Imperial);
        let widget = project.widgets.iter().find(|w| w.id == "distance").unwrap();
        assert_eq!(widget.measurement_system, Some(rusty_core::model::MeasurementSystem::Metric));
        let generated = rusty_core::codegen::generate_index_js(&project);
        assert!(generated.contains("hmSensor.id.DISTANCE"));
        // Nadpisanie widgetu wygrywa z domyślnym systemem projektu (imperial).
        assert!(generated.contains("(_meters / 1000).toFixed(2)) + ' km'"));
        assert!(!generated.contains("(_meters / 1609.344)"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn health_widgets_roundtrip_through_ffi() {
        let root = std::env::temp_dir().join(format!("rw-ffi-health-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Health").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 94));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        for kind in ["heart", "stress", "pai", "skin-temp", "sleep", "calories", "spo2"] {
            let kind_arg = CString::new(kind).unwrap();
            let added = take(rw_add_widget(
                project_file.as_ptr(),
                kind_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind} error: {}", added["error"]);
            let layers = added["data"]["layers"].as_array().unwrap();
            assert_eq!(layers.last().unwrap()["kind"], kind, "kind={kind}");
        }

        let no_unit = CString::new(
            r#"{"id":"heart","x":23,"y":150,"w":420,"h":60,"textSize":40,"color":"0xFFFFFFFF","align":"center","showUnit":false}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(
            project_file.as_ptr(),
            no_unit.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let heart = project.widgets.iter().find(|w| w.id == "heart").unwrap();
        assert!(!heart.show_unit);
        let generated = rusty_core::codegen::generate_index_js(&project);
        assert!(!generated.contains("' bpm'"), "JS: {generated}");

        // Starter zawiera już jeden widget czasu.
        assert_eq!(project.widgets.len(), 8);
        assert!(rusty_core::validate(&project).is_empty());
        let js = rusty_core::codegen::generate_index_js(&project);
        for sensor in ["HEART", "STRESS", "PAI", "BODY_TEMP", "SLEEP", "CALORIE", "SPO2"] {
            assert!(
                js.contains(&format!("hmSensor.id.{sensor}")),
                "sensor={sensor} JS: {js}"
            );
        }
        let zip = rusty_core::package::export_zip_bytes(Path::new(file)).unwrap();
        assert!(zip.len() > 100);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn new_widget_fields_roundtrip_through_ffi() {
        let root = std::env::temp_dir().join(format!("rw-ffi-newwidgets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets");
        let bundled_fonts = CString::new(bundled.join("Fonts").to_string_lossy().as_bytes()).unwrap();
        let bundled_hands = CString::new(bundled.join("Hands").to_string_lossy().as_bytes()).unwrap();
        let bundled_status = CString::new(bundled.join("Status").to_string_lossy().as_bytes()).unwrap();
        let bundled_weather = CString::new(bundled.join("Weather").to_string_lossy().as_bytes()).unwrap();
        let seeded = take(rw_ensure_defaults(
            root_arg.as_ptr(),
            bundled_fonts.as_ptr(),
            bundled_hands.as_ptr(),
            bundled_status.as_ptr(),
            bundled_weather.as_ptr(),
        ));
        assert_eq!(seeded["ok"], true, "error: {}", seeded["error"]);
        let name = CString::new("NewWidgets").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 95));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        for kind in [
            "sleep-score",
            "sleep-deep",
            "heart-min",
            "heart-max",
            "heart-avg",
            "weather-high",
            "weather-icon",
        ] {
            let kind_arg = CString::new(kind).unwrap();
            let added = take(rw_add_widget(
                project_file.as_ptr(),
                kind_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind} error: {}", added["error"]);
            assert_eq!(added["data"]["layers"].as_array().unwrap().last().unwrap()["kind"], kind);
        }

        let stage_settings = CString::new(
            r#"{"id":"sleep-deep","x":23,"y":150,"w":420,"h":60,"textSize":40,"color":"0xFFFFFFFF","align":"center","sleepStageMode":"percent"}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(project_file.as_ptr(), stage_settings.as_ptr(), root_arg.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        let day_settings = CString::new(
            r#"{"id":"weather-high","x":23,"y":150,"w":420,"h":60,"textSize":40,"color":"0xFFFFFFFF","align":"center","weatherDayOffset":3}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(project_file.as_ptr(), day_settings.as_ptr(), root_arg.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let deep = project.widgets.iter().find(|w| w.id == "sleep-deep").unwrap();
        assert_eq!(deep.sleep_stage_mode, rusty_core::model::SleepStageMode::Percent);
        let high = project.widgets.iter().find(|w| w.id == "weather-high").unwrap();
        assert_eq!(high.weather_day_offset, 3);

        let layers = take(rw_editor_layers(project_file.as_ptr()));
        let find_layer = |kind: &str| {
            layers["data"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["kind"] == kind)
                .unwrap()
                .clone()
        };
        assert_eq!(find_layer("sleep-deep")["sleepStageMode"], "percent");
        assert_eq!(find_layer("weather-high")["weatherDayOffset"], 3);
        let js = rusty_core::codegen::generate_index_js(&project);
        assert!(js.contains("_sleepStageText(_sModels && _sModels.DEEP_STAGE, true)"));
        assert!(js.contains("const _wd_"), "day constant missing: {js}");
        assert!(rusty_core::validate(&project).is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn package_one_widgets_roundtrip_through_ffi() {
        let root = std::env::temp_dir().join(format!("rw-ffi-package-one-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets");
        let bundled_fonts = CString::new(bundled.join("Fonts").to_string_lossy().as_bytes()).unwrap();
        let bundled_hands = CString::new(bundled.join("Hands").to_string_lossy().as_bytes()).unwrap();
        let bundled_status = CString::new(bundled.join("Status").to_string_lossy().as_bytes()).unwrap();
        let bundled_weather = CString::new(bundled.join("Weather").to_string_lossy().as_bytes()).unwrap();
        let seeded = take(rw_ensure_defaults(
            root_arg.as_ptr(),
            bundled_fonts.as_ptr(),
            bundled_hands.as_ptr(),
            bundled_status.as_ptr(),
            bundled_weather.as_ptr(),
        ));
        assert_eq!(seeded["ok"], true, "error: {}", seeded["error"]);

        let name = CString::new("Package One").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 96));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        for kind in [
            "heart-last",
            "spo2-hour-avg",
            "pai-prepai",
            "sleep-wake",
            "sun-current",
            "weather-current",
        ] {
            let kind_arg = CString::new(kind).unwrap();
            let added = take(rw_add_widget(
                project_file.as_ptr(),
                kind_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind} error: {}", added["error"]);
            assert_eq!(added["data"]["layers"].as_array().unwrap().last().unwrap()["kind"], kind);
        }

        let temp_settings = CString::new(
            r#"{"id":"weather-current","x":23,"y":150,"w":420,"h":60,"textSize":40,"color":"0xFFFFFFFF","align":"center","showUnit":false}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(project_file.as_ptr(), temp_settings.as_ptr(), root_arg.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        let wake_settings = CString::new(
            r#"{"id":"sleep-wake","x":23,"y":150,"w":420,"h":60,"textSize":40,"color":"0xFFFFFFFF","align":"center","sleepStageMode":"percent"}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(project_file.as_ptr(), wake_settings.as_ptr(), root_arg.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let temp = project.widgets.iter().find(|w| w.id == "weather-current").unwrap();
        assert_eq!(temp.text.as_deref(), Some("22°"), "text content is not editable");
        assert_eq!(temp.show_unit, false);
        let wake = project.widgets.iter().find(|w| w.id == "sleep-wake").unwrap();
        assert_eq!(wake.sleep_stage_mode, rusty_core::model::SleepStageMode::Percent);
        let js = rusty_core::codegen::generate_index_js(&project);
        for token in [
            "_hHeart.last", "_spo2Hour", "prepai0", "WAKE_STAGE", "_sunCurrentText()",
            "hmUI.widget.TEXT_FONT", "type: hmUI.data_type.WEATHER_CURRENT", "unit_type: 0",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}");
        }
        assert!(rusty_core::validate(&project).is_empty());

        let kind = CString::new("sun-current").unwrap();
        let overrides = CString::new(r#"{"dayOffset":0,"nowMinute":1297}"#).unwrap();
        let locale = CString::new("en").unwrap();
        let pack = CString::new("system-like").unwrap();
        let raw = take(rw_format_weather_field(
            kind.as_ptr(), overrides.as_ptr(), locale.as_ptr(), root_arg.as_ptr(), pack.as_ptr(),
        ));
        assert_eq!(raw["ok"], true, "error: {}", raw["error"]);
        let text = raw["data"]["text"].as_str().unwrap();
        assert!(text.contains(':'), "unexpected sun-current text: {text}");
        assert_ne!(text, "--:--");

        let temp_kind = CString::new("weather-current").unwrap();
        let temp_overrides = CString::new(r#"{"current":18}"#).unwrap();
        let raw = take(rw_format_weather_field(
            temp_kind.as_ptr(), temp_overrides.as_ptr(), locale.as_ptr(),
            root_arg.as_ptr(), pack.as_ptr(),
        ));
        assert_eq!(raw["ok"], true, "error: {}", raw["error"]);
        assert_eq!(raw["data"]["text"], "18°");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn progress_widgets_roundtrip_through_ffi() {
        let catalog = take(rw_supported_widgets());
        let kinds: Vec<&str> = catalog["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["category"] == "progress")
            .map(|item| item["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, vec!["arc-progress", "bar-progress"]);

        let root = std::env::temp_dir().join(format!("rw-ffi-progress-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Progress").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 97));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        for kind in kinds {
            let kind_arg = CString::new(kind).unwrap();
            let added = take(rw_add_widget(
                project_file.as_ptr(), kind_arg.as_ptr(), root_arg.as_ptr(),
                root_arg.as_ptr(), root_arg.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind}: {}", added["error"]);
        }

        let payload = CString::new(
            r#"{"id":"arc","x":33,"y":33,"w":400,"h":400,"alpha":255,"progress":{"source":"steps","min":0,"max":10000,"color":"0xFF00FF00","trackColor":"0xFF333333","thickness":20,"startAngle":-135,"endAngle":135,"corner":"round","radius":8,"segments":4,"gap":6,"orientation":"horizontal","invert":false,"showTrack":true}}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(project_file.as_ptr(), payload.as_ptr(), root_arg.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let arc = project.widgets.iter().find(|w| w.id == "arc").unwrap();
        let p = arc.progress.as_ref().unwrap();
        assert_eq!(p.segments, 4);
        assert_eq!(p.gap, 6);
        assert_eq!(p.source, rusty_core::model::ProgressSource::Steps);
        let js = rusty_core::codegen::generate_index_js(&project);
        assert!(js.contains("hmUI.widget.ARC_PROGRESS"));
        assert!(js.contains("hmUI.widget.FILL_RECT"));
        assert!(js.contains("corner_flag: 0"));
        assert!(rusty_core::validate(&project).is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn goal_widgets_roundtrip_through_ffi() {
        let value = take(rw_supported_widgets());
        assert_eq!(value["ok"], true);
        let goals: Vec<&str> = value["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["category"] == "goals")
            .map(|item| item["kind"].as_str().unwrap())
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

        let root = std::env::temp_dir().join(format!("rw-ffi-goals-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Goals").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 95));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        for kind in goals {
            let kind_arg = CString::new(kind).unwrap();
            let added = take(rw_add_widget(
                project_file.as_ptr(),
                kind_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind} error: {}", added["error"]);
            let layers = added["data"]["layers"].as_array().unwrap();
            assert_eq!(layers.last().unwrap()["kind"], kind, "kind={kind}");
        }

        let no_unit = CString::new(
            r#"{"id":"calories-target","x":23,"y":150,"w":420,"h":60,"textSize":40,"color":"0xFFFFFFFF","align":"center","showUnit":false}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(project_file.as_ptr(), no_unit.as_ptr(), root_arg.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let calories = project.widgets.iter().find(|w| w.id == "calories-target").unwrap();
        assert!(!calories.show_unit);
        assert_eq!(calories.text.as_deref(), Some("2000"));

        assert!(rusty_core::validate(&project).is_empty());
        let js = rusty_core::codegen::generate_index_js(&project);
        for token in [
            "hmSensor.id.STAND",
            "hmSensor.id.FAT_BURRING",
            "String(_tStep.target)",
            "String(_tStand.target) + ' h'",
            "String(_tFat.target) + ' min'",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}: {js}");
        }
        assert!(!js.contains("String(_tCalorie.target) + ' kcal'"));
        let zip = rusty_core::package::export_zip_bytes(Path::new(file)).unwrap();
        assert!(zip.len() > 100);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn additional_health_widgets_and_alpha_roundtrip_through_ffi() {
        let root = std::env::temp_dir().join(format!("rw-ffi-health-extra-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Health Extra").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 96));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        for kind in ["stand", "fat-burning", "pai-weekly"] {
            let kind_arg = CString::new(kind).unwrap();
            let added = take(rw_add_widget(
                project_file.as_ptr(),
                kind_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
                root_arg.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind} error: {}", added["error"]);
            let layers = added["data"]["layers"].as_array().unwrap();
            assert_eq!(layers.last().unwrap()["kind"], kind, "kind={kind}");
            assert_eq!(layers.last().unwrap()["alpha"], 255);
        }

        let settings = CString::new(
            r#"{"id":"stand","x":23,"y":150,"w":420,"h":60,"textSize":40,"color":"0xFFFFFFFF","align":"center","alpha":128}"#,
        )
        .unwrap();
        let updated = take(rw_set_widget(project_file.as_ptr(), settings.as_ptr(), root_arg.as_ptr()));
        assert_eq!(updated["ok"], true, "error: {}", updated["error"]);
        let stand_layer = updated["data"]["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|layer| layer["id"] == "stand")
            .unwrap();
        assert_eq!(stand_layer["alpha"], 128);

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        assert!(rusty_core::validate(&project).is_empty());
        let js = rusty_core::codegen::generate_index_js(&project);
        for token in [
            "hmSensor.id.STAND",
            "hmSensor.id.FAT_BURRING",
            "_hPaiWeekly.totalpai || 0",
            "alpha: 128",
        ] {
            assert!(js.contains(token), "generated JS lacks {token}: {js}");
        }

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn weather_widgets_and_icon_pack_roundtrip_through_ffi() {
        let root = std::env::temp_dir().join(format!("rw-ffi-weather-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets");
        let bundled_fonts = CString::new(bundled.join("Fonts").to_string_lossy().as_bytes()).unwrap();
        let bundled_hands = CString::new(bundled.join("Hands").to_string_lossy().as_bytes()).unwrap();
        let bundled_status = CString::new(bundled.join("Status").to_string_lossy().as_bytes()).unwrap();
        let bundled_weather = CString::new(bundled.join("Weather").to_string_lossy().as_bytes()).unwrap();
        let seeded = take(rw_ensure_defaults(
            root_arg.as_ptr(),
            bundled_fonts.as_ptr(),
            bundled_hands.as_ptr(),
            bundled_status.as_ptr(),
            bundled_weather.as_ptr(),
        ));
        assert_eq!(seeded["ok"], true, "error: {}", seeded["error"]);

        let name = CString::new("Weather Face").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 31));
        let file = created["data"]["file"].as_str().unwrap().to_string();
        let file_arg = CString::new(file.as_str()).unwrap();

        for kind in [
            "sleep-target",
            "weather-condition",
            "weather-city",
            "weather-icon",
        ] {
            let kind_arg = CString::new(kind).unwrap();
            let added = take(rw_add_widget(
                file_arg.as_ptr(),
                kind_arg.as_ptr(),
                root_arg.as_ptr(),
                bundled_hands.as_ptr(),
                bundled_status.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind}: {}", added["error"]);
        }

        // Przypisujemy domyślną paczkę i skalujemy WSZYSTKIE 29 ikon do ramki.
        let widget_arg = CString::new("weather-icon").unwrap();
        let classic = CString::new(rusty_core::weather::DEFAULT_WEATHER_PACK).unwrap();
        let switched = take(rw_set_weather_pack(
            file_arg.as_ptr(),
            widget_arg.as_ptr(),
            classic.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(switched["ok"], true, "error: {}", switched["error"]);

        // Jedno przejście attach_to_project skaluje wszystkie 29 ikon do
        // aktualnej ramki — drugie scale_project_icons byłoby niepotrzebne.
        let project_dir = Path::new(&file).parent().unwrap();
        let (_fx, _fy, default_w, default_h) = rusty_core::weather::default_frame();
        for index in 0..29 {
            let path = project_dir
                .join("assets/weather/system-like")
                .join(format!("w{index:02}.png"));
            let image = image::open(&path).unwrap();
            assert_eq!(
                image.dimensions(),
                (default_w as u32, default_h as u32),
                "{path:?}"
            );
        }

        let settings = CString::new(
            r#"{"id":"weather-icon","x":40,"y":40,"w":120,"h":120,"autoScale":true}"#,
        )
        .unwrap();
        assert_eq!(take(rw_set_widget(file_arg.as_ptr(), settings.as_ptr(), root_arg.as_ptr()))["ok"], true);

        for index in 0..29 {
            let path = project_dir
                .join("assets/weather/system-like")
                .join(format!("w{index:02}.png"));
            let image = image::open(&path).unwrap();
            assert_eq!(image.dimensions(), (120, 120), "{path:?}");
        }

        // Zmiana rozmiaru zachowuje wybraną paczkę i skaluje wszystkie ikony.
        let resized = CString::new(
            r#"{"id":"weather-icon","x":36,"y":36,"w":128,"h":128,"autoScale":false}"#,
        )
        .unwrap();
        assert_eq!(take(rw_set_widget(file_arg.as_ptr(), resized.as_ptr(), root_arg.as_ptr()))["ok"], true);
        let project = rusty_core::load_project(Path::new(&file)).unwrap();
        let icon_widget = project.widgets.iter()
            .find(|w| w.kind == rusty_core::model::WidgetKind::WeatherIcon)
            .unwrap();
        assert_eq!(icon_widget.weather_icon_set.as_deref(),
            Some(rusty_core::weather::DEFAULT_WEATHER_PACK));
        for index in 0..29 {
            let path = project_dir
                .join("assets/weather/system-like")
                .join(format!("w{index:02}.png"));
            let image = image::open(&path).unwrap();
            assert_eq!(image.dimensions(), (128, 128), "{path:?}");
        }

        let project = rusty_core::load_project(Path::new(&file)).unwrap();
        assert!(rusty_core::validate(&project).is_empty());
        let js = rusty_core::codegen::generate_index_js(&project);
        assert!(js.contains("hmSensor.id.WEATHER"));
        assert!(js.contains("hmSetting.getSleepTarget()"));
        assert!(js.contains("_cityName("));

        let zip = rusty_core::export_zip(Path::new(&file), &root.join("weather.zip")).unwrap();
        assert!(zip.exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn weather_preview_ffi_defaults_to_cloudy_warsaw() {
        let root = std::env::temp_dir().join(format!("rw-ffi-weather-format-{}", std::process::id()));
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let overrides = CString::new("{}").unwrap();
        let pl = CString::new("pl").unwrap();
        let en = CString::new("en").unwrap();
        let classic = CString::new("system-like").unwrap();
        let bundled_icon = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets/Weather/system-like/w00.png");
        let pack_dir = root.join("Weather").join("system-like");
        std::fs::create_dir_all(&pack_dir).unwrap();
        std::fs::copy(bundled_icon, pack_dir.join("w00.png")).unwrap();

        let city = CString::new("weather-city").unwrap();
        let pl_result = take(rw_format_weather_field(
            city.as_ptr(), overrides.as_ptr(), pl.as_ptr(), root_arg.as_ptr(), classic.as_ptr(),
        ));
        assert_eq!(pl_result["ok"], true, "error: {}", pl_result["error"]);
        assert_eq!(pl_result["data"]["text"], "Warszawa");
        let en_result = take(rw_format_weather_field(
            city.as_ptr(), overrides.as_ptr(), en.as_ptr(), root_arg.as_ptr(), classic.as_ptr(),
        ));
        assert_eq!(en_result["data"]["text"], "Warsaw");

        let condition = CString::new("weather-condition").unwrap();
        let condition_result = take(rw_format_weather_field(
            condition.as_ptr(), overrides.as_ptr(), pl.as_ptr(), root_arg.as_ptr(), classic.as_ptr(),
        ));
        assert_eq!(condition_result["data"]["text"], "Pochmurno");

        let icon = CString::new("weather-icon").unwrap();
        let icon_result = take(rw_format_weather_field(
            icon.as_ptr(), overrides.as_ptr(), pl.as_ptr(), root_arg.as_ptr(), classic.as_ptr(),
        ));
        let path = icon_result["data"]["iconPath"].as_str().unwrap();
        assert!(path.ends_with("w00.png"), "unexpected icon path: {path}");
        // Ikona nie może rysować "--" w podglądzie edytora.
        assert_eq!(icon_result["data"]["text"], "");

        // Brak pliku wybranej paczki nie może spaść do ikony domyślnej.
        let missing = CString::new("NoSuchPack").unwrap();
        let missing_result = take(rw_format_weather_field(
            icon.as_ptr(), overrides.as_ptr(), pl.as_ptr(), root_arg.as_ptr(), missing.as_ptr(),
        ));
        assert_eq!(missing_result["ok"], true, "error: {}", missing_result["error"]);
        assert_eq!(missing_result["data"]["iconPath"], "");
    }

    #[test]
    fn weather_conditions_ffi_returns_localized_names() {
        let pl = CString::new("pl").unwrap();
        let en = CString::new("en").unwrap();
        let pl_result = take(rw_weather_conditions(pl.as_ptr()));
        assert_eq!(pl_result["ok"], true, "error: {}", pl_result["error"]);
        let conditions = pl_result["data"]["conditions"].as_array().unwrap();
        assert_eq!(conditions.len(), 29);
        assert_eq!(conditions[0]["code"], 0);
        assert_eq!(conditions[0]["name"], "Pochmurno");
        assert_eq!(conditions[15]["name"], "Burza");

        let en_result = take(rw_weather_conditions(en.as_ptr()));
        let en_conditions = en_result["data"]["conditions"].as_array().unwrap();
        assert_eq!(en_conditions[0]["name"], "Cloudy");
        assert_eq!(en_conditions[15]["name"], "Thunderstorm");
    }

    #[test]
    fn preview_default_datetime_ffi_matches_source_frame() {
        let value = take(rw_preview_default_datetime());
        assert_eq!(value["ok"], true, "error: {}", value["error"]);
        let data = &value["data"];
        assert_eq!(data["day"], 31);
        assert_eq!(data["month"], 7);
        assert_eq!(data["year"], 2022);
        assert_eq!(data["hour"], 21);
        assert_eq!(data["minute"], 37);
        assert_eq!(data["second"], 40);

        let preset = rusty_core::preview::FRAME_PRESETS
            [rusty_core::preview::PREVIEW_FRAME_INDEX];
        assert_eq!(data["day"], preset.date.day);
        assert_eq!(data["month"], preset.date.month);
        assert_eq!(data["year"], preset.date.year);
        assert_eq!(data["hour"], preset.hour);
        assert_eq!(data["minute"], preset.minute);
        assert_eq!(data["second"], preset.second);
    }

    #[test]
    fn images_store_roundtrip() {
        let root = std::env::temp_dir().join(format!("rw-ffi-images-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Image Face").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 77));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let source = Path::new(file).parent().unwrap().join("assets/icon.png");
        let source_arg = CString::new(source.to_string_lossy().as_bytes()).unwrap();

        let added = take(rw_images_add(root_arg.as_ptr(), source_arg.as_ptr()));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);
        assert_eq!(added["data"]["image"]["file"], "icon.png");
        let listed = take(rw_images_list(root_arg.as_ptr()));
        assert_eq!(listed["data"]["images"].as_array().unwrap().len(), 1);

        let image = CString::new("icon.png").unwrap();
        let removed = take(rw_images_remove(root_arg.as_ptr(), image.as_ptr()));
        assert_eq!(removed["ok"], true, "error: {}", removed["error"]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn images_widget_ffi_attaches_and_exports_scaled_png() {
        let root = std::env::temp_dir().join(format!("rw-ffi-images-widget-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Picture Face").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 78));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        // Źródło: istniejący plik icon.png z nowego projektu.
        let source = Path::new(file).parent().unwrap().join("assets/icon.png");
        let source_arg = CString::new(source.to_string_lossy().as_bytes()).unwrap();
        let added = take(rw_images_add(root_arg.as_ptr(), source_arg.as_ptr()));
        assert_eq!(added["ok"], true, "error: {}", added["error"]);

        let kind = CString::new("images").unwrap();
        let widget = take(rw_add_widget(
            project_file.as_ptr(),
            kind.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(widget["ok"], true, "error: {}", widget["error"]);
        let id = widget["data"]["id"].as_str().unwrap();
        let id_arg = CString::new(id).unwrap();
        let file_arg = CString::new("icon.png").unwrap();
        let fit = CString::new("cover").unwrap();
        let set = take(rw_set_widget_image(
            project_file.as_ptr(),
            id_arg.as_ptr(),
            file_arg.as_ptr(),
            fit.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(set["ok"], true, "error: {}", set["error"]);
        let layer = set["data"]["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["id"] == id)
            .unwrap()
            .clone();
        assert_eq!(layer["kind"], "images");
        assert_eq!(layer["imageFit"], "cover");
        let src = layer["src"].as_str().unwrap();
        assert!(src.starts_with("images/"), "src={src}");

        let flat = rusty_core::export_paths::exported_asset_name(src);
        let js = rusty_core::codegen::generate_index_js(&rusty_core::load_project(Path::new(file)).unwrap());
        assert!(js.contains("hmUI.widget.IMG"));
        assert!(js.contains(&flat), "js: {js}");

        let zip_path = root.join("images.zip");
        let output = CString::new(zip_path.to_string_lossy().as_bytes()).unwrap();
        let exported = take(rw_export_zip(project_file.as_ptr(), output.as_ptr()));
        assert_eq!(exported["ok"], true, "error: {}", exported["error"]);
        let names = rusty_core::inspect_zip(&zip_path).unwrap();
        assert!(names.contains(&format!("assets/{flat}")), "{names}");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn format_time_honours_watchface_language() {
        let pattern = CString::new("h a").unwrap();
        let pl = CString::new("pl").unwrap();
        let en = CString::new("en").unwrap();

        let polish = take(rw_format_time(pattern.as_ptr(), 15, 0, 0, pl.as_ptr()));
        assert_eq!(polish["ok"], true, "error: {}", polish["error"]);
        assert_eq!(polish["data"]["text"], "3 pop.");

        let english = take(rw_format_time(pattern.as_ptr(), 15, 0, 0, en.as_ptr()));
        assert_eq!(english["ok"], true, "error: {}", english["error"]);
        assert_eq!(english["data"]["text"], "3 pm");

        let polish_month = take(rw_month_name(10, pl.as_ptr()));
        assert_eq!(polish_month["ok"], true, "error: {}", polish_month["error"]);
        assert_eq!(polish_month["data"]["name"], "Lis");
        let english_month = take(rw_month_name(10, en.as_ptr()));
        assert_eq!(english_month["data"]["name"], "Nov");
    }

    #[test]
    fn format_date_honours_watchface_language_and_pattern() {
        let pattern = CString::new("DD.MM.YYYY - MMMM").unwrap();
        let pl = CString::new("pl").unwrap();
        let en = CString::new("en").unwrap();

        let polish = take(rw_format_date(pattern.as_ptr(), 3, 0, 2026, pl.as_ptr()));
        assert_eq!(polish["ok"], true, "error: {}", polish["error"]);
        assert_eq!(polish["data"]["text"], "03.01.2026 - Styczeń");

        let english = take(rw_format_date(pattern.as_ptr(), 3, 0, 2026, en.as_ptr()));
        assert_eq!(english["ok"], true, "error: {}", english["error"]);
        assert_eq!(english["data"]["text"], "03.01.2026 - January");

        let invalid = CString::new("").unwrap();
        let rejected = take(rw_format_date(invalid.as_ptr(), 3, 0, 2026, en.as_ptr()));
        assert_eq!(rejected["ok"], false);
    }

    #[test]
    fn preview_generation_creates_branded_png_and_8_frame_gif() {
        let root = std::env::temp_dir().join(format!("rw-ffi-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let dir_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let name = CString::new("Preview Face").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(dir_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 12_345_678));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let project_file = created["data"]["file"].as_str().unwrap().to_string();
        let project_arg = CString::new(project_file.as_bytes()).unwrap();

        let locale = CString::new("en").unwrap();
        let series = take(rw_preview_random_series(locale.as_ptr()));
        assert_eq!(series["ok"], true, "error: {}", series["error"]);
        let series_json = series["data"].to_string();
        let series_arg = CString::new(series_json.as_bytes()).unwrap();

        for index in 0..8u32 {
            let frame = take(rw_render_preview_frame(
                project_arg.as_ptr(),
                index,
                series_arg.as_ptr(),
            ));
            assert_eq!(frame["ok"], true, "frame {index}: {}", frame["error"]);
        }
        let encoded = take(rw_encode_preview_gif(project_arg.as_ptr(), series_arg.as_ptr()));
        assert_eq!(encoded["ok"], true, "error: {}", encoded["error"]);
        assert_eq!(encoded["data"]["frames"], 8);
        assert_eq!(encoded["data"]["referenceFrame"], 1);

        let png = std::path::Path::new(&project_file)
            .parent()
            .unwrap()
            .join("preview.png");
        let gif = png.with_extension("gif");
        assert!(png.is_file() && gif.is_file());
        assert_eq!(image::open(&png).unwrap().dimensions(), (466, 466));
        assert!(std::fs::metadata(&gif).unwrap().len() > 1000);
        assert!(!png.parent().unwrap().join("preview-cache").exists());

        // Klatka 8 jest dokładną kopią 21:37:40 — surowe rendery są identyczne.
        let seventh = take(rw_render_preview_frame(
            project_arg.as_ptr(),
            6,
            series_arg.as_ptr(),
        ));
        let eighth = take(rw_render_preview_frame(
            project_arg.as_ptr(),
            7,
            series_arg.as_ptr(),
        ));
        assert_eq!(seventh["ok"], true, "error: {}", seventh["error"]);
        assert_eq!(eighth["ok"], true, "error: {}", eighth["error"]);
        let frame7 = std::path::Path::new(seventh["data"]["frame"].as_str().unwrap());
        let frame8 = std::path::Path::new(eighth["data"]["frame"].as_str().unwrap());
        assert_eq!(std::fs::read(frame7).unwrap(), std::fs::read(frame8).unwrap());

        // preview.png jest oprawiony — nie jest surową klatką: przezroczyste rogi.
        let branded = image::open(&png).unwrap().to_rgba8();
        assert_eq!(branded.get_pixel(0, 0).0[3], 0);
        assert_eq!(branded.get_pixel(465, 465).0[3], 0);
        let brown = branded.get_pixel(233, 40).0;
        assert!(brown[0] > brown[2], "brak brązowego pierścienia: {brown:?}");
        assert_ne!(std::fs::read(&png).unwrap(), std::fs::read(frame8).unwrap());
        std::fs::remove_dir_all(frame7.parent().unwrap()).ok();
        let gif_bytes = std::fs::read(&gif).unwrap();
        assert!(gif_bytes.windows(11).any(|w| w == b"NETSCAPE2.0"));
        assert!(gif_bytes.windows(2).any(|w| w == [0xAA, 0x00]), "delay != 170");

        // Ikona 314x314 trafia do ZIP jako oprawiony podgląd; GIF zostaje lokalny.
        let zip = png.parent().unwrap().join("Preview.Face.zip");
        let exported = rusty_core::export_zip(std::path::Path::new(&project_file), &zip).unwrap();
        let listing = rusty_core::inspect_zip(&exported).unwrap();
        assert!(listing.contains("assets/Preview.png"), "listing: {listing}");
        assert!(!listing.contains("preview.gif"), "listing: {listing}");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn system_widgets_roundtrip_through_ffi() {
        let root = std::env::temp_dir().join(format!("rw-ffi-system-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../native/RustyWatch/Assets");
        let root_arg = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let fonts_arg = CString::new(bundled.join("Fonts").to_string_lossy().as_bytes()).unwrap();
        let hands_arg = CString::new(bundled.join("Hands").to_string_lossy().as_bytes()).unwrap();
        let status_arg = CString::new(bundled.join("Status").to_string_lossy().as_bytes()).unwrap();
        let weather_arg = CString::new(bundled.join("Weather").to_string_lossy().as_bytes()).unwrap();
        assert_eq!(
            take(rw_ensure_defaults(
                root_arg.as_ptr(),
                fonts_arg.as_ptr(),
                hands_arg.as_ptr(),
                status_arg.as_ptr(),
                weather_arg.as_ptr()
            ))["ok"],
            true
        );

        let name = CString::new("System Test").unwrap();
        let author = CString::new("Dev").unwrap();
        let created = take(rw_create_project(root_arg.as_ptr(), name.as_ptr(), author.as_ptr(), 81));
        assert_eq!(created["ok"], true, "error: {}", created["error"]);
        let file = created["data"]["file"].as_str().unwrap();
        let project_file = CString::new(file).unwrap();

        for kind_name in ["battery", "status-alarm", "status-dnd", "status-bluetooth", "status-lock"] {
            let kind = CString::new(kind_name).unwrap();
            let added = take(rw_add_widget(
                project_file.as_ptr(),
                kind.as_ptr(),
                root_arg.as_ptr(),
                hands_arg.as_ptr(),
                status_arg.as_ptr(),
            ));
            assert_eq!(added["ok"], true, "kind={kind_name} error: {}", added["error"]);
        }

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        let js = rusty_core::codegen::generate_index_js(&project);
        assert!(js.contains("hmSensor.createSensor(hmSensor.id.BATTERY)"), "{js}");
        assert!(js.contains("hmUI.widget.IMG_STATUS"), "{js}");
        assert!(js.contains("hmUI.system_status.DISTURB"), "{js}");
        assert!(js.contains("hmUI.system_status.DISCONNECT"), "{js}");
        assert!(!js.contains("TODO: connect"), "{js}");

        let widget_id = project
            .widgets
            .iter()
            .find(|w| w.kind == rusty_core::model::WidgetKind::StatusIcon)
            .unwrap()
            .id
            .clone();
        let id_arg = CString::new(widget_id).unwrap();
        // `Classic` jest jedynym wbudowanym pakietem — żeby przetestować
        // przełączanie, dodajemy zwykłą paczkę z plików.
        let custom = root.join("Custom");
        std::fs::create_dir_all(&custom).unwrap();
        for file in ["alarm.png", "dnd.png", "bluetooth.png", "lock.png"] {
            std::fs::copy(bundled.join("Status").join("Classic").join(file), custom.join(file))
                .unwrap();
        }
        let folder_arg = CString::new(custom.to_string_lossy().as_bytes()).unwrap();
        assert_eq!(
            take(rw_status_packs_add(root_arg.as_ptr(), folder_arg.as_ptr()))["ok"],
            true
        );
        let custom_pack = CString::new("Custom").unwrap();
        let switched = take(rw_set_status_pack(
            project_file.as_ptr(),
            id_arg.as_ptr(),
            custom_pack.as_ptr(),
            root_arg.as_ptr(),
        ));
        assert_eq!(switched["ok"], true, "error: {}", switched["error"]);

        let project = rusty_core::load_project(Path::new(file)).unwrap();
        assert!(project.assets.iter().any(|a| a.path == "status/Custom/alarm.png"));
        let flat = rusty_core::export_paths::exported_asset_name("status/Custom/alarm.png");
        let js = rusty_core::codegen::generate_index_js(&project);
        assert!(js.contains(&flat), "{js}");

        let zip_path = root.join("System.zip");
        let output = CString::new(zip_path.to_string_lossy().as_bytes()).unwrap();
        let exported = take(rw_export_zip(project_file.as_ptr(), output.as_ptr()));
        assert_eq!(exported["ok"], true, "error: {}", exported["error"]);
        let listing = rusty_core::inspect_zip(&zip_path).unwrap();
        assert!(listing.contains(&format!("assets/{flat}")), "{listing}");

        std::fs::remove_dir_all(&root).ok();
    }
}



