# Installs OpenBot (the tirith command) on Windows 10 or 11 and starts its guide, which connects the agents.
#   irm https://github.com/andreseloysv/Tirith/releases/latest/download/install.ps1 | iex
#   & ([scriptblock]::Create((irm <that address>))) -NoSetup        (with options)
# -From <folder with a release>, -NoSetup; anything else goes to `tirith setup`. TIRITH_BASE_URL: another place to
# download. The files come only if SHA256SUMS is signed with OpenBot's release key and names their sum.
# OpenBot needs Git for Windows: lanes are git worktrees, and stages run in its sh.
param([string]$From = '', [switch]$NoSetup, [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest = @())
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue' # the progress bar makes downloads slow in Windows PowerShell
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$Key = @'
-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEEMqh12EKmR6Eeh0WfAy2RUc8O2XK
0vydFEXZCLAwXgH8fRKBIyFIiZF8f2ZchMGUb52KgAT+Tz/T4NZXyF9Tbg==
-----END PUBLIC KEY-----
'@
$Base = if ($env:TIRITH_BASE_URL) { $env:TIRITH_BASE_URL } else { 'https://github.com/andreseloysv/Tirith/releases/latest/download' }
$Zip = 'tirith-x86_64-pc-windows-msvc.zip' # x64; Windows on Arm runs it emulated

function Fetch($name) {
  $to = Join-Path $tmp $name
  if ($From) { Copy-Item (Join-Path $From $name) $to } else { Invoke-WebRequest -UseBasicParsing -Uri "$Base/$name" -OutFile $to }
  $to
}

# An ECDSA signature as OpenSSL writes it (DER: SEQUENCE of r and s) in the form .NET reads (r and s, 32 bytes each).
function Plain([byte[]]$der) {
  $out = New-Object byte[] 64
  $at = 2
  foreach ($half in 0, 1) {
    $len = $der[$at + 1]
    $int = [byte[]]$der[($at + 2)..($at + 1 + $len)]
    $int = [byte[]]($int | Select-Object -Last ([Math]::Min(32, $len))) # without the sign's zero
    [Array]::Copy($int, 0, $out, 32 * $half + 32 - $int.Length, $int.Length)
    $at += 2 + $len
  }
  $out
}

# Whether `sig` is the release key's signature of `file` (P-256, SHA-256).
function Signed($file, $sig) {
  $spki = [Convert]::FromBase64String((($Key -split "`r?`n") | Where-Object { $_ -and $_ -notmatch '-----' }) -join '')
  $point = New-Object System.Security.Cryptography.ECPoint
  $point.X = [byte[]]$spki[($spki.Length - 64)..($spki.Length - 33)]
  $point.Y = [byte[]]$spki[($spki.Length - 32)..($spki.Length - 1)]
  $params = New-Object System.Security.Cryptography.ECParameters
  $params.Curve = [System.Security.Cryptography.ECCurve+NamedCurves]::nistP256
  $params.Q = $point
  $ecdsa = [System.Security.Cryptography.ECDsa]::Create($params)
  $sha256 = [System.Security.Cryptography.HashAlgorithmName]::SHA256
  $ecdsa.VerifyData([IO.File]::ReadAllBytes($file), (Plain ([IO.File]::ReadAllBytes($sig))), $sha256)
}

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("tirith-install." + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $tmp | Out-Null
try {
  $zipFile = Fetch $Zip
  $sums = Fetch 'SHA256SUMS'
  if (-not (Signed $sums (Fetch 'SHA256SUMS.sig'))) { throw "install: SHA256SUMS is not signed with OpenBot's release key: nothing was installed" }
  $line = Get-Content $sums | Where-Object { $_.EndsWith("  $Zip") } | Select-Object -First 1
  $want = if ($line) { $line.Split(' ')[0] } else { '' }
  $got = (Get-FileHash -Algorithm SHA256 $zipFile).Hash.ToLower()
  if (-not $want -or $want -ne $got) { throw "install: $Zip does not match its signed sum: nothing was installed" }
  Write-Host 'Signature and sum are right.'
  Expand-Archive -Path $zipFile -DestinationPath (Join-Path $tmp 'x')
  $bin = Join-Path $HOME '.local\bin'
  New-Item -ItemType Directory -Force $bin | Out-Null
  $exe = Join-Path $bin 'tirith.exe'
  # A running tirith.exe cannot be replaced, but it can be renamed.
  if (Test-Path $exe) { Move-Item -Force $exe "$exe.old" }
  Copy-Item (Join-Path $tmp 'x\tirith.exe') $exe
  Remove-Item -Force "$exe.old" -ErrorAction SilentlyContinue
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

$user = '' + [Environment]::GetEnvironmentVariable('Path', 'User')
if (($user -split ';') -notcontains $bin) {
  [Environment]::SetEnvironmentVariable('Path', ($user.TrimEnd(';') + ";$bin").TrimStart(';'), 'User')
  $env:Path = "$env:Path;$bin"
  Write-Host "Added $bin to your PATH (new terminals see it)."
}
Write-Host "Installed $(& $exe --version) in $bin"
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
  Write-Host 'OpenBot needs Git for Windows: winget install --id Git.Git -e   (then open a new terminal)' -ForegroundColor Yellow
}
if ($NoSetup) { Write-Host "Next: tirith setup"; return }
& $exe setup @Rest
