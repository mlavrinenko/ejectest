//! File list filtering for selective processing via `--files-from`.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// A validated set of file paths used to restrict which files are processed.
///
/// All paths are canonicalized for reliable comparison. Created from a
/// newline-separated file list (or stdin) with strict validation: every
/// listed path must exist and reside under the given root directory.
pub struct FileFilter {
    paths: HashSet<PathBuf>,
}

impl FileFilter {
    /// Build a filter from a list of paths, validating each against `root`.
    ///
    /// Every path is canonicalized. Paths that do not exist or fall outside
    /// `root` produce an error unless `lenient` is `true`, in which case
    /// they are silently dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if a path cannot be canonicalized, does not exist
    /// (when not lenient), or is outside `root` (when not lenient).
    pub fn from_paths(root: &Path, paths: Vec<PathBuf>, lenient: bool) -> Result<Self> {
        let canon_root = root
            .canonicalize()
            .with_context(|| format!("cannot resolve root path {}", root.display()))?;
        let mut set = HashSet::new();
        for path in paths {
            match path.canonicalize() {
                Ok(canon) => {
                    if !canon.starts_with(&canon_root) {
                        if lenient {
                            continue;
                        }
                        bail!("path {} is outside root {}", path.display(), root.display());
                    }
                    set.insert(canon);
                }
                Err(err) => {
                    if lenient {
                        continue;
                    }
                    return Err(err)
                        .with_context(|| format!("cannot resolve path {}", path.display()));
                }
            }
        }
        Ok(Self { paths: set })
    }

    /// Returns `true` if the canonical form of `path` is in this filter.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        path.canonicalize()
            .is_ok_and(|canon| self.paths.contains(&canon))
    }
}

/// Read newline-separated file paths from a file or stdin (when `source` is
/// `"-"`). Empty lines are skipped.
///
/// # Errors
///
/// Returns an error if the file cannot be read or stdin cannot be consumed.
pub fn read_file_list(source: &str) -> Result<Vec<PathBuf>> {
    let text = if source == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read stdin")?;
        buf
    } else {
        std::fs::read_to_string(source)
            .with_context(|| format!("failed to read file list from {source}"))?
    };
    Ok(text
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}
