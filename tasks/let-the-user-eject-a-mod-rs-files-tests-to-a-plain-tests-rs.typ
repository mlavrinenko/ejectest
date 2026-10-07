#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Let the user eject a mod-rs file's tests to a plain tests.rs",
  status: proposed(2026, 10, 7),
)

== Summary

`ejectest apply` always names the extracted file `<stem>_tests.rs` and wires it with `#[path]` (`src/lib.rs:112`). For a mod-rs file (`mod.rs`, `lib.rs`, `main.rs`) that gives `filter/mod_tests.rs`, where Rust's own resolution of a plain `mod tests;` already lands on `filter/tests.rs`. A crate that enforces `clippy::self_named_module_files` (mindtape is moving to that layout) wants the plain form.

== Behaviour

A flag picks the style for mod-rs files only; the default keeps today's output.

```
ejectest apply src --mod-rs-tests sibling   # default: filter/mod.rs → filter/mod_tests.rs + #[path]
ejectest apply src --mod-rs-tests tests     # filter/mod.rs → filter/tests.rs + plain `mod tests;`
```

- Non-mod-rs files (`filter/eval.rs`) keep `eval_tests.rs` + `#[path]` under either value: a plain `mod tests;` there would resolve to `filter/eval/tests.rs` and create the `eval.rs` + `eval/` pair that lint rejects.
- `check` needs no change: `src/scanner/mod.rs:70` already reports any `mod tests;` as external.
- `--format json` and the text report name the file actually written.
- The library API (`eject_tests`) takes the choice too, so the CLI is not the only caller that can use it.
- Refuse, without writing, when the target `tests.rs` already exists.

== Done when

- A red test per case above lands first, then the fix.
- README and CHANGELOG describe the flag.
- A new minor version is released (`just release`) and published, so mindtape's dev shell can pin it.
