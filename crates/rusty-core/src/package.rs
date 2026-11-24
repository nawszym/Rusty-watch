use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::codegen::{
    generate_app_json, generate_app_js, generate_index_js_with_hands, HandPlacement,
};
use crate::hands::{export_asset_bytes, prepare_hand_assets};
use crate::model::AssetRole;
use crate::project::load_project;
use crate::validate::validate;

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
    let hands = prepare_hand_assets(&project, project_dir).map_err(anyhow::Error::msg)?;
    let placements: HashMap<String, HandPlacement> = hands
        .iter()
        .map(|(path, hand)| (path.clone(), hand.placement))
        .collect();
    let target = project.device.profile().target_key;

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    write_entry(&mut zip, "app.json", generate_app_json(&project).as_bytes(), options)?;
    write_entry(&mut zip, "app.js", generate_app_js().as_bytes(), options)?;
    write_entry(
        &mut zip,
        "watchface/index.js",
        generate_index_js_with_hands(&project, &placements).as_bytes(),
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
            None => export_asset_bytes(&project, project_dir, &asset.path)
                .map_err(anyhow::Error::msg)?,
        };
        write_entry(
            &mut zip,
            &format!("assets/{target}/{}", asset.path),
            &bytes,
            options,
        )?;
    }

    zip.finish()?;
    Ok(output.to_path_buf())
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

    let log = dir.join("zeus.log");
    Ok(EmulatorWorkspace {
        dir,
        app_dir,
        zip,
        log,
    })
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
    let hands = prepare_hand_assets(&project, project_dir).map_err(anyhow::Error::msg)?;
    let placements: HashMap<String, HandPlacement> = hands
        .iter()
        .map(|(path, hand)| (path.clone(), hand.placement))
        .collect();
    let target = project.device.profile().target_key;
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        write_entry(&mut zip, "app.json", generate_app_json(&project).as_bytes(), options)?;
        write_entry(&mut zip, "app.js", generate_app_js().as_bytes(), options)?;
        write_entry(
            &mut zip,
            "watchface/index.js",
            generate_index_js_with_hands(&project, &placements).as_bytes(),
            options,
        )?;
        for asset in &project.assets {
            let bytes = match hands.get(&asset.path) {
                Some(hand) => hand.bytes.clone(),
                None => export_asset_bytes(&project, project_dir, &asset.path)
                    .map_err(anyhow::Error::msg)?,
            };
            write_entry(&mut zip, &format!("assets/{target}/{}", asset.path), &bytes, options)?;
        }
        zip.finish()?;
    }
    Ok(cursor.into_inner())
}
