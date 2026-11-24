//! Osobny magazyn teł — nie dzieli się z obrazkami widgetu IMAGES.
//!
//! Pliki trafiają do `%APPDATA%\RustyWatch\Backgrounds`. Przy pierwszym starcie
//! kopiujemy tam starsze tła z `%APPDATA%\RustyWatch\Images`, więc istniejące
//! projekty i ich tła pozostają dostępne. W projekcie oryginał leży w
//! `assets/backgrounds/<plik>`, a wersja gotowa dla ZeppOS w `assets/bg.png`
//! (zawsze 466 × 466 przy włączonym automatycznym skalowaniu — `FitMode::Cover`).

use image::GenericImageView;
use std::fs;
use std::path::{Path, PathBuf};

use crate::image_fit::{fit_image, FitMode};
use crate::images::{is_image, sanitize_file_name, valid_asset_name, ImageAsset};
use crate::model::{Asset, AssetRole, BackgroundConfig, Project, SCREEN_SIZE};

pub const BACKGROUND_DIR: &str = "Backgrounds";
pub const BACKGROUND_FILE: &str = "bg.png";
pub const BACKGROUND_SOURCE_DIR: &str = "backgrounds";
pub const JS_BACKGROUND_PATH: &str = "bg.png";
/// Wbudowane tła są zwykłymi plikami w magazynie — widać je na liście Assets.
pub const BUILTIN_BLACK_FILE: &str = "black.png";
pub const BUILTIN_STARRY_FILE: &str = "starry.png";
/// Starsze zapisy bez rozszerzenia nadal rozpoznajemy (kompatybilność wsteczna).
pub const BUILTIN_BLACK: &str = "black";
pub const BUILTIN_STARRY: &str = "starry";

pub fn is_builtin(file: &str) -> bool {
    matches!(
        file,
        BUILTIN_BLACK_FILE | BUILTIN_STARRY_FILE | BUILTIN_BLACK | BUILTIN_STARRY
    )
}
/// Znacznik jednorazowej migracji — nowe obrazki IMAGES nie trafiają już do teł.
const MIGRATION_MARKER: &str = ".migrated";

pub fn backgrounds_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join(BACKGROUND_DIR)
}

/// Jednorazowa migracja: starsze wersje trzymały tła razem z obrazkami w `Images`.
/// Kopiujemy je do osobnego magazynu, nic nie nadpisując.
pub fn ensure_migrated(app_data_root: &Path) -> Result<(), String> {
    let target = backgrounds_dir(app_data_root);
    fs::create_dir_all(&target)
        .map_err(|e| format!("Nie można utworzyć katalogu teł: {e}"))?;
    // Wbudowane pliki dosiewamy przy każdym starcie, także gdy migracja już się odbyła.
    ensure_builtins(app_data_root)?;
    let marker = target.join(MIGRATION_MARKER);
    if marker.exists() {
        return Ok(());
    }
    let old = crate::images::images_dir(app_data_root);
    let entries = match fs::read_dir(&old) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("Nie można odczytać starego katalogu obrazków: {e}")),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || !is_image(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let destination = target.join(name);
        if !destination.exists() {
            fs::copy(&path, &destination)
                .map_err(|e| format!("Nie można przenieść tła {name}: {e}"))?;
        }
    }
    fs::write(&marker, b"1").map_err(|e| format!("Nie można zapisać znacznika migracji: {e}"))?;
    ensure_builtins(app_data_root)
}

/// Zapisuje wbudowane tła jako zwykłe pliki magazynu, więc widnieją na liście
/// Assets i można je wybrać tak samo jak zaimportowane obrazy. Nie nadpisuje
/// plików, które użytkownik już zmienił.
pub fn ensure_builtins(app_data_root: &Path) -> Result<(), String> {
    let dir = backgrounds_dir(app_data_root);
    fs::create_dir_all(&dir).map_err(|e| format!("Nie można utworzyć katalogu teł: {e}"))?;
    for (file, image) in [
        (BUILTIN_BLACK_FILE, crate::project::generate_black_background(SCREEN_SIZE)),
        (BUILTIN_STARRY_FILE, crate::project::generate_starry_background(SCREEN_SIZE)),
    ] {
        let target = dir.join(file);
        if target.exists() {
            continue;
        }
        crate::project::write_png(&target, &image)
            .map_err(|e| format!("Nie można zapisać wbudowanego tła {file}: {e}"))?;
    }
    Ok(())
}

