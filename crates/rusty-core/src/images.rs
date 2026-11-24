//! Magazyn zwykłych obrazków (widget IMAGES), współdzielony przez wszystkie projekty.
//!
//! Pliki trafiają do `%APPDATA%\RustyWatch\Images`. Tła mają osobny magazyn
//! (`backgrounds.rs`) — ten moduł obsługuje wyłącznie grafiki wstawiane widgetem
//! IMAGES. Wybrany obrazek jest przygotowywany (`image_fit`) do dokładnego
//! rozmiaru ramki i zapisywany w projekcie jako `assets/images/<widget-id>/<plik>`.
//! ZeppOS nie potrafi przeskalować zasobu `IMG` w runtime, więc fizyczny rozmiar
//! pliku PNG musi odpowiadać `w × h` widgetu.

use image::GenericImageView;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::image_fit::{fit_image, FitMode};
use crate::model::{Asset, AssetRole, Project, WidgetKind};

pub const IMAGE_DIR: &str = "Images";
/// Katalog zasobów projektu z przygotowanymi grafikami widgetów IMAGES.
pub const WIDGET_IMAGE_DIR: &str = "images";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub file: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

pub fn images_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join(IMAGE_DIR)
}

/// Lista obrazków w magazynie, posortowana po nazwie pliku.
pub fn list(app_data_root: &Path) -> Result<Vec<ImageAsset>, String> {
    let dir = images_dir(app_data_root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Nie można odczytać katalogu obrazków: {e}")),
    };
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać obrazka: {e}"))?;
        let path = entry.path();
        if !path.is_file() || !is_image(&path) {
            continue;
        }
        let image = image::open(&path)
            .map_err(|e| format!("Nie można odczytać obrazka {}: {e}", path.display()))?;
        let (width, height) = image.dimensions();
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        result.push(ImageAsset {
            file,
            width,
            height,
            bytes,
        });
    }
    result.sort_by(|a, b| a.file.to_lowercase().cmp(&b.file.to_lowercase()));
    Ok(result)
}

/// Kopiuje obrazek do magazynu i zwraca jego opis.
pub fn add(app_data_root: &Path, source: &Path) -> Result<ImageAsset, String> {
    if !source.is_file() {
        return Err(format!("Plik nie istnieje: {}", source.display()));
    }
    if !is_image(source) {
        return Err("Obrazek musi mieć rozszerzenie .png, .jpg lub .jpeg".to_string());
    }
    let image = image::open(source)
        .map_err(|e| format!("To nie jest prawidłowy obrazek: {e}"))?;
    let (width, height) = image.dimensions();
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image.png");
    let file = sanitize_file_name(name);
    let dir = images_dir(app_data_root);
    fs::create_dir_all(&dir).map_err(|e| format!("Nie można utworzyć katalogu obrazków: {e}"))?;
    let target = dir.join(&file);
    if target.exists() {
        return Err(format!("Obrazek {file} jest już w magazynie"));
    }
    fs::copy(source, &target).map_err(|e| format!("Nie można skopiować obrazka: {e}"))?;
    Ok(ImageAsset {
        file,
        width,
        height,
        bytes: fs::metadata(&target).map(|m| m.len()).unwrap_or(0),
    })
}

/// Usuwa obrazek z magazynu. Kopie w projektach zostają nienaruszone.
pub fn remove(app_data_root: &Path, file: &str) -> Result<(), String> {
    valid_asset_name(file)?;
    let path = images_dir(app_data_root).join(file);
    fs::remove_file(&path).map_err(|e| format!("Nie można usunąć obrazka: {e}"))
}

/// Ścieżka zasobu w projekcie dla przygotowanej grafiki jednego widgetu IMAGES.
pub fn widget_image_path(widget_id: &str, file: &str) -> String {
    format!("{WIDGET_IMAGE_DIR}/{widget_id}/{file}")
}

fn find_images_widget<'a>(project: &'a Project, widget_id: &str) -> Result<&'a crate::model::Widget, String> {
    project
        .widgets
        .iter()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::Images)
        .ok_or_else(|| format!("Nie znaleziono widgetu IMAGES: {widget_id}"))
}

