// Who works now, for the Home: the helpers (lanes) grouped under their boss — their project's Project Manager — and
// the bots and managers that have a turn of their own. A helper is a title made from its item and, when it is not
// simply working, its state in words. Nothing that is done.
import { ACTIVE } from '../../common.js';
import { intentOf } from '../../intent.js';
import { hit } from './bits.js';

/** A short title from an item: its first sentence, without its check, quotes of code and parentheses. */
export function titleOf(item) {
  const t = String(item || '').replace(/\s*\(check:\s*`[^`]*`\)\s*/i, ' ').replace(/`/g, '').replace(/\s+/g, ' ').trim();
  const head = t.split(/(?<=[.!?])\s|:\s|\s\(/)[0].replace(/[.:;,]+$/, '') || t;
  return head.length > 90 ? `${head.slice(0, 89).replace(/\s+\S*$/, '')}…` : head;
}

/** What a helper's state says, when it is not simply working ('you': it waits on the owner's answer). */
export const SAY = { you: 'Waiting for you', waiting: 'Asking its boss', queued: 'About to start', 'merge-queued': 'Finishing', merging: 'Checking its work' };

/* what a conversation's running turn does now, in a few words */
function doing(s, key) {
  const rows = (s.talks[key] && s.talks[key].rows) || [];
  const start = rows.map((r) => r.role === 'tool' || r.role === 'manager').lastIndexOf(false);
  return intentOf(rows.slice(start + 1).pop());
}

/** [{key, helpers: [{id, title, state, project}], busy, doing}]: bosses with helpers first, the most first.
    `project`: only that project's; `shown(key)`: whether a bot's turn may show (hidden bots' do not). */
export function crew(s, { project, q, cards, shown }) {
  const by = new Map();
  const group = (key) => by.get(key) || by.set(key, { key, helpers: [], busy: false, doing: '' }).get(key);
  for (const l of Object.values(s.lanes)) {
    if (!ACTIVE.includes(l.status) || (project && l.project !== project) || !hit(q, l.item, l.project)) continue;
    const you = l.status === 'waiting' && cards.some((o) => o.lane === l.id && o.kind !== 'lane');
    group(l.project).helpers.push({ id: l.id, title: titleOf(l.item) || 'A task', state: you ? 'you' : l.status, project: l.project, at: l.created_at });
  }
  for (const key of Object.keys(s.busy)) {
    const isProject = s.projects.some((p) => p.id === key);
    if (!s.busy[key] || (project ? key !== project : !isProject && !shown(key)) || (q && !hit(q, key))) continue;
    Object.assign(group(key), { busy: true, doing: doing(s, key) });
  }
  const list = [...by.values()];
  list.forEach((g) => g.helpers.sort((a, b) => String(a.at).localeCompare(String(b.at))));
  return list.sort((a, b) => b.helpers.length - a.helpers.length);
}
