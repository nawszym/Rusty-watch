//! Mapowanie ścieżek zasobów projektu na płaskie, unikalne nazwy w paczce ZIP.
//!
//! Referencyjna tarcza dla Active 2 trzyma wszystkie zasoby bezpośrednio w
//! `assets/` i używa krótkich, zahaszowanych nazw (`hand_150de475.png`).
//! Podfoldery (`hands/Classic/…`, `status/Classic/…`, `fonts/…`) są formatem
//! roboczym edytora — na realnym firmware mogą nie być rozwiązywane przez
//! `src`/`font`. Eksport spłaszcza drzewo i przepisuje odwołania w generowanym
//! JS tak, aby nazwa w kodzie i wpis w ZIP-ie były identyczne.

use std::path::Path;

/// Nazwa zasobu w płaskim katalogu `assets/` paczki.
///
/// `icon.png` zachowuje swoją nazwę (część konwencji ZeppOS). Pozostałe pliki
/// dostają prefix rodzaju oraz stabilny skrót oryginalnej ścieżki — dzięki
/// temu dwa zestawy wskazówek czy paków STATUS z różnymi nazwami nie kolidują.
/// Tła nie ma: jest stałym `FILL_RECT` w wygenerowanym JS.
pub fn exported_asset_name(project_path: &str) -> String {
    if project_path == "icon.png" {
        return project_path.to_string();
    }
    let extension = Path::new(project_path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    // Czcionki zostają w `fonts/` — referencyjna tarcza używa `fonts/<plik>.ttf`
    // i to działa na realnym firmware. Unikalność nazwy daje hash.
    if project_path.starts_with("fonts/") {
        return format!("fonts/font_{:08x}.{extension}", fnv1a(project_path));
    }
    let prefix = if project_path.starts_with("hands/") {
        "hand"
    } else if project_path.starts_with("status/") {
        "status"
    } else if project_path.starts_with("images/") {
        "img"
    } else {
        "asset"
    };
    format!("{prefix}_{:08x}.{extension}", fnv1a(project_path))
}

/// Deterministyczny skrót FNV-1a 32-bit — bez nowych zależności.
fn fnv1a(value: &str) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in value.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_flat_unique_and_stable() {
        let a = exported_asset_name("hands/Classic/hour.png");
        let b = exported_asset_name("hands/Blue/hour.png");
        assert_eq!(a, exported_asset_name("hands/Classic/hour.png"));
        assert_ne!(a, b);
        assert!(!a.contains('/'));
        assert!(a.starts_with("hand_"));
        assert!(a.ends_with(".png"));
        let font = exported_asset_name("fonts/Inter-Regular.ttf");
        assert!(font.starts_with("fonts/font_"), "{font}");
        assert!(!font.contains('/') || font.starts_with("fonts/"));
        assert_eq!(exported_asset_name("icon.png"), "icon.png");
    }
}
