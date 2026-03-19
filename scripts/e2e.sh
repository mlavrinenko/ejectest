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

echo "==> Scanning for inline test modules"
FOUND=0
EJECTED=0
while IFS= read -r file; do
    FOUND=$((FOUND + 1))
    if "$BIN" --dry-run "$file" >/dev/null 2>&1; then
        if "$BIN" "$file" 2>/dev/null; then
            EJECTED=$((EJECTED + 1))
        else
            echo "  WARN: failed to eject $file"
        fi
    fi
done < <(grep -rl '#\[cfg(test)\]' "$WORK/crate" --include='*.rs' || true)
echo "  Found $FOUND files with #[cfg(test)], ejected $EJECTED"

if [ "$EJECTED" -eq 0 ]; then
    echo "ERROR: no files were ejected — something is wrong"
    exit 1
fi

echo "==> Verifying crate still compiles"
(cd "$WORK/crate" && cargo check -q 2>&1)

echo "==> Running crate tests (lib + integration, skipping doctests)"
(cd "$WORK/crate" && cargo test --lib --tests -q 2>&1)

echo "==> E2E passed ($EJECTED files ejected, crate compiles and tests pass)"
