// `tirith safari click|type` through macOS Accessibility, so nobody turns on a Safari setting: the daemon runs
// osascript -l JavaScript -e <this> <click|type> <target> <text> <url>, the agent's words only as arguments.
// The page is the shown one whose URL starts with <url> (the daemon brought that tab forward), else the front
// window's. A click presses the element labelled with the words (exactly, else the shortest label holding them);
// typing focuses the field so named and sets its value, which the page sees as typing; a list opens and its
// option is pressed. The first time, macOS asks the person to let OpenBot use Accessibility.
ObjC.import('ApplicationServices');
ObjC.import('AppKit');
ObjC.bindFunction('AXUIElementCreateApplication', ['id', ['int']]);
ObjC.bindFunction('AXUIElementCopyAttributeValue', ['int', ['id', 'id', 'id*']]);
ObjC.bindFunction('AXUIElementSetAttributeValue', ['int', ['id', 'id', 'id']]);
ObjC.bindFunction('AXUIElementPerformAction', ['int', ['id', 'id']]);
ObjC.bindFunction('AXIsProcessTrustedWithOptions', ['bool', ['id']]);
const CLICKS = ['AXButton', 'AXLink', 'AXCheckBox', 'AXRadioButton', 'AXMenuItem', 'AXPopUpButton', 'AXMenuButton',
  'AXTab', 'AXDisclosureTriangle', 'AXComboBox'];
const FIELDS = ['AXTextField', 'AXTextArea', 'AXComboBox', 'AXSearchField', 'AXPopUpButton'];
const norm = (s) => String(s || '').replace(/\s+/g, ' ').trim().toLowerCase();
function get(el, name) {
  const ref = Ref();
  return $.AXUIElementCopyAttributeValue(el, $(name), ref) === 0 ? ref[0] : null;
}
const str = (el, name) => { const v = get(el, name); try { return v && typeof v.js === 'string' ? v.js : ''; } catch (e) { return ''; } };
function kids(el) {
  const v = get(el, 'AXChildren'), out = [];
  for (let i = 0; v && i < v.count; i += 1) out.push(v.objectAtIndex(i));
  return out;
}
/* what a person reads on an element: its title, description, value or placeholder, else the text inside it */
function said(el, depth = 0) {
  const own = ['AXTitle', 'AXDescription', 'AXValue', 'AXPlaceholderValue'].map((a) => str(el, a)).filter(Boolean).join(' ');
  return own || depth > 2 ? own : kids(el).map((k) => said(k, depth + 1)).filter(Boolean).join(' ');
}
function below(root, limit) {
  const all = [], queue = [root];
  while (queue.length && all.length < limit) { const el = queue.shift(); all.push({ el, role: str(el, 'AXRole') }); queue.push(...kids(el)); }
  return all;
}
const webOf = (win) => (below(win, 4000).find((e) => e.role === 'AXWebArea') || {}).el || null;
const urlOf = (web) => { const u = web && get(web, 'AXURL'); try { return u ? String(u.absoluteString.js) : ''; } catch (e) { return ''; } };
/* the page: the shown one whose URL starts with `url`, else the front window's */
function page(app, url) {
  let web = null;
  const wins = url ? get(app, 'AXWindows') : null;
  for (let i = 0; wins && i < wins.count && !web; i += 1) { const w = webOf(wins.objectAtIndex(i)); if (urlOf(w).startsWith(url)) web = w; }
  if (!url) web = webOf(get(app, 'AXFocusedWindow') || get(app, 'AXMainWindow'));
  return web ? below(web, 20000) : [];
}
function pick(all, want, roles) {
  const w = norm(want), mine = all.filter((e) => roles.includes(e.role));
  mine.forEach((e) => { e.label = norm(said(e.el)); });
  return mine.find((e) => e.label === w) || mine.filter((e) => e.label.includes(w)).sort((a, b) => a.label.length - b.label.length)[0];
}
const press = (el) => $.AXUIElementPerformAction(el, $('AXPress'));
const shown = (hit) => `${hit.role} «${said(hit.el).slice(0, 80)}»`;
function click(all, target) {
  let hit = pick(all, target, CLICKS) || pick(all, target, ['AXGroup', 'AXStaticText', 'AXImage', 'AXCell', 'AXRow']);
  if (!hit) return 'not found: ' + target;
  if (hit.role === 'AXStaticText') hit = { el: get(hit.el, 'AXParent') || hit.el, role: 'AXGroup' };
  /* a disabled button takes the press and does nothing: say so, never «clicked» */
  const enabled = get(hit.el, 'AXEnabled');
  if (enabled && ObjC.unwrap(enabled) === false) return 'ERROR not clicked, it is disabled: ' + shown(hit);
  return press(hit.el) === 0 ? 'clicked ' + shown(hit) : 'could not press ' + shown(hit);
}
function type(all, target, text) {
  const hit = pick(all, target, FIELDS);
  if (!hit) return 'not found: ' + target;
  if (hit.role === 'AXPopUpButton') {
    press(hit.el);
    delay(0.4);
    const option = pick(below(hit.el, 600), text, ['AXMenuItem']);
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
  const [cmd, target, text, url] = argv;
  if (!$.AXIsProcessTrustedWithOptions($({ AXTrustedCheckOptionPrompt: true }))) {
    return 'ERROR OpenBot may not use Accessibility yet: System Settings › Privacy & Security › Accessibility › turn OpenBot on';
  }
  const apps = $.NSRunningApplication.runningApplicationsWithBundleIdentifier('com.apple.Safari');
  if (!apps.count) return 'ERROR Safari is not open';
  const safari = $.AXUIElementCreateApplication(apps.objectAtIndex(0).processIdentifier);
  let all = page(safari, url || '');
  /* WebKit builds its page tree on the first Accessibility call it gets: once more after a second */
  if (!all.length) { delay(1); all = page(safari, url || ''); }
  if (!all.length) return url ? 'ERROR no shown page starts with ' + url : 'ERROR no page in Safari\'s front window';
  return cmd === 'click' ? click(all, target) : type(all, target, text || '');
}
