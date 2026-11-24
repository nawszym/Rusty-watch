//! Font assets shared between projects.
//!
//! Installed fonts live in `%APPDATA%\RustyWatch\Fonts` (one place for every
//! project), while exported watchfaces carry a copy under
//! `assets/<target>/fonts/<file>` and reference it as `fonts/<file>` in JS.
//! This module owns all of that path/logic so C# only displays results.

use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::{Asset, AssetRole, Project, WidgetKind};

/// Subfolder of the roaming app-data root that holds installed fonts.
pub const FONT_DIR: &str = "Fonts";
/// Font shipped with Rusty Watch (SIL Open Font License 1.1, see the repo's
/// `native/RustyWatch/Assets/Fonts/OFL-Inter.txt`).
pub const DEFAULT_FONT_FILE: &str = "Inter-Regular.ttf";
/// Prefix used by the watchface runtime (relative to the target's assets dir).
pub const JS_FONT_PREFIX: &str = "fonts/";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontAsset {
    /// File name inside the Fonts directory, e.g. `Inter-Regular.ttf`.
    pub file: String,
    /// Family name read from the font's `name` table (needed by the preview).
    pub family: String,
    /// `true` for the font bundled with the app (cannot be removed).
    pub is_default: bool,
}

pub fn fonts_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join(FONT_DIR)
}

pub fn js_font_path(file: &str) -> String {
    format!("{JS_FONT_PREFIX}{file}")
}

pub fn is_default(file: &str) -> bool {
    file.eq_ignore_ascii_case(DEFAULT_FONT_FILE)
}

/// Makes sure the bundled default font is present in app data.
/// `bundled_dir` is the `Assets/Fonts` directory next to the app binary.
pub fn ensure_default(app_data_root: &Path, bundled_dir: &Path) -> Result<(), String> {
    let dir = fonts_dir(app_data_root);
    fs::create_dir_all(&dir).map_err(|e| format!("Nie można utworzyć katalogu czcionek: {e}"))?;
    let target = dir.join(DEFAULT_FONT_FILE);
    if target.exists() {
        return Ok(());
    }
    let source = bundled_dir.join(DEFAULT_FONT_FILE);
    fs::copy(&source, &target).map_err(|e| {
        format!(
            "Nie można skopiować domyślnej czcionki z {}: {e}",
            source.display()
        )
    })?;
    Ok(())
}

/// Lists installed fonts, sorted by family name.
pub fn list(app_data_root: &Path) -> Result<Vec<FontAsset>, String> {
    let dir = fonts_dir(app_data_root);
    let mut result = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(result),
        Err(e) => return Err(format!("Nie można odczytać katalogu czcionek: {e}")),
    };
    for entry in entries {
        let entry = entry.map_err(|e| format!("Nie można odczytać czcionki: {e}"))?;
        let path = entry.path();
        if !path.is_file() || !is_ttf(&path) {
            continue;
        }
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let family = read_family_name(&path).unwrap_or_else(|_| {
            Path::new(&file)
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or(&file)
                .to_string()
        });
        result.push(FontAsset {
            is_default: is_default(&file),
            file,
            family,
        });
    }
    result.sort_by(|a, b| {
        a.is_default
            .cmp(&b.is_default)
            .reverse()
            .then_with(|| a.family.to_lowercase().cmp(&b.family.to_lowercase()))
    });
    Ok(result)
}

/// Copies a user-selected `.ttf` into app data and returns the installed asset.
pub fn add(app_data_root: &Path, source: &Path) -> Result<FontAsset, String> {
    if !source.is_file() {
        return Err(format!("Plik nie istnieje: {}", source.display()));
    }
    if !is_ttf(source) {
        return Err("Czcionka musi mieć rozszerzenie .ttf".to_string());
    }
    let family = read_family_name(source)
        .map_err(|e| format!("To nie jest prawidłowy plik TTF: {e}"))?;
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("font.ttf");
    let file = sanitize_file_name(name);
    if is_default(&file) {
        return Err("Nie można zastąpić domyślnej czcionki".to_string());
    }
    let dir = fonts_dir(app_data_root);
    fs::create_dir_all(&dir).map_err(|e| format!("Nie można utworzyć katalogu czcionek: {e}"))?;
    let target = dir.join(&file);
    if target.exists() {
        return Err(format!("Czcionka {file} jest już zainstalowana"));
    }
    fs::copy(source, &target).map_err(|e| format!("Nie można skopiować czcionki: {e}"))?;
    Ok(FontAsset {
        file,
        family,
        is_default: false,
    })
}

