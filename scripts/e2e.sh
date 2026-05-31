#!/usr/bin/env bash
# E2E test: clone a real crate, eject all test modules, verify compilation and tests.
# Usage: scripts/e2e.sh [REPO_URL]
set -euo pipefail

REPO="${1:?usage: e2e.sh <repo-url>}"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

echo "==> Cloning $REPO into $WORK"
git clone --depth 1 "$REPO" "$WORK/crate" 2>&1 | tail -1

if [ -n "${EJECTEST_BIN:-}" ]; then
    BIN="$EJECTEST_BIN"
else
    echo "==> Building ejectest"
    cargo build -q --release
    BIN="$(pwd)/target/release/ejectest"
fi

echo "==> Ejecting all inline test modules under the crate"
REPORT="$("$BIN" apply --format json "$WORK/crate")"
EJECTED="$(printf '%s' "$REPORT" | grep -o '"ejected":[0-9]*' | grep -o '[0-9]*')"
echo "  Ejected $EJECTED files"

if [ "${EJECTED:-0}" -eq 0 ]; then
    echo "ERROR: no files were ejected — something is wrong"
    exit 1
fi

echo "==> Verifying crate still compiles"
(cd "$WORK/crate" && cargo check -q 2>&1)

echo "==> Running crate tests (lib + integration, skipping doctests)"
(cd "$WORK/crate" && cargo test --lib --tests -q 2>&1)

echo "==> E2E passed ($EJECTED files ejected, crate compiles and tests pass)"
