# End-to-end demo: init -> reconcile -> verify -> report
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

Write-Host "== build CLI =="
cargo build -p veridata-cli

$DemoDir = if ($args.Count -gt 0) { $args[0] } else { ".veridata-demo" }
$Config = Join-Path $DemoDir "recon.yaml"
if (Test-Path $DemoDir) { Remove-Item -Recurse -Force $DemoDir }

Write-Host "== init =="
cargo run -p veridata-cli -- init --config $Config --data-dir $DemoDir

Write-Host "== reconcile (demo data) =="
cargo run -p veridata-cli -- reconcile --config $Config --demo

Write-Host "== verify =="
cargo run -p veridata-cli -- verify --config $Config

Write-Host "== report =="
cargo run -p veridata-cli -- report --config $Config

Write-Host ""
Write-Host "Demo complete. Proof store: $DemoDir\proofs\"
