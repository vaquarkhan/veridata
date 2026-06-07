# Run all mandatory CI gates locally.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

Write-Host "== build =="
cargo build --workspace

Write-Host "== unit + property tests =="
cargo test --workspace
cargo test -p veridata-core --features test-util --test proptest_recon

Write-Host "== P0 conformance =="
python conformance/validate_p0.py

Write-Host "== determinism gate =="
cargo test -p veridata-proof ac_c1_2_deterministic_except_created_at

Write-Host "== architecture gate =="
cargo test -p veridata-proof --test architecture

Write-Host "== E2E =="
cargo test -p veridata-e2e

Write-Host "== coverage gate =="
powershell -File scripts/run-coverage.ps1

Write-Host ""
Write-Host "All gates passed."
