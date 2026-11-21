// A plan's steps as a card draws them: each item's latest lane, each step's counts and status, and the current step.
import { ACTIVE } from '../../../common.js';

/** The latest lane of each item (as the daemon picks it: newest created_at), by `step\nitem`. */
export function latestLanes(lanes) {
  const latest = {};
  for (const l of lanes) {
    const k = `${l.step}\n${l.item}`;
    if (!latest[k] || String(l.created_at) > String(latest[k].created_at)) latest[k] = l;
  }
  return latest;
}

/** Each step with its total, done and status (empty, done, active or todo). */
export function summarize(plan, latest) {
  return (plan.steps || []).map((st) => {
    const total = st.items.length, done = st.items.filter((i) => i.done).length;
    const busy = st.items.some((i) => { const l = latest[`${st.num}\n${i.text}`] || i.lane; return !i.done && l && ACTIVE.includes(l.status); });
    const status = !total ? 'empty' : done === total ? 'done' : done || busy ? 'active' : 'todo';
    return { ...st, total, done, status };
  });
}

/** The step shown unfolded: the first with open work, else (a plan nobody filled yet) the first empty one. */
export const currentStep = (steps) => (steps.find((s) => s.status !== 'done' && s.status !== 'empty') || steps.find((s) => s.status === 'empty') || {}).num;
