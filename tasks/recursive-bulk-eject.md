# Recursive / bulk eject across a directory tree

## Motivation

ejectest today operates on a single file. Adopting the
sibling-test-file convention across an existing codebase means ejecting
dozens of files by hand, one invocation each. A directory mode makes
the one-time migration (and any later sweep) a single command.

Pairs with the `check` mode task: `check` finds the files, bulk-eject
fixes them. The two share the directory walker and the JSON report
structure (see check-mode-detect-inline-test-modules).

## CLI shape

Extends the `apply` subcommand to accept a directory (it already takes
a single file):

```bash
ejectest apply <dir>            # walk + eject every inline module
ejectest apply --dry-run <dir>  # preview
ejectest apply --format json <dir>
```

No new subcommand or `--recursive` flag: a directory path implies the
walk. JSON output uses the same structure as the single-file case
(see below) so a machine consumer parses both identically.

## JSON schema

`apply --format json` emits one object on stdout, mirroring the `check`
schema but with an `action` per file instead of a `status`:

```json
{
  "files": [
    { "path": "src/foo.rs", "action": "ejected", "test_file": "foo_tests.rs" },
    { "path": "src/bar.rs", "action": "skipped_external" },
    { "path": "src/baz.rs", "action": "skipped_no_tests" }
  ],
  "summary": {
    "total": 3, "ejected": 1, "would_eject": 0,
    "external": 1, "no_tests": 1
  }
}
```

`action` values: `ejected`, `would_eject` (under `--dry-run`),
`skipped_external`, `skipped_no_tests`. A single-file `apply` is just a
one-element `files` array. (Single-file `apply` currently errors on a
file with no inline module rather than reporting `skipped_no_tests`;
directory mode should skip such files instead — reconcile when
implementing.)

## Proposed behaviour

- `ejectest apply <dir>`: walk the tree, eject every file that carries
  an inline `#[cfg(test)] mod tests { ... }` block, skipping files
  already using an external `#[path]` test module and files with no
  test module.
- Report per file: ejected / skipped (already external) / no test
  module. Summary count at the end.
- Idempotent: re-running on an already-ejected tree changes nothing and
  exits zero.
- Honour `.gitignore` (the `ignore` crate, already wired for `check`,
  also skips hidden files and gitignored paths such as `target/`).
- Preserve the same correctness guarantees as single-file eject,
  including the `#[allow(...)]` preservation fix
  (see preserve-module-allow-attrs).

## Acceptance

- One invocation ejects all qualifying files under a directory.
- Already-ejected files are left byte-identical (idempotent).
- Mixed tree (some inline, some external, some no-tests) reports each
  correctly and ejects only the inline ones.
- JSON output matches the single-file structure.
- Regression test over a small fixture tree.
