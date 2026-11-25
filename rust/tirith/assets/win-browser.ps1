# `neobot chrome|edge open|read|find|click|type|wait`: the owner's own Chrome or Edge on Windows, through Windows UI
# Automation. Nothing is installed in the browser and no setting is turned on: it is the browser the person
# already has open, with their own sessions. The daemon runs this fixed script out of sight; the agent's words
# arrive only as environment variables, never as code:
#   NEOBOT_CMD read|find|click|type|wait|open   NEOBOT_APP chrome|msedge   NEOBOT_URL   NEOBOT_TARGET   NEOBOT_TEXT
# The page is one a window shows (UI Automation sees no tab behind another): the one whose address starts with
# NEOBOT_URL, else the front window's. A click presses the element named with the words (exactly, else the
# shortest name holding them); typing focuses the field so named and sets its value, which the page sees as
# typing; in a list the option with those words is chosen. `find` lists what can be pressed and typed into, each
# with those words and its state (with words: whatever on the page holds them all), and `wait` answers once
# something with the words is there, 15 seconds at most. It never reads cookies, storage or passwords.
# An answer that starts with ERROR is the command's failure: whatever did nothing is one, so commands chained in
# one shell call stop where a step did not happen; a command that finds nothing names what is close to its words.
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
# the page and its address as they are now: the shown one whose address starts with $url, else the front window's
function Seen {
  foreach ($win in (Windows)) {
    $doc = $win.FindFirst($T::Descendants, (Prop $A::ControlTypeProperty $C::Document))
    $at = if ($doc) { Address $win $doc }
    if ($doc -and (-not $url -or "$at".StartsWith($url))) { return $doc, "$at" }
  }
}
# the same, asked for again while a page is on its way; without one the command ends
function Page {
  foreach ($try in 1..6) {
    $got = Seen
    if ($got) { return $got }
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
$PRESSED = @($C::Button, $C::Hyperlink, $C::CheckBox, $C::RadioButton, $C::MenuItem, $C::TabItem, $C::ComboBox, $C::SplitButton, $C::ListItem)
$TYPED = @($C::Edit, $C::ComboBox)
$OTHER = @($C::Text, $C::Group, $C::Image, $C::DataItem)
# one element as a line: its kind, the words a click or typing takes, what a field or list holds, and its state
function Kind($el) {
  $k = ("$($el.Current.ControlType.ProgrammaticName)" -replace '^ControlType\.', '').ToLowerInvariant()
  $as = @{ hyperlink = 'link'; edit = 'field'; combobox = 'list'; menuitem = 'option'; listitem = 'option'; tabitem = 'tab'; radiobutton = 'radio'; splitbutton = 'button' }
  if ($as.ContainsKey($k)) { $as[$k] } else { $k }
}
function Line($el) {
  $kind = Kind $el; $more = ''
  $name = ("$($el.Current.Name)" -replace '\s+', ' ').Trim()
  if ($name.Length -gt 160) { $name = $name.Substring(0, 160) }
  if ($kind -eq 'field' -or $kind -eq 'list') {
    $held = ("$(Value $el)" -replace '\s+', ' ').Trim()
    if ($held.Length -gt 80) { $held = $held.Substring(0, 80) }
    if ($held) { $more = " holds «$held»" }
  }
  if ($kind -eq 'checkbox' -or $kind -eq 'radio') {
    $way = Pattern $el ([System.Windows.Automation.TogglePattern])
    $chosen = Pattern $el ([System.Windows.Automation.SelectionItemPattern])
    if ($way) { $more = if ("$($way.Current.ToggleState)" -eq 'On') { ' on' } else { ' off' } }
    elseif ($chosen) { $more = if ($chosen.Current.IsSelected) { ' on' } else { ' off' } }
  }
  if (-not $el.Current.IsEnabled) { $more += ' (disabled)' }
  "$kind «$name»$more"
}
# the elements of those kinds that have a name, in the page's order
function Named($doc, $types) {
  @($doc.FindAll($T::Descendants, (Kinds $types)) | Select-Object -First 3000 | Where-Object { "$($_.Current.Name)".Trim() })
}
# the lines of the elements whose name holds every word of $want, each once
function Lines($all, $want) {
  $words = @((Norm $want) -split ' ' | Where-Object { $_ })
  $out = New-Object System.Collections.Generic.List[string]
  foreach ($el in $all) {
    $label = Norm $el.Current.Name; $miss = $false
    foreach ($w in $words) { if (-not $label.Contains($w)) { $miss = $true; break } }
    if ($miss) { continue }
    $line = Line $el
    if (-not $out.Contains($line)) { $out.Add($line) }
  }
  , $out
}
# for a command that found nothing: what shares a word with what it asked for, else the page's first controls
function Near($doc, $want, $types) {
  $all = Named $doc $types
  $close = New-Object System.Collections.Generic.List[string]
  foreach ($w in @((Norm $want) -split ' ' | Where-Object { $_.Length -gt 2 } | Select-Object -First 4)) {
    foreach ($line in (Lines $all $w)) { if (-not $close.Contains($line)) { $close.Add($line) } }
  }
  # a list a function gives back is one thing until it is named: named, it is its lines
  $first = Lines $all ''
  $some = if ($close.Count) { @($close) } else { @($first) }
  if (-not $some.Count) { return '' }
  $lead = if ($close.Count) { 'Close to it' } else { 'The page has' }
  ". ${lead}: $(@($some | Select-Object -First 8) -join ', ')"
}

if ($cmd -eq 'open') { Start-Process $app -ArgumentList $url; Say "opened $url" }
if ($cmd -eq 'wait') {
  # until something with the words is there: the page itself may be on its way, so it is looked for again too
  $until = (Get-Date).AddSeconds(15)
  do {
    $now = Seen
    $hit = if ($now) { Pick $now[0] $target ($PRESSED + $TYPED) }
    if ($now -and -not $hit) { $hit = Pick $now[0] $target $OTHER }
    if ($hit) { Say "there: $(Line $hit)" }
    Start-Sleep -Milliseconds 500
  } while ((Get-Date) -lt $until)
  $now = Seen
  $close = if ($now) { Near $now[0] $target ($PRESSED + $TYPED) } else { '' }
  Say "ERROR not there after 15 s: $target$close"
}
$doc, $at = Page
if ($cmd -eq 'find') {
  $acted = $PRESSED + $TYPED
  $got = @(Soon { $some = Lines (Named $doc $acted) $target; if ($some.Count) { $some } })
  if (-not $got.Count -and $target) { $rest = Lines (Named $doc $OTHER) $target; $got = @($rest) }
  if (-not $got.Count -and $target) { Say "ERROR nothing with «$target» on this page$(Near $doc $target $acted)" }
  if (-not $got.Count) { Say 'ERROR this page shows nothing to press or type into' }
  $shown = @($got | Select-Object -First 60) -join "`n"
  if ($got.Count -gt 60) { $shown += "`n… and $($got.Count - 60) more: give words to see fewer" }
  Say $shown
}
if ($cmd -eq 'read') { Say "$at`n$($doc.Current.Name)`n$(Soon { Words $doc })" }
if ($cmd -eq 'click') {
  $hit = Soon { Pick $doc $target $PRESSED }
  if (-not $hit) { Say "ERROR not found: $target$(Near $doc $target $PRESSED)" }
  if (-not $hit.Current.IsEnabled) { Say "ERROR not clicked, it is disabled: $(Shown $hit)" }
  if (Press $hit) { Say "clicked $(Shown $hit)" } else { Say "ERROR could not press $(Shown $hit)" }
}
if ($cmd -eq 'type') {
  $hit = Soon { Pick $doc $target $TYPED }
  if (-not $hit) { Say "ERROR not found: $target$(Near $doc $target $TYPED)" }
  $list = if ($hit.Current.ControlType -eq $C::ComboBox) { Pattern $hit ([System.Windows.Automation.ExpandCollapsePattern]) }
  if ($list) {
    # a list shows its options only while it is open: opened (it may be already), the option pressed, and shut again
    $shut = { try { if ("$($list.Current.ExpandCollapseState)" -ne 'Collapsed') { $list.Collapse() } } catch { } }
    try { if ("$($list.Current.ExpandCollapseState)" -ne 'Expanded') { $list.Expand() } } catch { }
    $option = Soon { Pick $hit $text @($C::ListItem) }
    if (-not $option) { & $shut; Say "ERROR no option «$text» in $(Shown $hit)" }
    $chosen = "$($option.Current.Name)"
    if (-not (Press $option)) { & $shut; Say "ERROR could not choose «$text» in $(Shown $hit)" }
    Start-Sleep -Milliseconds 300
    & $shut
    Say "chose «$chosen» in $(Shown $hit); it now says «$(Value $hit)»"
  }
  $field = Pattern $hit ([System.Windows.Automation.ValuePattern])
  if (-not $field -or $field.Current.IsReadOnly) { Say "ERROR could not type into $(Shown $hit)" }
  try { $hit.SetFocus() } catch { }
  $field.SetValue($text)
  Start-Sleep -Milliseconds 300
  Say "typed into $(Shown $hit); it now says «$(Value $hit)»"
}
Say "ERROR neobot $app open <url> | read [<url>] | find [<words>] [<url>] | click <target> [<url>] | type <target> <text> [<url>] | wait <words> [<url>]"
