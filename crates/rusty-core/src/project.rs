use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use image::{ImageEncoder, Rgba, RgbaImage};

use crate::model::{
    FirstDayOfWeek, MeasurementSystem, Project, WidgetKind, SCHEMA_VERSION,
};

pub fn project_file(dir: &Path) -> PathBuf {
    dir.join("project.json")
}

/// Ustawia domyślny system jednostek projektu (z ustawień aplikacji).
/// Widgety dystansu bez własnego nadpisania dostają nowy tekst początkowy,
/// żeby podgląd edytora od razu pokazywał właściwą jednostkę.
pub fn set_measurement_system(project: &mut Project, system: MeasurementSystem) {
    project.measurement_system = system;
    for widget in &mut project.widgets {
        if widget.kind == WidgetKind::DistanceText && widget.measurement_system.is_none() {
            widget.text = Some(system.initial_distance_text());
        }
    }
}

/// Ustawia pierwszy dzień tygodnia w projekcie (wpływa na podgląd i JS).
pub fn set_first_day_of_week(project: &mut Project, first_day: FirstDayOfWeek) {
    project.first_day_of_week = first_day;
}

/// Dane tarczy podane przez użytkownika w oknie „New watchface".
#[derive(Debug, Clone)]
pub struct NewProject {
    pub name: String,
    pub author: String,
    pub app_id: u32,
}

impl NewProject {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("Nazwa tarczy nie może być pusta");
        }
        if self.author.trim().is_empty() {
            bail!("Autor tarczy nie może być pusty");
        }
        if self.app_id == 0 {
            bail!("App ID musi być większe od 0");
        }
        Ok(())
    }
}

/// Losowy, 8-cyfrowy App ID — unikalny dla każdej tarczy w Zepp.
pub fn random_app_id() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1);
    let pid = std::process::id().wrapping_mul(2_654_435_761);
    let mut seed = nanos ^ pid ^ (nanos.rotate_left(13));
    // xorshift32, żeby rozkład nie był zdominowany przez znaczniki czasu.
    seed ^= seed << 13;
    seed ^= seed >> 17;
    seed ^= seed << 5;
    10_000_000 + (seed % 90_000_000)
}

pub fn create_project(dir: &Path, name: &str) -> Result<Project> {
    create_project_with(
        dir,
        &NewProject {
            name: name.to_string(),
            author: "Rusty Watch".to_string(),
            app_id: 10_000_101,
        },
    )
}

pub fn create_project_with(dir: &Path, input: &NewProject) -> Result<Project> {
    input.validate()?;
    if dir.exists() {
        let file = project_file(dir);
        if file.exists() {
            bail!("project already exists at {}", file.display());
        }
    }
    std::fs::create_dir_all(dir.join("assets"))?;

    let project = Project::starter_with(
        input.name.trim(),
        input.author.trim(),
        input.app_id,
    );

    // Tło jest stałe: `FILL_RECT` w wygenerowanym JS (dokładnie jak w
    // działającej tarczy referencyjnej). Nie ma zasobu tła ani magazynu tła.

    let icon = generate_icon(100);
    write_png(&dir.join("assets/icon.png"), &icon)?;
    save_project(&project_file(dir), &project)?;

    Ok(project)
}

/// Bezpieczna nazwa folderu dla tarczy — user wybiera tylko katalog główny projektów.
pub fn watchface_folder_name(name: &str) -> String {
    let mut folder: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    while folder.ends_with([' ', '.']) {
        folder.pop();
    }
    if folder.is_empty() {
        folder = "Watchface".to_string();
    }
    // Windows zarezerwował CON, PRN, AUX, NUL oraz COM1..COM9 / LPT1..LPT9.
    let base = folder.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && base.as_bytes()[3].is_ascii_digit());
    if reserved {
        folder.insert(0, '_');
    }
    folder
}

