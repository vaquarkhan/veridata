# Generate line-coverage report for veridata-core + veridata-proof.
# Requires: cargo install cargo-llvm-cov; rustup component add llvm-tools-preview
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

if (-not (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
}

rustup component add llvm-tools-preview 2>$null

Write-Host "=== Running tests with coverage (core + proof) ==="
cargo llvm-cov `
  --package veridata-core `
  --package veridata-proof `
  --lib `
  --features veridata-core/test-util `
  --ignore-filename-regex testutil `
  --html `
  --output-dir target/llvm-cov/html `
  --summary-only

Write-Host ""
Write-Host "=== Enforcing 100% line coverage ==="
cargo llvm-cov `
  --package veridata-core `
  --package veridata-proof `
  --lib `
  --features veridata-core/test-util `
  --ignore-filename-regex testutil `
  --fail-under-lines 100 `
  --summary-only

Write-Host ""
Write-Host "HTML report: target/llvm-cov/html/index.html"
