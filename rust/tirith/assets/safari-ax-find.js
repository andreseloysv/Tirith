// The first half of the fixed Accessibility script (safari-ax.js is the second; act.rs joins them): finding the
// page a Safari window shows and, in it, the element the words name. WebKit is asked for the candidates itself
// (AXUIElementsForSearchPredicate, as VoiceOver does): the buttons, links or fields with those words, a few
// elements instead of every one of the page. A chat app's page holds some 10,000; walking them all took 4 to 9
// seconds a command. Only where that search does not exist is the page walked, as before.
ObjC.import('ApplicationServices');
ObjC.import('AppKit');
ObjC.bindFunction('AXUIElementCreateApplication', ['id', ['int']]);
ObjC.bindFunction('AXUIElementCopyAttributeValue', ['int', ['id', 'id', 'id*']]);
ObjC.bindFunction('AXUIElementCopyParameterizedAttributeValue', ['int', ['id', 'id', 'id', 'id*']]);
ObjC.bindFunction('AXUIElementSetAttributeValue', ['int', ['id', 'id', 'id']]);
ObjC.bindFunction('AXUIElementPerformAction', ['int', ['id', 'id']]);
ObjC.bindFunction('AXIsProcessTrustedWithOptions', ['bool', ['id']]);
const CLICKS = ['AXButton', 'AXLink', 'AXCheckBox', 'AXRadioButton', 'AXMenuItem', 'AXPopUpButton', 'AXMenuButton',
  'AXTab', 'AXDisclosureTriangle', 'AXComboBox'];
const OTHERS = ['AXGroup', 'AXStaticText', 'AXImage', 'AXCell', 'AXRow'];
const FIELDS = ['AXTextField', 'AXTextArea', 'AXComboBox', 'AXSearchField', 'AXPopUpButton'];
/* what WebKit is asked for: what a person presses, and where a person types */
const PRESSED = ['AXControlSearchKey', 'AXLinkSearchKey', 'AXButtonSearchKey', 'AXCheckBoxSearchKey', 'AXKeyboardFocusableSearchKey'];
const TYPED = ['AXTextFieldSearchKey'];
const norm = (s) => String(s || '').replace(/\s+/g, ' ').trim().toLowerCase();
function get(el, name) {
  const ref = Ref();
  return $.AXUIElementCopyAttributeValue(el, $(name), ref) === 0 ? ref[0] : null;
}
const str = (el, name) => { const v = get(el, name); try { return v && typeof v.js === 'string' ? v.js : ''; } catch (e) { return ''; } };
function list(v) {
  const out = [];
  for (let i = 0; v && i < v.count; i += 1) out.push(v.objectAtIndex(i));
  return out;
}
const kids = (el) => list(get(el, 'AXChildren'));
/* what a person reads on an element: its title, description, value or placeholder, else the text inside it */
function said(el, depth = 0) {
  const own = ['AXTitle', 'AXDescription', 'AXValue', 'AXPlaceholderValue'].map((a) => str(el, a)).filter(Boolean).join(' ');
  return own || depth > 2 ? own : kids(el).map((k) => said(k, depth + 1)).filter(Boolean).join(' ');
}
const role = (el) => ({ el, role: str(el, 'AXRole') });
/* every element under `root`, the near ones first: the old way, for a page whose search is missing */
function below(root, limit) {
  const all = [], queue = [root];
  while (queue.length && all.length < limit) { const el = queue.shift(); all.push(role(el)); queue.push(...kids(el)); }
  return all;
}
/* a window's page: the first web area under it, looked for no further than that */
function webOf(win) {
  const queue = win ? [win] : [];
  for (let seen = 0; queue.length && seen < 4000; seen += 1) {
    const el = queue.shift();
    if (str(el, 'AXRole') === 'AXWebArea') return el;
    queue.push(...kids(el));
  }
  return null;
}
const urlOf = (web) => { const u = web && get(web, 'AXURL'); try { return u ? String(u.absoluteString.js) : ''; } catch (e) { return ''; } };
/* the page: the shown one whose URL starts with `url`, else the front window's */
function pageOf(app, url) {
  if (!url) return webOf(get(app, 'AXFocusedWindow') || get(app, 'AXMainWindow') || list(get(app, 'AXWindows'))[0]);
  return list(get(app, 'AXWindows')).map(webOf).find((web) => urlOf(web).startsWith(url)) || null;
}
/* WebKit's own search under a page: the elements of those kinds, with `words` in a name, description or value
   when words are given; null where the search does not exist */
function search(web, keys, words, limit) {
  const ask = { AXSearchKey: keys, AXDirection: 'AXDirectionNext', AXResultsLimit: limit, AXImmediateDescendantsOnly: false };
  if (words) ask.AXSearchText = words;
  const ref = Ref();
  const failed = $.AXUIElementCopyParameterizedAttributeValue(web, $('AXUIElementsForSearchPredicate'), $(ask), ref) !== 0;
  return failed ? null : list(ref[0]).map(role);
}
function best(all, want, roles) {
  const w = norm(want), mine = all.filter((e) => roles.includes(e.role));
  mine.forEach((e) => { e.label = norm(said(e.el)); });
  return mine.find((e) => e.label === w) || mine.filter((e) => e.label.includes(w)).sort((a, b) => a.label.length - b.label.length)[0];
}
/* the element of `roles` the words name: exactly, else the shortest label holding them. WebKit is asked for the
   whole words, then for their first one alone (it matches spaces more strictly than a person reads them), and for
   fields for all of them (a field is often named by its placeholder, which that search does not read). */
function pick(web, want, roles, keys, every) {
  const first = norm(want).split(' ')[0];
  for (const words of every ? [''] : [want, first]) {
    const got = search(web, keys, words, every ? 3000 : 400);
    if (!got) return best(below(web, 20000), want, roles);
    const hit = best(got, want, roles);
    if (hit) return hit;
  }
  return null;
}
