// Everything about one lane (drawer body, or the open row in the list): Stop, item, status, question, report,
// the lane's facts, its log, its live activity and its pipelines.
import { html } from '../../../vendor/preact-htm.js';
import { useStore } from '../../store.js';
import { Span, ago, clockTime } from '../../common.js';
import { Md } from '../boards/md.js';
import { LogView } from '../boards/logview.js';
import { Need } from '../need.js';
import { STOPPABLE, startOf, endOf, questionOf, forYou, statusOf } from './lanes.js';
import { StopBtn } from './stop.js';
import { useLaneLog, Sec, KV, LanePipelines, Events, PlanItem } from './sections.js';

function status(a, q, active, st, last) {
  return html`<div class="astat"><span class=${`sbadge ${st}${forYou(q) ? ' you' : ''}`}>${st === 'active' ? html`<i class="pdot2"></i>` : null}${statusOf(a, q)}</span>
      <span class="mut">${a.status === 'waiting' ? 'open for ' : active ? 'running ' : 'took '}<${Span} start=${startOf(a)} end=${endOf(a)} /></span>
      ${active && last ? html`<span class="mut">last activity ${ago(last) === 'now' ? 'just now' : `${ago(last)} ago`}</span>` : null}
      ${a.attempts ? html`<span class="mut">try ${a.attempts + 1} of 3</span>` : null}</div>`;
}

function question(a, q) {
  return html`<${Sec} title=${forYou(q) ? 'Question for you' : 'Question to the manager'}>${forYou(q) ? null : html`<div class="lquest"><${Md} text=${a.question} /></div>`}
      <ul class="nlist lneed"><${Need} o=${forYou(q) ? q : { ...q, question: 'You can answer it too' }} full here /></ul></${Sec}>`;
}

function facts(a, projName, color) {
  return [
    ['Project', html`<span><i class="pdot" style=${`background:${color};animation:none;display:inline-block;margin-right:6px`}></i>${projName}</span>`],
    ['Step', a.step], ['Branch', a.branch && html`<code>${a.branch}</code>`], ['Worktree', a.workspace && html`<code title=${a.workspace}>${a.workspace}</code>`],
    ['Agent', a.agent], ['Model', a.model], ['Session', a.session && html`<code>${a.session}</code>`], ['Attempts', String(a.attempts || 0)],
    ['Created', clockTime(a.created_at)], ['Started', clockTime(a.started_at)], ['Ended', clockTime(a.ended_at)], ['Merged', clockTime(a.merged_at)],
    ['Lane', html`<code>${a.id}</code>`],
  ];
}

export function LaneDetail({ a, projName, color }) {
  const q = questionOf(useStore(), a);
  const active = ['queued', 'running', 'waiting', 'merge-queued', 'merging'].includes(a.status);
  const log = useLaneLog(a.id, active);
  const st = a.status === 'running' ? 'active' : a.status === 'merged' ? 'done' : ['failed', 'stopped'].includes(a.status) ? 'failed' : 'idle';
  const last = a.last_activity ? new Date(a.last_activity * 1000).toISOString() : '';
  return html`<div class="activity">
    ${STOPPABLE.includes(a.status) ? html`<div class="stoprow"><${StopBtn} a=${a} where="row" /></div>` : null}
    <${Sec} title="Item"><${PlanItem} a=${a} /></${Sec}>
    <${Sec} title="Status">${status(a, q, active, st, last)}</${Sec}>
    ${q ? question(a, q) : null}
    ${a.note ? html`<${Sec} title=${a.status === 'failed' ? 'Why it failed' : 'Report'}><div class=${`sum${a.status === 'failed' ? ' lbad' : ''}`}><${Md} text=${a.note} /></div></${Sec}>` : null}
    <${Sec} title="Lane"><${KV} rows=${facts(a, projName, color)} /></${Sec}>
    <${Sec} title="Log">${log == null ? html`<p class="quiet sm">Loading the log…</p>` : html`<${LogView} text=${log} />`}</${Sec}>
    <${Sec} title="Live activity"><${Events} id=${a.id} /></${Sec}>
    <${Sec} title="Pipelines"><${LanePipelines} id=${a.id} /></${Sec}>
  </div>`;
}
