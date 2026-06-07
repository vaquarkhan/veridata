#!/usr/bin/env bash
# Generate line-coverage report for veridata-core + veridata-proof.
# Requires: cargo install cargo-llvm-cov && rustup component add llvm-tools-preview
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "Installing cargo-llvm-cov..."
  cargo install cargo-llvm-cov
fi

rustup component add llvm-tools-preview 2>/dev/null || true

echo "=== Running tests with coverage (core + proof) ==="
cargo llvm-cov \
  --package veridata-core \
  --package veridata-proof \
  --lib \
  --features veridata-core/test-util \
  --ignore-filename-regex 'testutil' \
  --html \
  --output-dir target/llvm-cov/html \
  --summary-only

echo ""
echo "=== Enforcing 100% line coverage ==="
cargo llvm-cov \
  --package veridata-core \
  --package veridata-proof \
  --lib \
  --features veridata-core/test-util \
  --ignore-filename-regex 'testutil' \
  --fail-under-lines 100 \
  --summary-only

echo ""
echo "HTML report: target/llvm-cov/html/index.html"
