//! Magazyn paczek ikon pogody dla widgetu `weather-icon`.
//!
//! Pakiety żyją w `%APPDATA%\RustyWatch\Weather\<id>\` i zawierają dokładnie
//! 29 przezroczystych plików `w00.png` … `w28.png`, odpowiadających kodom
//! pogody z dokumentacji ZeppOS v1.0 (`forecastData.data[0].index`).
//! Wzorzec katalogów i cykl życia są takie same jak w `status_icons.rs`:
//! wbudowany zestaw `Classic` jest seedowany przy starcie,
//! wybrany zestaw jest kopiowany do `<projekt>/assets/weather/<id>/`,
//! a ZIP jest samowystarczalny.
//!
//! Ikony powstają z fontu Weather Icons (SIL OFL 1.1, Erik Flowers / Lukas
//! Bischoff). `Classic` to białe glify na przezroczystym tle. Pod przezroczystością
//! RGB jest zawsze zerowane — realny
//! firmware rysuje takie piksele jako białe prostokąty przy obrocie `IMG`.

use image::imageops::FilterType;
use image::{GenericImageView, ImageEncoder, Rgba, RgbaImage};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::asset_cache;
use crate::model::{Asset, AssetRole, Project, Widget, WidgetKind, SCREEN_SIZE};

pub const WEATHER_DIR: &str = "Weather";
pub const DEFAULT_WEATHER_PACK: &str = "Classic";
pub const BUILTIN_PACKS: [&str; 1] = [DEFAULT_WEATHER_PACK];
pub const JS_WEATHER_PREFIX: &str = "weather/";
/// Liczba kodów pogody w dokumentacji ZeppOS v1.0 (indeksy 0..=28).
pub const ICON_COUNT: usize = 29;
/// Domyślny rozmiar ramki nowego widgetu ikony pogody.
pub const DEFAULT_ICON_SIZE: i32 = 64;

/// Nazwy plików ikon — jedyne źródło odcisku paczki.
/// Statyczna tablica `&'static str`, żeby współdzielić API z `asset_cache`.
pub fn icon_names() -> Vec<&'static str> {
    const NAMES: [&str; ICON_COUNT] = [
        "w00.png", "w01.png", "w02.png", "w03.png", "w04.png", "w05.png", "w06.png", "w07.png",
        "w08.png", "w09.png", "w10.png", "w11.png", "w12.png", "w13.png", "w14.png", "w15.png",
        "w16.png", "w17.png", "w18.png", "w19.png", "w20.png", "w21.png", "w22.png", "w23.png",
        "w24.png", "w25.png", "w26.png", "w27.png", "w28.png",
    ];
    NAMES.to_vec()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherPack {
    pub id: String,
    pub name: String,
    /// Bok kwadratowej ikony w pikselach (np. 96).
    pub size: u32,
    /// Odcisk plików ikon — miniaturka jest odświeżana tylko przy jego zmianie.
    pub fingerprint: String,
}

pub fn weather_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join(WEATHER_DIR)
}

pub fn pack_dir(app_data_root: &Path, id: &str) -> PathBuf {
    weather_dir(app_data_root).join(id)
}

pub fn js_icon_path(id: &str, index: usize) -> String {
    format!("{JS_WEATHER_PREFIX}{id}/w{index:02}.png")
}

fn valid_set_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
    {
        return Err(format!("Nieprawidłowa nazwa paczki ikon pogody: {id}"));
    }
    Ok(())
}

fn sanitize_set_id(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    if cleaned.is_empty() {
        "Weather".to_string()
    } else {
        cleaned
    }
}

/// Paczka jest wykrywana tylko wtedy, gdy ma wszystkie 29 ikon tego samego rozmiaru.
fn describe_pack(path: &Path, id: &str) -> Option<WeatherPack> {
    if !path.is_dir() {
        return None;
    }
    let names = icon_names();
    let first = read_size(&path.join(&names[0]))?;
    for name in names.iter().skip(1) {
        if read_size(&path.join(name))? != first {
            return None;
        }
    }
    let fingerprint = asset_cache::files_fingerprint(path, &names).ok()?;
    Some(WeatherPack {
        id: id.to_string(),
        name: id.to_string(),
        size: first,
        fingerprint,
    })
}

