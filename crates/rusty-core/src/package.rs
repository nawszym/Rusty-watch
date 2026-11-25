use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use image::ImageEncoder;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::codegen::{
    generate_app_json, generate_app_js, generate_index_js_with_hands, HandPlacement,
    WATCHFACE_PREVIEW_ICON,
};
use crate::hands::{export_asset_bytes, prepare_hand_assets};
use crate::model::{AssetRole, Project, ScreenShape};
use crate::project::load_project;
use crate::validate::validate;

/// Wymiar miniatury tarczy oczekiwany przez zegarek (`assets/Preview.png`).
/// Referencyjna tarcza z innego edytora używa dokładnie 314 × 314 RGBA.
pub const WATCHFACE_PREVIEW_SIZE: u32 = 314;

/// Ikona tarczy w ZIP-ie: oprawiony podgląd `preview.png` z folderu projektu
/// przeskalowany do rozdzielczości wymaganej przez zegarek (314 × 314). Edytor
/// generuje ten plik przed otwarciem okna eksportu, więc oba obrazy pokazują
/// identyczny stan tarczy. Eksport CLI (bez wygenerowanego podglądu) renderuje
/// deterministycznie klatkę 21:37:40 i przechodzi przez tę samą oprawę.
fn watchface_icon_bytes(project: &Project, project_dir: &Path) -> Result<Vec<u8>> {
    let preview = project_dir.join("preview.png");
    let rendered = if preview.is_file() {
        image::open(&preview)
            .with_context(|| format!("nie można odczytać {}", preview.display()))?
            .to_rgba8()
    } else {
        let series = crate::preview::random_series(0, "en");
        let frame = crate::preview::frame_data(crate::preview::PREVIEW_FRAME_INDEX, &series)
            .map_err(anyhow::Error::msg)?;
        let raw = crate::preview::render_frame(project_dir, project, &frame, &series);
        crate::preview::compose_branded_preview(&raw, raw.width())
    };
    let mut icon = if rendered.dimensions() == (WATCHFACE_PREVIEW_SIZE, WATCHFACE_PREVIEW_SIZE) {
        rendered
    } else {
        image::imageops::resize(
            &rendered,
            WATCHFACE_PREVIEW_SIZE,
            WATCHFACE_PREVIEW_SIZE,
            image::imageops::FilterType::Lanczos3,
        )
    };
    mask_to_screen_shape(&mut icon, project.device.profile().screen_shape);
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes).write_image(
        icon.as_raw(),
        icon.width(),
        icon.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(bytes)
}

/// Przycina ikonę tarczy do kształtu ekranu urządzenia. Poza maską kanał alfa
/// jest zerowy — zegarek wyświetla podgląd na własnym tle, więc rogi nie mogą
/// tworzyć kwadratu. Krawędź ma 1-pikselowe wygładzenie.
fn mask_to_screen_shape(image: &mut image::RgbaImage, shape: ScreenShape) {
    let size = image.width().min(image.height()) as f64;
    let center = (size - 1.0) / 2.0;
    let radius = size / 2.0 - 1.0;
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let factor = match shape {
            ScreenShape::Round => {
                let dx = x as f64 - center;
                let dy = y as f64 - center;
                let distance = (dx * dx + dy * dy).sqrt();
                ((radius - distance) + 0.5).clamp(0.0, 1.0) as f32
            }
        };
        if factor < 1.0 {
            pixel.0[3] = (pixel.0[3] as f32 * factor).round() as u8;
            if factor == 0.0 {
                pixel.0[0] = 0;
                pixel.0[1] = 0;
                pixel.0[2] = 0;
            }
        }
    }
}