/// Lista teł w dedykowanym magazynie, posortowana po nazwie pliku.
pub fn list(app_data_root: &Path) -> Result<Vec<ImageAsset>, String> {
    let dir = backgrounds_dir(app_data_root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Nie można odczytać katalogu teł: {e}")),
    };
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać tła: {e}"))?;
        let path = entry.path();
        if !path.is_file() || !is_image(&path) {
            continue;
        }
        let image = image::open(&path)
            .map_err(|e| format!("Nie można odczytać tła {}: {e}", path.display()))?;
        let (width, height) = image.dimensions();
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        result.push(ImageAsset { file, width, height, bytes });
    }
    result.sort_by(|a, b| a.file.to_lowercase().cmp(&b.file.to_lowercase()));
    Ok(result)
}

/// Importuje tło do dedykowanego magazynu.
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
        .unwrap_or("background.png");
    let file = sanitize_file_name(name);
    let dir = backgrounds_dir(app_data_root);
    fs::create_dir_all(&dir).map_err(|e| format!("Nie można utworzyć katalogu teł: {e}"))?;
    let target = dir.join(&file);
    if target.exists() {
        return Err(format!("Tło {file} jest już w magazynie"));
    }
    fs::copy(source, &target).map_err(|e| format!("Nie można skopiować tła: {e}"))?;
    Ok(ImageAsset {
        file,
        width,
        height,
        bytes: fs::metadata(&target).map(|m| m.len()).unwrap_or(0),
    })
}

/// Usuwa tło z magazynu. Kopie w projektach zostają nienaruszone.
pub fn remove(app_data_root: &Path, file: &str) -> Result<(), String> {
    valid_asset_name(file)?;
    if is_builtin(file) {
        return Err("Wbudowanego tła nie można usunąć".to_string());
    }
    let path = backgrounds_dir(app_data_root).join(file);
    fs::remove_file(&path).map_err(|e| format!("Nie można usunąć tła: {e}"))
}

/// Jeśli otwarty projekt używa tła, którego nie ma jeszcze w dedykowanym magazynie,
/// kopiujemy oryginał z projektu. Dzięki temu „aktualny obrazek tarczy" jest
/// zawsze dostępny w nowym folderze teł, także w starszych projektach.
pub fn ensure_project_background(
    project: &Project,
    project_dir: &Path,
    app_data_root: &Path,
) -> Result<(), String> {
    let Some(config) = &project.background else { return Ok(()) };
    // Wbudowane tła nie mają pliku źródłowego w magazynie.
    if config.image == BUILTIN_BLACK || config.image == BUILTIN_STARRY {
        return Ok(());
    }
    valid_asset_name(&config.image)?;
    let target = backgrounds_dir(app_data_root).join(&config.image);
    if target.is_file() {
        return Ok(());
    }
    let source = project_dir
        .join("assets")
        .join(BACKGROUND_SOURCE_DIR)
        .join(&config.image);
    if !source.is_file() {
        return Ok(());
    }
    fs::create_dir_all(backgrounds_dir(app_data_root))
        .map_err(|e| format!("Nie można utworzyć katalogu teł: {e}"))?;
    fs::copy(&source, &target)
        .map_err(|e| format!("Nie można skopiować tła projektu do magazynu: {e}"))?;
    Ok(())
}

