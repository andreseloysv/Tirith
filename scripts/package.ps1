# Release build of the desktop app on Windows; prints the bundle path(s).
# Unsigned unless the signing settings in app/README.md are set. Extra args go to `tauri build`.
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location (Join-Path $root "app")

if (-not (Test-Path "node_modules")) { npm install; if ($LASTEXITCODE) { exit $LASTEXITCODE } }

# The daemon the bundle carries (bundle.externalBin): tauri.windows.conf.json runs no sidecar.sh here.
$triple = ((rustc -vV | Select-String '^host: ').Line -replace '^host: ', '')
cargo build --release --locked -p tirith --target $triple
if ($LASTEXITCODE) { exit $LASTEXITCODE }
New-Item -ItemType Directory -Force "src-tauri\binaries" | Out-Null
Copy-Item (Join-Path $root "target\$triple\release\neobot.exe") "src-tauri\binaries\neobot-$triple.exe" -Force
npm run tauri build -- @args
if ($LASTEXITCODE) { exit $LASTEXITCODE }

$bundle = Join-Path $root "target\release\bundle"
if (-not (Test-Path $bundle)) { Write-Error "no bundle directory at $bundle"; exit 1 }
Write-Output "bundles:"
Get-ChildItem -Path (Join-Path $bundle "msi"), (Join-Path $bundle "nsis") -Include *.msi, *.exe -ErrorAction SilentlyContinue |
  ForEach-Object { $_.FullName }
