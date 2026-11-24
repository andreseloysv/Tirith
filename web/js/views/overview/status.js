// The Overview's status card with no project selected: the metrics against their targets, the agents working, and
// the CEO's last pass with Improve now.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { clockTime } from '../../common.js';
import { memo, Svg, CHEV, REFRESH, clip } from './bits.js';
import { t } from '../../i18n.js';

export const num = (v) => (v == null ? '–' : Number.isInteger(v) ? String(v) : (Math.round(v * 10) / 10).toString());

function Metric({ m }) {
  const target = m.target == null ? '' : `${m.at_least ? '≥' : '≤'} ${num(m.target)}`;
  return html`<span class="stm" title=${t(m.facts || m.label)}><i class=${`dot ${m.target == null ? 'grey' : m.red ? 'red' : 'green'}`}></i>${t(m.label)}${' '}<b class=${m.red ? 'bad' : ''}>${num(m.value)}</b>${target ? html`<span class="mut"> / ${target}</span>` : null}</span>`;
}

function Review({ reviews, reviewing }) {
  const [open, setOpen] = useState(false);
  const [err, setErr] = useState('');
  const [asked, setAsked] = useState(false);
  const last = reviews[reviews.length - 1];
  const busy = reviewing || asked;
  const now = () => {
    setErr(''); setAsked(true);
    api('POST', '/api/review').catch((e) => setErr(String(e.message || e))).finally(() => setTimeout(() => setAsked(false), 3000));
  };
  return html`<div class="stl stceo"><span class="stk">${t('CEO')}</span>
      ${busy ? html`<span class="ovlive"><i class="pdot2"></i>${t('Reading the measures by factory version…')}</span>`
        : last ? html`<button class="iahead ovrev" aria-expanded=${open} onClick=${() => setOpen(!open)}>
            <${Svg} s=${CHEV} /><span><span class="mut">${t('Last pass {time}', { time: clockTime(last.at) })}${last.metric ? ` · ${t(last.metric)}` : ''} ·</span> ${clip(last.text, 110)}</span></button>`
        : html`<span class="mut">${t('No pass yet')}</span>`}
      <button class="ibtn ovnow" disabled=${busy} onClick=${now} title=${t('Ask the CEO for a pass now: it may propose one change to the factory')}>
        <${Svg} s=${REFRESH} /><span>${t('Improve now')}</span></button></div>
    ${open && last && !busy ? html`<div class="ia ovrevbody">${last.text}</div>` : null}
    ${err ? html`<div class="ierr ovinl">${err}</div>` : null}`;
}

export const Status = memo(function Status({ data, working }) {
  const list = (data && data.metrics) || [];
  const reviews = (data && data.reviews) || [];
  if (!data) return null;
  const red = list.filter((m) => m.red);
  const merged = list.find((m) => m.id === 'merged');
  const names = red.map((m) => t(m.label)).join(', ');
  const why = red.length ? (red.length === 1 ? t('1 metric off target: {names}', { names }) : t('{n} metrics off target: {names}', { n: red.length, names })) : t('Every metric on target');
  return html`<section class=${`card stcard ${red.length ? 'bad' : 'ok'}`}>
    <div class="stl stl1"><b>${red.length ? '🔴' : '🟢'} ${why}</b>${merged ? ` · ${t('{n} on main in 24 h', { n: num(merged.value) })}` : ''} · ${working === 1 ? t('1 agent working') : t('{n} agents working', { n: working })}</div>
    <div class="stl stms"><span class="stk">${t('Metrics')}</span><span class="stmlist">${list.map((m) => html`<${Metric} key=${m.id} m=${m} />`)}</span></div>
    <${Review} reviews=${reviews} reviewing=${!!data.reviewing} />
  </section>`;
});
