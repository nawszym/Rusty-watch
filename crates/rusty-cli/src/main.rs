use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rusty_core::{
    create_project_with, export_zip, inspect_zip, load_project, project, random_app_id, validate,
    NewProject,
};

#[derive(Parser)]
#[command(name = "rusty-cli", version, about = "Rusty Watch — narzędzia projektowe")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Tworzy nowy projekt w wskazanym katalogu
    New {
        path: PathBuf,
        #[arg(long, default_value = "My Watchface")]
        name: String,
        #[arg(long, default_value = "Rusty Watch")]
        author: String,
        /// Jeśli pominięty, zostaje wygenerowany losowy App ID.
        #[arg(long)]
        app_id: Option<u32>,
    },
    /// Sprawdza poprawność projektu
    Validate { project: PathBuf },
    /// Eksportuje projekt do pliku .zip
    Export {
        project: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Wyświetla zawartość wyeksportowanego pliku .zip
    Inspect { zip: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::New {
            path,
            name,
            author,
            app_id,
        } => {
            let app_id = app_id.unwrap_or_else(random_app_id);
            let project = create_project_with(
                &path,
                &NewProject {
                    name,
                    author,
                    app_id,
                },
            )?;
            println!("Utworzono projekt: {}", project::project_file(&path).display());
            println!(
                "Tarcza: {} | Autor: {} | App ID: {}",
                project.metadata.name, project.metadata.author, project.metadata.app_id
            );
            println!("Widgety: {}", project.widgets.len());
        }
        Command::Validate { project: path } => {
            let project = load_project(&path)?;
            let errors = validate(&project);
            if errors.is_empty() {
                println!("Projekt jest poprawny.");
            } else {
                println!("Błędy ({}):", errors.len());
                for error in errors {
                    println!("  - {error}");
                }
                bail!("walidacja nie powiodła się");
            }
        }
        Command::Export { project: path, output } => {
            let output = match output {
                Some(path) => path,
                None => {
                    let loaded = load_project(&path)?;
                    let safe: String = loaded
                        .metadata
                        .name
                        .chars()
                        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                        .collect();
                    path.parent()
                        .context("project file has no parent")?
                        .join(format!("{safe}.zip"))
                }
            };
            let output = export_zip(&path, &output)?;
            println!("Wyeksportowano: {}", output.display());
        }
        Command::Inspect { zip } => {
            print!("{}", inspect_zip(&zip)?);
        }
    }
    Ok(())
}
