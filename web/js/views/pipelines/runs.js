// The run list on the left: runs grouped by state, and each run's card (name, project, progress bar, current or
// failing stage, stage chips, lanes, duration).
import { html } from '../../../vendor/preact-htm.js';
import { Span, dur, elapsed, ago, clockTime } from '../../common.js';
import { sClass, pipeName, laneIds, StageChip, failedStage } from '../boards/pipe.js';

export const CAP = 20;
export const GROUPS = [['running', 'Running'], ['failed', 'Failed'], ['passed', 'Passed'], ['other', 'Other']];
export const group = (p) => { const s = sClass(p.status); return s === 'running' || s === 'failed' || s === 'passed' ? s : 'other'; };

function pct(p) {
  const st = p.stages || [];
  if (sClass(p.status) !== 'running') return 100;
  if (!st.length) return 6;
  return Math.max(6, Math.round((100 * st.filter((x) => x.status === 'passed').length) / (st.length + 1)));
}
export function total(p) { return p.ended_at ? dur(elapsed(p.started_at, p.ended_at)) : null; }

export function RunCard({ p, sel, projName, onOpen }) {
  const s = sClass(p.status), [kind, num] = pipeName(p), f = failedStage(p);
  const cur = (p.stages || []).find((x) => x.status === 'running');
  const lanes = laneIds(p);
  return html`<button class=${`card runcard${sel ? ' sel' : ''}`} onClick=${() => onOpen(p.id)}>
    <span class="rhead"><b>${kind} <span class="mut">${num}</span></b><span class="sm mut">${projName}</span></span>
    <span class="bar"><i class=${s === 'failed' ? 'red' : s === 'running' ? 'blue' : s === 'passed' ? 'green' : ''} style=${`width:${pct(p)}%`}></i></span>
    ${s === 'running' ? html`<span class="rline"><span class="rcur">${cur ? cur.stage : (p.stages || []).length ? 'merging…' : 'starting…'}</span><span><${Span} start=${p.started_at} /></span></span>` : null}
    ${s === 'failed' ? html`<span class="rerr">${f ? html`${f.status === 'timeout' ? 'timed out at ' : 'failed at '}<b>${f.stage}</b>` : 'failed before any stage ran'}</span>` : null}
    ${(p.stages || []).length ? html`<span class="pmini">${p.stages.map((st) => html`<${StageChip} key=${st.id} st=${st} />`)}</span>` : null}
    <span class="rline rmeta mut"><span>${lanes.length ? `${lanes.length} lane${lanes.length === 1 ? '' : 's'}` : 'main'}${total(p) ? ` · ${total(p)}` : ''}</span>
      <span title=${clockTime(p.started_at)}>${ago(p.started_at)}${ago(p.started_at) === 'now' ? '' : ' ago'}</span></span>
  </button>`;
}

/** The groups of runs, each capped with Show more. */
export function runList({ runs, cur, caps, setCaps, nameOf, open }) {
  return html`<div class="plist">${GROUPS.map(([g, title]) => {
      const l = runs.filter((p) => group(p) === g);
      if (!l.length) return null;
      const cap = caps[g] || CAP;
      return html`<h3>${title} <span class="n">${l.length}</span></h3>
        <div class="pcol">${l.slice(0, cap).map((p) => html`<${RunCard} key=${p.id} p=${p} sel=${p.id === cur.id} projName=${nameOf(p.project)} onOpen=${open} />`)}</div>
        ${l.length > cap ? html`<button class="link lshow" onClick=${() => setCaps({ ...caps, [g]: cap + CAP })}>Show ${Math.min(CAP, l.length - cap)} more</button>` : null}`;
    })}</div>`;
}
