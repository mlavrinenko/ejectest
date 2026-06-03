# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