/// `zip` zapisuje `version made by = Unix (3)` i nie ustawia DOS-owego bitu
/// katalogu (`0x10`) we wpisach katalogów. Paczka referencyjna ma `DOS/20`
/// oraz `external_attr = 0x41ff0010`. Część instalatorów na firmware tworzy
/// katalogi na podstawie tych atrybutów — symulator czyta samą nazwę z `/`.
/// Dogrywamy więc metadane central directory do postaci referencyjnej.
fn patch_zip_metadata(bytes: &mut [u8]) -> Result<()> {
    const EOCD_SIG: u32 = 0x0605_4b50;
    const CEN_SIG: u32 = 0x0201_4b50;

    fn u16at(bytes: &[u8], at: usize) -> Option<u16> {
        bytes.get(at..at + 2).and_then(|s| s.try_into().ok()).map(u16::from_le_bytes)
    }
    fn u32at(bytes: &[u8], at: usize) -> Option<u32> {
        bytes.get(at..at + 4).and_then(|s| s.try_into().ok()).map(u32::from_le_bytes)
    }

    let eocd = bytes
        .len()
        .checked_sub(22)
        .and_then(|start| (start..bytes.len()).rev().find(|&i| u32at(bytes, i) == Some(EOCD_SIG)))
        .context("ZIP bez EOCD")?;
    let count = u16at(bytes, eocd + 10).context("zły EOCD")? as usize;
    let mut at = u32at(bytes, eocd + 16).context("zły EOCD")? as usize;

    for _ in 0..count {
        if u32at(bytes, at) != Some(CEN_SIG) {
            bail!("uszkodzony central directory ZIP");
        }
        let name_len = u16at(bytes, at + 28).context("zły wpis ZIP")? as usize;
        let extra_len = u16at(bytes, at + 30).context("zły wpis ZIP")? as usize;
        let comment_len = u16at(bytes, at + 32).context("zły wpis ZIP")? as usize;
        let name_end = at + 46 + name_len;
        let entry_end = name_end + extra_len + comment_len;
        if entry_end > bytes.len() {
            bail!("obcięty central directory ZIP");
        }
        // DOS (0), wersja 20 — dokładnie jak w paczce z `zipfile` w Pythonie.
        bytes[at + 4..at + 6].copy_from_slice(&20u16.to_le_bytes());
        if bytes.get(name_end.saturating_sub(1)) == Some(&b'/') {
            bytes[at + 38..at + 42].copy_from_slice(&0x41ff_0010u32.to_le_bytes());
        }
        at = entry_end;
    }
    Ok(())
}

/// Gwarantuje, że spłaszczone nazwy nie kolidują ze sobą w paczce.
fn ensure_unique_asset_names(project: &Project) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for asset in &project.assets {
        let name = crate::export_paths::exported_asset_name(&asset.path);
        if !seen.insert(name.clone()) {
            bail!("kolizja nazw zasobów w ZIP: {name} (ścieżka: {})", asset.path);
        }
    }
    Ok(())
}

/// Eksportuje projekt do pliku .zip gotowego do wgrania przez Zepp / amazwatchfaces.
pub fn export_zip(project_file: &Path, output: &Path) -> Result<PathBuf> {
    let project = load_project(project_file)?;
    let errors = validate(&project);
    if !errors.is_empty() {
        bail!("projekt jest nieprawidłowy:\n  - {}", errors.join("\n  - "));
    }

    let project_dir = project_file
        .parent()
        .context("project file has no parent directory")?;
    ensure_unique_asset_names(&project)?;
    let hands = prepare_hand_assets(&project, project_dir).map_err(anyhow::Error::msg)?;
    let placements: HashMap<String, HandPlacement> = hands
        .iter()
        .map(|(path, hand)| (path.clone(), hand.placement))
        .collect();

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o666);

    write_directories(&mut zip)?;
    write_entry(&mut zip, "app.json", generate_app_json(&project).as_bytes(), options)?;
    write_entry(&mut zip, "app.js", generate_app_js().as_bytes(), options)?;
    write_entry(
        &mut zip,
        "watchface/index.js",
        generate_index_js_with_hands(&project, &placements).as_bytes(),
        options,
    )?;

    // Zegarek czyta miniaturę z `assets/Preview.png` (314 × 314). Wszystkie
    // zasoby są płaskie — dokładnie tak, jak odwołuje się wygenerowany JS.
    let icon_bytes = watchface_icon_bytes(&project, project_dir)?;
    write_entry(
        &mut zip,
        &format!("assets/{WATCHFACE_PREVIEW_ICON}"),
        &icon_bytes,
        options,
    )?;

    for asset in &project.assets {
        if asset.role == AssetRole::Icon && asset.path != "icon.png" {
            bail!("ikona musi mieć ścieżkę 'icon.png' (jest: '{}')", asset.path);
        }
        // Wskazówki mają już przygotowaną (przyciętą) grafikę i nowy pivot;
        // pozostałe zasoby przechodzą przez zwykłe skalowanie wskazówek/tła.
        let bytes = match hands.get(&asset.path) {
            Some(hand) => hand.bytes.clone(),
            // Grafiki IMAGES są przygotowywane do rozmiaru ramki tym samym
            // `image_fit`, co tło — w eksporcie tylko zabezpieczamy stare projekty.
            None if asset.path.starts_with("images/") => {
                crate::images::export_asset_bytes(&project, project_dir, &asset.path)
                    .map_err(anyhow::Error::msg)?
            }
            None if asset.path.starts_with("status/") => {
                crate::status_icons::export_asset_bytes(&project, project_dir, &asset.path)
                    .map_err(anyhow::Error::msg)?
            }
            None => export_asset_bytes(&project, project_dir, &asset.path)
                .map_err(anyhow::Error::msg)?,
        };
        let exported = crate::export_paths::exported_asset_name(&asset.path);
        write_entry(&mut zip, &format!("assets/{exported}"), &bytes, options)?;
    }

    zip.finish()?;
    let bytes = fs::read(output)?;
    let mut patched = bytes;
    patch_zip_metadata(&mut patched)?;
    fs::write(output, patched)?;
    Ok(output.to_path_buf())
}

