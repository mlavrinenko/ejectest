<p align="center">
  <img src="www/logo.svg" alt="ejectest logo" width="80">
</p>

# ejectest

[![CI](https://github.com/mlavrinenko/ejectest/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/ejectest/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ejectest.svg)](https://crates.io/crates/ejectest)
[![License: MIT](https://img.shields.io/crates/l/ejectest.svg)](LICENSE-MIT)

Extract tests to separate _test.rs file.

## Install

### From crates.io

```bash
cargo install ejectest
```

### From binary releases

Download a pre-built binary from the
[latest release](https://github.com/mlavrinenko/ejectest/releases/latest).

## Usage

```bash
ejectest
```

## Development

Prerequisites: [Nix](https://nixos.org/) with flakes enabled.

```bash
direnv allow   # or: nix develop

just check     # fmt + clippy + tests + file-size check
just build
just test
just cover     # code coverage (70% minimum)
just fmt       # format code
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions.

## License

MIT
