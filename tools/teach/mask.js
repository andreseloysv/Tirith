// What a field is called, and whether it holds a secret: a password, a one-time code or a card (the daemon masks
// the same fields again). Loaded before content.js in every page.
/* exported teachLabel, teachSecret, MASK */
var MASK = '•••';
var WORDS = ['cvv', 'cvc', 'otp', 'pin', 'código', 'codigo', 'tarjeta'];

/** The field's label: its <label>, aria-label, placeholder or name. */
function teachLabel(el) {
  var byFor = el.id && document.querySelector('label[for="' + CSS.escape(el.id) + '"]');
  var wrap = el.closest && el.closest('label');
  var text = (byFor && byFor.textContent) || (wrap && wrap.textContent) || el.getAttribute('aria-label')
    || el.getAttribute('placeholder') || el.name || '';
  return text.replace(/\s+/g, ' ').trim().slice(0, 200);
}

function named(text) {
  var t = String(text || '').toLowerCase();
  return t.includes('card') || t.includes('password') || t.split(/[^\p{L}\p{N}]+/u).some(function (w) { return WORDS.includes(w); });
}

/** Whether the field holds a password, a one-time code or a card. */
function teachSecret(el) {
  var auto = String(el.getAttribute('autocomplete') || '').toLowerCase();
  return (el.type || '').toLowerCase() === 'password' || auto === 'one-time-code' || auto.startsWith('cc-')
    || named(el.name) || named(el.id) || named(teachLabel(el));
}
