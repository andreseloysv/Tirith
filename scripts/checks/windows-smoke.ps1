# The Windows build runs: its version, then a daemon in a throwaway home that serves the page and answers
# the command line. CI's windows job runs it after `cargo build -p tirith` (.github/workflows/tirith.yml).
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path 'target/debug/tirith.exe').Path
& $exe --version
if ($LASTEXITCODE) { throw "tirith --version ended with $LASTEXITCODE" }
$env:TIRITH_HOME = Join-Path $env:RUNNER_TEMP 'tirith-home'
$env:TIRITH_PORT = '7799'
New-Item -ItemType Directory -Force $env:TIRITH_HOME | Out-Null
$log = Join-Path $env:RUNNER_TEMP 'tirith-serve'
$daemon = Start-Process -FilePath $exe -ArgumentList 'serve' -PassThru -NoNewWindow `
  -RedirectStandardOutput "$log.out" -RedirectStandardError "$log.err"
try {
  $page = $null
  foreach ($i in 1..60) {
    try { $page = Invoke-WebRequest -UseBasicParsing 'http://127.0.0.1:7799/' -TimeoutSec 2; break }
    catch { Start-Sleep -Milliseconds 500 }
  }
  if (-not $page -or $page.StatusCode -ne 200) { throw 'the daemon never served its page' }
  if ($page.Content -notmatch '<title>OpenBot</title>') { throw 'what it served is not the page' }
  & $exe status
  if ($LASTEXITCODE) { throw "tirith status ended with $LASTEXITCODE" }
  Write-Output 'the daemon serves its page and answers the command line'
} finally {
  Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  Get-Content "$log.out", "$log.err" -ErrorAction SilentlyContinue | Select-Object -Last 40
}
