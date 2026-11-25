// The second part of the fixed Accessibility script (after safari-ax-find.js, before safari-ax.js): what is on
// the page, in words. A page's text (`read`) names neither a button drawn as a picture nor anything inside a
// frame, so an agent that only reads has to guess the words a click takes, one failed command at a time. `find`
// lists what can be pressed and typed into, each with those words and its state; with words, whatever on the page
// holds them all. A click or typing that finds nothing says what is close to its words (`near`), and `wait` asks
// again until something with the words is there, which is how a chain of commands waits for a page.
const KINDS = { AXButton: 'button', AXLink: 'link', AXCheckBox: 'checkbox', AXRadioButton: 'radio', AXMenuItem: 'option',
  AXPopUpButton: 'list', AXMenuButton: 'menu', AXTab: 'tab', AXDisclosureTriangle: 'toggle', AXComboBox: 'field',
  AXTextField: 'field', AXTextArea: 'field', AXSearchField: 'field', AXStaticText: 'text', AXHeading: 'heading' };
const SHOWN = 60;
const kind = (r) => KINDS[r] || String(r || '').replace(/^AX/, '').toLowerCase();
const flat = (s) => String(s || '').replace(/\s+/g, ' ').trim();
const off = (el, name) => { const v = get(el, name); try { return !!v && !ObjC.unwrap(v); } catch (e) { return false; } };
/* one element as a line: its kind, the words a click or typing takes, what a field or list holds, and its state */
function line(e) {
  const k = kind(e.role);
  let name = flat(said(e.el)), more = '';
  if (k === 'field' || k === 'list') {
    const holds = flat(str(e.el, 'AXValue'));
    const own = flat(['AXTitle', 'AXDescription', 'AXPlaceholderValue'].map((a) => str(e.el, a)).filter(Boolean).join(' '));
    if (holds && own) { name = own; more = ` holds «${holds.slice(0, 80)}»`; }
  }
  if (k === 'checkbox' || k === 'radio') more = off(e.el, 'AXValue') ? ' off' : ' on';
  if (off(e.el, 'AXEnabled')) more += ' (disabled)';
  return `${k} «${name.slice(0, 160)}»${more}`;
}
/* the lines of `all` that hold every word of `want`, each once, in the page's order */
function lines(all, want) {
  const words = norm(want).split(' ').filter(Boolean), out = [];
  for (const e of all) {
    const label = norm(said(e.el));
    if (!label || !words.every((w) => label.includes(w))) continue;
    const text = line(e);
    if (!out.includes(text)) out.push(text);
  }
  return out;
}
const ACTED = PRESSED.concat(TYPED);
const acts = (e) => CLICKS.includes(e.role) || FIELDS.includes(e.role);
/* what `find` answers: the page's controls, or with words what holds them (controls first, then anything) */
function find(web, want) {
  const first = norm(want).split(' ')[0] || '';
  /* WebKit is asked for the whole words, then for their first one alone (it reads spaces more strictly than a person) */
  const asked = (keys, limit) => [want, first].reduce((got, w) => (got.length ? got : lines((search(web, keys, w, limit) || []).filter((e) => keys !== ACTED || acts(e)), want)), []);
  let got = want ? asked(ACTED, 400) : lines((search(web, ACTED, '', 400) || below(web, 20000)).filter(acts), '');
  /* a field named only by its placeholder is not found by its words: fields are few, all are looked at */
  if (want) got = got.concat(lines((search(web, TYPED, '', 400) || []).filter(acts), want).filter((l) => !got.includes(l)));
  if (want && !got.length) got = asked(['AXAnyTypeSearchKey'], 200);
  if (!got.length) return want ? `ERROR nothing with «${want}» on this page` + near(web, want, ACTED) : 'ERROR this page shows nothing to press or type into';
  const more = got.length > SHOWN ? `\n… and ${got.length - SHOWN} more: give words to see fewer` : '';
  return got.slice(0, SHOWN).join('\n') + more;
}
/* for a command that found nothing: the controls that share a word with what it asked for, else the page's first */
function near(web, want, keys) {
  const words = norm(want).split(' ').filter((w) => w.length > 2).slice(0, 4);
  let close = [];
  for (const w of words) close = close.concat(lines((search(web, keys, w, 40) || []).filter(acts), w).filter((l) => !close.includes(l)));
  const some = close.length ? close : lines((search(web, keys, '', 40) || []).filter(acts), '');
  if (!some.length) return '';
  return `. ${close.length ? 'Close to it' : 'The page has'}: ${some.slice(0, 8).join(', ')}`;
}
/* asks `look` again, half a second after each answer, until it finds something or `seconds` have passed */
function soon(look, seconds) {
  for (const until = Date.now() + seconds * 1000; ;) {
    const hit = look();
    if (hit || Date.now() >= until) return hit;
    delay(0.5);
  }
}
