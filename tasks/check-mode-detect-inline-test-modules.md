# `check` mode: detect inline test modules without modifying

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

## CLI shape

The CLI is split into subcommands (breaking change from the single
positional `ejectest <file>` form):

```bash
ejectest apply <path>    # extract (write); the old default behaviour
ejectest check <path>    # read-only detection, this task
```

Both accept `--format <text|json>` (default `text`). `apply` keeps
`--dry-run`.

`apply <path>` is expected to accept both files and directories in the
future (see recursive-bulk-eject); `check <path>` accepts both now.
JSON output uses the same structure for a single file and for a
directory tree (a single file is just a one-element `files` array).

## JSON schema

`check --format json` emits one object on stdout:

```json
{
  "files": [
    { "path": "src/foo.rs", "status": "inline" },
    { "path": "src/bar.rs", "status": "external" },
    { "path": "src/baz.rs", "status": "no_tests" }
  ],
  "summary": { "total": 3, "inline": 1, "external": 1, "no_tests": 1 }
}
```

`status` values: `inline` (would be ejected), `external` (already a
`#[path]` module), `no_tests`.

## Proposed behaviour

- `ejectest check <path>` (file or directory): scan, classify every
  Rust file, write nothing.
- Exit non-zero when any `inline` file is found, zero when clean (the
  `cargo fmt --check` / `prettier --check` idiom).
- Text format: list offending (`inline`) paths on stdout, one per line;
  silent on a clean tree. JSON format: emit the schema above.
- Respect the same skip rules as eject (already-external `#[path]`
  modules are not flagged).
- Directory mode recurses respecting `.gitignore` (via the `ignore`
  crate, which also skips hidden files); single files are checked
  regardless of ignore rules.

## Acceptance

- `ejectest check` on a tree with an inline test module exits non-zero
  and names the file; on a clean tree exits zero and is silent.
- `check` never writes or modifies any file.
- Directory mode recurses; respects `.gitignore`.
- `--format json` emits the documented schema and is valid JSON.
- Regression tests for: clean tree, inline-module tree, already-ejected
  tree, JSON output, gitignored file skipped.
