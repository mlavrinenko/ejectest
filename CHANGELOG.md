# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-19

### Added

- Extract inline `#[cfg(test)] mod tests { ... }` into separate `_tests.rs` files
- `--dry-run` flag to preview changes without writing files
- State-machine scanner handling strings, comments, raw strings, lifetimes, and nested block comments
- Optional `syn`-based validation of generated output (`validate` feature, enabled by default)
- Multi-platform release binaries (Linux, macOS, Windows)
- E2E testing script for validation against real crates
