// Everything about one lane (drawer body, or the open row in the list): Stop, item, status, question, report,
// the lane's facts, its log, its live activity and its pipelines.
import { html } from '../../../vendor/preact-htm.js';
import { useStore } from '../../store.js';
import { Span, agoText, clockTime, spentText } from '../../common.js';
import { Md } from '../boards/md.js';
import { LogView } from '../boards/logview.js';
import { Need } from '../need.js';
import { STOPPABLE, startOf, endOf, questionOf, forYou, statusOf } from './lanes.js';
import { StopBtn } from './stop.js';
import { useLaneLog, Sec, KV, LanePipelines, Events, PlanItem } from './sections.js';
import { t } from '../../i18n.js';

function status(a, q, active, st, last) {
  return html`<div class="astat"><span class=${`sbadge ${st}${forYou(q) ? ' you' : ''}`}>${st === 'active' ? html`<i class="pdot2"></i>` : null}${statusOf(a, q)}</span>
      <span class="mut">${a.status === 'waiting' ? t('open for') : active ? t('running') : t('took')} <${Span} start=${startOf(a)} end=${endOf(a)} /></span>
      ${active && last ? html`<span class="mut">${t('last activity {when}', { when: agoText(last) })}</span>` : null}
      ${a.attempts ? html`<span class="mut">${t('try {n} of 3', { n: a.attempts + 1 })}</span>` : null}</div>`;
}

function question(a, q) {
  return html`<${Sec} title=${forYou(q) ? t('Question for you') : t('Question to the manager')}>${forYou(q) ? null : html`<div class="lquest"><${Md} text=${a.question} /></div>`}
      <ul class="nlist lneed"><${Need} o=${forYou(q) ? q : { ...q, question: t('You can answer it too') }} full here /></ul></${Sec}>`;
}

function facts(a, projName, color) {
  return [
    [t('Project'), html`<span><i class="pdot" style=${`background:${color};animation:none;display:inline-block;margin-right:6px`}></i>${projName}</span>`],
    [t('Step'), a.step], [t('Branch'), a.branch && html`<code>${a.branch}</code>`], [t('Worktree'), a.workspace && html`<code title=${a.workspace}>${a.workspace}</code>`],
    [t('Agent'), a.agent], [t('Model'), a.model], [t('Session'), a.session && html`<code>${a.session}</code>`], [t('Attempts'), String(a.attempts || 0)], [t('Spent'), spentText(a.spent)],
    [t('Created'), clockTime(a.created_at)], [t('Started'), clockTime(a.started_at)], [t('Ended'), clockTime(a.ended_at)], [t('Merged'), clockTime(a.merged_at)],
    [t('Lane'), html`<code>${a.id}</code>`],
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
    <${Sec} title=${t('Item')}><${PlanItem} a=${a} /></${Sec}>
    <${Sec} title=${t('Status')}>${status(a, q, active, st, last)}</${Sec}>
    ${q ? question(a, q) : null}
    ${a.note ? html`<${Sec} title=${a.status === 'failed' ? t('Why it failed') : t('Report')}><div class=${`sum${a.status === 'failed' ? ' lbad' : ''}`}><${Md} text=${a.note} /></div></${Sec}>` : null}
    <${Sec} title=${t('Lane')}><${KV} rows=${facts(a, projName, color)} /></${Sec}>
    <${Sec} title=${t('Log')}>${log == null ? html`<p class="quiet sm">${t('Loading the log…')}</p>` : html`<${LogView} text=${log} />`}</${Sec}>
    <${Sec} title=${t('Live activity')}><${Events} id=${a.id} /></${Sec}>
    <${Sec} title=${t('Pipelines')}><${LanePipelines} id=${a.id} /></${Sec}>
  </div>`;
}
