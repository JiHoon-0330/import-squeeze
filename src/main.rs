use anyhow::{bail, Context, Result};
use clap::Parser;
use rayon::prelude::*;
use std::path::PathBuf;

use import_squeeze::config;
use import_squeeze::{process_file, FileResult};

#[derive(Parser, Debug)]
#[command(name = "import-squeeze", about = "Remove blank lines between import statements")]
struct Cli {
    /// Files to process. If omitted, reads from biome.json.
    files: Vec<PathBuf>,

    /// Check mode: report files that need changes without modifying them.
    #[arg(long)]
    check: bool,

    /// Write mode (default): modify files in place.
    #[arg(long)]
    write: bool,

    /// Path to biome.json config file.
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let check = cli.check;

    let files = if !cli.files.is_empty() {
        cli.files
    } else {
        resolve_files_from_config(cli.config.as_deref())?
    };

    if files.is_empty() {
        eprintln!("No files to process.");
        return Ok(());
    }

    let results: Vec<(PathBuf, Result<FileResult>)> = files
        .into_par_iter()
        .map(|path| {
            let result = process_file(&path, check);
            (path, result)
        })
        .collect();

    let mut changed_count = 0;
    let mut error_count = 0;

    for (path, result) in &results {
        match result {
            Ok(FileResult::Changed) => {
                changed_count += 1;
                if check {
                    println!("{}", path.display());
                }
            }
            Ok(FileResult::Unchanged) => {}
            Err(e) => {
                error_count += 1;
                eprintln!("Error processing {}: {}", path.display(), e);
            }
        }
    }

    if check {
        if changed_count > 0 {
            eprintln!("{} file(s) would be modified.", changed_count);
            bail!("Check failed: files need import squeezing.");
        }
    } else if changed_count > 0 {
        eprintln!("{} file(s) modified.", changed_count);
    }

    if error_count > 0 {
        bail!("{} file(s) had errors.", error_count);
    }

    Ok(())
}

fn resolve_files_from_config(config_path: Option<&std::path::Path>) -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir()?;

    let config_file = if let Some(path) = config_path {
        path.to_path_buf()
    } else {
        config::find_biome_config(&cwd)
            .context("No biome.json found. Provide files as arguments or use --config.")?
    };

    let content = std::fs::read_to_string(&config_file)
        .with_context(|| format!("Failed to read {}", config_file.display()))?;
    let biome_config = config::parse_biome_config(&content)?;

    let base_dir = config_file.parent().unwrap_or(&cwd);
    config::resolve_file_paths(&biome_config, base_dir)
}
