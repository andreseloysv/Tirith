// `neobot safari click|type` through macOS Accessibility, so nobody turns on a Safari setting: the daemon runs
// osascript -l JavaScript -e <safari-ax-find.js + this> <click|type> <target> <text> <url>, the agent's words only
// as arguments. The page is the shown one whose URL starts with <url> (the daemon brought that tab forward), else
// the front window's. A click presses the element labelled with the words (exactly, else the shortest label holding
// them); typing focuses the field so named and sets its value, which the page sees as typing; a list opens and its
// option is pressed. The first time, macOS asks the person to let NeoBot use Accessibility.
const press = (el) => $.AXUIElementPerformAction(el, $('AXPress'));
const shown = (hit) => `${hit.role} «${said(hit.el).slice(0, 80)}»`;
function click(web, target) {
  let hit = pick(web, target, CLICKS, PRESSED) || pick(web, target, OTHERS, ['AXAnyTypeSearchKey']);
  if (!hit) return 'not found: ' + target;
  if (hit.role === 'AXStaticText') hit = { el: get(hit.el, 'AXParent') || hit.el, role: 'AXGroup' };
  /* a disabled button takes the press and does nothing: say so, never «clicked» */
  const enabled = get(hit.el, 'AXEnabled');
  if (enabled && ObjC.unwrap(enabled) === false) return 'ERROR not clicked, it is disabled: ' + shown(hit);
  return press(hit.el) === 0 ? 'clicked ' + shown(hit) : 'could not press ' + shown(hit);
}
function type(web, target, text) {
  const hit = pick(web, target, FIELDS, TYPED, true) || pick(web, target, FIELDS, ['AXControlSearchKey'], true);
  if (!hit) return 'not found: ' + target;
  if (hit.role === 'AXPopUpButton') {
    press(hit.el);
    delay(0.4);
    const option = best(below(hit.el, 600), text, ['AXMenuItem']);
    if (!option) { press(hit.el); return `no option «${text}» in ${shown(hit)}`; }
    press(option.el);
    return `chose «${said(option.el)}» in ${shown(hit)}`;
  }
  /* focus first (an NSNumber: $.kCFBooleanTrue is a function here), then the value: WebKit then sends the page
     beforeinput/input as for typing, so editors like Teams' see the words; without focus nothing changes */
  $.AXUIElementSetAttributeValue(hit.el, $('AXFocused'), $.NSNumber.numberWithBool(true));
  delay(0.2);
  if ($.AXUIElementSetAttributeValue(hit.el, $('AXValue'), $(text)) !== 0) return 'could not type into ' + shown(hit);
  delay(0.3);
  return `typed into ${shown(hit)}; it now says «${str(hit.el, 'AXValue').slice(0, 200)}»`;
}
function run(argv) {
  const [cmd, target, text, url, pid] = argv; /* pid: a check's own WebKit window; the daemon never gives one */
  if (!$.AXIsProcessTrustedWithOptions($({ AXTrustedCheckOptionPrompt: true }))) {
    return 'ERROR NeoBot may not use Accessibility yet: System Settings › Privacy & Security › Accessibility › turn NeoBot on';
  }
  const apps = $.NSRunningApplication.runningApplicationsWithBundleIdentifier('com.apple.Safari');
  if (!pid && !apps.count) return 'ERROR Safari is not open';
  const safari = $.AXUIElementCreateApplication(pid ? Number(pid) : apps.objectAtIndex(0).processIdentifier);
  let web = pageOf(safari, url || '');
  /* WebKit builds its page tree on the first Accessibility call it gets: once more after a second */
  if (!web || !kids(web).length) { delay(1); web = pageOf(safari, url || ''); }
  if (!web) return url ? 'ERROR no shown page starts with ' + url : 'ERROR no page in Safari\'s front window';
  return cmd === 'click' ? click(web, target) : type(web, target, text || '');
}
