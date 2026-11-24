// One stored message that is not part of a manager turn: a routine's line, a card the manager sent, the owner's
// words (typed here or on their phone), or a note (Tirith's, another agent's, the CEO's or an agent's `neobot say`).
import { html } from '../../../vendor/preact-htm.js';
import { clockTime } from '../../common.js';
import { Need } from '../need.js';
import { CLOCK } from './icons.js';
import { pure, projName } from './util.js';
import { ownerParts } from './owner.js';
import { bubble, words } from './bubble.js';
import { mdOf, Pics } from './files.js';

const MERGED = /^merged on ([\w.-]+) at ([0-9a-f]{6,12}): ([\s\S]*)$/;
const CHECKS = /\s*\(check:\s*`[^`]*`\)/gi;
/* "author (room): text", at the first "(room): ": the author is Tirith's, what follows may say anything */
const ROOM = /^(.*?) \(([^()]*)\): ([\s\S]*)$/;
const PHONE = /paired phone\)$/; /* "name (the owner's paired phone)": the owner */
export const CARD = /^(o\d+) \((\w+)\): ([\s\S]*)$/;   /* a card the manager sent: "o12 (account): question" */
const ENDED = { answered: 'Answered', closed: 'Closed' };

function card({ m, open, ended }, time) {
  const [, id, kind, question] = CARD.exec(m.text) || [null, '', 'decision', m.text];
  return bubble({
    cls: open ? 'mgr mc-card open' : 'mgr mc-card',
    from: open ? 'Needs you' : 'Asked you',
    time,
    body: html`<ul class="nlist"><${Need} o=${open || { id, kind, question, at: m.at }} full done=${open ? false : ENDED[ended] || true} /></ul>`,
  });
}

function owner({ m, projects, onZoom }, time) {
  const p = ownerParts(m.text);
  return bubble({
    cls: 'me',
    time: p.project ? `on ${projName(projects, p.project)} · ${time}` : time,
    body: html`<${Pics} files=${p.files} onZoom=${onZoom} />${words(p.text ? mdOf(m.id, p.text) : '')}`,
  });
}

/** Who a note is from and its words: {text, from, cls, via}. The owner's own from the phone stand where the phone
    has them: on the right. */
function noteOf({ m, project, projects }) {
  let text = m.text, from = 'NeoBot', cls = 'mgr note';
  const merged = m.role === 'tirith' && MERGED.exec(text);
  /* a merge reads as one line; in the conversation of no project (rows from before the split) it names its project */
  if (merged) text = `On main${merged[1] === project ? '' : ` of ${projName(projects, merged[1])}`} (\`${merged[2]}\`): ${merged[3].replace(CHECKS, '')}`;
  if (m.role === 'room') {
    const r = ROOM.exec(text);
    if (r && PHONE.test(r[1])) return { text: r[3], from: '', cls: 'me', via: 'Your phone · ' };
    if (r) { from = r[1]; text = r[3]; } else from = 'Room';
    cls = 'mgr note room';
  } else if (m.role !== 'tirith') { from = ['ceo', 'reviewer', 'improver'].includes(m.role) ? 'CEO' : m.role; cls = 'mgr ceo'; } /* a turn role's rows (the CEO's passes, under whatever name the role had) and an agent's `neobot say`, under its role or lane */
  return { text, from, cls, via: '' };
}

function note(props, time) {
  const { text, from, cls, via } = noteOf(props);
  return bubble({ cls, from, time: `${via}${time}`, body: words(mdOf(props.m.id, text)) });
}

/** One stored message that is not part of a manager turn (`project`: the conversation it is drawn in; `open`: the
    card of a card row while it waits for the owner). */
export const Msg = pure((props) => {
  const { m } = props, time = clockTime(m.at);
  if (m.role === 'routine') return html`<div class="mc-ev" title=${time}>${CLOCK}<span>${m.text}</span></div>`;
  if (m.role === 'card') return card(props, time);
  if (m.role === 'owner') return owner(props, time);
  return note(props, time);
}, (a, b) => a.m === b.m && a.projects === b.projects && a.open === b.open && a.ended === b.ended);
