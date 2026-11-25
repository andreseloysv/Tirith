# `neobot chrome|edge open|read|click|type`: the owner's own Chrome or Edge on Windows, through Windows UI
# Automation. Nothing is installed in the browser and no setting is turned on: it is the browser the person
# already has open, with their own sessions. The daemon runs this fixed script out of sight; the agent's words
# arrive only as environment variables, never as code:
#   NEOBOT_CMD read|click|type|open   NEOBOT_APP chrome|msedge   NEOBOT_URL   NEOBOT_TARGET   NEOBOT_TEXT
# The page is one a window shows (UI Automation sees no tab behind another): the one whose address starts with
# NEOBOT_URL, else the front window's. A click presses the element named with the words (exactly, else the
# shortest name holding them); typing focuses the field so named and sets its value, which the page sees as
# typing; in a list the option with those words is chosen. It never reads cookies, storage or passwords.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$A = [System.Windows.Automation.AutomationElement]
$T = [System.Windows.Automation.TreeScope]
$C = [System.Windows.Automation.ControlType]
$cmd = "$env:NEOBOT_CMD"; $app = "$env:NEOBOT_APP"; $url = "$env:NEOBOT_URL"; $target = "$env:NEOBOT_TARGET"; $text = "$env:NEOBOT_TEXT"
$name = @{ chrome = 'Chrome'; msedge = 'Edge' }[$app]
$utf8 = New-Object System.IO.StreamWriter([Console]::OpenStandardOutput(), (New-Object System.Text.UTF8Encoding($false)))
function Say($words) { $utf8.Write($words); $utf8.Flush(); exit 0 }
function Prop($p, $v) { New-Object System.Windows.Automation.PropertyCondition($p, $v) }
function Kinds($types) {
  $conds = [System.Windows.Automation.Condition[]]@($types | ForEach-Object { Prop $A::ControlTypeProperty $_ })
  if ($conds.Length -eq 1) { return $conds[0] }
  New-Object System.Windows.Automation.OrCondition -ArgumentList (, $conds)
}
function Pattern($el, $kind) { $got = $null; if ($el.TryGetCurrentPattern($kind::Pattern, [ref]$got)) { $got } }
function Value($el) { $v = Pattern $el ([System.Windows.Automation.ValuePattern]); if ($v) { "$($v.Current.Value)" } else { '' } }
function Norm($s) { ("$s" -replace '\s+', ' ').Trim().ToLowerInvariant() }
function Shown($el) { "$($el.Current.LocalizedControlType) «$(("$($el.Current.Name)" -replace '\s+', ' ').Trim())»" }

