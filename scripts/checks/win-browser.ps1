# Proves rust/tirith/assets/win-browser.ps1 (`neobot chrome|edge open|read|click|type`) against a real Chrome and a
# real Edge on a Windows desktop: each shows a page of ours from a throwaway profile, and the script is run the
# way the daemon runs it (its text read from a file, the words in environment variables). CI's win-browser job
# runs it; nothing here runs on the owner's own machines. This file starts with a byte order mark: Windows
# PowerShell reads a script without one in the system's old code page, and its words here are not all ASCII.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$env:NEOBOT_SCRIPT = Join-Path $root 'rust\tirith\assets\win-browser.ps1'
$temp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
$page = Join-Path $temp 'neobot-page.html'
Set-Content -Path $page -Encoding UTF8 -Value @'
<!doctype html><html lang="en"><head><meta charset="utf-8"><title>NeoBot test page</title></head><body>
<h1>Hello from the page</h1>
<p id="state">nothing yet</p>
<button onclick="document.getElementById('state').textContent='pressed the button'">Press me</button>
<label>Your name <input oninput="document.getElementById('echo').textContent='typed: '+this.value"></label>
<p id="echo"></p>
<input placeholder="Search here" oninput="document.getElementById('found').textContent='searched: '+this.value">
<p id="found"></p>
<label>Colour <select onchange="document.getElementById('picked').textContent='picked: '+this.value">
<option>Red</option><option>Green</option><option>Blue</option></select></label>
<p id="picked"></p>
<a href="#second" onclick="document.title='Second title'">Go to the second part</a>
<button disabled>Never</button>
<p>Ünïcödé: «¿qué tal?» 你好</p>
</body></html>
'@
$address = 'file:///' + ($page -replace '\\', '/')

# This machine's usual profiles have never run: no welcome page in front of the page that is opened.
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\Google\Chrome\User Data", "$env:LOCALAPPDATA\Microsoft\Edge\User Data" | Out-Null
Set-Content -Path "$env:LOCALAPPDATA\Google\Chrome\User Data\First Run", "$env:LOCALAPPDATA\Microsoft\Edge\User Data\First Run" -Value ''
New-Item -Force 'HKCU:\Software\Policies\Microsoft\Edge' | Out-Null
Set-ItemProperty 'HKCU:\Software\Policies\Microsoft\Edge' -Name HideFirstRunExperience -Value 1 -Type DWord

function Run($app, $cmd, $target, $text, $url) {
  $env:NEOBOT_APP = $app; $env:NEOBOT_CMD = $cmd; $env:NEOBOT_TARGET = $target; $env:NEOBOT_TEXT = $text; $env:NEOBOT_URL = $url
  $run = '& ([ScriptBlock]::Create([IO.File]::ReadAllText($env:NEOBOT_SCRIPT, [Text.Encoding]::UTF8)))'
  $file = Join-Path $temp 'neobot-out.txt'
  $done = Start-Process powershell.exe -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', $run) -Wait -PassThru -NoNewWindow `
    -RedirectStandardOutput $file -RedirectStandardError "$file.err"
  $out = [IO.File]::ReadAllText($file, [Text.Encoding]::UTF8)
  $err = [IO.File]::ReadAllText("$file.err")
  Write-Host "--- $app $cmd '$target' '$text' '$url' -> exit $($done.ExitCode)`n$out$err"
  if ($done.ExitCode -ne 0) { throw "$app ${cmd}: the script ended with $($done.ExitCode): $err" }
  $out
}
function Must($what, $out, $want) { if (-not $out.Contains($want)) { throw "$what should say «$want», said: $out" } }

# What UI Automation sees of a browser's windows, for the log when something is not found.
function Tree($app) {
  Add-Type -AssemblyName UIAutomationClient; Add-Type -AssemblyName UIAutomationTypes
  $ids = @(Get-Process -Name $app -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
  $all = [System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($win in $all) {
    if ($ids -notcontains $win.Current.ProcessId) { continue }
    Write-Host "window «$($win.Current.Name)» class $($win.Current.ClassName)"
    $kids = $win.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    $kids | Select-Object -First 150 | ForEach-Object { Write-Host "  $($_.Current.ControlType.ProgrammaticName) «$($_.Current.Name)» class $($_.Current.ClassName)" }
  }
}

function Prove($app) {
  $profile = Join-Path $temp "neobot-profile-$app"
  $flags = @("--user-data-dir=$profile", '--no-first-run', '--no-default-browser-check', '--disable-sync', '--new-window', $address)
  $browser = Start-Process $app -ArgumentList $flags -PassThru
  try {
    Start-Sleep -Seconds 6
    $read = Run $app 'read' '' '' ''
    Must 'read' $read $address
    Must 'read' $read 'NeoBot test page'
    Must 'read' $read 'Hello from the page'
    Must 'read' $read 'nothing yet'
    Must 'read' $read '«¿qué tal?» 你好'
    Must 'click' (Run $app 'click' 'press me' '' '') 'clicked'
    Must 'after the click' (Run $app 'read' '' '' '') 'pressed the button'
    Must 'type' (Run $app 'type' 'Your name' 'Ada Lovelace' '') 'typed into'
    Must 'after typing' (Run $app 'read' '' '' $address) 'typed: Ada Lovelace'
    Must 'type by placeholder' (Run $app 'type' 'search here' 'ñandú' '') 'typed into'
    Must 'after typing' (Run $app 'read' '' '' '') 'searched: ñandú'
    Must 'a list' (Run $app 'type' 'Colour' 'Green' '') 'chose «Green»'
    Must 'after choosing' (Run $app 'read' '' '' '') 'picked: Green'
    Must 'a list without that option' (Run $app 'type' 'Colour' 'Purple' '') 'no option «Purple»'
    Must 'after that' (Run $app 'read' '' '' '') 'picked: Green'
    Must 'a link' (Run $app 'click' 'Go to the second part' '' '') 'clicked'
    Must 'after the link' (Run $app 'read' '' '' '') 'Second title'
    Must 'a disabled button' (Run $app 'click' 'Never' '' '') 'ERROR not clicked, it is disabled'
    Must 'no such thing' (Run $app 'click' 'A button that is not there' '' '') 'not found'
    Must 'another address' (Run $app 'read' '' '' 'https://nowhere.example/') 'ERROR no page'
    # A page opened by its address goes to the person's usual profile, in front: it is read by that address.
    Must 'open' (Run $app 'open' '' '' 'https://example.com/') 'opened https://example.com/'
    Start-Sleep -Seconds 6
    Must 'the opened page' (Run $app 'read' '' '' 'https://example.com') 'Example Domain'
    Write-Host "${app}: read, click, type, a list, a link, a disabled button and a missing page all behave"
  } catch {
    Tree $app
    throw
  } finally {
    Get-Process -Name $app -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  }
}

$which = if ($args.Count) { $args } else { @('chrome', 'msedge') }
foreach ($app in $which) { Prove $app }
Must 'a closed browser' (Run 'chrome' 'read' '' '' '') 'ERROR Chrome is not open'
Write-Host 'the owner''s Chrome and Edge are read and driven through UI Automation'
