# Development recipes

# List available recipes
default:
    @just --list

# Run all checks (fmt + clippy + tests + unused deps + file size)
check:
    just fmt-check
    cargo clippy --workspace --all-targets -q -- -D warnings
    cargo test --workspace -q
    just machete
    just check-file-size

# Run tests only
test *ARGS:
    cargo test --workspace {{ ARGS }}

# Run clippy only
clippy:
    cargo clippy --workspace --all-targets -q -- -D warnings

# Auto-fix clippy warnings
clippy-fix:
    cargo clippy --fix --workspace --all-targets -- -D warnings

# Build the project
build:
    cargo build --workspace -q

# Run coverage with tarpaulin
cover:
    cargo tarpaulin --workspace --skip-clean

# Format code
fmt:
    cargo fmt --all

# Format check (CI-friendly)
fmt-check:
    cargo fmt --all -- --check

# Check for unused dependencies
machete:
    cargo machete

# Count tests across workspace
count-tests:
    #!/usr/bin/env bash
    cargo test --workspace 2>&1 | grep "test result:" | awk '{sum += $4} END {print sum " tests"}'

# Show top 20 files by line count
file-sizes:
    #!/usr/bin/env bash
    find . -type f \( -name '*.rs' -o -name '*.md' \) ! -path './target/*' -exec wc -l {} + | sort -rn | head -20

# Check for oversized files (fails if any exceed limits)
check-file-size:
    linecop

# Default crates for E2E testing
E2E_CRATES := "https://github.com/BurntSushi/jiff https://github.com/dtolnay/anyhow https://github.com/rayon-rs/rayon https://github.com/BurntSushi/memchr https://github.com/BurntSushi/regex-automata"

# E2E: clone real crates, eject all test modules, verify they still compile and tests pass
e2e CRATES=E2E_CRATES:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -q --release
    export EJECTEST_BIN="$(pwd)/target/release/ejectest"
    echo "{{ CRATES }}" | tr ' ' '\n' | parallel --will-cite --halt now,fail=1 --tag ./scripts/e2e.sh {}
 
# Tag a release and push (usage: just release 0.1.0)
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    just check
    git tag -a "v{{ VERSION }}" -m "v{{ VERSION }}"
    git push origin "v{{ VERSION }}"
