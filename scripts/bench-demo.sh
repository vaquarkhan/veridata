#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -p veridata-cli
BIN="./target/release/veridata"
DEMO=".bench-demo"
rm -rf "$DEMO"
/usr/bin/time -f "init: %e s" $BIN init --config "$DEMO/recon.yaml" --data-dir "$DEMO" 2>&1 | tail -1
/usr/bin/time -f "reconcile: %e s" $BIN reconcile --config "$DEMO/recon.yaml" --demo 2>&1 | tail -1
/usr/bin/time -f "verify: %e s" $BIN verify --config "$DEMO/recon.yaml" 2>&1 | tail -1
