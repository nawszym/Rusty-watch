//! Magazyn paczek ikon stanu systemu (`hmUI.widget.IMG_STATUS`).
//!
//! Pakiety żyją w `%APPDATA%\RustyWatch\Status\<id>\` i zawierają cztery
//! przezroczyste pliki PNG: `alarm.png`, `dnd.png`, `bluetooth.png`, `lock.png`.
//! Wzorzec katalogów i cykl życia są celowo takie same jak w `hands.rs`:
//! wbudowane pakiety z `Assets/Status` są seedowane przy starcie, wybrany pakiet
//! jest kopiowany do `<projekt>/assets/status/<id>/`, a ZIP jest samowystarczalny.
//!
//! ZeppOS sam pokazuje/ukrywa ikonę w zależności od stanu zegarka — nie tworzymy
//! tu żadnych sensorów ani timerów.

use image::imageops::FilterType;
use image::{GenericImageView, ImageEncoder, Rgba, RgbaImage};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::asset_cache;
use crate::model::{Asset, AssetRole, Project, StatusType, WidgetKind};

pub const STATUS_DIR: &str = "Status";
pub const DEFAULT_STATUS_PACK: &str = "Classic";
/// Jedyny wbudowany pakiet — `Classic` (domyślny, jak czarne tło).
/// Pozostałe paczki można dodawać z plików; nie są one wbudowane.
pub const BUILTIN_PACKS: [&str; 1] = [DEFAULT_STATUS_PACK];
pub const JS_STATUS_PREFIX: &str = "status/";
const ALL_TYPES: [StatusType; 4] = [
    StatusType::Alarm,
    StatusType::Dnd,
    StatusType::Bluetooth,
    StatusType::Lock,
];

/// Nazwy wszystkich czterech plików ikon — jedyne źródło odcisku paczki.
fn icon_names() -> [&'static str; 4] {
    [
        StatusType::Alarm.file_name(),
        StatusType::Dnd.file_name(),
        StatusType::Bluetooth.file_name(),
        StatusType::Lock.file_name(),
    ]
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPack {
    pub id: String,
    pub name: String,
    /// Bok kwadratowej ikony w pikselach (np. 32).
    pub size: u32,
    /// Odcisk plików ikon — miniaturka jest odświeżana tylko przy jego zmianie.
    pub fingerprint: String,
}

pub fn status_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join(STATUS_DIR)
}

pub fn pack_dir(app_data_root: &Path, id: &str) -> PathBuf {
    status_dir(app_data_root).join(id)
}

pub fn js_status_path(id: &str, status: StatusType) -> String {
    format!("{JS_STATUS_PREFIX}{id}/{}", status.file_name())
}

fn valid_set_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
    {
        return Err(format!("Nieprawidłowa nazwa paczki ikon: {id}"));
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
        "Status".to_string()
    } else {
        cleaned
    }
}

/// Paczka jest wykrywana tylko wtedy, gdy ma **wszystkie cztery** pliki.
fn describe_pack(path: &Path, id: &str) -> Option<StatusPack> {
    if !path.is_dir() {
        return None;
    }
    let size = read_size(&path.join(StatusType::Alarm.file_name()))?;
    for status in ALL_TYPES {
        if read_size(&path.join(status.file_name()))? != size {
            return None;
        }
    }
    let fingerprint = asset_cache::files_fingerprint(path, &icon_names()).ok()?;
    Some(StatusPack {
        id: id.to_string(),
        name: id.to_string(),
        size,
        fingerprint,
    })
}

fn read_size(path: &Path) -> Option<u32> {
    let image = image::open(path).ok()?;
    let (w, h) = image.dimensions();
    (w == h).then_some(w)
}

