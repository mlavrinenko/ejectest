use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use ejectest::{FileFilter, ModRsTests, OutputFormat, read_file_list};

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
        /// How a mod-rs file (`mod.rs`, `lib.rs`, `main.rs`) names its extracted tests.
        #[arg(long, value_enum, default_value_t = ModRsStyle::Sibling)]
        mod_rs_tests: ModRsStyle,
        /// Replace a test file that already holds other content (otherwise the eject is refused).
        #[arg(long)]
        overwrite: bool,
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
        /// How a mod-rs file would name its extracted tests, to report the target `apply` would find.
        #[arg(long, value_enum, default_value_t = ModRsStyle::Sibling)]
        mod_rs_tests: ModRsStyle,
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

/// Print each message to stderr as `<level>: <message>`.
fn warn_all(level: &str, messages: &[String]) {
    for message in messages {
        eprintln!("{level}: {message}");
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
            mod_rs_tests,
            overwrite,
        } => {
            let filter = build_filter(files_from.as_deref(), &path, lenient)?;
            let report = ejectest::apply_path(
                &path,
                dry_run,
                filter.as_ref(),
                mod_rs_tests.into(),
                overwrite,
            )?;
            print!(
                "{}",
                ejectest::render_apply(&report, format.into(), dry_run)
            );
            warn_all("warning", &report.warnings());
            let refusals = report.refusals();
            warn_all("error", &refusals);
            Ok(if refusals.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Command::Check {
            path,
            format,
            files_from,
            lenient,
            mod_rs_tests,
        } => {
            let filter = build_filter(files_from.as_deref(), &path, lenient)?;
            let report = ejectest::check_path(&path, filter.as_ref(), mod_rs_tests.into())?;
            print!("{}", ejectest::render_check(&report, format.into()));
            warn_all("warning", &report.warnings());
            let would_refuse: Vec<String> = report
                .refusals()
                .into_iter()
                .map(|msg| format!("apply would refuse: {msg}"))
                .collect();
            warn_all("warning", &would_refuse);
            if report.has_inline() {
                Ok(ExitCode::FAILURE)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

/// CLI spelling of [`ModRsTests`].
#[derive(Clone, Copy, ValueEnum)]
enum ModRsStyle {
    /// `<stem>_tests.rs` with `#[path]` (default).
    Sibling,
    /// Plain `tests.rs` with `mod tests;`.
    Tests,
}

impl From<ModRsStyle> for ModRsTests {
    fn from(style: ModRsStyle) -> Self {
        match style {
            ModRsStyle::Sibling => Self::Sibling,
            ModRsStyle::Tests => Self::Tests,
        }
    }
}
