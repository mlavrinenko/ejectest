//! CLI orchestration: directory walking, `check` / `apply` commands, output
//! rendering. Gated behind the `cli` feature.

mod filelist;
mod render;
mod walk;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{Classification, ModRsTests, classify_source, eject_tests};

pub use filelist::{FileFilter, read_file_list};
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
#[derive(Debug)]
pub struct FileResult {
    /// Path to the source file.
    pub path: PathBuf,
    /// How the file's test module was classified.
    pub classification: Classification,
    /// Name of the (would-be) extracted test file, when inline.
    pub test_file: Option<String>,
    /// Whether an eject was actually written to disk for this file.
    pub applied: bool,
    /// What the test file's path holds relative to the eject, when inline.
    pub target: Option<TargetState>,
}

/// What an eject finds at the path it would write the tests to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    /// Nothing there: the test file is written.
    Missing,
    /// The file already holds exactly these tests: left as is, with a warning.
    Identical,
    /// The file holds other content and `overwrite` is off: the eject is refused.
    Conflict,
    /// The file holds other content and `overwrite` is on: it is replaced.
    Overwrite,
}

/// Per-file results from a `check` or `apply` run.
#[derive(Debug)]
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

    /// One warning per eject whose test file already holds the extracted tests.
    #[must_use]
    pub fn warnings(&self) -> Vec<String> {
        self.messages(
            TargetState::Identical,
            "already holds these tests; left as is",
        )
    }

    /// One message per eject that `apply` refuses because its test file holds other content.
    #[must_use]
    pub fn refusals(&self) -> Vec<String> {
        self.messages(
            TargetState::Conflict,
            "exists with other content; pass --overwrite to replace it",
        )
    }

    fn messages(&self, state: TargetState, text: &str) -> Vec<String> {
        self.results
            .iter()
            .filter(|res| res.target == Some(state))
            .map(|res| format!("{} {text}", render::test_path_for(res)))
            .collect()
    }
}

/// Scan `path` (file or directory) and classify every Rust file without
/// modifying anything.
///
/// For an inline file the report also says what `apply` would find at its
/// test file's path ([`FileResult::target`]), under `mod_rs_tests` and without
/// `--overwrite`.
///
/// When `filter` is provided, only files present in the filter are scanned.
///
/// # Errors
///
/// Returns an error if the path cannot be walked or a file cannot be read.
pub fn check_path(
    path: &Path,
    filter: Option<&FileFilter>,
    mod_rs_tests: ModRsTests,
) -> Result<Report> {
    let files = walk::collect_rust_files(path)?;
    let mut results = Vec::with_capacity(files.len());
    let mut planned = Planned::new();
    for file in files {
        if filter.is_some_and(|ff| !ff.contains(&file)) {
            continue;
        }
        let source = std::fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let classification = classify_source(&source);
        let plan = match classification {
            Classification::Inline => {
                plan_eject(&file, &source, mod_rs_tests, false, &mut planned).ok()
            }
            _ => None,
        };
        results.push(FileResult {
            path: file,
            classification,
            test_file: plan.as_ref().map(|pl| pl.test_file_name.clone()),
            applied: false,
            target: plan.map(|pl| pl.state),
        });
    }
    Ok(Report { results })
}

