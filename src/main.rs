use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use ejectest::OutputFormat;

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
    },
    /// Detect inline test modules without modifying (file or directory).
    Check {
        /// Rust source file or directory to scan.
        path: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
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

fn main() -> Result<ExitCode> {
    env_logger::init();

    match Cli::parse().command {
        Command::Apply {
            path,
            dry_run,
            format,
        } => {
            let report = ejectest::apply_path(&path, dry_run)?;
            print!(
                "{}",
                ejectest::render_apply(&report, format.into(), dry_run)
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { path, format } => {
            let report = ejectest::check_path(&path)?;
            print!("{}", ejectest::render_check(&report, format.into()));
            if report.has_inline() {
                Ok(ExitCode::FAILURE)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}
