# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `apply --overwrite` replaces a test file that holds other content
- `check --mod-rs-tests <sibling|tests>` to name the target it reports
- `check` and `apply --dry-run` report what `apply` would do with the test file: `"target"` (`missing`, `identical`, `conflict`, `overwrite`) and `"test_file"` in JSON, warnings and refusals on stderr; a refused file has action `refused`
- `Report::warnings`, `Report::refusals`, `FileResult::target` and `TargetState` in the library API

### Changed

- BREAKING: `apply` refuses, writing neither file, when the test file exists with content other than the extracted tests, for every target name (`<stem>_tests.rs`, `mod_tests.rs`, `tests.rs`), not only `tests.rs`; it used to overwrite silently. A directory run checks every target before writing anything. A test file that already holds exactly the tests is left as is with a warning, and the source is still edited
- BREAKING: `apply_path` takes an `overwrite` argument and `check_path` a `ModRsTests` argument

## [0.4.0] - 2026-10-07

### Added

- `apply --mod-rs-tests <sibling|tests>`: `tests` ejects a `mod.rs`, `lib.rs` or `main.rs` file's tests to a plain `tests.rs` behind `mod tests;` with no `#[path]`, matching `clippy::self_named_module_files`. The default `sibling` keeps `<stem>_tests.rs` with `#[path]`; other files are unaffected. `apply` refuses, without writing, when `tests.rs` already exists. Text and JSON reports name the file actually written
- `ModRsTests` in the library API

### Changed

- BREAKING: `eject_tests` and `apply_path` take a `ModRsTests` argument; pass `ModRsTests::Sibling` for the previous behaviour

## [0.3.0] - 2026-06-03

### Added

- `--files-from <PATH>` on `apply` and `check`: process only files listed in a newline-separated file; use `-` for stdin. Files outside the target root or missing produce errors
- `--lenient` flag on `apply` and `check`: when used with `--files-from`, missing files and paths outside the target root are silently skipped instead of erroring
- `FileFilter` / `read_file_list` in the library API for programmatic file-list filtering

## [0.2.0] - 2026-05-31

### Added

- `apply` now accepts a directory: walk the tree (honouring `.gitignore`) and eject every file carrying an inline `#[cfg(test)] mod tests { ... }` block in one invocation, skipping already-external and no-test files. Idempotent — re-running an ejected tree changes nothing
- `check` subcommand: scan a file or directory (recursively, honouring `.gitignore`) for inline `#[cfg(test)] mod tests { ... }` blocks without modifying anything; exits non-zero when any are found (CI / pre-commit gate)
- `--format <text|json>` on both subcommands; JSON output shares one structure for single-file and directory inputs. `apply --format json` reports an `action` per file (`ejected`, `would_eject`, `skipped_external`, `skipped_no_tests`)
- Library API `classify_source` / `Classification` for read-only detection (usable with `default-features = false`)

### Changed

- BREAKING: CLI now uses subcommands. `ejectest <file>` becomes `ejectest apply <file>`; `--dry-run` moves under `apply`

### Fixed

- Preserve outer attributes (e.g. `#[allow(...)]`) on the `mod tests` declaration by translating them to inner attributes (`#![...]`) at the top of the extracted `_tests.rs`; `#[cfg(test)]` stays on the stub

## [0.1.0] - 2026-03-19

### Added

- Extract inline `#[cfg(test)] mod tests { ... }` into separate `_tests.rs` files
- `--dry-run` flag to preview changes without writing files
- State-machine scanner handling strings, comments, raw strings, lifetimes, and nested block comments
- Optional `syn`-based validation of generated output (`validate` feature, enabled by default)
- Multi-platform release binaries (Linux, macOS, Windows)
- Library API (`ejectest::eject_tests`) usable with `default-features = false`
- `--version` flag
- E2E testing script for validation against real crates