fn source_file_name(src: &str) -> Option<String> {
    Path::new(src)
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
}

fn encode_png(image: &image::RgbaImage) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Nie można zakodować PNG: {e}"))?;
    Ok(bytes)
}

/// Przygotowuje obrazek z magazynu do rozmiaru ramki i podłącza go do widgetu.
/// Zwraca ścieżkę zasobu w projekcie (`images/<widget-id>/<plik>`).
pub fn set_widget_image(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
    file: &str,
    fit: FitMode,
) -> Result<String, String> {
    valid_asset_name(file)?;
    let (x, y, w, h) = {
        let widget = find_images_widget(project, widget_id)?;
        (widget.x, widget.y, widget.w, widget.h)
    };
    let source = images_dir(app_data_root).join(file);
    if !source.is_file() {
        return Err(format!("Obrazek nie jest zainstalowany: {file}"));
    }
    let image = image::open(&source).map_err(|e| format!("Nie można odczytać obrazka: {e}"))?;
    let prepared = fit_image(&image, w.max(1) as u32, h.max(1) as u32, fit);
    let path = widget_image_path(widget_id, file);
    write_prepared(project_dir, &path, &prepared)?;

    let old = find_images_widget(project, widget_id)?.src.clone();
    if let Some(old) = old {
        if old != path {
            project.assets.retain(|asset| asset.path != old);
            remove_prepared(project_dir, &old);
        }
    }
    if !project.assets.iter().any(|asset| asset.path == path) {
        project.assets.push(Asset {
            path: path.clone(),
            role: AssetRole::Image,
        });
    }
    let widget = project
        .widgets
        .iter_mut()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::Images)
        .unwrap();
    widget.src = Some(path.clone());
    widget.image_fit = fit;
    // Pozycja jest zachowywana —— plik ma fizycznie rozmiar ramki.
    let _ = (x, y);
    Ok(path)
}

fn write_prepared(project_dir: &Path, path: &str, image: &image::RgbaImage) -> Result<(), String> {
    let target = project_dir.join("assets").join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Nie można utworzyć katalogu obrazków projektu: {e}"))?;
    }
    crate::project::write_png(&target, image)
        .map_err(|e| format!("Nie można zapisać obrazka projektu: {e}"))
}

fn remove_prepared(project_dir: &Path, path: &str) {
    let target = project_dir.join("assets").join(path);
    if target.is_file() {
        let _ = fs::remove_file(target);
    }
}

/// Ponownie przygotowuje obrazek widgetu po zmianie geometrii lub trybu dopasowania.
/// Źródłem jest zawsze oryginał z magazynu — kolejne skalowania nie kumulują strat.
pub fn scale_widget_image(
    project: &Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
) -> Result<(), String> {
    let widget = find_images_widget(project, widget_id)?;
    let Some(src) = &widget.src else { return Ok(()) };
    let Some(file) = source_file_name(src) else { return Ok(()) };
    let store = images_dir(app_data_root).join(&file);
    let project_copy = project_dir.join("assets").join(src);
    let source = if store.is_file() {
        store
    } else if project_copy.is_file() {
        project_copy
    } else {
        return Ok(());
    };
    let image = image::open(&source).map_err(|e| format!("Nie można odczytać obrazka: {e}"))?;
    let prepared = fit_image(
        &image,
        widget.w.max(1) as u32,
        widget.h.max(1) as u32,
        widget.image_fit,
    );
    write_prepared(project_dir, src, &prepared)
}

/// Bajty zasobu w ZIP-ie. Grafiki IMAGES są już przygotowane do ramki w projekcie;
/// dla starszych projektów stosujemy to samo skalowanie jeszcze raz w locie.
pub fn export_asset_bytes(
    project: &Project,
    project_dir: &Path,
    path: &str,
) -> Result<Vec<u8>, String> {
    let raw = fs::read(project_dir.join("assets").join(path))
        .map_err(|e| format!("Nie można odczytać zasobu {path}: {e}"))?;
    if !path.starts_with(&format!("{WIDGET_IMAGE_DIR}/")) {
        return Ok(raw);
    }
    let Some(widget) = project
        .widgets
        .iter()
        .find(|w| w.kind == WidgetKind::Images && w.src.as_deref() == Some(path))
    else {
        return Ok(raw);
    };
    let image = image::load_from_memory(&raw)
        .map_err(|e| format!("Nie można odczytać obrazka {path}: {e}"))?;
    let prepared = fit_image(
        &image,
        widget.w.max(1) as u32,
        widget.h.max(1) as u32,
        widget.image_fit,
    );
    encode_png(&prepared)
}

