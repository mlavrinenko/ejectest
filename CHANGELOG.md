# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
