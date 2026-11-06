// Pipeline bits shared by the Pipelines view and the Agents drawer: names, status marks and stage chips
// (the old v-pipelines.js glyphs and .pchp chips).
import { html } from '../../../vendor/preact-htm.js';
import { ms } from '../../common.js';

const SVG = {
  passed: 'M3.2 6.4l2.1 2.1 3.6-4.2', failed: 'M3.6 3.6l4.8 4.8M8.4 3.6L3.6 8.4',
  warning: 'M6 2.8v3.6M6 8.6v.1', skipped: 'M2.6 3.4l2.6 2.6-2.6 2.6M6.2 3.4l2.6 2.6-2.6 2.6', canceled: 'M3.4 8.6l5.2-5.2',
};
export const LABEL = { passed: 'Passed', failed: 'Failed', running: 'Running', pending: 'Pending', canceled: 'Canceled', timeout: 'Timed out', skipped: 'Skipped' };

/** A stage or pipeline status as the old page's classes know it (timeout draws as failed). */
export const sClass = (s) => (s === 'timeout' ? 'failed' : s || 'pending');

export function glyph(status, size = 12) {
  const s = sClass(status);
  if (s === 'running') return html`<i class="pspin"></i>`;
  const d = SVG[s];
  return d ? html`<svg viewBox="0 0 12 12" width=${size} height=${size} fill="none" stroke="currentColor" stroke-width="1.7"
    stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d=${d} /></svg>` : null;
}
/** The stage mark: a pulsing dot while it runs, a ring while pending, else the glyph. */
export function mark(status) {
  const s = sClass(status);
  return s === 'running' ? html`<i class="pdot"></i>` : s === 'pending' ? html`<i class="pring"></i>` : glyph(s, 11);
}

/** "merge #4" / "main #3": the kind and the number at the end of the id. */
export function pipeName(p) {
  const n = /-(\d+)$/.exec(p.id || '');
  return [p.kind === 'main' ? 'main' : p.kind || 'pipeline', n ? `#${n[1]}` : p.id];
}

export function laneIds(p) {
  return String(p.lanes || '').split(',').filter(Boolean);
}

export function startMs(p) { return ms(p.started_at) || 0; }

export function StageChip({ st, sel, onClick }) {
  const s = sClass(st.status);
  return html`<span class=${`pchp s-${s}${sel ? ' sel' : ''}`} role="button" tabindex="0" title=${`${st.stage}: ${LABEL[st.status] || st.status}`}
    onClick=${onClick}>${mark(st.status)}<span>${st.stage}</span></span>`;
}

/** The first failed stage, for "failed at <b>test</b>". */
export function failedStage(p) {
  return (p.stages || []).find((x) => sClass(x.status) === 'failed');
}
