// In every page: while the bot is being taught (chrome.storage.local `teach.on`), the page (URL and title), each
// click (the element's text, role and a short selector) and each value typed (the field's label and value, a
// secret's masked) go to the background, which sends them to the daemon. When not recording, nothing is sent.
/* global teachLabel, teachSecret, teachMaskUrl, teachClickText, MASK */
var recording = false;
var lastHref = '';

function send(step) {
  if (!recording) return;
  step.at = new Date().toISOString();
  chrome.runtime.sendMessage({ step: step }).catch(function () {});
}

function page() {
  if (!recording || location.href === lastHref) return;
  lastHref = location.href;
  send({ kind: 'page', url: teachMaskUrl(location.href), title: document.title });
}

/** A short selector: the tag with its id, or its name, or two classes, under at most one parent. */
function selector(el) {
  var one = function (e) {
    if (!e || !e.tagName) return '';
    var tag = e.tagName.toLowerCase();
    if (e.id) return tag + '#' + CSS.escape(e.id);
    if (e.getAttribute('name')) return tag + '[name="' + e.getAttribute('name') + '"]';
    var cls = Array.from(e.classList || []).slice(0, 2).map(function (c) { return '.' + CSS.escape(c); }).join('');
    return tag + cls;
  };
  var me = one(el);
  return el.id ? me : [one(el.parentElement), me].filter(Boolean).join(' > ');
}

function roleOf(el) {
  var tag = el.tagName.toLowerCase();
  var implicit = { a: 'link', button: 'button', select: 'combobox', textarea: 'textbox', summary: 'button' };
  return el.getAttribute('role') || implicit[tag] || (tag === 'input' ? (el.type || 'text') : tag);
}

function textOf(el) {
  var t = el.innerText || el.value || el.getAttribute('aria-label') || el.getAttribute('title') || el.getAttribute('alt') || '';
  return String(t).replace(/\s+/g, ' ').trim().slice(0, 200);
}

document.addEventListener('click', function (ev) {
  var el = ev.target && ev.target.closest
    && ev.target.closest('a, button, [role], input, select, textarea, label, summary, [onclick]');
  el = el || ev.target;
  if (!recording || !el || !el.tagName) return;
  var field = /^(input|textarea)$/i.test(el.tagName) && !/^(button|submit|checkbox|radio)$/i.test(el.type || '');
  var editable = !!el.isContentEditable || el.getAttribute('role') === 'textbox'; // what is typed in it is never a click's text
  var text = teachClickText({ editable: editable, aria: el.getAttribute('aria-label') || el.getAttribute('placeholder'),
    field: field, label: field ? teachLabel(el) : '', text: editable || field ? '' : textOf(el) });
  send({ kind: 'click', text: text, role: roleOf(el), selector: selector(el) });
}, true);

document.addEventListener('change', function (ev) {
  var el = ev.target;
  if (!recording || !el || !/^(input|textarea|select)$/i.test(el.tagName)) return;
  var secret = teachSecret(el);
  var value = /^(checkbox|radio)$/i.test(el.type || '') ? String(el.checked) : String(el.value || '');
  send({
    kind: 'type', label: teachLabel(el), name: el.name || el.id || '', type: el.type || '',
    autocomplete: el.getAttribute('autocomplete') || '', sensitive: secret, value: secret ? MASK : value.slice(0, 500),
  });
}, true);

function take(state) {
  var was = recording;
  recording = !!(state && state.on && Date.now() < (state.until || 0));
  if (recording && !was) { lastHref = ''; page(); }
}

chrome.storage.local.get('teach').then(function (got) { take(got.teach); });
chrome.storage.onChanged.addListener(function (changes) { if (changes.teach) take(changes.teach.newValue); });
setInterval(page, 1000); // a page that changes its address without loading (history.pushState)
// A page opened or shown again asks the background to check with the daemon (the page's Grabar may have started it).
chrome.runtime.sendMessage({ sync: true }).catch(function () {});
document.addEventListener('visibilitychange', function () {
  if (!document.hidden) chrome.runtime.sendMessage({ sync: true }).catch(function () {});
});
