# ejectest

## General Rules

- Use `just` recipes instead of raw cargo commands (see `Justfile`)
- Use `-q` for cargo commands — only show errors/warnings
- After any code changes, run `just check` and fix all warnings
- If clippy suggests `--fix`, use `cargo clippy --fix --workspace --all-targets`

See [CONTRIBUTING.md](CONTRIBUTING.md) for project conventions and code standards.

## Commits

- Conventional Commits;
- English language;
- Every commit made inside a task session carries a `Refs: <task-slug>` footer, where `<task-slug>` is the bound task's filename without its extension.