# the browser's windows, the front one first (other programs made with Chromium share the window class)
function Windows {
  $ids = @(Get-Process -Name $app -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
  if (-not $ids) { Say "ERROR $name is not open" }
  $all = $A::RootElement.FindAll($T::Children, (Prop $A::ClassNameProperty 'Chrome_WidgetWin_1'))
  @($all | Where-Object { $ids -contains $_.Current.ProcessId -and $_.Current.Name })
}
# a page's address: the document's own value, else what the window's address bar shows
function Address($win, $doc) {
  $at = Value $doc
  if ($at) { return $at }
  $bar = $win.FindFirst($T::Descendants, (Prop $A::ControlTypeProperty $C::Edit))
  if ($bar) { Value $bar } else { '' }
}
# the page and its address: the shown one whose address starts with $url, else the front window's
function Page {
  foreach ($try in 1..6) {
    foreach ($win in (Windows)) {
      $doc = $win.FindFirst($T::Descendants, (Prop $A::ControlTypeProperty $C::Document))
      $at = if ($doc) { Address $win $doc }
      if ($doc -and (-not $url -or "$at".StartsWith($url))) { return $doc, "$at" }
    }
    Start-Sleep -Milliseconds 700
  }
  if ($url) { Say "ERROR no page $name shows starts with $url (only the tab in front of each window can be seen): open it first" }
  Say "ERROR no page in $name's front window"
}
# Chromium builds a page's tree at the first UI Automation call it gets: what is not there yet is asked for again
function Soon($look) { foreach ($try in 1..5) { $got = & $look; if ($got) { return $got }; Start-Sleep -Milliseconds 700 } }
function Words($doc) {
  $all = Pattern $doc ([System.Windows.Automation.TextPattern])
  if ($all) { $said = $all.DocumentRange.GetText(20000); if ("$said".Trim()) { return $said } }
  $names = @($doc.FindAll($T::Descendants, (Prop $A::ControlTypeProperty $C::Text)) | Select-Object -First 3000 | ForEach-Object { $_.Current.Name })
  (($names | Where-Object { $_ }) -join "`n").Trim()
}
function Pick($doc, $want, $types) {
  $w = Norm $want; $best = $null; $size = 0
  foreach ($el in $doc.FindAll($T::Descendants, (Kinds $types))) {
    $label = Norm $el.Current.Name
    if ($label -eq $w) { return $el }
    if ($label.Contains($w) -and (-not $best -or $label.Length -lt $size)) { $best = $el; $size = $label.Length }
  }
  $best
}
function Press($el) {
  $way = Pattern $el ([System.Windows.Automation.InvokePattern]); if ($way) { $way.Invoke(); return $true }
  $way = Pattern $el ([System.Windows.Automation.TogglePattern]); if ($way) { $way.Toggle(); return $true }
  $way = Pattern $el ([System.Windows.Automation.SelectionItemPattern]); if ($way) { $way.Select(); return $true }
  $way = Pattern $el ([System.Windows.Automation.ExpandCollapsePattern]); if ($way) { $way.Expand(); return $true }
  $false
}

if ($cmd -eq 'open') { Start-Process $app -ArgumentList $url; Say "opened $url" }
$doc, $at = Page
if ($cmd -eq 'read') { Say "$at`n$($doc.Current.Name)`n$(Soon { Words $doc })" }
if ($cmd -eq 'click') {
  $hit = Soon { Pick $doc $target @($C::Button, $C::Hyperlink, $C::CheckBox, $C::RadioButton, $C::MenuItem, $C::TabItem, $C::ComboBox, $C::SplitButton, $C::ListItem) }
  if (-not $hit) { Say "not found: $target" }
  if (-not $hit.Current.IsEnabled) { Say "ERROR not clicked, it is disabled: $(Shown $hit)" }
  if (Press $hit) { Say "clicked $(Shown $hit)" } else { Say "could not press $(Shown $hit)" }
}
if ($cmd -eq 'type') {
  $hit = Soon { Pick $doc $target @($C::Edit, $C::ComboBox) }
  if (-not $hit) { Say "not found: $target" }
  $list = if ($hit.Current.ControlType -eq $C::ComboBox) { Pattern $hit ([System.Windows.Automation.ExpandCollapsePattern]) }
  if ($list) {
    # a list shows its options only while it is open: opened (it may be already), the option pressed, and shut again
    $shut = { try { if ("$($list.Current.ExpandCollapseState)" -ne 'Collapsed') { $list.Collapse() } } catch { } }
    try { if ("$($list.Current.ExpandCollapseState)" -ne 'Expanded') { $list.Expand() } } catch { }
    $option = Soon { Pick $hit $text @($C::ListItem) }
    if (-not $option) { & $shut; Say "no option «$text» in $(Shown $hit)" }
    $chosen = "$($option.Current.Name)"
    if (-not (Press $option)) { & $shut; Say "could not choose «$text» in $(Shown $hit)" }
    Start-Sleep -Milliseconds 300
    & $shut
    Say "chose «$chosen» in $(Shown $hit); it now says «$(Value $hit)»"
  }
  $field = Pattern $hit ([System.Windows.Automation.ValuePattern])
  if (-not $field -or $field.Current.IsReadOnly) { Say "could not type into $(Shown $hit)" }
  try { $hit.SetFocus() } catch { }
  $field.SetValue($text)
  Start-Sleep -Milliseconds 300
  Say "typed into $(Shown $hit); it now says «$(Value $hit)»"
}
Say "ERROR neobot $app open <url> | read [<url>] | click <target> [<url>] | type <target> <text> [<url>]"