/// Tworzy `<destination>/<nazwa tarczy>/project.json` — katalog główny wybiera użytkownik.
pub fn create_project_in(destination: &Path, input: &NewProject) -> Result<(PathBuf, Project)> {
    input.validate()?;
    let dir = destination.join(watchface_folder_name(&input.name));
    if dir.exists() {
        bail!("Folder already exists: {}", dir.display());
    }
    let project = create_project_with(&dir, input)?;
    Ok((dir, project))
}

/// Zapisuje kopię projektu w nowym folderze (Save as...) razem z zasobami.
pub fn save_project_as(
    source_file: &Path,
    destination: &Path,
    input: &NewProject,
) -> Result<(PathBuf, Project)> {
    let mut project = load_project(source_file)?;
    let (dir, _) = create_project_in(destination, input)?;
    project.metadata.name = input.name.trim().to_string();
    project.metadata.author = input.author.trim().to_string();
    project.metadata.app_id = input.app_id;
    project.metadata.description = format!("{} — created with Rusty Watch", project.metadata.name);

    let file = project_file(&dir);
    save_project(&file, &project)?;

    let source_dir = source_file.parent().context("project file has no parent")?;
    for asset in &project.assets {
        let from = source_dir.join("assets").join(&asset.path);
        let to = dir.join("assets").join(&asset.path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&from, &to)
            .with_context(|| format!("cannot copy asset {}", asset.path))?;
    }
    Ok((file, project))
}

/// Aktualizuje autora i wersję tarczy. Nazwa jest celowo pomijana — folder projektu
/// i lista ostatnich projektów są od niej zależne, więc pozostaje niezmieniona.
pub fn update_project_metadata(path: &Path, author: &str, version: &str) -> Result<Project> {
    let author = author.trim();
    let version = version.trim();
    if author.is_empty() {
        bail!("Autor tarczy nie może być pusty");
    }
    if version.is_empty() {
        bail!("Wersja tarczy nie może być pusta");
    }
    if version.chars().count() > 32 {
        bail!("Wersja tarczy może mieć najwyżej 32 znaki");
    }
    if version.chars().any(|c| c.is_control() || c.is_whitespace()) {
        bail!("Wersja tarczy nie może zawierać białych znaków ani znaków kontrolnych");
    }

    let mut project = load_project(path)?;
    project.metadata.author = author.to_string();
    project.metadata.version = version.to_string();
    save_project(path, &project)?;
    Ok(project)
}

pub fn load_project(path: &Path) -> Result<Project> {
    if !path.exists() {
        bail!("project file not found: {}", path.display());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let project: Project =
        serde_json::from_str(&raw).with_context(|| format!("invalid project JSON: {}", path.display()))?;
    if project.schema_version != SCHEMA_VERSION {
        bail!(
            "unsupported schema version {} (expected {})",
            project.schema_version,
            SCHEMA_VERSION
        );
    }
    Ok(project)
}

pub fn save_project(path: &Path, project: &Project) -> Result<()> {
    let raw = serde_json::to_string_pretty(project)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{raw}\n"))
        .with_context(|| format!("cannot write {}", path.display()))
}

pub(crate) fn write_png(path: &Path, image: &RgbaImage) -> Result<()> {
    let file = std::fs::File::create(path)?;
    image::codecs::png::PngEncoder::new(file).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(())
}

fn generate_icon(size: u32) -> RgbaImage {
    let mut img = RgbaImage::new(size, size);
    let center = size as f32 / 2.0;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let dx = x as f32 - center;
        let dy = y as f32 - center;
        let dist = (dx * dx + dy * dy).sqrt();
        let inside = dist < center * 0.92;
        let ring = (dist - center * 0.78).abs() < center * 0.08;
        *pixel = if inside {
            if ring {
                Rgba([0x7C, 0xB8, 0xFF, 255])
            } else {
                Rgba([0x18, 0x1B, 0x2A, 255])
            }
        } else {
            Rgba([0, 0, 0, 0])
        };
    }
    img
}