/// Katalogi w ZIP-ie — referencyjna paczka Zeppa zawiera jawne wpisy katalogów.
/// Zasoby są płaskie; wyjątkiem jest `fonts/`, którego realny firmware oczekuje.
fn write_directories<W: Write + Seek>(zip: &mut ZipWriter<W>) -> Result<()> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o777);
    for dir in ["assets/", "assets/fonts/", "watchface/"] {
        zip.add_directory(dir, options)?;
    }
    Ok(())
}

fn write_entry<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
    bytes: &[u8],
    options: SimpleFileOptions,
) -> Result<()> {
    zip.start_file(name, options)?;
    zip.write_all(bytes)?;
    Ok(())
}

/// Katalog roboczy testu na symulatorze ZeppOS.
///
/// Eksportujemy tarczę do ZIP-a, a następnie rozpakowujemy ją do `<root>/app-<appId>/app`,
/// żeby `zeus install`/`zeus dev` miały gotowy projekt ZeppOS. Katalog jest za każdym
/// razem tworzony od zera — dzięki temu nie zostają w nim stare pliki.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmulatorWorkspace {
    /// Katalog roboczy zawierający ZIP, rozpakowany projekt i log polecenia zeus.
    pub dir: PathBuf,
    /// Rozpakowany projekt — przekazywany jako `--path` do `zeus install`.
    pub app_dir: PathBuf,
    /// Eksportowany plik ZIP (do weryfikacji i ewentualnej instalacji ręcznej).
    pub zip: PathBuf,
    /// Plik, do którego edytor przekierowuje stdout/stderr polecenia zeus.
    pub log: PathBuf,
}

/// Przygotowuje projekt do uruchomienia w oficjalnym symulatorze ZeppOS.
pub fn prepare_emulator_workspace(project_file: &Path, root: &Path) -> Result<EmulatorWorkspace> {
    let project = load_project(project_file)?;
    let dir = root.join(format!("app-{}", project.metadata.app_id));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("cannot clear emulator workspace: {}", dir.display()))?;
    }
    std::fs::create_dir_all(&dir)?;

    let zip = dir.join("watchface.zip");
    export_zip(project_file, &zip)?;

    let app_dir = dir.join("app");
    std::fs::create_dir_all(&app_dir)?;
    let mut archive = ZipArchive::new(std::fs::File::open(&zip)?)?;
    archive.extract(&app_dir)?;

    // Paczka dla zegarka jest płaska (v2). `zeus` buduje z katalogu i wymaga
    // v3 z blokiem `targets` oraz zasobów w `assets/<target>/` — przygotowujemy
    // taką kopię wyłącznie w katalogu roboczym symulatora.
    convert_export_for_zeus(&app_dir, &project, project.device.profile().target_key)?;

    let log = dir.join("zeus.log");
    Ok(EmulatorWorkspace {
        dir,
        app_dir,
        zip,
        log,
    })
}

