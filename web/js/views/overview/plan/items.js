// A project's plan as read and drawn row by row: the plan itself (refetched when it changes), a progress bar, and
// an item with the state of its latest lane.
import { html, useState, useEffect } from '../../../../vendor/preact-htm.js';
import { api, onMessage, useStore } from '../../../store.js';
import { openCards } from '../../need.js';
import { laneLabel } from '../../../common.js';
import { Svg, OK, OPEN, Rich } from '../bits.js';

const CHECK = /\s*\(check:\s*`[^`]*`\)\s*/i;

/** GET /api/plan/<id>, refetched when the daemon says this project's plan changed or one of its lanes merged. */
export function usePlan(id) {
  const [st, set] = useState({ data: null, error: null });
  useEffect(() => {
    let t = 0, live = true;
    const load = () => api('GET', `/api/plan/${encodeURIComponent(id)}`).then(
      (data) => live && set({ data, error: null }), (error) => live && set((s) => ({ data: s.data, error })));
    load();
    const off = onMessage((m) => {
      const d = m.d || {};
      const mine = (m.t === 'plan' && (!d.project || d.project === id)) || (m.t === 'lane' && d.project === id && d.status === 'merged');
      if (!mine) return;
      clearTimeout(t);
      t = setTimeout(load, 250);
    });
    return () => { live = false; off(); clearTimeout(t); };
  }, [id]);
  return st;
}

export function Bar({ done, total, cls }) {
  const pct = total ? Math.round((100 * done) / total) : 0;
  return html`<span class=${`bar ${cls || ''}`}><i class=${done === total && total ? 'green' : 'blue'} style=${`width:${pct}%`}></i></span>`;
}

/** The old item state chip, from the item's latest lane. */
function LaneChip({ lane }) {
  const st = useStore();
  if (!lane) return null;
  const s = lane.status;
  const you = s === 'waiting' && openCards(st).some((o) => o.lane === lane.id && o.kind !== 'lane');
  const cls = s === 'running' || s === 'queued' ? 'progress' : s === 'failed' ? 'blocked' : s === 'stopped' ? 'reopened' : '';
  if (s === 'merged') return null;
  return html`<span class=${`pstate ${cls}`} title=${`Lane ${lane.id}`}>${you ? 'Waiting for you' : laneLabel(s)}</span>`;
}

export function Item({ i, lane }) {
  const text = i.text.replace(CHECK, ' ').trim();
  return html`<li class=${i.done ? 'ok' : ''}>
    <${Svg} cls="pi" s=${i.done ? OK : OPEN} />
    <span class="pit"><${Rich} t=${text} /></span>
    ${i.check ? html`<span class="pwhen pchk" title="The command that proves this item is done"><code>${i.check}</code></span>` : null}
    ${i.done ? null : html`<${LaneChip} lane=${lane} />`}
  </li>`;
}
