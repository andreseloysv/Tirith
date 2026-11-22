// What a field is called, and whether it holds a secret: a password (also one shown as text since), a one-time code or a card
// (words.js has the words; the daemon masks the same fields again). Loaded before content.js in every page.
/* global teachNamed, teachSecretField */
/* exported teachLabel, teachSecret */
var wasPassword = new WeakSet();

/** The field's label: its <label>, aria-label, placeholder or name. */
function teachLabel(el) {
  var byFor = el.id && document.querySelector('label[for="' + CSS.escape(el.id) + '"]');
  var wrap = el.closest && el.closest('label');
  var text = (byFor && byFor.textContent) || (wrap && wrap.textContent) || el.getAttribute('aria-label')
    || el.getAttribute('placeholder') || el.name || '';
  return text.replace(/\s+/g, ' ').trim().slice(0, 200);
}

/** Whether the field holds a password (it is one, or was one before the page showed it as text), a one-time code or a card. */
function teachSecret(el) {
  return teachSecretField({ type: el.type, revealed: wasPassword.has(el), autocomplete: el.getAttribute('autocomplete'),
    name: el.name, id: el.id, label: teachLabel(el) });
}

// A password field the page turns into text («show password») stays a password: remembered when it is focused or changes type.
function remember(el) { if (el && el.type === 'password') wasPassword.add(el); }
document.addEventListener('focusin', function (ev) { remember(ev.target); }, true);
document.querySelectorAll('input[type=password]').forEach(remember);
new MutationObserver(function (list) {
  list.forEach(function (m) { if (m.oldValue && m.oldValue.toLowerCase() === 'password') wasPassword.add(m.target); });
}).observe(document, { subtree: true, attributes: true, attributeFilter: ['type'], attributeOldValue: true });
