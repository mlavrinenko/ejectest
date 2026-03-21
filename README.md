<p align="center">
  <img src="www/logo.svg" alt="ejectest logo" width="80">
</p>

# ejectest

[![CI](https://github.com/mlavrinenko/ejectest/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/ejectest/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ejectest.svg)](https://crates.io/crates/ejectest)
[![License: MIT](https://img.shields.io/crates/l/ejectest.svg)](LICENSE-MIT)

Extract inline `#[cfg(test)] mod tests { ... }` into separate `_tests.rs` files.

## Why?

Inline tests are convenient — until your files grow too large.
Manually moving tests to a separate file means editing the source
and creating a new test file with the right module path.
That's busywork. **ejectest** does it in one command.

## Install

```bash
cargo install ejectest
```

Or download a pre-built binary from the
[latest release](https://github.com/mlavrinenko/ejectest/releases/latest).

## Usage

```bash
ejectest src/lib.rs              # extract tests into src/lib_tests.rs
ejectest --dry-run src/lib.rs    # preview without writing files
ejectest --help                  # show all options
```

## Library usage

Add to your `Cargo.toml` with default features disabled:

```toml
ejectest = { version = "0.1", default-features = false }
```

```rust
let result = ejectest::eject_tests(&source, "lib")?;
// result.modified_source  — source with tests replaced by a #[path] stub
// result.test_content     — extracted test file contents
// result.test_file_name   — e.g. "lib_tests.rs"
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and coding conventions.

## License

MIT