/// Eject inline test modules from `path` (a single file or a directory).
///
/// For a directory, the tree is walked (honouring `.gitignore`) and every
/// file carrying an inline `#[cfg(test)] mod tests { ... }` block is ejected;
/// files already external or without a test module are skipped and reported.
/// Re-running on an already-ejected tree is a no-op (idempotent).
///
/// For a single file, the file must carry an inline module: external or
/// no-test files are reported as errors rather than skipped.
///
/// A missing test file is written. A test file that already holds exactly the
/// extracted tests is left as is (the source is still edited; see
/// [`Report::warnings`]). A test file holding anything else makes the whole
/// run fail before any file is written, unless `overwrite` is set. With
/// `dry_run`, nothing is written and a refusal is reported in
/// [`Report::refusals`] instead of failing.
///
/// When `filter` is provided, only files present in the filter are processed.
///
/// # Errors
///
/// Returns an error if the path cannot be walked, a file cannot be read or
/// written, a file name is invalid, a test file would be overwritten without
/// `overwrite`, or - for a single-file input - no inline test module is present.
pub fn apply_path(
    path: &Path,
    dry_run: bool,
    filter: Option<&FileFilter>,
    mod_rs_tests: ModRsTests,
    overwrite: bool,
) -> Result<Report> {
    let single = !path.is_dir();
    let files = if single {
        vec![path.to_path_buf()]
    } else {
        let mut files = walk::collect_rust_files(path)?;
        files.retain(|file| filter.is_none_or(|ff| ff.contains(file)));
        files
    };
    let mut planned = Planned::new();
    let mut results = Vec::with_capacity(files.len());
    let mut plans = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let classification = classify_source(&source);
        if !single && classification != Classification::Inline {
            results.push(FileResult {
                path: file,
                classification,
                test_file: None,
                applied: false,
                target: None,
            });
            continue;
        }
        let plan = plan_eject(&file, &source, mod_rs_tests, overwrite, &mut planned)?;
        results.push(FileResult {
            path: file,
            classification: Classification::Inline,
            test_file: Some(plan.test_file_name.clone()),
            applied: !dry_run && plan.state != TargetState::Conflict,
            target: Some(plan.state),
        });
        plans.push(plan);
    }
    let report = Report { results };
    if !dry_run {
        let refusals = report.refusals();
        if !refusals.is_empty() {
            anyhow::bail!("{}", refusals.join("\n"));
        }
        for plan in &plans {
            write_plan(plan)?;
        }
    }
    Ok(report)
}

/// Test file contents already claimed by an earlier file in the same run.
type Planned = HashMap<PathBuf, String>;

/// One file's eject, decided but not yet written.
struct Plan {
    source_path: PathBuf,
    modified_source: String,
    test_path: PathBuf,
    test_content: String,
    test_file_name: String,
    state: TargetState,
}

/// Extract the inline test module from `source` and decide what to do with its
/// test file's path, without writing anything.
fn plan_eject(
    path: &Path,
    source: &str,
    mod_rs_tests: ModRsTests,
    overwrite: bool,
    planned: &mut Planned,
) -> Result<Plan> {
    let file_stem = path
        .file_stem()
        .and_then(|os| os.to_str())
        .context("invalid file name")?;
    let result = eject_tests(source, file_stem, mod_rs_tests)?;
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let test_path = parent.join(&result.test_file_name);
    let existing = match planned.get(&test_path) {
        Some(claimed) => Some(claimed.clone()),
        None => read_existing(&test_path)?,
    };
    let state = match existing {
        None => TargetState::Missing,
        Some(content) if content == result.test_content => TargetState::Identical,
        Some(_) if overwrite => TargetState::Overwrite,
        Some(_) => TargetState::Conflict,
    };
    if state != TargetState::Conflict {
        planned.insert(test_path.clone(), result.test_content.clone());
    }
    Ok(Plan {
        source_path: path.to_path_buf(),
        modified_source: result.modified_source,
        test_path,
        test_content: result.test_content,
        test_file_name: result.test_file_name,
        state,
    })
}

/// The test file's current content, or `None` when it does not exist.
fn read_existing(path: &Path) -> Result<Option<String>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
    }
}

/// Write a plan: the test file unless it already holds the tests, then the source edit.
fn write_plan(plan: &Plan) -> Result<()> {
    if plan.state != TargetState::Identical {
        std::fs::write(&plan.test_path, &plan.test_content)
            .with_context(|| format!("failed to write {}", plan.test_path.display()))?;
    }
    std::fs::write(&plan.source_path, &plan.modified_source)
        .with_context(|| format!("failed to write {}", plan.source_path.display()))
}
