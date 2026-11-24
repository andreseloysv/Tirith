// Overview — the Home, redone on 2026-11-23 for anyone, not only a developer («I only want to see things that need
// to be done or where the bots are working»): a greeting and one sentence, who works now — each boss with the titles
// of its helpers (working.js), a few lines — and the questions that wait for the owner, whole, with their answers.
// Inside a project: its name, its questions, its helpers, what is up next in its plan (next.js) and its chat on the
// right. Nothing that is done shows; the factory's health is in Settings; nothing at all: the agents asleep.
import { html } from '../../../vendor/preact-htm.js';
import { useStore } from '../../store.js';
import { useUnread } from '../../unread.js';
import { keyOf } from '../bots.js';
import { Chat } from '../chat.js';
import { openCards } from '../need.js';
import { hit } from './bits.js';
import { useWho } from './who.js';
import { crew } from './crew.js';
import { Hello } from './hello.js';
import { Asks } from './asks.js';
import { Working } from './working.js';
import { Next } from './next.js';
import { Quiet } from './quiet.js';
import { Setup } from './setup.js';
import { Start } from './start.js';

const newest = (a, b) => String(b.at).localeCompare(String(a.at));

export function Overview({ project }) {
  const s = useStore();
  const who = useWho();
  const bots = useUnread().bots.filter((b) => b.show !== false);
  const q = String(s.q || '').trim().toLowerCase();
  const p = s.projects.find((x) => x.id === project);
  const cards = openCards(s);
  const asks = cards.filter((o) => (!p || o.project === p.id || (!o.project && o.kind === 'device')) && hit(q, o.question, o.kind, o.project)).sort(newest);
  const shown = new Set(bots.map(keyOf));
  const groups = crew(s, { project: p ? p.id : '', q, cards, shown: (k) => shown.has(k) });
  const helpers = groups.reduce((n, g) => n + g.helpers.length, 0);
  const lanes = p ? Object.values(s.lanes).filter((l) => l.project === p.id) : [];
  const calm = !asks.length && !groups.length;
  const main = html`<div class="home">
    <${Hello} place=${p ? p.name || p.id : ''} asks=${asks.length} helpers=${helpers} bosses=${groups.length} line=${!(calm && !p && !q)} />
    ${p ? html`<${Setup} project=${p.id} name=${p.name} />` : html`<${Start} />`}
    <${Working} groups=${groups} who=${who} />
    <${Asks} list=${asks} who=${who} />
    ${calm && !p && !q ? html`<${Quiet} faces=${bots} />` : null}
    ${p ? html`<${Next} p=${p} lanes=${lanes} q=${q} />` : null}
  </div>`;
  if (!p) return html`<div class="hm-page">${main}</div>`;
  return html`<div class="ovg hm-proj"><div class="hm-page">${main}</div>
    <aside class="ovside"><${Chat} key=${p.id} project=${p.id} /></aside></div>`;
}
