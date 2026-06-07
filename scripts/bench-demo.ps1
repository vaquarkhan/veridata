# Wall-clock benchmark for demo pipeline
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
cargo build --release -p veridata-cli
$bin = ".\target\release\veridata.exe"
$demo = ".bench-demo"
if (Test-Path $demo) { Remove-Item -Recurse -Force $demo }

foreach ($step in @(
    @{ Name = "init"; Args = @("init", "--config", "$demo/recon.yaml", "--data-dir", $demo) },
    @{ Name = "reconcile"; Args = @("reconcile", "--config", "$demo/recon.yaml", "--demo") },
    @{ Name = "verify"; Args = @("verify", "--config", "$demo/recon.yaml") }
)) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & $bin @($step.Args) | Out-Null
    $sw.Stop()
    Write-Host "$($step.Name): $($sw.Elapsed.TotalSeconds.ToString('F3')) s"
}