fn read_size(path: &Path) -> Option<u32> {
    let image = image::open(path).ok()?;
    let (w, h) = image.dimensions();
    (w == h).then_some(w)
}

/// Kopiuje wbudowane zestawy do magazynu i odświeża je, gdy pliki zmieniły się.
pub fn ensure_default(app_data_root: &Path, bundled_dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(bundled_dir)
        .map_err(|e| format!("Nie można odczytać wbudowanych ikon pogody: {e}"))?;
    let mut found = false;
    let names = icon_names();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać paczki ikon: {e}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let Some(_pack) = describe_pack(&entry.path(), &id) else {
            continue;
        };
        valid_set_id(&id)?;
        found = true;
        let bundled = entry.path();
        let bundled_fingerprint = asset_cache::files_fingerprint(&bundled, &names)?;
        let target = pack_dir(app_data_root, &id);
        let unchanged = describe_pack(&target, &id).is_some()
            && asset_cache::read_bundled_version(&target).as_deref()
                == Some(bundled_fingerprint.as_str());
        if unchanged {
            continue;
        }
        fs::create_dir_all(&target)
            .map_err(|e| format!("Nie można utworzyć katalogu ikon pogody: {e}"))?;
        for name in &names {
            fs::copy(bundled.join(name), target.join(name))
                .map_err(|e| format!("Nie można skopiować ikony pogody: {e}"))?;
        }
        asset_cache::write_bundled_version(&target, &bundled_fingerprint)?;
        generate_preview(app_data_root, &id)?;
    }
    if !found {
        return Err(format!(
            "Nie znaleziono wbudowanych paczek ikon pogody: {}",
            bundled_dir.display()
        ));
    }
    ensure_previews(app_data_root)?;
    Ok(())
}

/// Generuje `preview.png` tylko dla paczek, których pliki się zmieniły.
pub fn ensure_previews(app_data_root: &Path) -> Result<(), String> {
    for pack in list(app_data_root)? {
        let dir = pack_dir(app_data_root, &pack.id);
        if asset_cache::fingerprint_matches(&dir, &icon_names())? {
            continue;
        }
        generate_preview(app_data_root, &pack.id)?;
    }
    Ok(())
}

/// Miniaturka: słonecznie / lekki deszcz / śnieg / burza (2×2).
pub fn generate_preview(app_data_root: &Path, id: &str) -> Result<(), String> {
    const SIZE: u32 = 192;
    const CELL: u32 = 96;
    const ICON: u32 = 80;
    let source = pack_dir(app_data_root, id);
    let mut canvas = RgbaImage::from_pixel(SIZE, SIZE, Rgba([28, 28, 32, 255]));
    for (slot, index) in [3usize, 5, 9, 15].iter().enumerate() {
        let path = source.join(format!("w{index:02}.png"));
        let original = image::open(&path)
            .map_err(|e| format!("Nie można odczytać ikony pogody: {e}"))?
            .to_rgba8();
        let premultiplied = premultiply_alpha(&original);
        let scaled = image::imageops::resize(&premultiplied, ICON, ICON, FilterType::Lanczos3);
        let icon = unpremultiply_alpha(&scaled);
        let x = (slot % 2) as u32 * CELL + (CELL - ICON) / 2;
        let y = (slot / 2) as u32 * CELL + (CELL - ICON) / 2;
        image::imageops::overlay(&mut canvas, &icon, x as i64, y as i64);
    }
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes).write_image(
        canvas.as_raw(),
        SIZE,
        SIZE,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| format!("Nie można zapisać miniaturki paczki: {e}"))?;
    fs::write(source.join("preview.png"), bytes)
        .map_err(|e| format!("Nie można zapisać miniaturki paczki: {e}"))?;
    asset_cache::write_fingerprint(&source, &icon_names())?;
    Ok(())
}

