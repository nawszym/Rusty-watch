//! Wspólny mechanizm cache'owania miniatur zasobów (czcionki, wskazówki, ikony STATUS).
//!
//! Podgląda się wyłącznie na pliki źródłowe w katalogu zasobu. Jeśli żaden z nich
//! nie zmienił się (nazwa, rozmiar, czas modyfikacji), generowanie `preview.png`
//! można pominąć — dzięki temu uruchomienie aplikacji nie dekoduje i nie koduje
//! dziesiątek PNG-ów przy każdym starcie.
//!
//! Dla zasobów wbudowanych (dostarczanych z instalatorem) osobny plik
//! `.bundled-version` przechowuje odcisk wersji źródła. Gdy instalator przyniesie
//! nowsze pliki, znacznik się różni i magazyn użytkownika zostaje odświeżony.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Nazwa pliku z odciskiem aktualnego `preview.png`.
pub const FINGERPRINT_FILE: &str = "preview.fingerprint";
/// Nazwa pliku z odciskiem wersji wbudowanego źródła.
pub const BUNDLED_VERSION_FILE: &str = ".bundled-version";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= *byte as u64;
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn hash_u64(hash: &mut u64, value: u64) {
    hash_bytes(hash, &value.to_le_bytes());
}

/// Stabilny odcisk wymienionych plików, które istnieją w `dir`.
///
/// Uwzględniamy nazwę, długość w bajtach oraz czas modyfikacji (nanosekundy od
/// epoki). Nazwy nieistniejących plików (lub nie będących plikami) są pomijane,
/// więc opcjonalne wskazówki nie muszą istnieć. Wynik to 16 znaków szesnastkowych.
pub fn files_fingerprint(dir: &Path, names: &[&str]) -> Result<String, String> {
    let mut hash = FNV_OFFSET;
    for name in names {
        let path = dir.join(name);
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        hash_bytes(&mut hash, name.as_bytes());
        hash_u64(&mut hash, meta.len());
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0);
        hash_u64(&mut hash, modified);
    }
    Ok(format!("{hash:016x}"))
}

/// Czy zapisany odcisk miniaturki odpowiada aktualnym plikom źródłowym?
///
/// Brak pliku `preview.fingerprint` albo brak `preview.png` oznacza zawsze
/// „nie”, niezależnie od zawartości katalogu.
pub fn fingerprint_matches(dir: &Path, names: &[&str]) -> Result<bool, String> {
    if !dir.join("preview.png").is_file() {
        return Ok(false);
    }
    let stored = match fs::read_to_string(dir.join(FINGERPRINT_FILE)) {
        Ok(text) => text.trim().to_string(),
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("Nie można odczytać odcisku miniaturki: {e}")),
    };
    let current = files_fingerprint(dir, names)?;
    Ok(stored == current)
}

/// Zapisuje odcisk miniaturki i zwraca go (żeby można go było też zwrócić przez FFI).
pub fn write_fingerprint(dir: &Path, names: &[&str]) -> Result<String, String> {
    let fingerprint = files_fingerprint(dir, names)?;
    fs::write(dir.join(FINGERPRINT_FILE), &fingerprint)
        .map_err(|e| format!("Nie można zapisać odcisku miniaturki: {e}"))?;
    Ok(fingerprint)
}

/// Ścieżka znacznika wymuszającego odświeżenie wbudowanego zasobu.
///
/// `force_marker_path` i `bundled_version_path` wskazują ten sam plik — obie
/// nazwy są dostępne, żeby łatwo było odczytać intencję w kodzie wywołującym.
pub fn force_marker_path(dir: &Path) -> PathBuf {
    dir.join(BUNDLED_VERSION_FILE)
}

pub fn bundled_version_path(dir: &Path) -> PathBuf {
    force_marker_path(dir)
}

/// Odcisk wersji wbudowanego źródła zapisany w magazynie użytkownika.
/// `None`, gdy zasób nie był jeszcze seedowany albo znacznik zaginął.
pub fn read_bundled_version(dir: &Path) -> Option<String> {
    fs::read_to_string(bundled_version_path(dir))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// Zapisuje odcisk wbudowanego źródła obok zainstalowanego zasobu.
pub fn write_bundled_version(dir: &Path, fingerprint: &str) -> Result<(), String> {
    fs::write(bundled_version_path(dir), fingerprint)
        .map_err(|e| format!("Nie można zapisać znacznika wersji wbudowanej: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn fingerprint_tracks_file_changes_and_skips_missing_names() {
        let dir = std::env::temp_dir().join(format!("rw-asset-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.png"), b"aaaa").unwrap();
        fs::write(dir.join("b.png"), b"bbbbbb").unwrap();

        let first = files_fingerprint(&dir, &["a.png", "b.png", "missing.png"]).unwrap();
        assert_eq!(first.len(), 16);
        assert_eq!(first, files_fingerprint(&dir, &["a.png", "b.png"]).unwrap());
        assert_ne!(first, files_fingerprint(&dir, &["a.png"]).unwrap());

        // Brak sidecaru i/lub miniaturki => zawsze false.
        assert!(!fingerprint_matches(&dir, &["a.png", "b.png"]).unwrap());
        fs::write(dir.join("preview.fingerprint"), format!("  {first} \n")).unwrap();
        assert!(!fingerprint_matches(&dir, &["a.png", "b.png"]).unwrap());
        fs::write(dir.join("preview.png"), b"png").unwrap();
        assert!(fingerprint_matches(&dir, &["a.png", "b.png"]).unwrap());

        // Zmiana długości pliku zmienia odcisk.
        fs::write(dir.join("b.png"), b"bbbbbbbbbb").unwrap();
        assert!(!fingerprint_matches(&dir, &["a.png", "b.png"]).unwrap());

        // Tylko czas modyfikacji też jest częścią odcisku.
        let before = files_fingerprint(&dir, &["a.png"]).unwrap();
        let later = SystemTime::now() + Duration::from_secs(120);
        fs::File::options()
            .write(true)
            .open(dir.join("a.png"))
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert_ne!(before, files_fingerprint(&dir, &["a.png"]).unwrap());

        assert_eq!(write_fingerprint(&dir, &["a.png", "b.png"]).unwrap().len(), 16);
        assert!(fingerprint_matches(&dir, &["a.png", "b.png"]).unwrap());

        write_bundled_version(&dir, "abcdef0123456789").unwrap();
        assert_eq!(
            read_bundled_version(&dir).as_deref(),
            Some("abcdef0123456789")
        );
        assert_eq!(force_marker_path(&dir), bundled_version_path(&dir));

        fs::remove_dir_all(&dir).ok();
    }
}