/// Removes a user-installed font. The bundled default is protected.
pub fn remove(app_data_root: &Path, file: &str) -> Result<(), String> {
    if is_default(file) {
        return Err("Domyślnej czcionki nie można usunąć".to_string());
    }
    if file.contains('/') || file.contains('\\') || file.contains("..") || !is_ttf(Path::new(file)) {
        return Err("Nieprawidłowa nazwa pliku czcionki".to_string());
    }
    let path = fonts_dir(app_data_root).join(file);
    fs::remove_file(&path).map_err(|e| format!("Nie można usunąć czcionki: {e}"))
}

fn valid_asset_name(file: &str) -> Result<(), String> {
    if file.is_empty()
        || file.contains('/')
        || file.contains('\\')
        || file.contains("..")
        || !is_ttf(Path::new(file))
    {
        return Err("Nieprawidłowa nazwa pliku czcionki".to_string());
    }
    Ok(())
}

/// Kopiuje czcionkę z magazynu aplikacji do projektu i rejestruje ją w `assets`.
/// Dzięki temu projekt (i wyeksportowany ZIP) jest samowystarczalny — można go
/// wrzucić do repozytorium razem z plikiem czcionki.
pub fn attach_to_project(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
    file: &str,
) -> Result<(), String> {
    valid_asset_name(file)?;
    let source = fonts_dir(app_data_root).join(file);
    if !source.is_file() {
        return Err(format!("Czcionka nie jest zainstalowana: {file}"));
    }
    let relative = format!("{JS_FONT_PREFIX}{file}");
    let target = project_dir.join("assets").join(&relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Nie można utworzyć katalogu fontów: {e}"))?;
    }
    if !target.exists() {
        fs::copy(&source, &target).map_err(|e| format!("Nie można skopiować czcionki do projektu: {e}"))?;
    }
    if !project.assets.iter().any(|a| a.path == relative) {
        project.assets.push(Asset {
            path: relative,
            role: AssetRole::Font,
        });
    }
    Ok(())
}

/// Usuwa z projektu kopie czcionek, których nie używa już żaden widget.
/// Bez tego przełączenie czcionki zostawiałoby stary TTF w pakiecie ZIP.
pub fn prune_unused(project: &mut Project, project_dir: &Path) -> Result<(), String> {
    let used: std::collections::HashSet<String> = project
        .widgets
        .iter()
        .filter_map(|w| w.font.as_ref().map(|f| format!("{JS_FONT_PREFIX}{f}")))
        .collect();
    let mut keep = Vec::new();
    for asset in &project.assets {
        if asset.role == AssetRole::Font && !used.contains(&asset.path) {
            let path = project_dir.join("assets").join(&asset.path);
            if path.is_file() {
                fs::remove_file(&path)
                    .map_err(|e| format!("Nie można usunąć starej czcionki z projektu: {e}"))?;
            }
        } else {
            keep.push(asset.clone());
        }
    }
    project.assets = keep;
    Ok(())
}

/// Ustawia domyślną czcionkę na wszystkich widgetach tekstowych nowego projektu.
pub fn apply_default_font(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
) -> Result<(), String> {
    let mut changed = false;
    for widget in &mut project.widgets {
        if widget.kind != WidgetKind::Image && widget.kind != WidgetKind::AnalogTime {
            widget.font = Some(DEFAULT_FONT_FILE.to_string());
            changed = true;
        }
    }
    if changed {
        attach_to_project(project, project_dir, app_data_root, DEFAULT_FONT_FILE)?;
    }
    Ok(())
}

/// Uzupełnia domyślną czcionkę tylko tam, gdzie widget nie ma jeszcze żadnej.
/// W przeciwieństwie do `apply_default_font` nie nadpisuje czcionek wybranych
/// przez użytkownika — dzięki temu podgląd i eksport mogą używać tego samego TTF
/// także w starszych projektach, bez zmiany świadomych wyborów.
pub fn apply_default_font_to_missing(
    project: &mut Project,
    project_dir: &Path,
    app_data_root: &Path,
) -> Result<(), String> {
    let mut changed = false;
    for widget in &mut project.widgets {
        if widget.kind != WidgetKind::Image
            && widget.kind != WidgetKind::AnalogTime
            && widget.font.is_none()
        {
            widget.font = Some(DEFAULT_FONT_FILE.to_string());
            changed = true;
        }
    }
    if changed {
        attach_to_project(project, project_dir, app_data_root, DEFAULT_FONT_FILE)?;
    }
    Ok(())
}

fn is_ttf(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("ttf"))
        .unwrap_or(false)
}

