#!/usr/bin/env bash
# End-to-end demo: init -> reconcile -> verify -> report (< 30 min first proof)
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== build CLI =="
cargo build -p veridata-cli

DEMO_DIR="${1:-.veridata-demo}"
CONFIG="${DEMO_DIR%/}/recon.yaml"
rm -rf "$DEMO_DIR"

echo "== init =="
cargo run -p veridata-cli -- init --config "$CONFIG" --data-dir "$DEMO_DIR"

echo "== reconcile (demo data) =="
cargo run -p veridata-cli -- reconcile --config "$CONFIG" --demo

echo "== verify =="
cargo run -p veridata-cli -- verify --config "$CONFIG"

echo "== report =="
cargo run -p veridata-cli -- report --config "$CONFIG"

echo ""
echo "Demo complete. Proof store: $DEMO_DIR/proofs/"
