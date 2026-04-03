#!/usr/bin/env bash
# semver-check.sh — Runs cargo-semver-checks on each SDK crate locally.
# Detects breaking API changes before publishing.
#
# Usage:
#   ./scripts/semver-check.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# SDK crates to check
CRATES=(
  "system"
  "model"
  "assistant"
  "skill"
)

# Verify cargo-semver-checks is installed
if ! command -v cargo-semver-checks &>/dev/null; then
  echo "cargo-semver-checks is not installed."
  echo "Install it with: cargo install cargo-semver-checks --locked"
  exit 1
fi

echo "=== Running cargo-semver-checks on SDK crates ==="

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
  if cargo semver-checks check-release --manifest-path "$MANIFEST"; then
    echo "  ✓ $crate: no breaking changes detected"
  else
    echo "  ✗ $crate: breaking changes detected (see above)"
    FAILED=1
  fi
done

echo ""
if [[ $FAILED -ne 0 ]]; then
  echo "Breaking changes detected in one or more crates. See output above."
  echo "If intentional, bump the major version and add a migration guide."
  exit 1
else
  echo "All crates passed semver checks."
fi
