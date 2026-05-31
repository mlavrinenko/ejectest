//! Directory walking for the `check` / `apply` commands.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

/// Collect the `.rs` files to process under `root`.
///
/// A file path is returned as-is (one element), regardless of ignore rules.
/// A directory is walked recursively via the `ignore` crate, which honours
/// `.gitignore` and skips hidden entries. Results are sorted for determinism.
///
/// # Errors
///
/// Returns an error if a directory entry cannot be read while walking.
pub(crate) fn collect_rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut files = Vec::new();
    // Honour `.gitignore` even outside a git checkout (default needs `.git`).
    for entry in WalkBuilder::new(root).require_git(false).build() {
        let entry = entry.with_context(|| format!("failed to walk {}", root.display()))?;
        let path = entry.path();
        let is_file = entry.file_type().is_some_and(|ft| ft.is_file());
        if is_file && path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}
