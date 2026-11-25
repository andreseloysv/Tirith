// `neobot safari click|type|find|wait` through macOS Accessibility, so nobody turns on a Safari setting: the daemon
// runs osascript -l JavaScript -e <safari-ax-find.js + safari-ax-look.js + this> <command> <target> <text> <url>,
// the agent's words only as arguments. The page is the shown one whose URL starts with <url> (the daemon brought that tab forward), else
// the front window's. A click presses the element labelled with the words (exactly, else the shortest label holding
// them); typing focuses the field so named and sets its value, which the page sees as typing; a list opens and its
// option is pressed. What is not there yet is looked for again for two seconds (a page still drawing itself), and
// a command that finds nothing names what is close. The first time, macOS asks the person to let NeoBot use
// Accessibility. An answer that starts with ERROR is the command's failure (the daemon makes it one): whatever
// did nothing is one, so commands chained in one shell call stop where a step did not happen and never press Send
// after a typing that found no field.
const press = (el) => $.AXUIElementPerformAction(el, $('AXPress'));
const shown = (hit) => `${hit.role} «${said(hit.el).slice(0, 80)}»`;
function click(web, target) {
  const look = () => pick(web, target, CLICKS, PRESSED) || pick(web, target, OTHERS, ['AXAnyTypeSearchKey']);
  let hit = soon(look, 2), close = '';
  if (!hit) { close = near(web, target, PRESSED); hit = look(); } /* it may have come while that was read */
  if (!hit) return 'ERROR not found: ' + target + close;
  if (hit.role === 'AXStaticText') hit = { el: get(hit.el, 'AXParent') || hit.el, role: 'AXGroup' };
  /* a disabled button takes the press and does nothing: say so, never «clicked» */
  const enabled = get(hit.el, 'AXEnabled');
  if (enabled && ObjC.unwrap(enabled) === false) return 'ERROR not clicked, it is disabled: ' + shown(hit);
  return press(hit.el) === 0 ? 'clicked ' + shown(hit) : 'ERROR could not press ' + shown(hit);
}
function type(web, target, text) {
  const look = () => pick(web, target, FIELDS, TYPED, true) || pick(web, target, FIELDS, ['AXControlSearchKey'], true);
  let hit = soon(look, 2), close = '';
  if (!hit) { close = near(web, target, TYPED); hit = look(); }
  if (!hit) return 'ERROR not found: ' + target + close;
  if (hit.role === 'AXPopUpButton') {
    press(hit.el);
    delay(0.4);
    const option = best(below(hit.el, 600), text, ['AXMenuItem']);
    if (!option) { press(hit.el); return `ERROR no option «${text}» in ${shown(hit)}`; }
    press(option.el);
    return `chose «${said(option.el)}» in ${shown(hit)}`;
  }
  /* focus first (an NSNumber: $.kCFBooleanTrue is a function here), then the value: WebKit then sends the page
     beforeinput/input as for typing, so editors like Teams' see the words; without focus nothing changes */
  $.AXUIElementSetAttributeValue(hit.el, $('AXFocused'), $.NSNumber.numberWithBool(true));
  delay(0.2);
  if ($.AXUIElementSetAttributeValue(hit.el, $('AXValue'), $(text)) !== 0) return 'ERROR could not type into ' + shown(hit);
  delay(0.3);
  return `typed into ${shown(hit)}; it now says «${str(hit.el, 'AXValue').slice(0, 200)}»`;
}
/* until something with the words is there, at most fifteen seconds: the page may be another one by then */
function wait(page, words) {
  const look = (web) => web && (pick(web, words, CLICKS.concat(FIELDS), ACTED) || pick(web, words, OTHERS, ['AXAnyTypeSearchKey']));
  const hit = soon(() => look(page()), 15);
  if (hit) return 'there: ' + line(hit);
  const web = page();
  return `ERROR not there after 15 s: ${words}` + (web ? near(web, words, ACTED) : '');
}
function run(argv) {
  const [cmd, target, text, url, pid] = argv; /* pid: a check's own WebKit window; the daemon never gives one */
  if (!$.AXIsProcessTrustedWithOptions($({ AXTrustedCheckOptionPrompt: true }))) {
    return 'ERROR NeoBot may not use Accessibility yet: System Settings › Privacy & Security › Accessibility › turn NeoBot on';
  }
  const apps = $.NSRunningApplication.runningApplicationsWithBundleIdentifier('com.apple.Safari');
  if (!pid && !apps.count) return 'ERROR Safari is not open';
  const safari = $.AXUIElementCreateApplication(pid ? Number(pid) : apps.objectAtIndex(0).processIdentifier);
  if (cmd === 'wait') return wait(() => pageOf(safari, url || ''), target);
  let web = pageOf(safari, url || '');
  /* WebKit builds its page tree on the first Accessibility call it gets: once more after a second */
  if (!web || !kids(web).length) { delay(1); web = pageOf(safari, url || ''); }
  if (!web) return url ? 'ERROR no shown page starts with ' + url : 'ERROR no page in Safari\'s front window';
  if (cmd === 'find') return find(web, target || '');
  return cmd === 'click' ? click(web, target) : type(web, target, text || '');
}