/// Ustawia tło projektu. `auto_scale` przycina obrazek do 466×466 metodą `Cover`
/// (bez białych pasków); w przeciwnym razie zachowuje oryginalne wymiary.
pub fn set_background(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
    file: &str,
    auto_scale: bool,
) -> Result<(), String> {
    let file = match file {
        // Starsze projekty i wywołania FFI bez rozszerzenia.
        BUILTIN_STARRY => BUILTIN_STARRY_FILE,
        BUILTIN_BLACK => BUILTIN_BLACK_FILE,
        "" => BUILTIN_BLACK_FILE,
        other => other,
    };
    if is_builtin(file) {
        // Wbudowane tła to zwykłe pliki magazynu — zawsze są dostępne na liście.
        ensure_builtins(app_data_root)?;
    }
    valid_asset_name(file)?;
    let store = backgrounds_dir(app_data_root).join(file);
    let legacy_store = crate::images::images_dir(app_data_root).join(file);
    let project_source = project_dir.join("assets").join(BACKGROUND_SOURCE_DIR).join(file);
    let source = if store.is_file() {
        store.clone()
    } else if legacy_store.is_file() {
        legacy_store.clone()
    } else if project_source.is_file() {
        project_source.clone()
    } else {
        return Err(format!("Obrazek nie jest zainstalowany: {file}"));
    };

    let image = image::open(&source).map_err(|e| format!("Nie można odczytać obrazka: {e}"))?;
    let prepared = if auto_scale {
        fit_image(&image, SCREEN_SIZE, SCREEN_SIZE, FitMode::Cover)
    } else {
        image.to_rgba8()
    };
    let bg_path = project_dir.join("assets").join(BACKGROUND_FILE);
    crate::project::write_png(&bg_path, &prepared)
        .map_err(|e| format!("Nie można zapisać tła projektu: {e}"))?;

    if !project_source.is_file() {
        if let Some(parent) = project_source.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Nie można utworzyć katalogu obrazów projektu: {e}"))?;
        }
        fs::copy(&source, &project_source)
            .map_err(|e| format!("Nie można skopiować obrazka do projektu: {e}"))?;
    }
    // Tło znalezione w starym magazynie staje się dostępne w nowym, dedykowanym.
    if !store.is_file() {
        fs::create_dir_all(backgrounds_dir(app_data_root))
            .map_err(|e| format!("Nie można utworzyć katalogu teł: {e}"))?;
        fs::copy(&source, &store)
            .map_err(|e| format!("Nie można zapisać tła w nowym magazynie: {e}"))?;
    }

    if !project
        .assets
        .iter()
        .any(|a| a.path == JS_BACKGROUND_PATH && a.role == AssetRole::Background)
    {
        project.assets.push(Asset {
            path: JS_BACKGROUND_PATH.to_string(),
            role: AssetRole::Background,
        });
    }
    let source_path = format!("{BACKGROUND_SOURCE_DIR}/{file}");
    if !project.assets.iter().any(|a| a.path == source_path) {
        project.assets.push(Asset {
            path: source_path,
            role: AssetRole::Image,
        });
    }
    project.background = Some(BackgroundConfig {
        image: file.to_string(),
        auto_scale,
    });
    Ok(())
}