/// Kopiuje każdą wbudowaną paczkę do magazynu i odświeża ją, gdy pliki
/// dostarczone z instalatorem zmieniły się (np. po aktualizacji aplikacji).
///
/// Nie wolno wcześnie wracać po `Classic` — nowe paczki dołączone do
/// `Assets/Status` muszą pojawić się także w istniejących instalacjach.
pub fn ensure_default(app_data_root: &Path, bundled_dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(bundled_dir)
        .map_err(|e| format!("Nie można odczytać wbudowanych ikon statusu: {e}"))?;
    let mut found = false;
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
        let bundled_fingerprint = asset_cache::files_fingerprint(&bundled, &icon_names())?;
        let target = pack_dir(app_data_root, &id);
        // Znacznik jest zapisywany przy każdym seedowaniu. Jego brak (stara
        // instalacja) albo zmiana odcisku źródła oznacza konieczność kopiowania.
        let unchanged = describe_pack(&target, &id).is_some()
            && asset_cache::read_bundled_version(&target).as_deref()
                == Some(bundled_fingerprint.as_str());
        if unchanged {
            continue;
        }
        fs::create_dir_all(&target)
            .map_err(|e| format!("Nie można utworzyć katalogu ikon statusu: {e}"))?;
        for status in ALL_TYPES {
            fs::copy(bundled.join(status.file_name()), target.join(status.file_name()))
                .map_err(|e| format!("Nie można skopiować ikony statusu: {e}"))?;
        }
        asset_cache::write_bundled_version(&target, &bundled_fingerprint)?;
        // Podmiana plików musi natychmiast odnowić miniaturkę — `ensure_previews`
        // na końcu zobaczy już aktualny odcisk i pominie tę paczkę.
        generate_preview(app_data_root, &id)?;
    }
    if !found {
        return Err(format!(
            "Nie znaleziono wbudowanych paczek ikon statusu: {}",
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

/// Rysuje miniaturkę: `alarm`, `dnd` w górnym rzędzie; `bluetooth`, `lock` w dolnym.
pub fn generate_preview(app_data_root: &Path, id: &str) -> Result<(), String> {
    const SIZE: u32 = 128;
    const CELL: u32 = 64;
    const ICON: u32 = 48;
    let source = pack_dir(app_data_root, id);
    let mut canvas = RgbaImage::from_pixel(SIZE, SIZE, Rgba([0, 0, 0, 0]));
    for (index, status) in ALL_TYPES.iter().enumerate() {
        let path = source.join(status.file_name());
        let original = image::open(&path)
            .map_err(|e| format!("Nie można odczytać ikony statusu: {e}"))?
            .to_rgba8();
        let icon = image::imageops::resize(&original, ICON, ICON, FilterType::Lanczos3);
        let x = (index % 2) as u32 * CELL + (CELL - ICON) / 2;
        let y = (index / 2) as u32 * CELL + (CELL - ICON) / 2;
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
pub fn list(app_data_root: &Path) -> Result<Vec<StatusPack>, String> {
    let entries = match fs::read_dir(status_dir(app_data_root)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Nie można odczytać katalogu ikon statusu: {e}")),
    };
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać paczki ikon: {e}"))?;
        let id = entry.file_name().to_string_lossy().to_string();
        if let Some(pack) = describe_pack(&entry.path(), &id) {
            result.push(pack);
        }
    }
    result.sort_by(|a, b| {
        (a.id != DEFAULT_STATUS_PACK)
            .cmp(&(b.id != DEFAULT_STATUS_PACK))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

/// Dodaje paczkę z wybranego folderu. Wymagane są wszystkie cztery pliki.
pub fn add(app_data_root: &Path, folder: &Path) -> Result<StatusPack, String> {
    if !folder.is_dir() {
        return Err(format!("Folder nie istnieje: {}", folder.display()));
    }
    let pack = describe_pack(folder, "tmp")
        .ok_or_else(|| "Paczka musi zawierać alarm.png, dnd.png, bluetooth.png i lock.png (kwadratowe, tego samego rozmiaru)".to_string())?;
    let raw_name = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Status");
    let id = sanitize_set_id(raw_name);
    valid_set_id(&id)?;
    let target = pack_dir(app_data_root, &id);
    if target.exists() {
        return Err(format!("Paczka ikon statusu {id} jest już zainstalowana"));
    }
    fs::create_dir_all(&target).map_err(|e| format!("Nie można utworzyć paczki: {e}"))?;
    for status in ALL_TYPES {
        fs::copy(folder.join(status.file_name()), target.join(status.file_name()))
            .map_err(|e| format!("Nie można skopiować ikony statusu: {e}"))?;
    }
    generate_preview(app_data_root, &id)?;
    let fingerprint = asset_cache::files_fingerprint(&target, &icon_names())?;
    Ok(StatusPack {
        id,
        name: pack.name,
        size: pack.size,
        fingerprint,
    })
}

pub fn remove(app_data_root: &Path, id: &str) -> Result<(), String> {
    valid_set_id(id)?;
    if BUILTIN_PACKS.contains(&id) {
        return Err("Wbudowanej paczki ikon statusu nie można usunąć".to_string());
    }
    let path = pack_dir(app_data_root, id);
    if !path.is_dir() {
        return Err(format!("Paczka ikon statusu nie jest zainstalowana: {id}"));
    }
    fs::remove_dir_all(&path).map_err(|e| format!("Nie można usunąć paczki ikon: {e}"))
}

/// Przypisuje paczkę do widgetu statusu i kopiuje jej pliki do projektu.
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
        .ok_or_else(|| format!("Paczka ikon statusu nie jest zainstalowana: {set_id}"))?;

    let target_dir = project_dir.join("assets").join(JS_STATUS_PREFIX).join(set_id);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu ikon w projekcie: {e}"))?;

    let status = project
        .widgets
        .iter()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::StatusIcon)
        .and_then(|w| w.status_type)
        .ok_or_else(|| format!("Nie znaleziono widgetu statusu: {widget_id}"))?;

    for kind in ALL_TYPES {
        fs::copy(source.join(kind.file_name()), target_dir.join(kind.file_name()))
            .map_err(|e| format!("Nie można skopiować ikony do projektu: {e}"))?;
        let path = js_status_path(set_id, kind);
        if !project.assets.iter().any(|a| a.path == path) {
            project.assets.push(Asset {
                path,
                role: AssetRole::StatusIcon,
            });
        }
    }

    let widget = project
        .widgets
        .iter_mut()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::StatusIcon)
        .unwrap();
    widget.status_icon_set = Some(set_id.to_string());
    widget.status_type = Some(status);
    widget.src = Some(js_status_path(set_id, status));
    Ok(())
}

/// Daje domyślną paczkę widgetom statusu, które jej jeszcze nie mają.
pub fn attach_default_to_missing(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
) -> Result<(), String> {
    let missing: Vec<String> = project
        .widgets
        .iter()
        .filter(|w| w.kind == WidgetKind::StatusIcon && w.status_icon_set.is_none())
        .map(|w| w.id.clone())
        .collect();
    for id in missing {
        attach_to_project(project, project_dir, app_data_root, &id, DEFAULT_STATUS_PACK)?;
    }
    Ok(())
}

/// Podmienia kopie ikon w projekcie na wersje z magazynu, gdy te się zmieniły.
///
/// Dotyczy projektów utworzonych przed aktualizacją wbudowanych paczek — bez
/// tego stara kopia `Classic` zostawałaby w ZIP-ie mimo odświeżenia magazynu.
/// Brakujące pliki w magazynie są pomijane, żeby nie uszkodzić starszych projektów.
pub fn refresh_attached(
    app_data_root: &Path,
    project_dir: &Path,
    project: &mut Project,
) -> Result<bool, String> {
    let mut changed = false;
    for asset in &project.assets {
        if asset.role != AssetRole::StatusIcon {
            continue;
        }
        let relative = asset
            .path
            .strip_prefix(JS_STATUS_PREFIX)
            .unwrap_or(&asset.path);
        let mut parts = relative.split('/');
        let (Some(pack_id), Some(file_name)) = (parts.next(), parts.next()) else {
            continue;
        };
        if parts.next().is_some()
            || pack_id.is_empty()
            || file_name.is_empty()
            || file_name == ".."
            || file_name.contains(['/', '\\'])
            || valid_set_id(pack_id).is_err()
        {
            continue;
        }
        let source = pack_dir(app_data_root, pack_id).join(file_name);
        let target = project_dir.join("assets").join(&asset.path);
        if !source.is_file() || !target.is_file() {
            continue;
        }
        let pristine = fs::read(&source)
            .map_err(|e| format!("Nie można odczytać ikony z magazynu: {e}"))?;
        if fs::read(&target).unwrap_or_default() != pristine {
            fs::write(&target, &pristine)
                .map_err(|e| format!("Nie można odnowić ikony w projekcie: {e}"))?;
            changed = true;
        }
    }
    Ok(changed)
}

/// Usuwa z projektu ikony, których nie używa żaden widget.
pub fn prune_unused(project: &mut Project, project_dir: &Path) {
    let mut used = std::collections::HashSet::new();
    for widget in &project.widgets {
        if let Some(src) = &widget.src {
            used.insert(src.as_str());
        }
    }
    project.assets.retain(|asset| {
        if asset.role != AssetRole::StatusIcon || used.contains(asset.path.as_str()) {
            return true;
        }
        let path = project_dir.join("assets").join(&asset.path);
        if path.is_file() {
            let _ = fs::remove_file(path);
        }
        false
    });
}

/// Rozmiar docelowy ikony dla danej ścieżki zasobu.
fn target_size(project: &Project, path: &str) -> Option<(u32, u32)> {
    project
        .widgets
        .iter()
        .find(|w| w.kind == WidgetKind::StatusIcon && w.src.as_deref() == Some(path))
        .map(|w| (w.w.max(1) as u32, w.h.max(1) as u32))
}

fn resized_png(bytes: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let image =
        image::load_from_memory(bytes).map_err(|e| format!("Nie można odczytać ikony statusu: {e}"))?;
    if image.dimensions() == (width, height) {
        return Ok(bytes.to_vec());
    }
    let resized = image.resize_exact(width, height, FilterType::Lanczos3).to_rgba8();
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out).write_image(
        resized.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| format!("Nie można zapisać ikony statusu: {e}"))?;
    Ok(out)
}

/// Bajty ikony wstawiane do ZIP — zabezpieczenie dla starszych projektów.
pub fn export_asset_bytes(
    project: &Project,
    project_dir: &Path,
    path: &str,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(project_dir.join("assets").join(path))
        .map_err(|e| format!("Nie można odczytać zasobu {path}: {e}"))?;
    match target_size(project, path) {
        Some((w, h)) => resized_png(&bytes, w, h),
        None => Ok(bytes),
    }
}

/// Fizycznie dopasowuje kopię ikony do rozmiaru ramki widgetu.
/// Źródłem jest zawsze oryginał z magazynu, więc kolejne skalowania nie kumulują się.
pub fn scale_project_icons(
    project: &Project,
    project_dir: &Path,
    app_data_root: &Path,
    widget_id: &str,
) -> Result<(), String> {
    let (set_id, status, w, h) = project
        .widgets
        .iter()
        .find(|w| w.id == widget_id && w.kind == WidgetKind::StatusIcon)
        .and_then(|widget| {
            Some((
                widget.status_icon_set.clone()?,
                widget.status_type?,
                widget.w.max(1) as u32,
                widget.h.max(1) as u32,
            ))
        })
        .ok_or_else(|| format!("Nie znaleziono widgetu statusu: {widget_id}"))?;
    let from = pack_dir(app_data_root, &set_id).join(status.file_name());
    let bytes = fs::read(&from).map_err(|e| format!("Nie można odczytać ikony z magazynu: {e}"))?;
    let prepared = resized_png(&bytes, w, h)?;
    let target = project_dir.join("assets").join(js_status_path(&set_id, status));
    fs::write(target, prepared).map_err(|e| format!("Nie można zapisać ikony w projekcie: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Project, Widget};
    use image::{ImageEncoder, Rgba, RgbaImage};
    use std::fs::File;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// Zapisuje ikonę w tym samym rozmiarze, ale z inną treścią i czasem modyfikacji.
    fn rewrite_icon(path: &Path, size: u32, color: Rgba<u8>) {
        let image = RgbaImage::from_pixel(size, size, color);
        let file = File::create(path).unwrap();
        image::codecs::png::PngEncoder::new(file)
            .write_image(
                image.as_raw(),
                size,
                size,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        let later = SystemTime::now() + Duration::from_secs(60);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(later)
            .unwrap();
    }

    fn bundled(root: &Path, id: &str, size: u32) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        let image = RgbaImage::from_pixel(size, size, Rgba([255, 255, 255, 255]));
        for status in ALL_TYPES {
            let file = File::create(dir.join(status.file_name())).unwrap();
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

    fn roots(test: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("rw-status-icons-{test}-{}", std::process::id()));
        let app = base.join("appdata");
        let bundles = base.join("bundled");
        let project = base.join("project");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&bundles).unwrap();
        fs::create_dir_all(&project).unwrap();
        (app, bundles, project)
    }

    fn status_project() -> Project {
        let mut project = Project::starter_with("Test", "Dev", 1);
        project.widgets.clear();
        project.widgets.push(Widget {
            id: "status-dnd".to_string(),
            name: "DND".to_string(),
            kind: WidgetKind::StatusIcon,
            x: 100,
            y: 24,
            w: 32,
            h: 32,
            z: 0,
            text: None,
            text_size: None,
            font: None,
            time_format: None,
            color: None,
            src: None,
            image_fit: Default::default(),
            hand_set: None,
            hour_src: None,
            minute_src: None,
            second_src: None,
            cap_src: None,
            data_source: None,
            align: None,
            v_align: Default::default(),
            show_level: Default::default(),
            auto_scale: false,
            text_style: Default::default(),
            visible: true,
            measurement_system: None,
            show_unit: true,
            status_icon_set: None,
            status_type: Some(StatusType::Dnd),
        });
        project
    }

    #[test]
    fn pack_preview_uses_two_by_two_layout() {
        let (app, bundles, _) = roots("preview");
        bundled(&bundles, DEFAULT_STATUS_PACK, 32);
        ensure_default(&app, &bundles).unwrap();
        let preview = pack_dir(&app, DEFAULT_STATUS_PACK).join("preview.png");
        let image = image::open(&preview).unwrap();
        assert_eq!(image.dimensions(), (128, 128));
    }

    #[test]
    fn pack_needs_all_four_icons() {
        let (app, bundles, _) = roots("validation");
        bundled(&bundles, DEFAULT_STATUS_PACK, 32);
        bundled(&bundles, "Broken", 32);
        fs::remove_file(bundles.join("Broken").join("lock.png")).unwrap();
        ensure_default(&app, &bundles).unwrap();
        let packs = list(&app).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].id, DEFAULT_STATUS_PACK);
    }

    #[test]
    fn ensure_default_seeds_every_bundled_pack() {
        let (app, bundles, _) = roots("seed");
        bundled(&bundles, DEFAULT_STATUS_PACK, 32);
        bundled(&bundles, "Extra", 24);
        ensure_default(&app, &bundles).unwrap();
        let packs = list(&app).unwrap();
        assert_eq!(packs.len(), 2);
        assert_eq!(packs[0].id, DEFAULT_STATUS_PACK);
    }

    #[test]
    fn attach_and_scale_keeps_export_self_contained() {
        let (app, bundles, project_dir) = roots("attach");
        bundled(&bundles, DEFAULT_STATUS_PACK, 32);
        ensure_default(&app, &bundles).unwrap();
        let mut project = status_project();
        attach_to_project(&mut project, &project_dir, &app, "status-dnd", DEFAULT_STATUS_PACK)
            .unwrap();
        project.widgets[0].w = 48;
        project.widgets[0].h = 48;
        scale_project_icons(&project, &project_dir, &app, "status-dnd").unwrap();
        let path = js_status_path(DEFAULT_STATUS_PACK, StatusType::Dnd);
        let bytes = export_asset_bytes(&project, &project_dir, &path).unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!(image.dimensions(), (48, 48));
        assert_eq!(
            project
                .assets
                .iter()
                .filter(|a| a.role == AssetRole::StatusIcon)
                .count(),
            4
        );
    }

    #[test]
    fn builtin_packs_cannot_be_removed() {
        let (app, _, _) = roots("remove");
        assert!(remove(&app, DEFAULT_STATUS_PACK).is_err());
    }

    #[test]
    fn preview_is_regenerated_only_when_icons_change() {
        let (app, bundles, _) = roots("preview-cache");
        bundled(&bundles, DEFAULT_STATUS_PACK, 32);
        ensure_default(&app, &bundles).unwrap();

        let pack = pack_dir(&app, DEFAULT_STATUS_PACK);
        let names = icon_names();
        let preview = pack.join("preview.png");
        let sidecar = pack.join("preview.fingerprint");
        assert!(preview.is_file(), "miniaturka musi powstać przy seedowaniu");
        assert!(sidecar.is_file(), "musi powstać plik z odciskiem");
        let first_bytes = fs::read(&preview).unwrap();
        let first_stamp = fs::metadata(&preview).unwrap().modified().unwrap();
        let first_fp = asset_cache::files_fingerprint(&pack, &names).unwrap();
        assert_eq!(fs::read_to_string(&sidecar).unwrap().trim(), first_fp);

        // Niezmieniona paczka: ani dekodowania, ani zapisu preview.png.
        std::thread::sleep(Duration::from_millis(30));
        ensure_previews(&app).unwrap();
        assert_eq!(fs::read(&preview).unwrap(), first_bytes);
        assert_eq!(fs::metadata(&preview).unwrap().modified().unwrap(), first_stamp);

        // Zmiana jednej ikony (ten sam rozmiar, inna treść) musi odnowić podgląd.
        rewrite_icon(
            &pack.join(StatusType::Alarm.file_name()),
            32,
            Rgba([255, 0, 0, 255]),
        );
        let second_fp = asset_cache::files_fingerprint(&pack, &names).unwrap();
        assert_ne!(second_fp, first_fp, "odcisk musi się zmienić");
        ensure_previews(&app).unwrap();
        assert_ne!(fs::read(&preview).unwrap(), first_bytes);
        assert_eq!(fs::read_to_string(&sidecar).unwrap().trim(), second_fp);
    }

    #[test]
    fn ensure_default_refreshes_changed_bundled_pack() {
        let (app, bundles, _) = roots("bundled-refresh");
        bundled(&bundles, DEFAULT_STATUS_PACK, 32);
        ensure_default(&app, &bundles).unwrap();

        let installed = pack_dir(&app, DEFAULT_STATUS_PACK);
        let icon = installed.join(StatusType::Alarm.file_name());
        let before = fs::read(&icon).unwrap();
        assert!(asset_cache::read_bundled_version(&installed).is_some());

        // Symulujemy nową wersję ikony dostarczoną z instalatorem.
        let source_dir = bundles.join(DEFAULT_STATUS_PACK);
        let source = source_dir.join(StatusType::Alarm.file_name());
        rewrite_icon(&source, 32, Rgba([0, 255, 0, 255]));
        let bundled_fp = asset_cache::files_fingerprint(&source_dir, &icon_names()).unwrap();

        ensure_default(&app, &bundles).unwrap();
        assert_ne!(fs::read(&icon).unwrap(), before);
        assert_eq!(fs::read(&icon).unwrap(), fs::read(&source).unwrap());
        assert_eq!(
            asset_cache::read_bundled_version(&installed).as_deref(),
            Some(bundled_fp.as_str())
        );

        // Kolejne uruchomienie nie powinno już nic przepisywać.
        let stamp = fs::metadata(&icon).unwrap().modified().unwrap();
        ensure_default(&app, &bundles).unwrap();
        assert_eq!(fs::metadata(&icon).unwrap().modified().unwrap(), stamp);
    }

    #[test]
    fn refresh_attached_replaces_stale_project_icons() {
        let (app, bundles, project_dir) = roots("refresh-attached");
        bundled(&bundles, DEFAULT_STATUS_PACK, 32);
        ensure_default(&app, &bundles).unwrap();
        let mut project = status_project();
        attach_to_project(&mut project, &project_dir, &app, "status-dnd", DEFAULT_STATUS_PACK)
            .unwrap();

        let path = js_status_path(DEFAULT_STATUS_PACK, StatusType::Dnd);
        let target = project_dir.join("assets").join(&path);
        let store_icon = pack_dir(&app, DEFAULT_STATUS_PACK).join(StatusType::Dnd.file_name());
        rewrite_icon(&store_icon, 32, Rgba([0, 0, 255, 255]));

        assert!(refresh_attached(&app, &project_dir, &mut project).unwrap());
        assert_eq!(fs::read(&target).unwrap(), fs::read(&store_icon).unwrap());
        assert!(!refresh_attached(&app, &project_dir, &mut project).unwrap());
    }
}
