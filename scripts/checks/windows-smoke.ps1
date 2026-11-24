# The Windows build runs: its version, then a daemon in a throwaway home that answers the command line and
# opens its page to a sign-in link, the way the desktop app opens it. CI's windows job runs it after
# `cargo build -p tirith` (.github/workflows/tirith.yml).
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path 'target/debug/neobot.exe').Path
& $exe --version
if ($LASTEXITCODE) { throw "neobot --version ended with $LASTEXITCODE" }
$env:TIRITH_HOME = Join-Path $env:RUNNER_TEMP 'tirith-home'
$env:TIRITH_PORT = '7799'
$base = 'http://127.0.0.1:7799'
New-Item -ItemType Directory -Force $env:TIRITH_HOME | Out-Null
$log = Join-Path $env:RUNNER_TEMP 'tirith-serve'
$daemon = Start-Process -FilePath $exe -ArgumentList 'serve' -PassThru -NoNewWindow `
  -RedirectStandardOutput "$log.out" -RedirectStandardError "$log.err"
try {
  # Up when it answers at all: before a sign-in the page says 403.
  $up = $false
  foreach ($i in 1..120) {
    try { Invoke-WebRequest -UseBasicParsing "$base/" -TimeoutSec 2 -SkipHttpErrorCheck | Out-Null; $up = $true; break }
    catch { Start-Sleep -Milliseconds 500 }
  }
  if (-not $up) { throw 'the daemon never answered on 7799' }
  & $exe status
  if ($LASTEXITCODE) { throw "neobot status ended with $LASTEXITCODE" }
  $link = "$(& $exe open link | Select-Object -Last 1)".Trim()
  if ($link -notmatch '^http://127\.0\.0\.1:7799/auth/') { throw "no sign-in link: $link" }
  Invoke-WebRequest -UseBasicParsing $link -SessionVariable signedIn | Out-Null
  $page = Invoke-WebRequest -UseBasicParsing "$base/" -WebSession $signedIn
  if ($page.Content -notmatch '<title>NeoBot</title>') { throw 'the sign-in link did not open the page' }
  Write-Output 'the daemon answers the command line and opens its page to a sign-in link'
} finally {
  Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  Get-Content "$log.out", "$log.err" -ErrorAction SilentlyContinue | Select-Object -Last 40
}
