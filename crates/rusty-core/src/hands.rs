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
use image::{GenericImageView, ImageEncoder, RgbaImage};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::HandPlacement;
use crate::model::{Asset, AssetRole, Project, WidgetKind, SCREEN_SIZE};

pub const HAND_DIR: &str = "Hands";
pub const DEFAULT_HAND_SET: &str = "Classic";
pub const JS_HAND_PREFIX: &str = "hands/";

const HOUR_FILE: &str = "hour.png";
const MINUTE_FILE: &str = "minute.png";
const SECOND_FILE: &str = "second.png";
const CAP_FILE: &str = "cap.png";
const REQUIRED_FILES: [&str; 2] = [HOUR_FILE, MINUTE_FILE];
const OPTIONAL_FILES: [&str; 2] = [SECOND_FILE, CAP_FILE];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandSet {
    pub id: String,
    pub name: String,
    pub has_second: bool,
    pub has_cap: bool,
    /// Bok kwadratowego obrazu wskazówki w pikselach (np. 400).
    pub size: u32,
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

/// Kopiuje wbudowane zestawy do magazynu, jeśli jeszcze ich nie ma.
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
        let target = hand_set_dir(app_data_root, &id);
        if target.join(HOUR_FILE).is_file() && target.join(MINUTE_FILE).is_file() {
            continue;
        }
        fs::create_dir_all(&target)
            .map_err(|e| format!("Nie można utworzyć katalogu wskazówek: {e}"))?;
        for file in REQUIRED_FILES.iter().chain(OPTIONAL_FILES.iter()) {
            let from = entry.path().join(file);
            if from.is_file() {
                fs::copy(&from, target.join(file))
                    .map_err(|e| format!("Nie można skopiować wskazówki {file}: {e}"))?;
            }
        }
    }
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
    Ok(HandSet {
        id: id.clone(),
        name: id,
        has_second: target.join(SECOND_FILE).is_file(),
        has_cap: target.join(CAP_FILE).is_file(),
        size,
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

/// Docelowy rozmiar wskazówki dla ścieżki zasobu użytej przez widget analogowy.
/// ZeppOS nie skaluje zasobu `IMG` w runtime, więc plik PNG musi mieć dokładnie
/// rozmiar `w/h` widgetu — inaczej tarcza pokazuje tylko środkowy wycinek obrazu.
fn hand_target_size(project: &Project, path: &str) -> Option<(u32, u32)> {
    project
        .widgets
        .iter()
        .find(|w| {
            w.kind == WidgetKind::AnalogTime
                && [&w.hour_src, &w.minute_src, &w.second_src, &w.cap_src]
                    .iter()
                    .any(|src| src.as_deref() == Some(path))
        })
        .map(|w| (w.w.max(1) as u32, w.h.max(1) as u32))
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

/// Przygotowuje wskazówki do eksportu: dopasowuje rozmiar do ramki widgetu,
/// a następnie (gdy to bezpieczne) przycina przezroczyste marginesy.
///
/// ZeppOS nie skaluje zasobu `IMG` — rysuje go w naturalnym rozmiarze i przycina
/// do prostokąta `w×h` widgetu (potwierdzone w dokumentacji API i w symulatorze).
/// Dlatego przycinanie NIE MOŻE używać ciasnego prostokąta wokół grafiki: po
/// obrocie wskazówka wychodziłaby poza taki prostokąt i została obcięta do
/// kilku pikseli przy pivocie. W zamian wycinamy KWADRAT wyśrodkowany na punkcie
/// obrotu, o promieniu równym maksymalnej odległości nieprzezroczystego piksela
/// od środka (+1 px zapasu na zaokrąglenia obrotu). Po dowolnym kącie obrotu
/// wszystkie piksele nadal mieszczą się w tym kwadracie, więc nic nie ginie.
///
/// Pivot pozostaje dokładnie w środku kwadratu, dzięki czemu
/// `x + center_x == widget.x + w/2` i podgląd (który obraca pełny obraz wokół
/// środka) pozostaje zgodny z tarczą 1:1. Gdy grafika sięga krawędzi ramki albo
/// zysk byłby pomijalny, zwracamy pełny kwadrat bez zmiany geometrii.
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
        let target_w = widget.w.max(1) as u32;
        let target_h = widget.h.max(1) as u32;
        let full = HandPlacement {
            x: widget.x,
            y: widget.y,
            w: widget.w,
            h: widget.h,
            center_x: widget.w / 2,
            center_y: widget.h / 2,
        };
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
            let image = if decoded.dimensions() == (target_w, target_h) {
                decoded
            } else {
                image::imageops::resize(&decoded, target_w, target_h, FilterType::Lanczos3)
            };

            let pivot_x = target_w / 2;
            let pivot_y = target_h / 2;
            let mut farthest_squared = 0u64;
            let mut visible = 0u32;
            for (x, y, pixel) in image.enumerate_pixels() {
                if pixel[3] > 0 {
                    visible += 1;
                    let dx = x as i64 - pivot_x as i64;
                    let dy = y as i64 - pivot_y as i64;
                    farthest_squared = farthest_squared.max((dx * dx + dy * dy) as u64);
                }
            }

            // +1 px zapasu na zaokrąglenia przy obrocie (np. 156° dla minut).
            let radius = (farthest_squared as f64).sqrt().ceil() as u32 + 1;
            let size = radius.saturating_mul(2).saturating_add(1);
            let can_crop = visible > 0
                && size < target_w
                && size < target_h
                && (size as u64) * (size as u64)
                    < (target_w as u64) * (target_h as u64) * 85 / 100;

            let (bytes, placement) = if can_crop {
                let left = pivot_x - radius;
                let top = pivot_y - radius;
                let cropped =
                    image::imageops::crop_imm(&image, left, top, size, size).to_image();
                (
                    encode_png(&cropped)?,
                    HandPlacement {
                        x: widget.x + left as i32,
                        y: widget.y + top as i32,
                        w: size as i32,
                        h: size as i32,
                        center_x: radius as i32,
                        center_y: radius as i32,
                    },
                )
            } else {
                (encode_png(&image)?, full)
            };
            prepared.insert(
                src.clone(),
                PreparedHand {
                    bytes,
                    placement,
                },
            );
        }
    }
    Ok(prepared)
}