/// Ustawia jedno z wbudowanych teł jako zwykły plik źródłowy projektu.
pub fn set_builtin_background(
    project: &mut Project,
    project_dir: &Path,
    file: &str,
) -> Result<(), String> {
    if let Some(config) = project.background.take() {
        if !is_builtin(&config.image) {
            let source_path = format!("{BACKGROUND_SOURCE_DIR}/{}", config.image);
            project.assets.retain(|a| a.path != source_path);
            let path = project_dir.join("assets").join(&source_path);
            if path.is_file() {
                fs::remove_file(&path)
                    .map_err(|e| format!("Nie można usunąć źródła tła z projektu: {e}"))?;
            }
        }
    }
    let image = if file == BUILTIN_STARRY_FILE {
        crate::project::generate_starry_background(SCREEN_SIZE)
    } else {
        crate::project::generate_black_background(SCREEN_SIZE)
    };
    crate::project::write_png(&project_dir.join("assets").join(BACKGROUND_FILE), &image)
        .map_err(|e| format!("Nie można zapisać wbudowanego tła: {e}"))?;
    let source_dir = project_dir.join("assets").join(BACKGROUND_SOURCE_DIR);
    fs::create_dir_all(&source_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu teł projektu: {e}"))?;
    crate::project::write_png(&source_dir.join(file), &image)
        .map_err(|e| format!("Nie można zapisać źródła wbudowanego tła: {e}"))?;
    if !project
        .assets
        .iter()
        .any(|a| a.path == JS_BACKGROUND_PATH && a.role == AssetRole::Background)
    {
        project.assets.push(Asset {
            path: JS_BACKGROUND_PATH.to_string(),
            role: AssetRole::Background,
        });
    }
    let source_path = format!("{BACKGROUND_SOURCE_DIR}/{file}");
    if !project.assets.iter().any(|a| a.path == source_path) {
        project.assets.push(Asset {
            path: source_path,
            role: AssetRole::Image,
        });
    }
    project.background = Some(BackgroundConfig {
        image: file.to_string(),
        auto_scale: true,
    });
    Ok(())
}

/// Przywraca domyślne (całkowicie czarne) tło i usuwa źródło poprzedniego obrazka.
pub fn clear_background(project: &mut Project, project_dir: &Path) -> Result<(), String> {
    set_builtin_background(project, project_dir, BUILTIN_BLACK_FILE)
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
    fn background_store_is_separate_and_migrates_legacy_images() {
        let root = std::env::temp_dir().join(format!("rw-bg-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = source_image(&root);
        crate::images::add(&root, &source).unwrap();

        ensure_migrated(&root).unwrap();
        // Migracja dodaje stary obrazek, a wbudowane tła są widocznymi plikami.
        assert_eq!(list(&root).unwrap().len(), 3);
        assert_eq!(crate::images::list(&root).unwrap().len(), 1);
        // Ten sam plik nie może zostać zaimportowany drugi raz do magazynu teł.
        assert!(add(&root, &source).is_err());
        remove(&root, "wide.png").unwrap();
        assert_eq!(list(&root).unwrap().len(), 2);
        // Obrazek w ogólnym magazynie zostaje nienaruszony.
        assert_eq!(crate::images::list(&root).unwrap().len(), 1);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn set_background_covers_and_toggles_auto_scale() {
        let root = std::env::temp_dir().join(format!("rw-images-bg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = source_image(&root);
        add(&root, &source).unwrap();

        let (dir, mut project) = create_project_in(
            &root,
            &NewProject { name: "Background".into(), author: "Dev".into(), app_id: 5 },
        )
        .unwrap();

        set_background(&mut project, &dir, &root, "wide.png", true).unwrap();
        let bg = image::open(dir.join("assets/bg.png")).unwrap();
        assert_eq!(bg.dimensions(), (SCREEN_SIZE, SCREEN_SIZE));
        assert_eq!(bg.get_pixel(0, 0).0[3], 255);
        assert_eq!(project.background.as_ref().unwrap().image, "wide.png");
        assert!(dir.join("assets/backgrounds/wide.png").is_file());
        assert!(project.assets.iter().any(|a| a.path == "backgrounds/wide.png" && a.role == AssetRole::Image));

        set_background(&mut project, &dir, &root, "wide.png", false).unwrap();
        let bg = image::open(dir.join("assets/bg.png")).unwrap();
        assert_eq!(bg.dimensions(), (800, 300));
        assert!(!project.background.as_ref().unwrap().auto_scale);

        clear_background(&mut project, &dir).unwrap();
        assert_eq!(project.background.as_ref().unwrap().image, BUILTIN_BLACK_FILE);
        assert!(!dir.join("assets/backgrounds/wide.png").exists());
        assert!(dir.join("assets/backgrounds/black.png").is_file());
        assert_eq!(image::open(dir.join("assets/bg.png")).unwrap().dimensions(), (SCREEN_SIZE, SCREEN_SIZE));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn builtin_black_is_default_and_starry_is_selectable() {
        let root = std::env::temp_dir().join(format!("rw-bg-builtin-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let (dir, mut project) = create_project_in(
            &root,
            &NewProject { name: "Builtin".into(), author: "Dev".into(), app_id: 6 },
        )
        .unwrap();

        // Nowy projekt domyślnie ma całkowicie czarne tło jako zwykły plik.
        let bg = image::open(dir.join("assets/bg.png")).unwrap().to_rgba8();
        for (x, y, pixel) in bg.enumerate_pixels() {
            assert_eq!(pixel.0, [0, 0, 0, 255], "pixel {x},{y} is not black");
        }
        assert_eq!(project.background.as_ref().unwrap().image, BUILTIN_BLACK_FILE);
        assert!(dir.join("assets/backgrounds/black.png").is_file());

        // Wbudowane tła są plikami w magazynie — widać je na liście Assets.
        ensure_builtins(&root).unwrap();
        let listed: Vec<String> = list(&root).unwrap().into_iter().map(|i| i.file).collect();
        assert!(listed.iter().any(|f| f == BUILTIN_BLACK_FILE));
        assert!(listed.iter().any(|f| f == BUILTIN_STARRY_FILE));
        assert!(remove(&root, BUILTIN_BLACK_FILE).is_err());

        // Dawne tło jest dostępne jako „Starry".
        set_background(&mut project, &dir, &root, BUILTIN_STARRY_FILE, true).unwrap();
        assert_eq!(project.background.as_ref().unwrap().image, BUILTIN_STARRY_FILE);
        let starry = image::open(dir.join("assets/bg.png")).unwrap();
        assert!(starry.get_pixel(233, 233).0[2] > starry.get_pixel(233, 233).0[0]);
        assert!(dir.join("assets/backgrounds/starry.png").is_file());

        // Powrót do domyślnego — znowu pełna czerń.
        set_background(&mut project, &dir, &root, "", true).unwrap();
        assert_eq!(project.background.as_ref().unwrap().image, BUILTIN_BLACK_FILE);
        let black = image::open(dir.join("assets/bg.png")).unwrap().to_rgba8();
        assert_eq!(black.get_pixel(0, 0).0, [0, 0, 0, 255]);
        fs::remove_dir_all(&root).ok();
    }
}
