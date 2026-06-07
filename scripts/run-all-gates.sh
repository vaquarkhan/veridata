#!/usr/bin/env bash
# Run all mandatory CI gates locally.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== build =="
cargo build --workspace

echo "== unit + property tests =="
cargo test --workspace
cargo test -p veridata-core --features test-util --test proptest_recon

echo "== P0 conformance =="
python conformance/validate_p0.py

echo "== determinism gate =="
cargo test -p veridata-proof ac_c1_2_deterministic_except_created_at

echo "== architecture gate =="
cargo test -p veridata-proof --test architecture

echo "== E2E =="
cargo test -p veridata-e2e

echo "== coverage gate =="
./scripts/run-coverage.sh

echo ""
echo "All gates passed."
