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

## Usage

```bash
ejectest apply src/lib.rs             # extract tests into src/lib_tests.rs
ejectest apply src/                   # eject every inline module under a tree
ejectest apply --dry-run src/         # preview without writing files
ejectest check src/                   # CI gate: fail if any inline test module remains
ejectest apply --files-from hot.txt src/  # process only files listed in hot.txt
ejectest check --files-from - src/    # read file list from stdin (e.g. piped from linecop)
ejectest --help                       # show all options
```

`apply` takes a file or a directory. Given a directory it walks the tree
(recursively, honouring `.gitignore`), ejecting every file with an inline
`#[cfg(test)] mod tests { ... }` block and skipping files already external
or without a test module. Re-running on an ejected tree is a no-op, so the
one-time migration and any later sweep are a single command.

`check` scans a file or directory (recursively, honouring `.gitignore`)
and exits non-zero when any file still carries an inline
`#[cfg(test)] mod tests { ... }` block — the `cargo fmt --check` idiom
for the sibling-test-file convention.

Both subcommands accept `--format <text|json>`. The JSON output differs
by subcommand:

```bash
ejectest check --format json src/
# {"files":[{"path":"src/lib.rs","status":"inline"}],"summary":{"total":1,"inline":1,"external":0,"no_tests":0}}

ejectest apply --format json src/     # actions: ejected|would_eject|skipped_external|skipped_no_tests
# {"files":[{"path":"src/lib.rs","action":"ejected","test_file":"lib_tests.rs"}],"summary":{"total":1,"ejected":1,"would_eject":0,"external":0,"no_tests":0}}
```

Both commands also accept `--files-from <PATH>` to restrict processing to
paths listed in a newline-separated file (`-` for stdin), and `--lenient`
to silently skip missing or out-of-root entries in that file list.

## Install

```bash
cargo install ejectest
```

Or download a pre-built binary from the
[latest release](https://github.com/mlavrinenko/ejectest/releases/latest).

### Nix flake

Add `ejectest` as a flake input and include it in your dev shell:

```nix
{
  inputs = {
    ejectest = {
      url = "github:mlavrinenko/ejectest";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # ... other inputs
  };

  outputs = { ejectest, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system: {
      devShells.default = nixpkgs.legacyPackages.${system}.mkShell {
        nativeBuildInputs = [
          ejectest.packages.${system}.default
        ];
      };
    });
}
```

Or run it directly without installing:

```bash
nix run github:mlavrinenko/ejectest -- apply src/lib.rs
```

## Library usage

Add to your `Cargo.toml` with default features disabled:

```toml
ejectest = { version = "0.3", default-features = false }
```

```rust
let result = ejectest::eject_tests(source, "lib")?;
// result.modified_source  — source with tests replaced by a #[path] stub
// result.test_content     — extracted test file contents
// result.test_file_name   — e.g. "lib_tests.rs"

// Read-only detection (powers `ejectest check`):
match ejectest::classify_source(source) {
    ejectest::Classification::Inline => { /* would be ejected */ }
    ejectest::Classification::External => { /* already a #[path] module */ }
    ejectest::Classification::NoTests => {}
}

// Programmatic file-list filtering (gated behind `cli` feature):
use ejectest::{FileFilter, read_file_list};
let paths = read_file_list("hot.txt")?;
let filter = FileFilter::from_paths("src/".as_ref(), paths, false)?;

// Walk a directory and apply (or check) with that filter:
let report = ejectest::apply_path("src/".as_ref(), false, Some(&filter))?;
let output = ejectest::render_apply(&report, ejectest::OutputFormat::Json, false);
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and coding conventions.

## License

MIT
