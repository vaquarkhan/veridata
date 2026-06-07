# Install local git hooks (no git config changes).
$ErrorActionPreference = "Stop"
$root = Join-Path $PSScriptRoot ".."
$hooksSrc = Join-Path $root ".githooks"
$hooksDst = Join-Path $root ".git\hooks"

New-Item -ItemType Directory -Force -Path $hooksDst | Out-Null
foreach ($hook in @("commit-msg", "pre-commit")) {
    Copy-Item (Join-Path $hooksSrc $hook) (Join-Path $hooksDst $hook) -Force
    Write-Host "Installed $hook"
}
Write-Host "Done. Commits will reject Cursor/agent attribution."
