// In every page: a rehearsal's click or typing, asked by the worker (rehearse.js), found by its selector or its text
// or label; and, on the rehearsal's tab, the visible notice «Tirith está ensayando» until the rehearsal ends.
/* global teachLabel, textOf */
var BANNER = 'tirith-rehearsal';

function norm(t) { return String(t || '').replace(/\s+/g, ' ').trim().toLowerCase(); }
function shown(el) { var r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; }
function bySelector(q) { try { return document.querySelector(q); } catch (e) { return null; } }

/** The first shown element of `pick` whose words (`words(el)`) are `q`, else contain it. */
function byWords(q, pick, words) {
  var want = norm(q);
  var all = Array.from(document.querySelectorAll(pick)).filter(function (el) { return el.id !== BANNER && shown(el); });
  var said = function (el) { return words(el).map(norm); };
  return all.find(function (el) { return said(el).includes(want); })
    || all.find(function (el) { return said(el).some(function (w) { return w && w.includes(want); }); });
}

var CLICKABLE = 'a, button, [role], input[type=submit], input[type=button], input[type=checkbox], input[type=radio], summary, label, [onclick]';
var FIELDS = 'input:not([type=hidden]):not([type=submit]):not([type=button]), textarea, select, [contenteditable=true]';

function click(cmd) {
  var el = bySelector(cmd.target) || byWords(cmd.target, CLICKABLE, function (e) { return [textOf(e)]; });
  if (!el) return { ok: false, result: 'nothing to click called «' + cmd.target + '» on «' + document.title + '»' };
  el.scrollIntoView({ block: 'center' });
  el.click();
  return { ok: true, result: 'clicked «' + (textOf(el) || cmd.target) + '» on «' + document.title + '»' };
}

function setValue(el, value) {
  if (/^(checkbox|radio)$/i.test(el.type || '')) { if (el.checked !== /^(true|1|sí|si|yes|on)$/i.test(value)) el.click(); return; }
  if (el.isContentEditable) { el.textContent = value; } else {
    var proto = Object.getPrototypeOf(el);
    var desc = Object.getOwnPropertyDescriptor(proto, 'value');
    if (desc && desc.set) desc.set.call(el, value); else el.value = value;
  }
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
}

function type(cmd) {
  var words = function (e) { return [teachLabel(e), e.getAttribute('placeholder'), e.getAttribute('aria-label'), e.name, e.id]; };
  var el = bySelector(cmd.target) || byWords(cmd.target, FIELDS, words);
  if (!el) return { ok: false, result: 'no field called «' + cmd.target + '» on «' + document.title + '»' };
  el.scrollIntoView({ block: 'center' });
  el.focus();
  setValue(el, String(cmd.value || ''));
  return { ok: true, result: 'typed in «' + (teachLabel(el) || cmd.target) + '» on «' + document.title + '»' };
}

/** The notice on the rehearsal's tab until `until` (ms); 0 takes it away. */
function banner(until) {
  var old = document.getElementById(BANNER);
  if (old) old.remove();
  if (!until || Date.now() >= until) return;
  var at = new Date(until).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  var el = document.createElement('div');
  el.id = BANNER;
  el.setAttribute('role', 'status');
  el.textContent = 'Tirith está ensayando · para sola a las ' + at;
  el.style.cssText = 'position:fixed;top:10px;left:50%;transform:translateX(-50%);z-index:2147483647;pointer-events:none;'
    + 'background:#ef4444;color:#fff;font:600 14px system-ui,sans-serif;padding:8px 16px;border-radius:999px;box-shadow:0 4px 18px rgba(0,0,0,.35)';
  (document.body || document.documentElement).appendChild(el);
  setTimeout(function () { banner(until); }, Math.min(until - Date.now() + 50, 60000));
}

chrome.runtime.onMessage.addListener(function (m, _sender, reply) {
  if ('banner' in m) { banner(m.banner); reply({ ok: true }); return false; }
  if (!m.rehearse) return false;
  var r;
  try { r = m.rehearse.kind === 'type' ? type(m.rehearse) : click(m.rehearse); } catch (e) { r = { ok: false, result: String(e.message || e) }; }
  reply(r);
  return false;
});
// A page loaded in the rehearsal's tab shows the notice again.
chrome.runtime.sendMessage({ rehearsing: true }).then(function (r) { if (r && r.ok) banner(r.r); }).catch(function () {});