/// Lista zainstalowanych paczek — katalog jest skanowany dynamicznie.
pub fn list(app_data_root: &Path) -> Result<Vec<WeatherPack>, String> {
    let entries = match fs::read_dir(weather_dir(app_data_root)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Nie można odczytać katalogu ikon pogody: {e}")),
    };
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać paczki ikon: {e}"))?;
        let id = entry.file_name().to_string_lossy().to_string();
        // Dawną paczkę Reversed zostawiamy na dysku użytkownika, ale nie jest
        // już widoczna ani wybieralna w edytorze.
        if id == "Reversed" {
            continue;
        }
        if let Some(pack) = describe_pack(&entry.path(), &id) {
            result.push(pack);
        }
    }
    result.sort_by(|a, b| {
        (a.id != DEFAULT_WEATHER_PACK)
            .cmp(&(b.id != DEFAULT_WEATHER_PACK))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

/// Dodaje paczkę z wybranego folderu (wymagane wszystkie 29 plików `wNN.png`).
pub fn add(app_data_root: &Path, folder: &Path) -> Result<WeatherPack, String> {
    if !folder.is_dir() {
        return Err(format!("Folder nie istnieje: {}", folder.display()));
    }
    let pack = describe_pack(folder, "tmp")
        .ok_or_else(|| "Paczka musi zawierać 29 kwadratowych plików w00.png … w28.png".to_string())?;
    let raw_name = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Weather");
    let id = sanitize_set_id(raw_name);
    valid_set_id(&id)?;
    let target = pack_dir(app_data_root, &id);
    if target.exists() {
        return Err(format!("Paczka ikon pogody {id} jest już zainstalowana"));
    }
    fs::create_dir_all(&target).map_err(|e| format!("Nie można utworzyć paczki: {e}"))?;
    for name in icon_names() {
        fs::copy(folder.join(&name), target.join(&name))
            .map_err(|e| format!("Nie można skopiować ikony pogody: {e}"))?;
    }
    generate_preview(app_data_root, &id)?;
    let fingerprint = asset_cache::files_fingerprint(&target, &icon_names())?;
    Ok(WeatherPack {
        id,
        name: pack.name,
        size: pack.size,
        fingerprint,
    })
}

pub fn remove(app_data_root: &Path, id: &str) -> Result<(), String> {
    valid_set_id(id)?;
    if BUILTIN_PACKS.contains(&id) {
        return Err("Wbudowanej paczki ikon pogody nie można usunąć".to_string());
    }
    let path = pack_dir(app_data_root, id);
    if !path.is_dir() {
        return Err(format!("Paczka ikon pogody nie jest zainstalowana: {id}"));
    }
    fs::remove_dir_all(&path).map_err(|e| format!("Nie można usunąć paczki ikon: {e}"))
}

/// Przypisuje paczkę do widgetu i kopiuje jej 29 plików do projektu.
pub fn attach_to_project(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
    set_id: &str,
) -> Result<(), String> {
    valid_set_id(set_id)?;
    let source = pack_dir(app_data_root, set_id);
    describe_pack(&source, set_id)
        .ok_or_else(|| format!("Paczka ikon pogody nie jest zainstalowana: {set_id}"))?;
    if !project
        .widgets
        .iter()
        .any(|w| w.id == widget_id && w.kind == WidgetKind::WeatherIcon)
    {
        return Err(format!("Nie znaleziono widgetu ikony pogody: {widget_id}"));
    }
    let (w, h) = project
        .widgets
        .iter()
        .find(|w| w.id == widget_id)
        .map(|w| (w.w.max(1) as u32, w.h.max(1) as u32))
        .unwrap();
    let target_dir = project_dir.join("assets").join(JS_WEATHER_PREFIX).join(set_id);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu ikon w projekcie: {e}"))?;
    let names = icon_names();
    for name in &names {
        let bytes = fs::read(source.join(name))
            .map_err(|e| format!("Nie można odczytać ikony z magazynu: {e}"))?;
        let prepared = resize_png(&bytes, w, h)?;
        fs::write(target_dir.join(name), prepared)
            .map_err(|e| format!("Nie można zapisać ikony w projekcie: {e}"))?;
        let path = format!("{JS_WEATHER_PREFIX}{set_id}/{name}");
        if !project.assets.iter().any(|a| a.path == path) {
            project.assets.push(Asset {
                path,
                role: AssetRole::WeatherIcon,
            });
        }
    }
    let widget = project
        .widgets
        .iter_mut()
        .find(|w| w.id == widget_id)
        .unwrap();
    widget.weather_icon_set = Some(set_id.to_string());
    // Widoczna w edytorze ikona to słonecznie (indeks 3).
    widget.src = Some(js_icon_path(set_id, 3));
    Ok(())
}

/// Daje domyślny zestaw widgetom, które go jeszcze nie mają.
pub fn attach_default_to_missing(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
) -> Result<(), String> {
    let missing: Vec<String> = project
        .widgets
        .iter()
        .filter(|w| w.kind == WidgetKind::WeatherIcon && w.weather_icon_set.is_none())
        .map(|w| w.id.clone())
        .collect();
    for id in missing {
        attach_to_project(project, project_dir, app_data_root, &id, DEFAULT_WEATHER_PACK)?;
    }
    Ok(())
}

/// Usuwa z projektu zestawy, których nie używa żaden widget.
pub fn prune_unused(project: &mut Project, project_dir: &Path) {
    let mut used_prefixes = std::collections::HashSet::new();
    for widget in &project.widgets {
        if widget.kind == WidgetKind::WeatherIcon {
            if let Some(set) = &widget.weather_icon_set {
                used_prefixes.insert(format!("{JS_WEATHER_PREFIX}{set}/"));
            }
        }
    }
    project.assets.retain(|asset| {
        if asset.role != AssetRole::WeatherIcon
            || used_prefixes.iter().any(|prefix| asset.path.starts_with(prefix))
        {
            return true;
        }
        let path = project_dir.join("assets").join(&asset.path);
        if path.is_file() {
            let _ = fs::remove_file(path);
        }
        false
    });
}

/// Skalowanie w przestrzeni premultiplied alpha — filtr nie wciąga białego RGB
/// spod przezroczystych pikseli w krawędzie glifu (jak w `hands.rs`).
fn premultiply_alpha(image: &RgbaImage) -> RgbaImage {
    let mut out = image.clone();
    for pixel in out.pixels_mut() {
        let alpha = pixel.0[3] as u32;
        if alpha == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        } else {
            pixel.0[0] = ((pixel.0[0] as u32 * alpha + 127) / 255) as u8;
            pixel.0[1] = ((pixel.0[1] as u32 * alpha + 127) / 255) as u8;
            pixel.0[2] = ((pixel.0[2] as u32 * alpha + 127) / 255) as u8;
        }
    }
    out
}

/// Powrót do zwykłego (straight) alpha przed zapisem PNG.
fn unpremultiply_alpha(image: &RgbaImage) -> RgbaImage {
    let mut out = image.clone();
    for pixel in out.pixels_mut() {
        let alpha = pixel.0[3] as u32;
        if alpha == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        } else {
            for channel in &mut pixel.0[..3] {
                *channel = ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    out
}

/// Wyzerowuje RGB tam, gdzie `alpha == 0` — realny firmware rysuje te piksele,
/// co daje białe prostokąty / ząbkowane brzegi wokół glifu na tarczy.
fn clear_transparent_rgb(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        if pixel.0[3] == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
}

/// Sprowadza obraz do postaci premultiplied tylko wtedy, gdy nie jest nią już
/// zapisany. Kopie w projekcie powstają w tej postaci (jak wskazówki w
/// `hands.rs`), ale stare projekty miały białe RGB przy częściowej alrfie —
/// bez tej detekcji eksport znów wypuściłby białe ząbki na zegarku.
fn ensure_premultiplied(image: &RgbaImage) -> RgbaImage {
    let straight_rgb = image.pixels().any(|pixel| {
        let alpha = pixel.0[3] as i32;
        alpha > 0 && alpha < 255 && pixel.0[..3].iter().any(|&channel| channel as i32 > alpha + 2)
    });
    if straight_rgb {
        premultiply_alpha(image)
    } else {
        image.clone()
    }
}

/// Przygotowuje ikonę do eksportu: kolor krawędzi pozostaje w przestrzeni
/// premultiplied (jak w działającej tarczy referencyjnej i w `hands.rs`).
/// Dzięki temu piksele o słabej alrfie są ciemnoszare zamiast białych —
/// firmware, który ignoruje częściową alrfę, nie rysuje białej obwódki.
fn resize_png(bytes: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    let image =
        image::load_from_memory(bytes).map_err(|e| format!("Nie można odczytać ikony pogody: {e}"))?;
    let source = ensure_premultiplied(&image.to_rgba8());
    let mut resized = if source.dimensions() == (w, h) {
        source
    } else {
        image::imageops::resize(&source, w, h, FilterType::Lanczos3)
    };
    clear_transparent_rgb(&mut resized);
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out).write_image(
        resized.as_raw(),
        w,
        h,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| format!("Nie można zapisać ikony pogody: {e}"))?;
    Ok(out)
}

/// Fizycznie skaluje **wszystkie 29 ikon** do rozmiaru ramki widgetu.
/// Źródłem jest zawsze oryginał z magazynu — kolejne skalowania się nie kumulują.
pub fn scale_project_icons(
    project: &Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
) -> Result<(), String> {
    let widget: &Widget = project
        .widgets
        .iter()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::WeatherIcon)
        .ok_or_else(|| format!("Nie znaleziono widgetu ikony pogody: {widget_id}"))?;
    let set_id = widget
        .weather_icon_set
        .as_deref()
        .ok_or_else(|| "Widget ikony pogody nie ma przypisanej paczki".to_string())?;
    let (w, h) = (widget.w.max(1) as u32, widget.h.max(1) as u32);
    let source_dir = pack_dir(app_data_root, set_id);
    let target_dir = project_dir.join("assets").join(JS_WEATHER_PREFIX).join(set_id);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu ikon w projekcie: {e}"))?;
    for name in icon_names() {
        let bytes = fs::read(source_dir.join(&name))
            .map_err(|e| format!("Nie można odczytać ikony z magazynu: {e}"))?;
        let prepared = resize_png(&bytes, w, h)?;
        fs::write(target_dir.join(&name), prepared)
            .map_err(|e| format!("Nie można zapisać ikony w projekcie: {e}"))?;
    }
    Ok(())
}

/// Bajty ikony w ZIP, dopasowane do rozmiaru ramki dla starszych projektów.
pub fn export_asset_bytes(
    project: &Project,
    project_dir: &Path,
    path: &str,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(project_dir.join("assets").join(path))
        .map_err(|e| format!("Nie można odczytać zasobu {path}: {e}"))?;
    let size = project
        .widgets
        .iter()
        .find(|w| w.kind == WidgetKind::WeatherIcon)
        .map(|w| (w.w.max(1) as u32, w.h.max(1) as u32));
    match size {
        Some((w, h)) => resize_png(&bytes, w, h),
        None => Ok(bytes),
    }
}

/// Domyślna ramka nowej ikony — kwadrat wyśrodkowany na tarczy.
pub fn default_frame() -> (i32, i32, i32, i32) {
    let size = DEFAULT_ICON_SIZE;
    (
        (SCREEN_SIZE as i32 - size) / 2,
        (SCREEN_SIZE as i32 - size) / 2,
        size,
        size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, RgbaImage};
    use std::fs::File;

    fn roots(test: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("rw-weather-{test}-{}", std::process::id()));
        let app = base.join("appdata");
        let bundles = base.join("bundled");
        let project = base.join("project");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&bundles).unwrap();
        fs::create_dir_all(&project).unwrap();
        (app, bundles, project)
    }

    fn bundled(root: &Path, id: &str, size: u32) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        let image = RgbaImage::from_pixel(size, size, Rgba([255, 255, 255, 255]));
        for name in icon_names() {
            let file = File::create(dir.join(name)).unwrap();
            image::codecs::png::PngEncoder::new(file)
                .write_image(
                    image.as_raw(),
                    size,
                    size,
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
        }
    }

    #[test]
    fn maps_all_29_weather_codes() {
        let names = icon_names();
        assert_eq!(names.len(), ICON_COUNT);
        assert_eq!(names[0], "w00.png");
        assert_eq!(names[28], "w28.png");
        assert_eq!(js_icon_path("Classic", 28), "weather/Classic/w28.png");
    }

    #[test]
    fn resize_png_never_exports_white_edges() {
        // Stara kopia w projekcie: białe RGB przy częściowej alrfie.
        let mut original = RgbaImage::from_pixel(16, 16, Rgba([255, 255, 255, 0]));
        original.put_pixel(8, 8, Rgba([255, 255, 255, 128]));
        original.put_pixel(9, 8, Rgba([255, 255, 255, 12]));
        original.put_pixel(8, 9, Rgba([255, 255, 255, 255]));
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(
                original.as_raw(),
                16,
                16,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();

        for size in [16u32, 8] {
            let out = resize_png(&bytes, size, size).unwrap();
            let prepared = image::load_from_memory(&out).unwrap().to_rgba8();
            for pixel in prepared.pixels() {
                let alpha = pixel.0[3] as i32;
                if alpha == 0 {
                    assert_eq!(pixel.0, [0, 0, 0, 0], "transparent RGB not cleared");
                } else {
                    // Premultiplied colour: never brighter than the alpha itself,
                    // otherwise the watch paints a jagged white halo.
                    assert!(
                        pixel.0[..3]
                            .iter()
                            .all(|&channel| channel as i32 <= alpha + 2),
                        "bright edge in prepared icon: {pixel:?}"
                    );
                }
            }
        }

        // Already premultiplied input must not be darkened a second time.
        let mut source = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        source.put_pixel(4, 4, Rgba([128, 128, 128, 128]));
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(source.as_raw(), 8, 8, image::ExtendedColorType::Rgba8)
            .unwrap();
        let out = image::load_from_memory(&resize_png(&bytes, 8, 8).unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(out.get_pixel(4, 4).0, [128, 128, 128, 128]);
    }

    #[test]
    fn ensure_default_seeds_classic_and_resizes_all_icons() {
        let (app, bundles, project) = roots("resize");
        let base = app.parent().unwrap().to_path_buf();
        bundled(&bundles, "Classic", 96);
        ensure_default(&app, &bundles).unwrap();
        let packs: Vec<_> = list(&app).unwrap().into_iter().map(|p| p.id).collect();
        assert_eq!(packs, vec!["Classic"]);

        let mut project_model = crate::Project::starter_with("Weather", "Dev", 1);
        project_model.widgets.clear();
        let mut widget = crate::model::Widget {
            id: "weather-icon".into(),
            name: "Weather".into(),
            kind: WidgetKind::WeatherIcon,
            x: 0,
            y: 0,
            w: 96,
            h: 96,
            z: 0,
            text: None,
            text_size: None,
            font: None,
            time_format: None,
            color: None,
            src: None,
            image_fit: crate::image_fit::FitMode::Contain,
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: Some(crate::model::DataSource::WeatherIcon),
            align: None,
            v_align: crate::model::VAlign::Center,
            show_level: crate::model::ShowLevel::Normal,
            auto_scale: false,
            text_style: crate::model::TextStyle::Ellipsis,
            visible: true,
            measurement_system: None,
            show_unit: true,
            alpha: 255,
            status_icon_set: None,
            status_type: None,
            weather_icon_set: None,
        weather_day_offset: 0,
        sleep_stage_mode: Default::default(),
        };
        let (x, y, w, h) = default_frame();
        widget.x = x;
        widget.y = y;
        widget.w = w;
        widget.h = h;
        project_model.widgets.push(widget);
        attach_to_project(&mut project_model, &project, &app, "weather-icon", "Classic").unwrap();

        // Czerwona ramka: zmieniamy rozmiar i skalujemy WSZYSTKIE ikony.
        project_model.widgets[0].w = 120;
        project_model.widgets[0].h = 120;
        scale_project_icons(&project_model, &project, &app, "weather-icon").unwrap();
        for name in icon_names() {
            let path = project.join("assets").join(JS_WEATHER_PREFIX).join("Classic").join(name);
            let image = image::open(&path).unwrap();
            assert_eq!(image.dimensions(), (120, 120), "{path:?}");
        }

        fs::remove_dir_all(&base).ok();
    }
}
