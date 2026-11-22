// What a field, a label or a parameter is called, as whole words, and what is secret in a recorded address: no page and no
// browser in it, so a test runs it as it is (the daemon reads the same words: rust/tirith/src/teach/names.rs and url.rs; both pass
// rust/tirith/checks/teach_table.json). Loaded before mask.js and content.js in every page.
/* exported MASK, teachNamed, teachMaskUrl, teachSecretField, teachClickText */
var MASK = '•••';
var SECRET_WORDS = ['pwd', 'pass', 'passwd', 'passcode', 'password', 'passwords', 'contraseña', 'contraseñas', 'contrasena', 'contrasenas',
  'clave', 'claves', 'secret', 'secrets', 'secreto', 'token', 'tokens', 'otp', 'pin', 'cvv', 'cvc', 'código', 'codigo', 'códigos', 'codigos',
  'tarjeta', 'card', '2fa', 'mfa'];
var SECRET_INSIDE = ['password', 'passwd', 'contraseña', 'contrasena'];
var CODE_KINDS = ['verification', 'security', 'confirmation', 'sms', 'auth', 'authentication', 'access'];
var PARAM_WORDS = ['code', 'token', 'tokens', 'key', 'password', 'passwd', 'pwd', 'pass', 'passcode', 'secret', 'sig', 'signature', 'session',
  'sid', 'auth', 'otp', 'jwt', 'bearer', 'credential', 'credentials', 'apikey', 'ticket', 'assertion', 'pin', 'cvv'];
var PARAM_INSIDE = ['token', 'password', 'secret', 'apikey', 'session', 'signature'];

/** The words of `text`, lowercase: anything but a letter or a digit splits, and so does a camel-case step (userPassword, PINCode). */
function teachWords(text) {
  return String(text || '').replace(/(\p{Ll})(\p{Lu})/gu, '$1 $2').replace(/(\p{Lu})(\p{Lu}\p{Ll})/gu, '$1 $2')
    .toLowerCase().match(/[\p{L}\p{N}]+/gu) || [];
}

/** Whether `text` names a password, a one-time code, a PIN or a card (not «passenger», not «company»). */
function teachNamed(text) {
  var w = teachWords(text);
  return w.some(function (x, i) {
    return SECRET_WORDS.includes(x) || SECRET_INSIDE.some(function (l) { return x.includes(l); })
      || (x === 'code' && i > 0 && CODE_KINDS.includes(w[i - 1]));
  });
}

/** Whether a field holds a secret: its type (or it was one: `revealed`), its autocomplete, its name, id or label. */
function teachSecretField(f) {
  var auto = String(f.autocomplete || '').toLowerCase();
  return String(f.type || '').toLowerCase() === 'password' || !!f.revealed || auto === 'one-time-code' || auto.startsWith('cc-')
    || auto.includes('password') || teachNamed(f.name) || teachNamed(f.id) || teachNamed(f.label);
}

function secretParam(name) {
  return teachWords(name).some(function (w) { return PARAM_WORDS.includes(w) || PARAM_INSIDE.some(function (i) { return w.includes(i); }); });
}
function maskParams(q) {
  return q.split('&').map(function (p) {
    var at = p.indexOf('=');
    return at > 0 && at < p.length - 1 && secretParam(p.slice(0, at)) ? p.slice(0, at) + '=' + MASK : p;
  }).join('&');
}
function maskFragment(f) {
  var q = f.indexOf('?');
  if (q >= 0) return f.slice(0, q) + '?' + maskParams(f.slice(q + 1));
  return f.includes('=') ? maskParams(f) : f;
}
function maskUserinfo(base) {
  var m = /^([^:/?#]+:\/\/)([^/@:]*):[^/@]*@([^/]*)(.*)$/s.exec(base);
  return m ? m[1] + m[2] + ':' + MASK + '@' + m[3] + m[4] : base;
}

/** The address with its secrets masked: the password of user:password@, and the value of a secret query or fragment parameter. */
function teachMaskUrl(url) {
  var s = String(url || '');
  var h = s.indexOf('#');
  var frag = h >= 0 ? s.slice(h + 1) : null;
  var rest = h >= 0 ? s.slice(0, h) : s;
  var q = rest.indexOf('?');
  var out = maskUserinfo(q >= 0 ? rest.slice(0, q) : rest);
  if (q >= 0) out += '?' + maskParams(rest.slice(q + 1));
  return frag === null ? out : out + '#' + maskFragment(frag);
}

/** The text a click is recorded with: what a field or a contenteditable is called (never what is typed in it), else the element's own words. */
function teachClickText(c) {
  if (c.editable) return c.aria || 'editable text';
  return c.field ? c.label : c.text;
}
if (typeof module !== 'undefined') module.exports = { teachWords, teachNamed, teachSecretField, teachMaskUrl, teachClickText };