/// Zamienia płaski eksport v2 na roboczy projekt v3 rozumiany przez `zeus`.
/// Dotyczy tylko katalogu symulatora — plik ZIP dla zegarka pozostaje płaski.
fn convert_export_for_zeus(app_dir: &Path, project: &Project, target: &str) -> Result<()> {
    use serde_json::json;

    let app_json_path = app_dir.join("app.json");
    let mut app: serde_json::Value =
        serde_json::from_slice(&fs::read(&app_json_path).context("brak app.json w eksporcie")?)?;
    let module = app.get("module").cloned().unwrap_or(json!({}));
    let platforms = app.get("platforms").cloned().unwrap_or(json!([]));
    let design_width = app.get("designWidth").cloned().unwrap_or(json!(466));
    app["configVersion"] = json!("v3");
    app["targets"] = json!({
        target: {
            "module": module,
            "platforms": platforms,
            "designWidth": design_width,
        }
    });
    fs::write(&app_json_path, serde_json::to_vec_pretty(&app)?)?;

    let assets = app_dir.join("assets");
    let target_assets = assets.join(target);
    let mut files: Vec<String> = project
        .assets
        .iter()
        .map(|asset| crate::export_paths::exported_asset_name(&asset.path))
        .collect();
    files.push(WATCHFACE_PREVIEW_ICON.to_string());
    for exported in files {
        // W rozpakowanym ZIP-ie pliki mają już spłaszczone nazwy.
        let from = assets.join(&exported);
        let to = target_assets.join(&exported);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&from, &to)
            .with_context(|| format!("cannot copy {} to zeus workspace", from.display()))?;
    }
    Ok(())
}

/// Podgląd zawartości zipa — przydatne przy weryfikacji eksportu.
pub fn inspect_zip(path: &Path) -> Result<String> {
    let mut archive = ZipArchive::new(std::fs::File::open(path)?)?;
    let mut out = String::new();
    out.push_str(&format!("{} entries:\n", archive.len()));
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        out.push_str(&format!("  {} ({} bytes)\n", entry.name(), entry.size()));
    }
    if let Ok(mut app) = archive.by_name("app.json") {
        let mut raw = String::new();
        app.read_to_string(&mut raw)?;
        out.push_str("\napp.json:\n");
        out.push_str(&raw);
    }
    Ok(out)
}

/// Eksport do pamięci (użyteczne dla UI i testów).
pub fn export_zip_bytes(project_file: &Path) -> Result<Vec<u8>> {
    let project = load_project(project_file)?;
    let errors = validate(&project);
    if !errors.is_empty() {
        bail!("projekt jest nieprawidłowy:\n  - {}", errors.join("\n  - "));
    }
    let project_dir = project_file.parent().context("no parent dir")?;
    ensure_unique_asset_names(&project)?;
    let hands = prepare_hand_assets(&project, project_dir).map_err(anyhow::Error::msg)?;
    let placements: HashMap<String, HandPlacement> = hands
        .iter()
        .map(|(path, hand)| (path.clone(), hand.placement))
        .collect();
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o666);
    write_directories(&mut zip)?;
    write_entry(&mut zip, "app.json", generate_app_json(&project).as_bytes(), options)?;
        write_entry(&mut zip, "app.js", generate_app_js().as_bytes(), options)?;
        write_entry(
            &mut zip,
            "watchface/index.js",
            generate_index_js_with_hands(&project, &placements).as_bytes(),
            options,
        )?;
        // Oprawiony podgląd przeskalowany do rozdzielczości wymaganej przez zegarek.
        let icon_bytes = watchface_icon_bytes(&project, project_dir)?;
        write_entry(
            &mut zip,
            &format!("assets/{WATCHFACE_PREVIEW_ICON}"),
            &icon_bytes,
            options,
        )?;

    for asset in &project.assets {
            let bytes = match hands.get(&asset.path) {
                Some(hand) => hand.bytes.clone(),
                None if asset.path.starts_with("images/") => {
                    crate::images::export_asset_bytes(&project, project_dir, &asset.path)
                        .map_err(anyhow::Error::msg)?
                }
                None if asset.path.starts_with("status/") => {
                    crate::status_icons::export_asset_bytes(&project, project_dir, &asset.path)
                        .map_err(anyhow::Error::msg)?
                }
                None => export_asset_bytes(&project, project_dir, &asset.path)
                    .map_err(anyhow::Error::msg)?,
            };
            let exported = crate::export_paths::exported_asset_name(&asset.path);
        write_entry(&mut zip, &format!("assets/{exported}"), &bytes, options)?;
        }
        zip.finish()?;
    }
    let mut bytes = cursor.into_inner();
    patch_zip_metadata(&mut bytes)?;
    Ok(bytes)
}
