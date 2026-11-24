//! Lista ostatnio otwartych projektów.
//!
//! Lista jest zapisywana jako JSON w pliku wskazanym przez UI (katalog danych aplikacji).
//! Logika przechowywania jest w core, dzięki czemu może być testowana niezależnie od WinUI.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Maksymalna liczba wpisów widocznych na stronie HOME.
pub const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub name: String,
    /// Ścieżka do `project.json`.
    pub file: String,
    /// Znacznik czasu otwarcia (milisekundy od epochy Unix).
    pub opened_at: i64,
}

pub fn store_file(data_dir: &Path) -> PathBuf {
    data_dir.join("recent.json")
}

pub fn load(store: &Path) -> Result<Vec<RecentProject>> {
    if !store.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(store)
        .with_context(|| format!("cannot read {}", store.display()))?;
    // Pliki zapisane przez Windows PowerShell mają BOM — serde_json go nie akceptuje.
    let raw = raw.trim_start_matches('\u{feff}');
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_str(raw)
        .with_context(|| format!("invalid recent projects file: {}", store.display()))?;

    // Gościnny format: canonicalnie jest to tablica, ale skrypty PowerShellowe potrafią
    // zapisać pojedynczy wpis jako obiekt, a tablicę jako `{"value":[...],"Count":N}`.
    // Taki plik nie może zablokować otwarcia projektu.
    let entries = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(mut map) => {
            if let Some(serde_json::Value::Array(items)) = map.remove("value") {
                items
            } else if map.contains_key("name") && map.contains_key("file") {
                vec![serde_json::Value::Object(map)]
            } else {
                anyhow::bail!("invalid recent projects file: {}", store.display());
            }
        }
        _ => anyhow::bail!("invalid recent projects file: {}", store.display()),
    };

    entries
        .into_iter()
        .map(|item| {
            serde_json::from_value(item)
                .with_context(|| format!("invalid recent project entry in {}", store.display()))
        })
        .collect()
}

fn write(store: &Path, items: &[RecentProject]) -> Result<()> {
    if let Some(parent) = store.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(items)?;
    std::fs::write(store, format!("{raw}\n"))
        .with_context(|| format!("cannot write {}", store.display()))
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Klucz porównywania ścieżek: znormalizowany, bez rozróżniania wielkości liter (Windows).
fn file_key(file: &Path) -> String {
    let resolved = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    resolved.to_string_lossy().replace('/', "\\").to_lowercase()
}

/// Dodaje projekt na początek listy, usuwając poprzedni wpis tego samego pliku.
pub fn record(store: &Path, file: &Path, name: &str) -> Result<Vec<RecentProject>> {
    let key = file_key(file);
    let mut items = load(store)?;
    items.retain(|item| file_key(Path::new(&item.file)) != key);
    items.insert(
        0,
        RecentProject {
            name: name.to_string(),
            file: file.to_string_lossy().into_owned(),
            opened_at: now_millis(),
        },
    );
    items.truncate(MAX_RECENT);
    write(store, &items)?;
    Ok(items)
}

/// Usuwa jeden wpis z listy.
pub fn remove(store: &Path, file: &Path) -> Result<Vec<RecentProject>> {
    let key = file_key(file);
    let mut items = load(store)?;
    items.retain(|item| file_key(Path::new(&item.file)) != key);
    write(store, &items)?;
    Ok(items)
}

/// Czyści całą listę.
pub fn clear(store: &Path) -> Result<Vec<RecentProject>> {
    write(store, &[])?;
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rusty-watch-recent-{id}-{label}",
            id = std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        store_file(&dir)
    }

    #[test]
    fn missing_store_is_empty() {
        let store = temp_store("missing");
        assert!(load(&store).unwrap().is_empty());
    }

    #[test]
    fn load_accepts_utf8_bom() {
        let store = temp_store("bom");
        let file = Path::new("C:\\proj\\bom\\project.json");
        record(&store, file, "With BOM").unwrap();
        let raw = std::fs::read_to_string(&store).unwrap();
        std::fs::write(&store, format!("\u{feff}{raw}")).unwrap();
        let items = load(&store).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "With BOM");

        std::fs::remove_dir_all(store.parent().unwrap()).ok();
    }

    #[test]
    fn load_accepts_powershell_object_shapes() {
        let store = temp_store("ps-shapes");
        std::fs::create_dir_all(store.parent().unwrap()).unwrap();
        let wrapper = r#"{"value":[{"name":"A","file":"C:\\a\\project.json","openedAt":1}],"Count":1}"#;
        std::fs::write(&store, wrapper).unwrap();
        let items = load(&store).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "A");

        // Pojedynczy wpis zapisany jako obiekt zamiast jednoelementowej tablicy.
        let single = r#"{"name":"B","file":"C:\\b\\project.json","openedAt":2}"#;
        std::fs::write(&store, single).unwrap();
        let items = load(&store).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "B");

        // Po zapisie ponownie mamy kanoniczną tablicę.
        record(&store, Path::new("C:\\c\\project.json"), "C").unwrap();
        let raw = std::fs::read_to_string(&store).unwrap();
        assert!(raw.trim_start().starts_with('['));

        std::fs::remove_dir_all(store.parent().unwrap()).ok();
    }

    #[test]
    fn record_dedupes_moves_to_front_and_caps() {
        let store = temp_store("dedupe");
        let first = Path::new("C:\\proj\\one\\project.json");
        let second = Path::new("C:\\proj\\two\\project.json");

        record(&store, first, "One").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        record(&store, second, "Two").unwrap();
        record(&store, first, "One again").unwrap();

        let items = load(&store).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "One again");
        assert_eq!(items[1].name, "Two");
        assert!(items[0].opened_at >= items[1].opened_at);

        for index in 0..12 {
            let file = PathBuf::from(format!("C:\\proj\\p{index}\\project.json"));
            record(&store, &file, &format!("P{index}")).unwrap();
        }
        let items = load(&store).unwrap();
        assert_eq!(items.len(), MAX_RECENT);
        assert_eq!(items[0].name, "P11");

        std::fs::remove_dir_all(store.parent().unwrap()).ok();
    }

    #[test]
    fn remove_and_clear() {
        let store = temp_store("remove");
        let file = Path::new("D:\\proj\\x\\project.json");
        record(&store, file, "X").unwrap();
        assert_eq!(load(&store).unwrap().len(), 1);

        let items = remove(&store, file).unwrap();
        assert!(items.is_empty());
        assert!(load(&store).unwrap().is_empty());

        record(&store, file, "X").unwrap();
        assert!(clear(&store).unwrap().is_empty());
        assert!(load(&store).unwrap().is_empty());

        std::fs::remove_dir_all(store.parent().unwrap()).ok();
    }
}
