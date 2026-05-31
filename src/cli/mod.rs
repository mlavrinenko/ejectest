//! CLI orchestration: directory walking, `check` / `apply` commands, output
//! rendering. Gated behind the `cli` feature.

mod render;
mod walk;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{Classification, classify_source, eject_tests};

pub use render::{render_apply, render_check};

/// Output format for command results.
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// Human-readable plain text.
    Text,
    /// Machine-readable JSON.
    Json,
}

/// Classification result for a single scanned file.
pub struct FileResult {
    /// Path to the source file.
    pub path: PathBuf,
    /// How the file's test module was classified.
    pub classification: Classification,
    /// Name of the (would-be) extracted test file, when inline.
    pub test_file: Option<String>,
    /// Whether an eject was actually written to disk for this file.
    pub applied: bool,
}

/// Per-file results from a `check` or `apply` run.
pub struct Report {
    /// One entry per scanned source file.
    pub results: Vec<FileResult>,
}

impl Report {
    /// True if any scanned file still carries an inline test module.
    #[must_use]
    pub fn has_inline(&self) -> bool {
        self.results
            .iter()
            .any(|res| res.classification == Classification::Inline)
    }
}

/// Scan `path` (file or directory) and classify every Rust file without
/// modifying anything.
///
/// # Errors
///
/// Returns an error if the path cannot be walked or a file cannot be read.
pub fn check_path(path: &Path) -> Result<Report> {
    let files = walk::collect_rust_files(path)?;
    let mut results = Vec::with_capacity(files.len());
    for file in files {
        let source = std::fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        results.push(FileResult {
            path: file,
            classification: classify_source(&source),
            test_file: None,
            applied: false,
        });
    }
    Ok(Report { results })
}

/// Eject the inline test module from a single file.
///
/// Directories are not yet supported (see recursive-bulk-eject).
///
/// # Errors
///
/// Returns an error if `path` is a directory, the file cannot be read or
/// written, the file name is invalid, or no inline test module is present.
pub fn apply_path(path: &Path, dry_run: bool) -> Result<Report> {
    if path.is_dir() {
        anyhow::bail!("apply does not support directories yet; see recursive-bulk-eject");
    }

    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let file_stem = path
        .file_stem()
        .and_then(|os| os.to_str())
        .context("invalid file name")?;

    let result = eject_tests(&source, file_stem)?;
    let test_file = result.test_file_name.clone();

    if !dry_run {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let test_path = parent.join(&result.test_file_name);
        std::fs::write(&test_path, &result.test_content)
            .with_context(|| format!("failed to write {}", test_path.display()))?;
        std::fs::write(path, &result.modified_source)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(Report {
        results: vec![FileResult {
            path: path.to_path_buf(),
            classification: Classification::Inline,
            test_file: Some(test_file),
            applied: !dry_run,
        }],
    })
}
