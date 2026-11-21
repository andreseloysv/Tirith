// One stored message that is not part of a manager turn: a routine's line, a card the manager sent, the owner's
// words, or a note (Tirith's, a room's, the CEO's or an agent's `tirith say`).
import { html } from '../../../vendor/preact-htm.js';
import { clockTime } from '../../common.js';
import { esc, ctxHtml } from '../../md.js';
import { Need } from '../need.js';
import { CLOCK } from './icons.js';
import { pure, projName } from './util.js';
import { ownerParts } from './owner.js';
import { mdOf, Pics } from './files.js';

const MERGED = /^merged on ([\w.-]+) at ([0-9a-f]{6,12}): ([\s\S]*)$/;
const CHECKS = /\s*\(check:\s*`[^`]*`\)/gi;
export const CARD = /^(o\d+) \((\w+)\): ([\s\S]*)$/;   /* a card the manager sent: "o12 (account): question" */
const ENDED = { answered: 'Answered', closed: 'Closed' };

function card({ m, open, ended }, time) {
  const [, id, kind, question] = CARD.exec(m.text) || [null, '', 'decision', m.text];
  return html`<div class="mc-m mc-card"><ul class="nlist"><${Need} o=${open || { id, kind, question, at: m.at }} full done=${open ? false : ENDED[ended] || true} /></ul>
      <div class="mc-meta" dangerouslySetInnerHTML=${{ __html: ctxHtml(open ? 'Needs you' : 'Asked you') + ' · ' + esc(time) }} /></div>`;
}

function owner({ m, projects, onZoom }, time) {
  const p = ownerParts(m.text);
  const ctx = p.project ? `on ${projName(projects, p.project)}` : '';
  return html`<div class="mc-m me"><${Pics} files=${p.files} onZoom=${onZoom} />
      ${p.text ? html`<div class="mc-b">${p.text}</div>` : null}
      <div class="mc-meta" dangerouslySetInnerHTML=${{ __html: (ctx ? ctxHtml(ctx) + ' · ' : '') + esc(time) }} /></div>`;
}

/** Who a note is from and its words: {text, ctx, cls}. */
function noteOf({ m, project, projects }) {
  let text = m.text, ctx = 'Tirith', cls = 'mgr note';
  const merged = m.role === 'tirith' && MERGED.exec(text);
  /* a merge reads as one line; in the conversation of no project (rows from before the split) it names its project */
  if (merged) text = `On main${merged[1] === project ? '' : ` of ${projName(projects, merged[1])}`} (\`${merged[2]}\`): ${merged[3].replace(CHECKS, '')}`;
  if (m.role === 'room') {
    /* "author (room): text"; the author of a paired phone reads "name (the owner's paired phone)" */
    const r = /^(.*) \(([^()]*)\): ([\s\S]*)$/.exec(text);
    if (r) { ctx = /paired phone\)$/.test(r[1]) ? 'Your phone' : r[1]; text = r[3]; } else ctx = 'Room';
    cls = 'mgr room';
  } else if (m.role !== 'tirith') { ctx = ['ceo', 'reviewer', 'improver'].includes(m.role) ? 'CEO' : m.role; cls = 'mgr ceo'; } /* a turn role's rows (the CEO's passes, under whatever name the role had) and an agent's `tirith say`, under its role or lane */
  return { text, ctx, cls };
}

function note(props, time) {
  const { text, ctx, cls } = noteOf(props);
  return html`<div class=${`mc-m ${cls}`}><div class="mc-b md" dangerouslySetInnerHTML=${{ __html: mdOf(props.m.id, text) }} />
    <div class="mc-meta" dangerouslySetInnerHTML=${{ __html: ctxHtml(ctx) + ' · ' + esc(time) }} /></div>`;
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
