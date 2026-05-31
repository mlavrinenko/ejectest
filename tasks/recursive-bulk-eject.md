# Recursive / bulk eject across a directory tree

## Motivation

ejectest today operates on a single file. Adopting the
sibling-test-file convention across an existing codebase means ejecting
dozens of files by hand, one invocation each. A directory mode makes
the one-time migration (and any later sweep) a single command.

Pairs with the `--check` mode task: `--check` finds the files,
bulk-eject fixes them.

## Proposed behaviour

- `ejectest <dir>` (or `ejectest --recursive <dir>` / `--all`): walk the
  tree, eject every file that carries an inline
  `#[cfg(test)] mod tests { ... }` block, skipping files already using
  an external `#[path]` test module.
- Report per file: ejected / skipped (already external) / no test
  module. Summary count at the end.
- Idempotent: re-running on an already-ejected tree changes nothing and
  exits zero.
- Honour `.gitignore` and skip `target/` (or document the exclusion
  model).
- Preserve the same correctness guarantees as single-file eject,
  including the `#[allow(...)]` preservation fix
  (see preserve-module-allow-attrs).

## Acceptance

- One invocation ejects all qualifying files under a directory.
- Already-ejected files are left byte-identical (idempotent).
- Mixed tree (some inline, some external, some no-tests) reports each
  correctly and ejects only the inline ones.
- Regression test over a small fixture tree.