/// Usuwa przygotowane grafiki i wpisy zasobów, których nie używa już żaden widget.
pub fn prune_unused(project: &mut Project, project_dir: &Path) {
    let prefix = format!("{WIDGET_IMAGE_DIR}/");
    let used: Vec<String> = project
        .widgets
        .iter()
        .filter(|w| w.kind == WidgetKind::Images)
        .filter_map(|w| w.src.clone())
        .collect();
    let unused: Vec<String> = project
        .assets
        .iter()
        .filter(|asset| asset.path.starts_with(&prefix) && !used.contains(&asset.path))
        .map(|asset| asset.path.clone())
        .collect();
    project
        .assets
        .retain(|asset| !asset.path.starts_with(&prefix) || used.contains(&asset.path));
    for path in unused {
        remove_prepared(project_dir, &path);
    }
}

pub(crate) fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            e.eq_ignore_ascii_case("png")
                || e.eq_ignore_ascii_case("jpg")
                || e.eq_ignore_ascii_case("jpeg")
        })
        .unwrap_or(false)
}

pub(crate) fn valid_asset_name(file: &str) -> Result<(), String> {
    if file.is_empty()
        || file.contains('/')
        || file.contains('\\')
        || file.contains("..")
        || !is_image(Path::new(file))
    {
        return Err("Nieprawidłowa nazwa pliku obrazka".to_string());
    }
    Ok(())
}

pub(crate) fn sanitize_file_name(name: &str) -> String {
    let safe_input = name.replace(['/', '\\'], "_");
    let path = Path::new(&safe_input);
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("image");
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();
    let ext = if ext == "jpeg" { "jpg" } else { ext.as_str() };
    let mut safe: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.trim_matches('.').trim().is_empty() {
        safe = "image".to_string();
    }
    safe.truncate(80);
    format!("{safe}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{create_project_in, NewProject};
    use image::{Rgba, RgbaImage};

    fn source_image(root: &Path) -> PathBuf {
        let img = RgbaImage::from_pixel(800, 300, Rgba([15, 60, 120, 255]));
        let path = root.join("wide.png");
        crate::project::write_png(&path, &img).unwrap();
        path
    }

    #[test]
    fn store_adds_lists_and_removes_images() {
        let root = std::env::temp_dir().join(format!("rw-images-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = source_image(&root);

        let added = add(&root, &source).unwrap();
        assert_eq!(added.file, "wide.png");
        assert_eq!((added.width, added.height), (800, 300));
        assert_eq!(list(&root).unwrap().len(), 1);
        remove(&root, "wide.png").unwrap();
        assert!(list(&root).unwrap().is_empty());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn images_widget_prepares_png_and_tracks_asset() {
        let root = std::env::temp_dir().join(format!("rw-images-widget-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = source_image(&root);
        add(&root, &source).unwrap();

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject { name: "ImageFace".into(), author: "Dev".into(), app_id: 5 },
        )
        .unwrap();
        let widget = crate::widgets::add_widget(&mut project, "images").unwrap();
        let path = set_widget_image(&mut project, &dir, &root, &widget.id, "wide.png", FitMode::Contain).unwrap();
        assert_eq!(path, format!("images/{}/wide.png", widget.id));
        let prepared = image::open(dir.join("assets").join(&path)).unwrap();
        assert_eq!(prepared.dimensions(), (200, 200));
        assert!(project.assets.iter().any(|a| a.path == path && a.role == AssetRole::Image));
        assert!(!crate::validate::validate(&project).iter().any(|e| e.contains(&path)));

        let bytes = export_asset_bytes(&project, &dir, &path).unwrap();
        let exported = image::load_from_memory(&bytes).unwrap();
        assert_eq!(exported.dimensions(), (200, 200));

        fs::remove_dir_all(&root).ok();
    }
}