/// Bajty zasobu wstawiane do eksportowanego ZIP-a. Wskazówki są dopasowywane do
/// rozmiaru ramki widgetu — dzięki temu także starsze projekty (z plikami 400×400
/// i pomniejszoną ramką) eksportują poprawne, niesprzycinane wskazówki.
pub fn export_asset_bytes(
    project: &Project,
    project_dir: &Path,
    path: &str,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(project_dir.join("assets").join(path))
        .map_err(|e| format!("Nie można odczytać zasobu {path}: {e}"))?;
    match hand_target_size(project, path) {
        Some((w, h)) => resized_png(&bytes, w, h),
        None => Ok(bytes),
    }
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
    Some(HandSet {
        id: id.to_string(),
        name: id.to_string(),
        has_second: dir.join(SECOND_FILE).is_file(),
        has_cap: dir.join(CAP_FILE).is_file(),
        size,
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

    fn make_hand(dir: &Path, name: &str, size: u32) {
        fs::create_dir_all(dir).unwrap();
        crate::project::write_png(
            &dir.join(name),
            &RgbaImage::from_pixel(size, size, Rgba([200, 200, 210, 255])),
        )
        .unwrap();
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
    fn cropping_transparent_margins_keeps_rotation_center_1_to_1() {
        let root = std::env::temp_dir().join(format!("rw-hands-crop-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let set = hand_set_dir(&root, "Sparse");
        // Oba prostokąty zawierają środek płótna (200,200).
        make_sparse_hand(&set, HOUR_FILE, 180, 50, 40, 200);
        make_sparse_hand(&set, MINUTE_FILE, 185, 20, 30, 230);
        // Grafika nie sięga środka — dla niej nie wolno zmienić pivota.
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
        assert!(hour.placement.w < 200 && hour.placement.h < 200);
        assert_eq!(
            hour.placement.x + hour.placement.center_x,
            project.widgets[0].x + 100
        );
        assert_eq!(
            hour.placement.y + hour.placement.center_y,
            project.widgets[0].y + 100
        );
        let bytes = image::load_from_memory(&hour.bytes).unwrap().to_rgba8();
        assert_eq!(
            bytes.dimensions(),
            (hour.placement.w as u32, hour.placement.h as u32)
        );
        assert_eq!(hour.placement.w, hour.placement.h, "crop musi być kwadratem");
        assert_eq!(hour.placement.center_x, hour.placement.w / 2);
        assert_eq!(hour.placement.center_y, hour.placement.h / 2);
        // Każdy nieprzezroczysty piksel mieści się w okręgu wpisanym w kwadrat —
        // po dowolnym obrocie nic nie zostanie obcięte przez ZeppOS.
        for (x, y, pixel) in bytes.enumerate_pixels() {
            if pixel[3] > 0 {
                let dx = x as i64 - hour.placement.center_x as i64;
                let dy = y as i64 - hour.placement.center_y as i64;
                assert!(
                    (dx * dx + dy * dy) as u64
                        <= (hour.placement.center_x as u64) * (hour.placement.center_x as u64),
                    "piksel {x},{y} wypada poza okrąg obrotu"
                );
            }
        }

        // Wskazówka bez środka w bboxie pozostaje pełnym kwadratem.
        let cap = &prepared["hands/Sparse/cap.png"];
        assert_eq!((cap.placement.w, cap.placement.h), (200, 200));
        assert_eq!((cap.placement.center_x, cap.placement.center_y), (100, 100));

        let placements = prepared
            .iter()
            .map(|(path, hand)| (path.clone(), hand.placement))
            .collect();
        let js = crate::codegen::generate_index_js_with_hands(&project, &placements);
        assert!(js.contains(&format!("center_x: {}", hour.placement.center_x)));
        assert!(js.contains(&format!("w: {}", hour.placement.w)));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn crop_of_a_horizontal_hand_is_square_not_a_tight_bbox() {
        let root = std::env::temp_dir().join(format!("rw-hands-square-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let set = hand_set_dir(&root, "Horizontal");
        // Wąska wskazówka pozioma: bbox ma 81×5 px, ale po obrocie o 90°
        // potrzebuje kwadratu co najmniej 81×81, inaczej ZeppOS ją obetnie.
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
        // Lanczos3 rozlewa lekko alfa poza oryginalny prostokąt, więc dokładny
        // rozmiar zależy od filtra — istotne, że to KWADRAT, a nie ciasny bbox
        // (który miałby wysokość kilku pikseli).
        assert!(hour.placement.w >= 81 && hour.placement.w <= 110);
        assert_eq!(hour.placement.w, hour.placement.h);
        assert_eq!(hour.placement.center_x, hour.placement.w / 2);
        assert_eq!(hour.placement.center_y, hour.placement.h / 2);
        assert_eq!(
            hour.placement.x + hour.placement.center_x,
            project.widgets[0].x + 100
        );
        assert_eq!(
            hour.placement.y + hour.placement.center_y,
            project.widgets[0].y + 100
        );
        let img = image::load_from_memory(&hour.bytes).unwrap().to_rgba8();
        assert_eq!(
            img.dimensions(),
            (hour.placement.w as u32, hour.placement.h as u32)
        );
        let r = hour.placement.center_x as u64;
        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel[3] > 0 {
                let dx = x as i64 - r as i64;
                let dy = y as i64 - r as i64;
                assert!((dx * dx + dy * dy) as u64 <= r * r, "piksel {x},{y}");
            }
        }

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

        // Eksport starszego projektu (plik 120×120, ramka 175×175) skaluje w locie.
        project.widgets[0].w = 175;
        project.widgets[0].h = 175;
        let bytes = export_asset_bytes(&project, &dir, "hands/Full/hour.png").unwrap();
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().dimensions(),
            (175, 175)
        );
        // Zwykłe zasoby przechodzą bez zmian.
        let bg = fs::read(dir.join("assets/bg.png")).unwrap();
        assert_eq!(export_asset_bytes(&project, &dir, "bg.png").unwrap(), bg);

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
