//! Magazyn zestawów wskazówek do tarczy analogowej.
//!
//! Zestawy żyją w `%APPDATA%\RustyWatch\Hands\<id>\` i zawierają pliki
//! `hour.png`, `minute.png`, opcjonalnie `second.png` i `cap.png`. Każdy plik
//! to przezroczysty PNG z wskazówką narysowaną na środku — tarcza obraca go
//! wokół środka obrazu (`center_x`/`center_y` w ZeppOS).
//!
//! Wybrany zestaw jest kopiowany do `<projekt>/assets/hands/<id>/`, dzięki czemu
//! wyeksportowany ZIP jest samowystarczalny. Wbudowany zestaw `Classic` (z lekkim
//! cieniem) jest instalowany przy pierwszym uruchomieniu aplikacji.

use image::imageops::FilterType;
use image::{GenericImageView, ImageEncoder, Rgba, RgbaImage};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::asset_cache;
use crate::codegen::HandPlacement;
use crate::model::{Asset, AssetRole, Project, WidgetKind, SCREEN_SIZE};
use crate::preview::rotate_hand;

pub const HAND_DIR: &str = "Hands";
pub const DEFAULT_HAND_SET: &str = "Classic";
pub const JS_HAND_PREFIX: &str = "hands/";

const HOUR_FILE: &str = "hour.png";
const MINUTE_FILE: &str = "minute.png";
const SECOND_FILE: &str = "second.png";
const CAP_FILE: &str = "cap.png";
const REQUIRED_FILES: [&str; 2] = [HOUR_FILE, MINUTE_FILE];
const OPTIONAL_FILES: [&str; 2] = [SECOND_FILE, CAP_FILE];

/// Nazwy wszystkich plików zestawu — jedyne źródło odcisku (opcjonalne są pomijane).
fn hand_file_names() -> [&'static str; 4] {
    [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE]
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandSet {
    pub id: String,
    pub name: String,
    pub has_second: bool,
    pub has_cap: bool,
    /// Bok kwadratowego obrazu wskazówki w pikselach (np. 400).
    pub size: u32,
    /// Odcisk plików wskazówek — miniaturka jest odświeżana tylko przy jego zmianie.
    pub fingerprint: String,
}

pub fn hands_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join(HAND_DIR)
}

pub fn hand_set_dir(app_data_root: &Path, id: &str) -> PathBuf {
    hands_dir(app_data_root).join(id)
}

pub fn js_hand_path(id: &str, file: &str) -> String {
    format!("{JS_HAND_PREFIX}{id}/{file}")
}