fn sanitize_file_name(name: &str) -> String {
    let safe_input = name.replace(['/', '\\'], "_");
    let stem = Path::new(&safe_input)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("font");
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
        safe = "font".to_string();
    }
    safe.truncate(80);
    safe.push_str(".ttf");
    safe
}

/// Reads the family name (name table ID 1) from a TTF/OTF file.
fn read_family_name(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    parse_family_name(&bytes)
}

fn parse_family_name(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() < 12 {
        return Err("za krótki plik".to_string());
    }
    let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let mut name_table = None;
    for i in 0..num_tables {
        let offset = 12 + i * 16;
        if offset + 16 > bytes.len() {
            break;
        }
        if &bytes[offset..offset + 4] == b"name" {
            let start = u32::from_be_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]) as usize;
            let length = u32::from_be_bytes([
                bytes[offset + 12],
                bytes[offset + 13],
                bytes[offset + 14],
                bytes[offset + 15],
            ]) as usize;
            if start
                .checked_add(length)
                .map(|end| end <= bytes.len())
                .unwrap_or(false)
            {
                name_table = Some(&bytes[start..start + length]);
            }
            break;
        }
    }
    let table = name_table.ok_or_else(|| "brak tablicy 'name'".to_string())?;
    if table.len() < 6 {
        return Err("uszkodzona tablica 'name'".to_string());
    }
    let count = u16::from_be_bytes([table[2], table[3]]) as usize;
    let string_offset = u16::from_be_bytes([table[4], table[5]]) as usize;
    let mut fallback = None;
    for i in 0..count {
        let offset = 6 + i * 12;
        if offset + 12 > table.len() {
            break;
        }
        let platform = u16::from_be_bytes([table[offset], table[offset + 1]]);
        let name_id = u16::from_be_bytes([table[offset + 6], table[offset + 7]]);
        if name_id != 1 {
            continue;
        }
        let length = u16::from_be_bytes([table[offset + 8], table[offset + 9]]) as usize;
        let rel = u16::from_be_bytes([table[offset + 10], table[offset + 11]]) as usize;
        let start = string_offset + rel;
        if start
            .checked_add(length)
            .map(|end| end > table.len())
            .unwrap_or(true)
        {
            continue;
        }
        let raw = &table[start..start + length];
        let decoded = if platform == 3 {
            let units: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            raw.iter().map(|&b| b as char).collect()
        };
        let cleaned: String = decoded
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>()
            .trim()
            .to_string();
        if cleaned.is_empty() {
            continue;
        }
        if platform == 3 {
            return Ok(cleaned);
        }
        fallback = Some(cleaned);
    }
    fallback.ok_or_else(|| "brak nazwy rodziny w tablicy 'name'".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_font_file_names() {
        assert_eq!(sanitize_file_name("My Font.ttf"), "My Font.ttf");
        assert_eq!(sanitize_file_name("a/b;c.ttf"), "a_b_c.ttf");
        assert!(sanitize_file_name("..ttf").starts_with("font"));
    }

    #[test]
    fn prune_removes_only_unused_font_assets() {
        let root = std::env::temp_dir().join(format!("rusty-fonts-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets/Fonts");
        ensure_default(&root, &bundled).unwrap();
        fs::create_dir_all(fonts_dir(&root)).unwrap();
        fs::copy(
            bundled.join(DEFAULT_FONT_FILE),
            fonts_dir(&root).join("Second.ttf"),
        ).unwrap();

        let (dir, mut project) = crate::project::create_project_in(&root, &crate::project::NewProject {
            name: "prune".into(),
            author: "Dev".into(),
            app_id: 7,
        }).unwrap();
        attach_to_project(&mut project, &dir, &root, DEFAULT_FONT_FILE).unwrap();
        attach_to_project(&mut project, &dir, &root, "Second.ttf").unwrap();
        assert_eq!(project.assets.iter().filter(|a| a.role == AssetRole::Font).count(), 2);

        project.widgets[0].font = Some("Second.ttf".into());
        prune_unused(&mut project, &dir).unwrap();
        let fonts: Vec<&str> = project.assets.iter()
            .filter(|a| a.role == AssetRole::Font)
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(fonts, vec!["fonts/Second.ttf"]);
        assert!(!dir.join("assets/fonts/Inter-Regular.ttf").exists());
        assert!(dir.join("assets/fonts/Second.ttf").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parses_real_inter_font_family() {
        // The font bundled with the app is the canonical fixture.
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/RustyWatch/Assets/Fonts/Inter-Regular.ttf");
        let family = read_family_name(&path).expect("Inter TTF must parse");
        assert_eq!(family, "Inter");
    }
}
