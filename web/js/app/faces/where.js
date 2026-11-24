// What a face's hover card says under its name: where it works now (the Project Manager: in which project; a bot:
// what it is doing), else its last message (the Project Manager has one per project: where it worked last).
import { ACTIVE, agoText, ms } from '../../common.js';
import { intentOf } from '../../intent.js';
import { shownAnswers } from '../../views/chat.js';
import { t } from '../../i18n.js';

const nameOf = (s, id) => (s.projects.find((p) => p.id === id) || {}).name || id;
const isProject = (s, id) => s.projects.some((p) => p.id === id);
const and = (xs) => (xs.length > 2 ? t('{a}, {b} and {n} more', { a: xs[0], b: xs[1], n: xs.length - 2 }) : xs.length === 2 ? t('{a} and {b}', { a: xs[0], b: xs[1] }) : xs.join(''));

/** The projects where the Project Manager works now: its chat has a turn there, or helpers of it work there. */
export function pmProjects(s) {
  const now = new Set(Object.keys(s.busy).filter((k) => s.busy[k] && isProject(s, k)));
  for (const l of Object.values(s.lanes)) if (ACTIVE.includes(l.status) && isProject(s, l.project)) now.add(l.project);
  return [...now];
}

/* a lane's last sign of life in ms: `last_activity` is in seconds, the others are ISO times */
const laneTime = (l) => (typeof l.last_activity === 'number' && l.last_activity > 0 ? l.last_activity * 1000 : ms(l.ended_at || l.started_at || l.created_at));

/** `{id, at}`: the project the Project Manager worked on last (its turns and its helpers' work), or null. */
export function lastProject(s) {
  let best = null;
  const see = (id, t) => { if (t && isProject(s, id) && (!best || t > best.t)) best = { id, t, at: new Date(t).toISOString() }; };
  for (const l of Object.values(s.lanes)) see(l.project, laneTime(l));
  for (const e of s.events) if (e.kind === 'turn.done' || e.kind === 'card.opened') see(e.project, ms(e.at));
  return best;
}

/** Markdown and line breaks out, for one or two lines of plain words. */
export const plain = (t) => String(t || '').replace(/```[\s\S]*?```/g, ' ').replace(/[*_`#>]+/g, '').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/\s+/g, ' ').trim();

function botLine(face, s) {
  const talk = s.talks[face.key], rows = (talk && talk.rows) || [];
  if (face.busy) { /* the turn's newest step: the rows after what started it (the owner, the phone, a routine) */
    const start = rows.map((r) => r.role === 'tool' || r.role === 'manager').lastIndexOf(false);
    return t('Working · {what}', { what: intentOf(rows.slice(start + 1).pop()) });
  }
  const last = shownAnswers(rows, false).pop();
  return last ? plain(last.text) : talk && talk.loaded ? t('No messages yet') : '';
}

/** The hover card's line for `face` (see useFaces). */
export function whereOf(face, s) {
  if (!face.pm) return botLine(face, s);
  const now = pmProjects(s);
  if (now.length) return t('Working on {names}', { names: and(now.map((id) => nameOf(s, id))) });
  const last = lastProject(s);
  const when = agoText(last && last.at).replace(/ /g, '\u00a0'), n = s.projects.length;
  return last ? t('Last worked on {name} · {when}', { name: nameOf(s, last.id), when }) : n === 1 ? t('Manages 1 project') : t('Manages {n} projects', { n });
}