/// Kopiuje wbudowane zestawy do magazynu i odświeża je, gdy źródło się zmieniło.
///
/// Nie wolno tu wcześnie wracać po zainstalowaniu `Classic`: nowe zestawy
/// dołączone do `Assets/Hands` muszą pojawić się także w istniejących
/// instalacjach po aktualizacji (bez reinstallu i bez kasowania danych).
pub fn ensure_default(app_data_root: &Path, bundled_dir: &Path) -> Result<(), String> {
    let classic = bundled_dir.join(DEFAULT_HAND_SET);
    if !classic.join(HOUR_FILE).is_file() || !classic.join(MINUTE_FILE).is_file() {
        return Err(format!(
            "Nie znaleziono wbudowanego zestawu wskazówek: {}",
            classic.display()
        ));
    }

    let entries = fs::read_dir(bundled_dir)
        .map_err(|e| format!("Nie można odczytać wbudowanych wskazówek: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać zestawu wskazówek: {e}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        // Katalogi pomocnicze (np. README) nie są zestawami wskazówek.
        if !entry.path().join(HOUR_FILE).is_file()
            || !entry.path().join(MINUTE_FILE).is_file()
        {
            continue;
        }
        valid_set_id(&id)?;
        let bundled = entry.path();
        let bundled_fingerprint = asset_cache::files_fingerprint(&bundled, &hand_file_names())?;
        let target = hand_set_dir(app_data_root, &id);
        // Stare instalacje nie mają znacznika — wtedy również trzeba nadpisać
        // pliki, żeby poprawione wskazówki dotarły do użytkownika po aktualizacji.
        let unchanged = target.join(HOUR_FILE).is_file()
            && target.join(MINUTE_FILE).is_file()
            && asset_cache::read_bundled_version(&target).as_deref()
                == Some(bundled_fingerprint.as_str());
        if unchanged {
            continue;
        }
        fs::create_dir_all(&target)
            .map_err(|e| format!("Nie można utworzyć katalogu wskazówek: {e}"))?;
        for file in REQUIRED_FILES.iter().chain(OPTIONAL_FILES.iter()) {
            let from = bundled.join(file);
            if from.is_file() {
                fs::copy(&from, target.join(file))
                    .map_err(|e| format!("Nie można skopiować wskazówki {file}: {e}"))?;
            } else if OPTIONAL_FILES.contains(file) {
                // Źródło już nie ma tej wskazówki — nie zostawiaj starej kopii.
                let stale = target.join(file);
                if stale.is_file() {
                    let _ = fs::remove_file(stale);
                }
            }
        }
        asset_cache::write_bundled_version(&target, &bundled_fingerprint)?;
        generate_preview(app_data_root, &id)?;
    }
    ensure_previews(app_data_root)?;
    Ok(())
}

/// Generuje `preview.png` tylko dla zestawów, których pliki się zmieniły.
pub fn ensure_previews(app_data_root: &Path) -> Result<(), String> {
    for set in list(app_data_root)? {
        let dir = hand_set_dir(app_data_root, &set.id);
        if asset_cache::fingerprint_matches(&dir, &hand_file_names())? {
            continue;
        }
        generate_preview(app_data_root, &set.id)?;
    }
    Ok(())
}

/// Rysuje jedną miniaturkę zestawu (hour → minute → second → cap).
pub fn generate_preview(app_data_root: &Path, id: &str) -> Result<(), String> {
    const SIZE: u32 = 128;
    const HAND: u32 = 96;
    let source = hand_set_dir(app_data_root, id);
    let mut canvas = RgbaImage::from_pixel(SIZE, SIZE, Rgba([0, 0, 0, 0]));
    // 21:37:11 — godz. 288.5°, min. 223.1°, sek. 66°.
    for (file, angle) in [
        (HOUR_FILE, 288.5),
        (MINUTE_FILE, 223.1),
        (SECOND_FILE, 66.0),
        (CAP_FILE, 0.0),
    ] {
        let path = source.join(file);
        if !path.is_file() {
            continue;
        }
        let original = image::open(&path)
            .map_err(|e| format!("Nie można odczytać wskazówki {file}: {e}"))?
            .to_rgba8();
        let hand = image::imageops::resize(&original, HAND, HAND, FilterType::Lanczos3);
        let rotated = rotate_hand(&hand, angle);
        image::imageops::overlay(&mut canvas, &rotated, 16, 16);
    }
    fs::write(source.join("preview.png"), encode_png(&canvas)?)
        .map_err(|e| format!("Nie można zapisać miniaturki wskazówek: {e}"))?;
    asset_cache::write_fingerprint(&source, &hand_file_names())?;
    Ok(())
}

/// Lista zainstalowanych zestawów. Zestaw bez `hour.png`/`minute.png` jest pomijany.
pub fn list(app_data_root: &Path) -> Result<Vec<HandSet>, String> {
    let dir = hands_dir(app_data_root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Nie można odczytać katalogu wskazówek: {e}")),
    };
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać zestawu wskazówek: {e}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if let Some(set) = describe_set(&entry.path(), &id) {
            result.push(set);
        }
    }
    result.sort_by(|a, b| {
        (a.id != DEFAULT_HAND_SET)
            .cmp(&(b.id != DEFAULT_HAND_SET))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

/// Dodaje zestaw z wybranego folderu. Wymaga `hour.png` i `minute.png`;
/// `second.png` i `cap.png` są opcjonalne.
pub fn add(app_data_root: &Path, folder: &Path) -> Result<HandSet, String> {
    if !folder.is_dir() {
        return Err(format!("Folder nie istnieje: {}", folder.display()));
    }
    for file in REQUIRED_FILES {
        if !folder.join(file).is_file() {
            return Err(format!("W folderze brakuje pliku {file}"));
        }
    }
    let size = read_size(&folder.join(HOUR_FILE))?;
    if read_size(&folder.join(MINUTE_FILE))? != size {
        return Err("Wskazówki godzin i minut muszą mieć ten sam rozmiar".to_string());
    }
    for file in OPTIONAL_FILES {
        let path = folder.join(file);
        if path.is_file() && read_size(&path)? != size {
            return Err(format!("Plik {file} ma inny rozmiar niż pozostałe wskazówki"));
        }
    }

    let raw_name = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Hands");
    let id = sanitize_set_id(raw_name);
    valid_set_id(&id)?;
    let target = hand_set_dir(app_data_root, &id);
    if target.exists() {
        return Err(format!("Zestaw wskazówek {id} jest już zainstalowany"));
    }
    fs::create_dir_all(&target).map_err(|e| format!("Nie można utworzyć zestawu: {e}"))?;
    for file in REQUIRED_FILES.iter().chain(OPTIONAL_FILES.iter()) {
        let from = folder.join(file);
        if from.is_file() {
            fs::copy(&from, target.join(file))
                .map_err(|e| format!("Nie można skopiować {file}: {e}"))?;
        }
    }
    generate_preview(app_data_root, &id)?;
    let fingerprint = asset_cache::files_fingerprint(&target, &hand_file_names())?;
    Ok(HandSet {
        id: id.clone(),
        name: id,
        has_second: target.join(SECOND_FILE).is_file(),
        has_cap: target.join(CAP_FILE).is_file(),
        size,
        fingerprint,
    })
}

/// Usuwa zestaw z magazynu. Kopie w projektach zostają nienaruszone.
pub fn remove(app_data_root: &Path, id: &str) -> Result<(), String> {
    valid_set_id(id)?;
    if id == DEFAULT_HAND_SET {
        return Err("Wbudowanego zestawu wskazówek nie można usunąć".to_string());
    }
    let path = hand_set_dir(app_data_root, id);
    if !path.is_dir() {
        return Err(format!("Zestaw wskazówek nie jest zainstalowany: {id}"));
    }
    fs::remove_dir_all(&path).map_err(|e| format!("Nie można usunąć zestawu wskazówek: {e}"))
}

/// Przypisuje zestaw do widgetu analogowego i kopiuje jego pliki do projektu.
///
/// Nowy widget (`hand_set == None`) dostaje ramkę wielkości plików, wyśrodkowaną
/// na tarczy. Podmiana zestawu w istniejącym widgetcie zachowuje jednak jego
/// pozycję i rozmiar — użytkownik mógł już ramkę przesunąć i przeskalować, więc
/// zmiana wyglądu wskazówek nie może jej cofać. Nowe pliki są następnie
/// dopasowywane do tej ramki przez `scale_project_hands`.
pub fn attach_to_project(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
    set_id: &str,
) -> Result<u32, String> {
    valid_set_id(set_id)?;
    let source = hand_set_dir(app_data_root, set_id);
    let set = describe_set(&source, set_id)
        .ok_or_else(|| format!("Zestaw wskazówek nie jest zainstalowany: {set_id}"))?;

    let target_dir = project_dir.join("assets").join(JS_HAND_PREFIX).join(set_id);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu wskazówek projektu: {e}"))?;
    for file in REQUIRED_FILES.iter().chain(OPTIONAL_FILES.iter()) {
        let from = source.join(file);
        if from.is_file() {
            fs::copy(&from, target_dir.join(file))
                .map_err(|e| format!("Nie można skopiować wskazówki do projektu: {file}: {e}"))?;
            let path = js_hand_path(set_id, file);
            if !project.assets.iter().any(|a| a.path == path) {
                project.assets.push(Asset {
                    path,
                    role: AssetRole::Hand,
                });
            }
        }
    }

    let widget = project
        .widgets
        .iter_mut()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::AnalogTime)
        .ok_or_else(|| format!("Nie znaleziono widgetu analogowego: {widget_id}"))?;
    // Podmiana zestawu nie może cofnąć ręcznie ustawionej ramki.
    let frame = widget.hand_set.as_ref().map(|_| (widget.x, widget.y, widget.w, widget.h));
    widget.hand_set = Some(set_id.to_string());
    widget.hour_src = Some(js_hand_path(set_id, HOUR_FILE));
    widget.minute_src = Some(js_hand_path(set_id, MINUTE_FILE));
    widget.second_src = set.has_second.then(|| js_hand_path(set_id, SECOND_FILE));
    widget.cap_src = set.has_cap.then(|| js_hand_path(set_id, CAP_FILE));
    if let Some((x, y, w, h)) = frame {
        widget.x = x;
        widget.y = y;
        widget.w = w;
        widget.h = h;
    } else {
        widget.w = set.size as i32;
        widget.h = set.size as i32;
        widget.x = ((SCREEN_SIZE - set.size) / 2) as i32;
        widget.y = ((SCREEN_SIZE - set.size) / 2) as i32;
    }
    Ok(set.size)
}

/// Włącza albo wyłącza poszczególne wskazówki (godzinę, minutę, sekundę i zaślepkę).
/// `None` oznacza „zostaw tak jak jest”. Włączenie przywraca plik z magazynu do
/// projektu, więc działa też wtedy, gdy wyłączenie usunęło go przez `prune_unused`.
pub fn set_hand_visibility(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
    hour: Option<bool>,
    minute: Option<bool>,
    second: Option<bool>,
    cap: Option<bool>,
) -> Result<(), String> {
    let set_id = project
        .widgets
        .iter()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::AnalogTime)
        .and_then(|w| w.hand_set.clone())
        .ok_or_else(|| format!("Nie znaleziono widgetu analogowego: {widget_id}"))?;
    valid_set_id(&set_id)?;

    let source_dir = hand_set_dir(app_data_root, &set_id);
    let target_dir = project_dir.join("assets").join(JS_HAND_PREFIX).join(&set_id);
    let enable = |file: &str| -> Result<Option<String>, String> {
        let from = source_dir.join(file);
        if !from.is_file() {
            return Ok(None);
        }
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Nie można utworzyć katalogu wskazówek projektu: {e}"))?;
        fs::copy(&from, target_dir.join(file))
            .map_err(|e| format!("Nie można skopiować wskazówki do projektu: {file}: {e}"))?;
        Ok(Some(js_hand_path(&set_id, file)))
    };

    let widget = project
        .widgets
        .iter_mut()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::AnalogTime)
        .ok_or_else(|| format!("Nie znaleziono widgetu analogowego: {widget_id}"))?;
    if let Some(value) = hour {
        widget.hour_src = if value { enable(HOUR_FILE)? } else { None };
    }
    if let Some(value) = minute {
        widget.minute_src = if value { enable(MINUTE_FILE)? } else { None };
    }
    if let Some(value) = second {
        widget.second_src = if value { enable(SECOND_FILE)? } else { None };
    }
    if let Some(value) = cap {
        widget.cap_src = if value { enable(CAP_FILE)? } else { None };
    }

    let used: Vec<String> = project
        .widgets
        .iter()
        .flat_map(|w| {
            [&w.hour_src, &w.minute_src, &w.second_src, &w.cap_src]
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect();
    for path in used {
        if !project.assets.iter().any(|a| a.path == path) {
            project.assets.push(Asset {
                path,
                role: AssetRole::Hand,
            });
        }
    }
    prune_unused(project, project_dir);
    Ok(())
}

/// Usuwa z projektu pliki i wpisy zasobów zestawów, których już żaden widget nie używa.
pub fn prune_unused(project: &mut Project, project_dir: &Path) {
    let mut used = std::collections::HashSet::new();
    for widget in &project.widgets {
        for src in [
            &widget.hour_src,
            &widget.minute_src,
            &widget.second_src,
            &widget.cap_src,
        ] {
            if let Some(src) = src {
                used.insert(src.as_str());
            }
        }
    }
    project.assets.retain(|asset| {
        if asset.role != AssetRole::Hand || used.contains(asset.path.as_str()) {
            return true;
        }
        let path = project_dir.join("assets").join(&asset.path);
        if path.is_file() {
            let _ = fs::remove_file(path);
        }
        false
    });
}

/// Skaluje dane PNG do zadanego rozmiaru, zwracając oryginalne bajty, gdy rozmiar
/// się zgadza. Filter Lanczos3 zachowuje przezroczystość i delikatny cień wskazówek.
fn resized_png(bytes: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let image =
        image::load_from_memory(bytes).map_err(|e| format!("Nie można odczytać wskazówki: {e}"))?;
    if image.dimensions() == (width, height) {
        return Ok(bytes.to_vec());
    }
    let resized = image
        .resize_exact(width, height, FilterType::Lanczos3)
        .to_rgba8();
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            resized.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Nie można zapisać wskazówki: {e}"))?;
    Ok(out)
}

/// Grafika wskazówki przygotowana do eksportu wraz z jej pozycją i pivotem.
#[derive(Debug, Clone)]
pub struct PreparedHand {
    pub bytes: Vec<u8>,
    pub placement: HandPlacement,
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Nie można zapisać wskazówki: {e}"))?;
    Ok(out)
}

/// Margines przezroczystości wokół przyciętej wskazówki — dokładnie jak
/// `imaging.pad_rect(..., 8)` w generatorze Python, z którego pochodzi
/// działająca tarcza referencyjna.
const HAND_CROP_MARGIN: u32 = 8;

/// Prostokąt nieprzezroczystych pikseli `(x, y, w, h)` — odpowiednik
/// `imaging.alpha_bbox()` w generatorze Python. `None` = obraz pusty.
fn alpha_bbox(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = image.dimensions();
    let mut left = width;
    let mut right = 0u32;
    let mut top = height;
    let mut bottom = 0u32;
    for y in 0..height {
        for x in 0..width {
            if image.get_pixel(x, y).0[3] > 0 {
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    if right < left || bottom < top {
        None
    } else {
        Some((left, top, right - left + 1, bottom - top + 1))
    }
}

/// Powiększa bbox o przezroczysty margines, nie wychodząc poza płótno.
fn pad_bbox(
    bbox: (u32, u32, u32, u32),
    width: u32,
    height: u32,
    padding: u32,
) -> (u32, u32, u32, u32) {
    let (x, y, w, h) = bbox;
    let x0 = x.saturating_sub(padding);
    let y0 = y.saturating_sub(padding);
    let x1 = (x + w + padding).min(width);
    let y1 = (y + h + padding).min(height);
    (x0, y0, x1.saturating_sub(x0).max(1), y1.saturating_sub(y0).max(1))
}

/// Przygotowuje wskazówki do eksportu dokładnie tak jak generator Python:
/// jedna wspólna skala dla całego zestawu i ciasne przycięcie do widocznej
/// treści z 8-pikselowym marginesem. `pos_x/pos_y` wskazują oryginalne miejsce
/// na płótnie, a obrót odbywa się wokół środka tarczy (233,233) w ramce
/// pełnoekranowej — tak jak w 100% działającej tarczy referencyjnej.
///
/// Pełnoekranowa tekstura 466×466 kosztuje ~868 KiB zdekodowanej pamięci;
/// cztery takie wskazówki potrafią zresetować realny firmware. Przycięte
/// wskazówki zajmują 20–50 KB każda i wyglądają identycznie.
pub fn prepare_hand_assets(
    project: &Project,
    project_dir: &Path,
) -> Result<HashMap<String, PreparedHand>, String> {
    let mut prepared = HashMap::new();
    for widget in project
        .widgets
        .iter()
        .filter(|w| w.kind == WidgetKind::AnalogTime)
    {
        let mut sources: Vec<(String, RgbaImage)> = Vec::new();
        for src in [
            &widget.hour_src,
            &widget.minute_src,
            &widget.second_src,
            &widget.cap_src,
        ] {
            let Some(src) = src else { continue };
            let bytes = fs::read(project_dir.join("assets").join(src))
                .map_err(|e| format!("Nie można odczytać wskazówki {src}: {e}"))?;
            let decoded = image::load_from_memory(&bytes)
                .map_err(|e| format!("Nie można odczytać wskazówki {src}: {e}"))?
                .to_rgba8();
            sources.push((src.clone(), decoded));
        }

        // `imaging.scale_to_box`: największe płótno zestawu mieści się w ramce.
        // Wszystkie wskazówki dostają tę samą skalę, więc ich wzajemne długości
        // pozostają takie same jak w edytorze.
        let canvas = sources
            .iter()
            .map(|(_, image)| image.width().max(image.height()))
            .max()
            .unwrap_or(1)
            .max(1);
        let scale = (widget.w.max(1) as f64).min(widget.h.max(1) as f64) / canvas as f64;
        let pivot = HandPlacement {
            x: widget.x + widget.w / 2,
            y: widget.y + widget.h / 2,
            w: 1,
            h: 1,
            center_x: widget.w / 2,
            center_y: widget.h / 2,
        };

        for (src, decoded) in sources {
            let target_w = ((decoded.width() as f64) * scale).round().max(1.0) as u32;
            let target_h = ((decoded.height() as f64) * scale).round().max(1.0) as u32;
            let scaled = if decoded.dimensions() == (target_w, target_h) {
                decoded
            } else {
                image::imageops::resize(&decoded, target_w, target_h, FilterType::Lanczos3)
            };

            let (bytes, placement) = match alpha_bbox(&scaled) {
                Some(bbox) => {
                    let (left, top, crop_w, crop_h) =
                        pad_bbox(bbox, scaled.width(), scaled.height(), HAND_CROP_MARGIN);
                    let cropped =
                        image::imageops::crop_imm(&scaled, left, top, crop_w, crop_h).to_image();
                    (
                        encode_png(&cropped)?,
                        HandPlacement {
                            x: widget.x + left as i32,
                            y: widget.y + top as i32,
                            w: crop_w as i32,
                            h: crop_h as i32,
                            center_x: pivot.center_x,
                            center_y: pivot.center_y,
                        },
                    )
                }
                None => (
                    encode_png(&RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])))?,
                    pivot.clone(),
                ),
            };
            prepared.insert(
                src,
                PreparedHand {
                    bytes,
                    placement,
                },
            );
        }
    }
    Ok(prepared)
}

/// Bajty zasobu wstawiane do eksportowanego ZIP-a. Przycięcie wskazówek odbywa
/// się w `prepare_hand_assets`; pozostałe zasoby przechodzą bez zmian.
pub fn export_asset_bytes(
    _project: &Project,
    project_dir: &Path,
    path: &str,
) -> Result<Vec<u8>, String> {
    fs::read(project_dir.join("assets").join(path))
        .map_err(|e| format!("Nie można odczytać zasobu {path}: {e}"))
}

/// Fizycznie dopasowuje kopie wskazówek w projekcie do nowego rozmiaru ramki.
/// Źródłem jest oryginał z magazynu (gdy dostępny), żeby kolejne skalowania nie
/// kumulowały strat jakości. Wywoływane po zmianie geometrii widgetu analogowego
/// oraz po ponownym włączeniu wskazówki.
pub fn scale_project_hands(
    project: &Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
) -> Result<(), String> {
    let widget = project
        .widgets
        .iter()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::AnalogTime)
        .ok_or_else(|| format!("Nie znaleziono widgetu analogowego: {widget_id}"))?;
    let size = (widget.w.max(1) as u32, widget.h.max(1) as u32);
    let Some(set_id) = &widget.hand_set else {
        return Ok(());
    };
    let source_dir = hand_set_dir(app_data_root, set_id);

    for src in [
        &widget.hour_src,
        &widget.minute_src,
        &widget.second_src,
        &widget.cap_src,
    ] {
        let Some(src) = src else { continue };
        let target = project_dir.join("assets").join(src);
        if !target.is_file() {
            continue;
        }
        let file_name = Path::new(src)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let original = source_dir.join(file_name);
        let input = if original.is_file() {
            fs::read(&original)
                .map_err(|e| format!("Nie można odczytać wskazówki {file_name}: {e}"))?
        } else {
            fs::read(&target).map_err(|e| format!("Nie można odczytać wskazówki {src}: {e}"))?
        };
        let png = resized_png(&input, size.0, size.1)?;
        if png != fs::read(&target).unwrap_or_default() {
            fs::write(&target, &png)
                .map_err(|e| format!("Nie można zapisać wskazówki {src}: {e}"))?;
        }
    }
    Ok(())
}

fn describe_set(dir: &Path, id: &str) -> Option<HandSet> {
    if !dir.join(HOUR_FILE).is_file() || !dir.join(MINUTE_FILE).is_file() {
        return None;
    }
    let size = read_size(&dir.join(HOUR_FILE)).ok()?;
    let fingerprint = asset_cache::files_fingerprint(dir, &hand_file_names()).ok()?;
    Some(HandSet {
        id: id.to_string(),
        name: id.to_string(),
        has_second: dir.join(SECOND_FILE).is_file(),
        has_cap: dir.join(CAP_FILE).is_file(),
        size,
        fingerprint,
    })
}

fn read_size(path: &Path) -> Result<u32, String> {
    let image = image::open(path).map_err(|e| format!("Nie można odczytać wskazówki: {e}"))?;
    let (w, h) = image.dimensions();
    if w != h || w == 0 {
        return Err("Wskazówki muszą być kwadratowymi plikami PNG".to_string());
    }
    Ok(w)
}

fn valid_set_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || id != sanitize_set_id(id)
    {
        return Err("Nieprawidłowa nazwa zestawu wskazówek".to_string());
    }
    Ok(())
}

