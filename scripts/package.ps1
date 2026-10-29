# Release build of the desktop app on Windows; prints the bundle path(s).
# Unsigned unless the signing settings in app/README.md are set. Extra args go to `tauri build`.
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location (Join-Path $root "app")

if (-not (Test-Path "node_modules")) { npm install; if ($LASTEXITCODE) { exit $LASTEXITCODE } }
npm run tauri build -- @args
if ($LASTEXITCODE) { exit $LASTEXITCODE }

$bundle = Join-Path $root "app\src-tauri\target\release\bundle"
if (-not (Test-Path $bundle)) { Write-Error "no bundle directory at $bundle"; exit 1 }
Write-Output "bundles:"
Get-ChildItem -Path (Join-Path $bundle "msi"), (Join-Path $bundle "nsis") -Include *.msi, *.exe -ErrorAction SilentlyContinue |
  ForEach-Object { $_.FullName }
