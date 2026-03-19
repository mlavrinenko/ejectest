use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

/// Extract inline `#[cfg(test)] mod tests { ... }` into a separate `_tests.rs` file.
#[derive(Parser)]
#[command(name = "ejectest", version, about)]
struct Cli {
    /// Rust source file to process.
    file: PathBuf,

    /// Show what would be done without writing files.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let source = std::fs::read_to_string(&cli.file)
        .with_context(|| format!("failed to read {}", cli.file.display()))?;

    let file_stem = cli
        .file
        .file_stem()
        .and_then(|os| os.to_str())
        .context("invalid file name")?;

    let result = ejectest::eject_tests(&source, file_stem)?;

    let parent = cli.file.parent().unwrap_or_else(|| Path::new("."));
    let test_path = parent.join(&result.test_file_name);

    if cli.dry_run {
        log::info!("dry-run mode: no files written");
        println!("Would create: {}", test_path.display());
        println!("Would modify: {}\n", cli.file.display());
        println!("--- {} ---", result.test_file_name);
        print!("{}", result.test_content);
        println!("--- end ---");
    } else {
        std::fs::write(&test_path, &result.test_content)
            .with_context(|| format!("failed to write {}", test_path.display()))?;
        std::fs::write(&cli.file, &result.modified_source)
            .with_context(|| format!("failed to write {}", cli.file.display()))?;
        println!("Created: {}", test_path.display());
        println!("Modified: {}", cli.file.display());
    }

    Ok(())
}
