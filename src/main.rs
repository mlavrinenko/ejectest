use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use ejectest::{FileFilter, OutputFormat, read_file_list};

/// Extract inline `#[cfg(test)] mod tests { ... }` into separate `_tests.rs` files.
#[derive(Parser)]
#[command(name = "ejectest", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract inline test modules from a file or directory (writes by default).
    Apply {
        /// Rust source file or directory to process.
        path: PathBuf,
        /// Show what would be done without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Process only files listed in the given file (use `-` for stdin).
        #[arg(long)]
        files_from: Option<String>,
        /// Allow missing files and paths outside root in the file list.
        #[arg(long)]
        lenient: bool,
    },
    /// Detect inline test modules without modifying (file or directory).
    Check {
        /// Rust source file or directory to scan.
        path: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Check only files listed in the given file (use `-` for stdin).
        #[arg(long)]
        files_from: Option<String>,
        /// Allow missing files and paths outside root in the file list.
        #[arg(long)]
        lenient: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

impl From<Format> for OutputFormat {
    fn from(format: Format) -> Self {
        match format {
            Format::Text => Self::Text,
            Format::Json => Self::Json,
        }
    }
}

fn build_filter(
    files_from: Option<&str>,
    root: &std::path::Path,
    lenient: bool,
) -> Result<Option<FileFilter>> {
    match files_from {
        Some(source) => {
            let paths = read_file_list(source)?;
            Ok(Some(FileFilter::from_paths(root, paths, lenient)?))
        }
        None => Ok(None),
    }
}

fn main() -> Result<ExitCode> {
    env_logger::init();

    match Cli::parse().command {
        Command::Apply {
            path,
            dry_run,
            format,
            files_from,
            lenient,
        } => {
            let filter = build_filter(files_from.as_deref(), &path, lenient)?;
            let report = ejectest::apply_path(&path, dry_run, filter.as_ref())?;
            print!(
                "{}",
                ejectest::render_apply(&report, format.into(), dry_run)
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Check {
            path,
            format,
            files_from,
            lenient,
        } => {
            let filter = build_filter(files_from.as_deref(), &path, lenient)?;
            let report = ejectest::check_path(&path, filter.as_ref())?;
            print!("{}", ejectest::render_check(&report, format.into()));
            if report.has_inline() {
                Ok(ExitCode::FAILURE)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}