fn sanitize_set_id(name: &str) -> String {
    // Id trafia do ścieżek w pakiecie tarczy — transliterujemy polskie znaki,
    // żeby nazwa zestawu nie zepsuła buildu zeusa na zegarku.
    let transliterated: String = name
        .chars()
        .map(|c| match c {
            'ą' | 'Ą' => 'a',
            'ć' | 'Ć' => 'c',
            'ę' | 'Ę' => 'e',
            'ł' | 'Ł' => 'l',
            'ń' | 'Ń' => 'n',
            'ó' | 'Ó' => 'o',
            'ś' | 'Ś' => 's',
            'ź' | 'Ź' => 'z',
            'ż' | 'Ż' => 'z',
            other => other,
        })
        .collect();
    let safe: String = transliterated
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim_matches('_');
    if safe.is_empty() {
        "Hands".to_string()
    } else {
        safe.chars().take(60).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{create_project_in, NewProject};
    use image::{Rgba, RgbaImage};
    use std::time::{Duration, SystemTime};

    /// Zapisuje wskazówkę tego samego rozmiaru, ale z inną treścią i czasem modyfikacji.
    fn rewrite_hand(path: &Path, size: u32, color: Rgba<u8>) {
        crate::project::write_png(path, &RgbaImage::from_pixel(size, size, color)).unwrap();
        let later = SystemTime::now() + Duration::from_secs(60);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(later)
            .unwrap();
    }

    fn make_hand(dir: &Path, name: &str, size: u32) {
        fs::create_dir_all(dir).unwrap();
        crate::project::write_png(
            &dir.join(name),
            &RgbaImage::from_pixel(size, size, Rgba([200, 200, 210, 255])),
        )
        .unwrap();
    }

    #[test]
    fn pack_preview_is_rendered_at_21_37_11() {
        let root = std::env::temp_dir().join(format!("rw-hand-preview-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let pack = hand_set_dir(&root, "Preview");
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            make_hand(&pack, file, 32);
        }
        generate_preview(&root, "Preview").unwrap();
        let preview = image::open(pack.join("preview.png")).unwrap();
        assert_eq!(preview.dimensions(), (128, 128));
        fs::remove_dir_all(&root).ok();
    }

    /// Wskazówka na przezroczystym płótnie 400×400 z grafiką tylko w danym prostokącie.
    fn make_sparse_hand(dir: &Path, name: &str, left: u32, top: u32, width: u32, height: u32) {
        fs::create_dir_all(dir).unwrap();
        let mut image = RgbaImage::from_pixel(400, 400, Rgba([0, 0, 0, 0]));
        for y in top..top + height {
            for x in left..left + width {
                image.put_pixel(x, y, Rgba([220, 220, 230, 255]));
            }
        }
        crate::project::write_png(&dir.join(name), &image).unwrap();
    }

    #[test]
    fn store_lists_adds_and_removes_hand_sets() {
        let root = std::env::temp_dir().join(format!("rw-hands-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        ensure_default(&root, Path::new("__missing__")).expect_err("missing bundled set");
        let bundled = root.join("bundled");
        make_hand(&bundled.join(DEFAULT_HAND_SET), HOUR_FILE, 400);
        make_hand(&bundled.join(DEFAULT_HAND_SET), MINUTE_FILE, 400);
        make_hand(&bundled.join(DEFAULT_HAND_SET), SECOND_FILE, 400);
        make_hand(&bundled.join(DEFAULT_HAND_SET), CAP_FILE, 400);
        // Drugi, wbudowany zestaw musi zostać zainstalowany również wtedy,
        // gdy `Classic` już istnieje w magazynie (aktualizacja aplikacji).
        make_hand(&bundled.join("Light"), HOUR_FILE, 300);
        make_hand(&bundled.join("Light"), MINUTE_FILE, 300);
        make_hand(&root.join("Hands").join(DEFAULT_HAND_SET), HOUR_FILE, 400);
        make_hand(&root.join("Hands").join(DEFAULT_HAND_SET), MINUTE_FILE, 400);
        make_hand(&root.join("Hands").join(DEFAULT_HAND_SET), SECOND_FILE, 400);
        make_hand(&root.join("Hands").join(DEFAULT_HAND_SET), CAP_FILE, 400);
        ensure_default(&root, &bundled).unwrap();
        ensure_default(&root, &bundled).unwrap();
        let sets = list(&root).unwrap();
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].id, DEFAULT_HAND_SET);
        assert!(sets[0].has_second && sets[0].has_cap);
        assert_eq!(sets[0].size, 400);
        assert_eq!(sets[1].id, "Light");
        assert_eq!(sets[1].size, 300);
        assert!(remove(&root, DEFAULT_HAND_SET).is_err());

        let custom = root.join("Moje Wskazówki");
        make_hand(&custom, HOUR_FILE, 300);
        make_hand(&custom, MINUTE_FILE, 300);
        make_hand(&custom, CAP_FILE, 300);
        let added = add(&root, &custom).unwrap();
        assert_eq!(added.id, "Moje_Wskazowki");
        assert!(!added.has_second);
        assert_eq!(list(&root).unwrap().len(), 3);
        remove(&root, &added.id).unwrap();
        assert_eq!(list(&root).unwrap().len(), 2);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn attach_set_to_analog_widget() {
        let root = std::env::temp_dir().join(format!("rw-hands-attach-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        make_hand(&hand_set_dir(&root, "Blue"), HOUR_FILE, 320);
        make_hand(&hand_set_dir(&root, "Blue"), MINUTE_FILE, 320);
        make_hand(&hand_set_dir(&root, "Blue"), SECOND_FILE, 320);

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject {
                name: "Analog".into(),
                author: "Dev".into(),
                app_id: 6,
            },
        )
        .unwrap();
        project.widgets[0].kind = WidgetKind::AnalogTime;
        project.widgets[0].id = "analog".to_string();

        attach_to_project(&mut project, &dir, &root, "analog", "Blue").unwrap();
        let widget = &project.widgets[0];
        assert_eq!(widget.hour_src.as_deref(), Some("hands/Blue/hour.png"));
        assert_eq!(widget.second_src.as_deref(), Some("hands/Blue/second.png"));
        assert_eq!(widget.cap_src, None);
        assert_eq!((widget.w, widget.h), (320, 320));
        assert_eq!((widget.x, widget.y), (73, 73));
        assert!(dir.join("assets/hands/Blue/hour.png").is_file());
        assert!(project
            .assets
            .iter()
            .any(|a| a.path == "hands/Blue/hour.png" && a.role == AssetRole::Hand));

        attach_to_project(&mut project, &dir, &root, "analog", "Blue").unwrap();
        assert_eq!(
            project
                .assets
                .iter()
                .filter(|a| a.role == AssetRole::Hand)
                .count(),
            3
        );
        prune_unused(&mut project, &dir);
        assert_eq!(
            project
                .assets
                .iter()
                .filter(|a| a.role == AssetRole::Hand)
                .count(),
            3
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn changing_hand_set_keeps_user_frame() {
        let root = std::env::temp_dir().join(format!("rw-hands-frame-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (id, size) in [("Full", 400u32), ("Small", 300u32)] {
            let set = hand_set_dir(&root, id);
            for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
                make_hand(&set, file, size);
            }
        }

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject {
                name: "Frame".into(),
                author: "Dev".into(),
                app_id: 12,
            },
        )
        .unwrap();
        project.widgets[0].kind = WidgetKind::AnalogTime;
        project.widgets[0].id = "analog".into();
        attach_to_project(&mut project, &dir, &root, "analog", "Full").unwrap();
        assert_eq!((project.widgets[0].w, project.widgets[0].h), (400, 400));
        assert_eq!((project.widgets[0].x, project.widgets[0].y), (33, 33));

        // Użytkownik przesunął i przeskalował ramkę, po czym zmienia wygląd.
        {
            let w = &mut project.widgets[0];
            w.x = 100;
            w.y = 120;
            w.w = 167;
            w.h = 167;
        }
        attach_to_project(&mut project, &dir, &root, "analog", "Small").unwrap();
        let widget = &project.widgets[0];
        assert_eq!((widget.x, widget.y, widget.w, widget.h), (100, 120, 167, 167));
        assert_eq!(widget.hand_set.as_deref(), Some("Small"));
        assert_eq!(widget.hour_src.as_deref(), Some("hands/Small/hour.png"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hands_are_cropped_to_tight_bbox_like_reference() {
        let root = std::env::temp_dir().join(format!("rw-hands-crop-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let set = hand_set_dir(&root, "Sparse");
        make_sparse_hand(&set, HOUR_FILE, 180, 50, 40, 200);
        make_sparse_hand(&set, MINUTE_FILE, 185, 20, 30, 230);
        make_sparse_hand(&set, CAP_FILE, 10, 10, 30, 30);

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject {
                name: "Cropped".into(),
                author: "Dev".into(),
                app_id: 10,
            },
        )
        .unwrap();
        project.widgets[0].kind = WidgetKind::AnalogTime;
        project.widgets[0].id = "analog".into();
        attach_to_project(&mut project, &dir, &root, "analog", "Sparse").unwrap();
        project.widgets[0].w = 200;
        project.widgets[0].h = 200;

        let prepared = prepare_hand_assets(&project, &dir).unwrap();
        let hour = &prepared["hands/Sparse/hour.png"];
        // Skala 0.5 z 400×400 na 200×200, bbox (90,25,20,100) + 8 px marginesu.
        assert!(hour.placement.w >= 30 && hour.placement.w <= 50, "{hour:?}");
        assert!(hour.placement.h >= 110 && hour.placement.h <= 125, "{hour:?}");
        assert!(hour.placement.w < hour.placement.h, "to nie jest kwadrat");
        // Lanczos3 rozlewa lekko alfa poza oryginalny prostokąt, więc dokładne
        // położenie zależy od filtra; istotne, że `pos_x` odtwarza miejsce
        // wskazówki na płótnie, a nie przesuwa jej do środka ramki.
        let offset_x = hour.placement.x - project.widgets[0].x;
        assert!((75..=85).contains(&offset_x), "offset_x={offset_x}");
        let bytes = image::load_from_memory(&hour.bytes).unwrap().to_rgba8();
        assert_eq!(
            bytes.dimensions(),
            (hour.placement.w as u32, hour.placement.h as u32)
        );
        // Zgodnie z referencją obrót wokół środka tarczy odbywa się w ramce
        // pełnoekranowej; pivot z `placement` nie jest używany przez JS.
        assert_eq!(hour.placement.center_x, 100);
        assert_eq!(hour.placement.center_y, 100);

        let cap = &prepared["hands/Sparse/cap.png"];
        assert!(cap.placement.w <= 50 && cap.placement.h <= 50, "{cap:?}");
        let total: u64 = prepared
            .values()
            .map(|hand| hand.placement.w as u64 * hand.placement.h as u64 * 4)
            .sum();
        assert!(total < 100 * 1024, "za dużo pamięci: {total} B");

        let placements = prepared
            .iter()
            .map(|(path, hand)| (path.clone(), hand.placement))
            .collect();
        let js = crate::codegen::generate_index_js_with_hands(&project, &placements);
        assert!(js.contains("center_x: 233"));
        assert!(js.contains(&format!("pos_x: {}", hour.placement.x)));
        assert!(js.contains("w: 466"));
        assert!(js.contains("auto_scale: false"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn horizontal_hand_stays_tight_and_uses_full_screen_rotation_frame() {
        let root = std::env::temp_dir().join(format!("rw-hands-square-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let set = hand_set_dir(&root, "Horizontal");
        make_sparse_hand(&set, HOUR_FILE, 120, 198, 160, 4);
        make_sparse_hand(&set, MINUTE_FILE, 185, 20, 30, 230);

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject {
                name: "Square".into(),
                author: "Dev".into(),
                app_id: 11,
            },
        )
        .unwrap();
        project.widgets[0].kind = WidgetKind::AnalogTime;
        project.widgets[0].id = "analog".into();
        attach_to_project(&mut project, &dir, &root, "analog", "Horizontal").unwrap();
        project.widgets[0].w = 200;
        project.widgets[0].h = 200;

        let prepared = prepare_hand_assets(&project, &dir).unwrap();
        let hour = &prepared["hands/Horizontal/hour.png"];
        // Wąska pozioma wskazówka zostaje wąska — nie rozdmuchujemy jej do kwadratu.
        assert!(hour.placement.w >= 80 && hour.placement.w <= 110, "{hour:?}");
        assert!(hour.placement.h >= 10 && hour.placement.h <= 30, "{hour:?}");
        let offset_x = hour.placement.x - project.widgets[0].x;
        assert!((45..=55).contains(&offset_x), "offset_x={offset_x}");
        let img = image::load_from_memory(&hour.bytes).unwrap().to_rgba8();
        assert_eq!(
            img.dimensions(),
            (hour.placement.w as u32, hour.placement.h as u32)
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn scaling_analog_frame_resizes_hand_pngs_and_export() {
        let root = std::env::temp_dir().join(format!("rw-hands-scale-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let set = hand_set_dir(&root, "Full");
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            make_hand(&set, file, 400);
        }

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject {
                name: "Scaled".into(),
                author: "Dev".into(),
                app_id: 8,
            },
        )
        .unwrap();
        project.widgets[0].kind = WidgetKind::AnalogTime;
        project.widgets[0].id = "analog".into();
        attach_to_project(&mut project, &dir, &root, "analog", "Full").unwrap();
        assert_eq!(
            image::open(dir.join("assets/hands/Full/hour.png"))
                .unwrap()
                .dimensions(),
            (400, 400)
        );

        project.widgets[0].w = 175;
        project.widgets[0].h = 175;
        scale_project_hands(&project, &dir, &root, "analog").unwrap();
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            assert_eq!(
                image::open(dir.join("assets/hands/Full").join(file))
                    .unwrap()
                    .dimensions(),
                (175, 175),
                "hand {file}"
            );
        }

        // Kolejna zmiana rozmiaru znów startuje od oryginału 400×400 z magazynu.
        project.widgets[0].w = 120;
        project.widgets[0].h = 120;
        scale_project_hands(&project, &dir, &root, "analog").unwrap();
        assert_eq!(
            image::open(dir.join("assets/hands/Full/hour.png"))
                .unwrap()
                .dimensions(),
            (120, 120)
        );

        // `export_asset_bytes` przepuszcza zasoby bez zmian — przycinanie
        // wskazówek odbywa się wyłącznie w `prepare_hand_assets`.
        project.widgets[0].w = 175;
        project.widgets[0].h = 175;
        let bytes = export_asset_bytes(&project, &dir, "hands/Full/hour.png").unwrap();
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().dimensions(),
            (120, 120)
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn restored_hand_matches_current_frame_size() {
        let root = std::env::temp_dir().join(format!("rw-hands-restore-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let set = hand_set_dir(&root, "Full");
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            make_hand(&set, file, 300);
        }
        let (dir, mut project) = create_project_in(
            &root,
            &NewProject {
                name: "Restored".into(),
                author: "Dev".into(),
                app_id: 9,
            },
        )
        .unwrap();
        project.widgets[0].kind = WidgetKind::AnalogTime;
        project.widgets[0].id = "analog".into();
        attach_to_project(&mut project, &dir, &root, "analog", "Full").unwrap();

        project.widgets[0].w = 140;
        project.widgets[0].h = 140;
        set_hand_visibility(
            &mut project, &dir, &root, "analog",
            Some(false), Some(false), Some(false), Some(false),
        )
        .unwrap();
        set_hand_visibility(
            &mut project, &dir, &root, "analog",
            Some(true), Some(true), Some(true), Some(true),
        )
        .unwrap();
        scale_project_hands(&project, &dir, &root, "analog").unwrap();
        assert_eq!(
            image::open(dir.join("assets/hands/Full/second.png"))
                .unwrap()
                .dimensions(),
            (140, 140)
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn preview_is_regenerated_only_when_hands_change() {
        let root = std::env::temp_dir().join(format!("rw-hand-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let pack = hand_set_dir(&root, "Cached");
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            make_hand(&pack, file, 32);
        }
        generate_preview(&root, "Cached").unwrap();

        let names = hand_file_names();
        let preview = pack.join("preview.png");
        let sidecar = pack.join("preview.fingerprint");
        assert!(preview.is_file());
        assert!(sidecar.is_file());
        let first_bytes = fs::read(&preview).unwrap();
        let first_stamp = fs::metadata(&preview).unwrap().modified().unwrap();
        let first_fp = asset_cache::files_fingerprint(&pack, &names).unwrap();
        assert_eq!(fs::read_to_string(&sidecar).unwrap().trim(), first_fp);

        // Bez zmian: podgląd pozostaje nietknięty (brak dekodowania/kodowania).
        std::thread::sleep(Duration::from_millis(30));
        ensure_previews(&root).unwrap();
        assert_eq!(fs::read(&preview).unwrap(), first_bytes);
        assert_eq!(fs::metadata(&preview).unwrap().modified().unwrap(), first_stamp);

        // Zmiana jednej wskazówki musi odnowić miniaturkę i odcisk.
        rewrite_hand(&pack.join(SECOND_FILE), 32, Rgba([255, 0, 0, 255]));
        let second_fp = asset_cache::files_fingerprint(&pack, &names).unwrap();
        assert_ne!(second_fp, first_fp);
        ensure_previews(&root).unwrap();
        assert_ne!(fs::read(&preview).unwrap(), first_bytes);
        assert_eq!(fs::read_to_string(&sidecar).unwrap().trim(), second_fp);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ensure_default_refreshes_changed_bundled_hands() {
        let root = std::env::temp_dir().join(format!("rw-hand-bundled-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let bundled = root.join("bundled");
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            make_hand(&bundled.join(DEFAULT_HAND_SET), file, 64);
        }
        ensure_default(&root, &bundled).unwrap();

        let installed = hand_set_dir(&root, DEFAULT_HAND_SET);
        let hour = installed.join(HOUR_FILE);
        let before = fs::read(&hour).unwrap();
        let source_dir = bundled.join(DEFAULT_HAND_SET);
        let source = source_dir.join(HOUR_FILE);
        rewrite_hand(&source, 64, Rgba([10, 20, 30, 255]));
        let bundled_fp = asset_cache::files_fingerprint(&source_dir, &hand_file_names()).unwrap();

        ensure_default(&root, &bundled).unwrap();
        assert_ne!(fs::read(&hour).unwrap(), before);
        assert_eq!(fs::read(&hour).unwrap(), fs::read(&source).unwrap());
        assert_eq!(
            asset_cache::read_bundled_version(&installed).as_deref(),
            Some(bundled_fp.as_str())
        );

        let stamp = fs::metadata(&hour).unwrap().modified().unwrap();
        ensure_default(&root, &bundled).unwrap();
        assert_eq!(fs::metadata(&hour).unwrap().modified().unwrap(), stamp);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn all_hands_can_be_disabled_and_restored() {
        let root = std::env::temp_dir().join(format!("rw-hands-visibility-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let set = hand_set_dir(&root, "Full");
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            make_hand(&set, file, 300);
        }

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject {
                name: "Analog".into(),
                author: "Dev".into(),
                app_id: 7,
            },
        )
        .unwrap();
        project.widgets[0].kind = WidgetKind::AnalogTime;
        project.widgets[0].id = "analog".to_string();
        attach_to_project(&mut project, &dir, &root, "analog", "Full").unwrap();
        assert!(project.widgets[0].second_src.is_some());
        assert!(project.widgets[0].cap_src.is_some());

        set_hand_visibility(
            &mut project, &dir, &root, "analog",
            Some(false), Some(false), Some(false), Some(false),
        )
        .unwrap();
        let widget = &project.widgets[0];
        assert_eq!(widget.hour_src, None);
        assert_eq!(widget.minute_src, None);
        assert_eq!(widget.second_src, None);
        assert_eq!(widget.cap_src, None);
        assert!(!dir.join("assets/hands/Full/hour.png").exists());
        assert!(!dir.join("assets/hands/Full/minute.png").exists());
        assert!(!dir.join("assets/hands/Full/second.png").exists());
        assert!(!dir.join("assets/hands/Full/cap.png").exists());
        assert_eq!(
            project.assets.iter().filter(|a| a.role == AssetRole::Hand).count(),
            0
        );

        // Włączenie przywraca pliki z magazynu także wtedy, gdy wyłączenie
        // usunęło ich kopie z projektu.
        set_hand_visibility(
            &mut project, &dir, &root, "analog",
            Some(true), Some(true), Some(true), Some(true),
        )
        .unwrap();
        let widget = &project.widgets[0];
        assert_eq!(widget.hour_src.as_deref(), Some("hands/Full/hour.png"));
        assert_eq!(widget.minute_src.as_deref(), Some("hands/Full/minute.png"));
        assert_eq!(widget.second_src.as_deref(), Some("hands/Full/second.png"));
        assert_eq!(widget.cap_src.as_deref(), Some("hands/Full/cap.png"));
        for file in [HOUR_FILE, MINUTE_FILE, SECOND_FILE, CAP_FILE] {
            assert!(dir.join("assets/hands/Full").join(file).is_file());
        }
        assert_eq!(
            project.assets.iter().filter(|a| a.role == AssetRole::Hand).count(),
            4
        );

        fs::remove_dir_all(&root).ok();
    }
}
