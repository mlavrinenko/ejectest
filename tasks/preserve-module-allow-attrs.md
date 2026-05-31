# Eject drops `#[allow(...)]` attributes on the `mod tests` declaration

## Severity

Bug — produces code that fails `clippy -D warnings` after a successful eject.

## Symptom

Ejecting a test module that carries lint allowances on the `mod tests`
declaration loses them. The extracted `_tests.rs` then trips the very
lints the original suppressed.

Input:

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    // ... tests using unwrap() / arr[i] ...
}
```

After `ejectest`:

```rust
// original file
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

```rust
// foo_tests.rs  — the #[allow(...)] is gone
use super::*;
// ... unwrap() / arr[i] now DENY-level errors ...
```

`cargo clippy --all-targets -- -D warnings` then fails until the author
manually re-adds `#![allow(clippy::unwrap_used, clippy::indexing_slicing)]`
to the top of `foo_tests.rs`.

## Root cause

`extract` (src/lib.rs) copies only the module's inner items
(`region.inner_start..region.inner_end`) into the new file. Any outer
attributes on the `mod tests` line other than `#[cfg(test)]` — most
commonly `#[allow(...)]` — are discarded; they were neither kept on the
`mod tests;` stub nor translated into the moved file.

## Expected

Outer attributes on the original `mod tests` (excluding `#[cfg(test)]`,
which the stub keeps) should survive the move. Translate each to an
inner attribute at the top of the generated `_tests.rs`:

```rust
// foo_tests.rs
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
use super::*;
```

This matches the idiom already used by hand-written sibling test files
(e.g. `#![allow(...)]` at the top of `*_tests.rs`).

## Acceptance

- Ejecting a module with `#[allow(...)]` on `mod tests` emits the
  equivalent `#![allow(...)]` as the first line(s) of the `_tests.rs`.
- A round-trip on such a module leaves `clippy -D warnings` green with
  no manual edit.
- `#[cfg(test)]` stays on the stub and is not duplicated into the file.
- Regression test covering the allow-bearing module shape.

## Found

Surfaced in the speconaut repo: ejecting
`src/commands/audit_traces_cmd/lift.rs` (carried
`#[allow(clippy::unwrap_used, clippy::indexing_slicing)]`) produced a
`_tests.rs` that failed clippy with ~14 `indexing_slicing` errors until
the inner allow was re-added by hand.
