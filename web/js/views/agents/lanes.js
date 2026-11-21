// What the Agents view reads off a lane: its column, its times, its item without the check, whether it is live,
// its open question and its status in words; and the layout the owner chose (kanban or list).
import { ms, laneLabel } from '../../common.js';
import { openCards } from '../need.js';

export const DAY = 86400000;
export const CAP = 30;
export const COLS = [
  ['queued', 'Queued', 'grey', ['queued']],
  ['running', 'Working', 'blue', ['running']],
  ['waiting', 'Waiting for an answer', 'amber', ['waiting']],
  ['merging', 'Merging', 'purple', ['merge-queued', 'merging']],
  ['merged', 'Done', 'green', ['merged']],
  ['failed', 'Failed / Stopped', 'red', ['failed', 'stopped']],
];
export const STOPPABLE = ['queued', 'running', 'waiting', 'merge-queued'];
export const DOT = { running: 'blue pulse', queued: 'grey', waiting: 'amber', 'merge-queued': 'blue', merging: 'blue', merged: 'green', failed: 'red', stopped: 'grey' };

export const startOf = (a) => a.started_at || a.created_at;
export const endOf = (a) => (['queued', 'running', 'waiting', 'merge-queued', 'merging'].includes(a.status) ? null : a.ended_at || a.merged_at || null);
export const newest = (a) => ms(a.merged_at || a.ended_at || a.started_at || a.created_at) || 0;
export const isOld = (a) => ['merged', 'failed', 'stopped'].includes(a.status) && Date.now() - newest(a) > DAY;
export const itemText = (t) => String(t || '').replace(/\s*\(check:\s*`?[^)]*`?\)\s*$/, '');
export const live = (a) => a.status === 'running' && a.last_activity && Date.now() / 1000 - a.last_activity < 90;
/** The open question of a waiting lane: a card for the owner when the manager passed it on (--lane), else the lane's own. */
export const questionOf = (s, a) => (a.status === 'waiting' ? openCards(s).find((o) => o.lane === a.id) || null : null);
export const forYou = (q) => !!q && q.kind !== 'lane';
export const statusOf = (a, q) => (forYou(q) ? 'Waiting for you' : laneLabel(a.status));

export function loadMode() {
  const u = new URLSearchParams(location.search).get('mode');
  if (u === 'list' || u === 'kanban') return u;
  try { return localStorage.getItem('tirith.agents.mode') === 'list' ? 'list' : 'kanban'; } catch { return 'kanban'; } }
export function saveMode(m) { try { localStorage.setItem('tirith.agents.mode', m); } catch { /* private window */ } }
