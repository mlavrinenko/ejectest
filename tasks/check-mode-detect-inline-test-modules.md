# `--check` mode: detect inline test modules without modifying

## Motivation

Projects that want the sibling-test-file convention (`foo.rs` +
`foo_tests.rs`) need a way to enforce it in CI / a pre-commit gate:
fail when any source file still carries an inline
`#[cfg(test)] mod tests { ... }` block that should be ejected.

ejectest already has the detection logic (it locates the inline module
to extract). Exposing it as a read-only check turns "we have a
convention" into "the convention is enforced by a mechanism" — no
reviewer memory required.

Concrete driver: in the speconaut repo, editing test code that lives in
the same file as logic changes that file's content hash, which drifts
the coverage traces of every scenario covering it (repeated re-lock
churn). Keeping tests in sibling files removes that whole class — but
only if something stops inline test modules from creeping back.

## Proposed behaviour

- `ejectest --check <path>` (file or directory): scan, report every
  file containing an inline `#[cfg(test)] mod tests { ... }` block that
  ejectest would extract, write nothing.
- Exit non-zero when any such file is found, zero when clean (the
  `cargo fmt --check` / `prettier --check` idiom).
- List offending paths on stdout/stderr for the operator.
- Respect the same skip rules as eject (already-external `#[path]`
  modules are not flagged).

## Acceptance

- `ejectest --check` on a tree with an inline test module exits
  non-zero and names the file; on a clean tree exits zero and is
  silent.
- `--check` never writes or modifies any file.
- Directory mode recurses; respects `.gitignore` (or documents that it
  does not).
- Regression tests for: clean tree, inline-module tree, already-ejected
  tree.
