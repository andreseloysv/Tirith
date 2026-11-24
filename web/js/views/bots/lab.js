// The Improvements tab: the bot's experiments from /api/lab?bot=<id> (the Coach's, docs/BOTS.md part 3), newest first, in
// short lines a phone reads: the change and its hypothesis, the mode and the routine, the comparisons judged against
// the factory's rule (a running shadow says it costs double), A's and B's numbers once decided, and Tirith's decision in the one line it said in the chat.
import { html } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';
import { t } from '../../i18n.js';

const n = (v) => typeof v === 'number' && isFinite(v);
const int = (v) => (n(v) ? Math.round(v).toLocaleString('en') : '—');
const secs = (ms) => (!n(ms) || !ms ? '—' : ms < 60000 ? t('{n} s', { n: Math.round(ms / 100) / 10 }) : t('{n} min', { n: Math.round(ms / 6000) / 10 }));
const money = (v) => (n(v) ? `$${v < 0.01 ? v.toFixed(4) : v.toFixed(3)}` : null);
const STATUS = { running: t('Running'), adopted: t('Adopted'), discarded: t('Discarded') };

/** One side's numbers: its runs and their mean tokens, time and dollars. */
function side(name, s) {
  if (!s || !s.runs) return html`<span class="bmut">${name} · ${t('no run')}</span>`;
  const usd = money(s.cost);
  return html`<span class="bmut"><b class="blabv">${name}</b> ${s.runs === 1 ? t('1 run') : t('{n} runs', { n: s.runs })} · ${t('{n} tokens', { n: int(s.tokens) })} · ${secs(s.ms)}${usd ? ` · ${usd}` : ''}</span>`;
}

/** The comparisons so far: how many of the rule's inputs, and B's wins, ties and losses once decided. */
function compared(e, rule) {
  const ta = e.numbers && e.numbers.tally;
  const n = e.compared, total = e.status === 'running' && rule ? rule.inputs : 0;
  const count = total ? t('{n} of {total} comparisons', { n, total }) : n === 1 ? t('1 comparison') : t('{n} comparisons', { n });
  return ta ? `${count} · ${t('B won {wins}, tied {ties}, lost {losses}', ta)}` : count;
}

function experiment(e, bot, rule) {
  const r = (bot.routines || []).find((x) => x.id === e.routine);
  const nb = e.numbers || {};
  return html`<li key=${e.id} class="bnr blab">
    <span class="blabh"><b>${e.change || e.branch}</b><i class=${`blabs ${e.status}`}>${STATUS[e.status] || e.status}</i></span>
    <span class="bnn">${e.hypothesis}</span>
    <span class="bmut">${e.mode} · ${r ? r.title : e.routine} · ${t('branch')} <span class="bmono">${e.branch}</span></span>
    <span class="bmut">${compared(e, rule)}</span>
    ${e.mode === 'shadow' && e.status === 'running' ? html`<span class="bmut">${t('Shadow: B runs beside every real run, so this routine costs double while it lasts.')}</span>` : null}
    ${e.status === 'running' ? null : html`${side('A', nb.a)}${side('B', nb.b)}`}
    ${nb.line ? html`<span class="blabd">${nb.line}</span>` : null}
  </li>`;
}

function ruleLine(rule) {
  if (!rule) return null;
  const pct = (v) => `${Math.round(v * 100)}%`;
  return html`<p class="bnote">${t("The rule, the factory's: after {inputs} comparisons or {days} days, B is adopted when it wins or ties {share} of them and wins as often as it loses, has no new failure and no 👎, and saves {saving} of the cost or the time.", { inputs: rule.inputs, days: rule.max_days, share: pct(rule.not_worse_share), saving: pct(rule.min_saving) })}</p>`;
}

export function Lab({ bot }) {
  const [lab, , err] = useFetch(`/api/lab?bot=${encodeURIComponent(bot.id)}`, ['lab', 'turn'], [bot.id]);
  if (!lab) return html`<p class="bnote">${err ? t("Could not read {name}'s experiments: {error}", { name: bot.name, error: err.message }) : t('Loading…')}</p>`;
  const list = lab.experiments || [];
  return html`<div class="blib bnum">
    ${list.length ? html`<ul class="brout">${list.map((e) => experiment(e, bot, lab.rule))}</ul>`
      : html`<p class="bnote">${t('No experiment yet. The Coach tries one change at a time on a routine of {name}: its real runs alternate the current version and the change, and NeoBot keeps the change only when it is no worse and cheaper or faster.', { name: bot.name })}</p>`}
    ${ruleLine(lab.rule)}
  </div>`;
}
