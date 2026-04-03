#!/usr/bin/env bash
# publish.sh — Runs `cargo publish --dry-run` for each SDK crate in dependency order.
# Pass --publish to perform the actual publish instead of a dry-run.
#
# Usage:
#   ./scripts/publish.sh              # dry-run (default)
#   ./scripts/publish.sh --dry-run    # explicit dry-run
#   ./scripts/publish.sh --publish    # actual publish to crates.io

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Crates in dependency order (no inter-dependencies, but consistent ordering)
CRATES=(
  "system"
  "model"
  "assistant"
  "skill"
)

MODE="--dry-run"
if [[ "${1:-}" == "--publish" ]]; then
  MODE=""
  echo "=== Publishing SDK crates to crates.io ==="
else
  echo "=== Dry-run publish for SDK crates ==="
fi

FAILED=0

for crate in "${CRATES[@]}"; do
  MANIFEST="$SDK_DIR/$crate/Cargo.toml"
  if [[ ! -f "$MANIFEST" ]]; then
    echo "ERROR: $MANIFEST not found, skipping $crate"
    FAILED=1
    continue
  fi

  echo ""
  echo "--- $crate ---"
  if cargo publish $MODE --manifest-path "$MANIFEST"; then
    echo "  ✓ $crate passed"
  else
    echo "  ✗ $crate failed"
    FAILED=1
  fi
done

echo ""
if [[ $FAILED -ne 0 ]]; then
  echo "Some crates failed. See output above."
  exit 1
else
  echo "All crates passed."
fi
