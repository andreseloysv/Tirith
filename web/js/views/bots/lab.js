// The Mejoras tab: the bot's experiments from /api/lab?bot=<id> (the Coach's, docs/BOTS.md part 3), newest first, in
// short lines a phone reads: the change and its hypothesis, the mode and the routine, the comparisons judged against
// the factory's rule (a running shadow says it costs double), A's and B's numbers once decided, and Tirith's decision in the one line it said in the chat.
import { html } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';

const n = (v) => typeof v === 'number' && isFinite(v);
const int = (v) => (n(v) ? Math.round(v).toLocaleString('en') : '—');
const secs = (ms) => (!n(ms) || !ms ? '—' : ms < 60000 ? `${Math.round(ms / 100) / 10} s` : `${Math.round(ms / 6000) / 10} min`);
const money = (v) => (n(v) ? `$${v < 0.01 ? v.toFixed(4) : v.toFixed(3)}` : null);
const STATUS = { running: 'Running', adopted: 'Adopted', discarded: 'Discarded' };

/** One side's numbers: its runs and their mean tokens, time and dollars. */
function side(name, s) {
  if (!s || !s.runs) return html`<span class="bmut">${name} · no run</span>`;
  const usd = money(s.cost);
  return html`<span class="bmut"><b class="blabv">${name}</b> ${s.runs} run${s.runs === 1 ? '' : 's'} · ${int(s.tokens)} tokens · ${secs(s.ms)}${usd ? ` · ${usd}` : ''}</span>`;
}

/** The comparisons so far: how many of the rule's inputs, and B's wins, ties and losses once decided. */
function compared(e, rule) {
  const t = e.numbers && e.numbers.tally;
  const of = rule ? ` of ${rule.inputs}` : '';
  const score = t ? ` · B won ${t.wins}, tied ${t.ties}, lost ${t.losses}` : '';
  return `${e.compared}${e.status === 'running' ? of : ''} comparison${e.compared === 1 ? '' : 's'}${score}`;
}

function experiment(e, bot, rule) {
  const r = (bot.routines || []).find((x) => x.id === e.routine);
  const nb = e.numbers || {};
  return html`<li key=${e.id} class="bnr blab">
    <span class="blabh"><b>${e.change || e.branch}</b><i class=${`blabs ${e.status}`}>${STATUS[e.status] || e.status}</i></span>
    <span class="bnn">${e.hypothesis}</span>
    <span class="bmut">${e.mode} · ${r ? r.title : e.routine} · branch <span class="bmono">${e.branch}</span></span>
    <span class="bmut">${compared(e, rule)}</span>
    ${e.mode === 'shadow' && e.status === 'running' ? html`<span class="bmut">Shadow: B runs beside every real run, so this routine costs double while it lasts.</span>` : null}
    ${e.status === 'running' ? null : html`${side('A', nb.a)}${side('B', nb.b)}`}
    ${nb.line ? html`<span class="blabd">${nb.line}</span>` : null}
  </li>`;
}

function ruleLine(rule) {
  if (!rule) return null;
  const pct = (v) => `${Math.round(v * 100)}%`;
  return html`<p class="bnote">The rule, the factory's: after ${rule.inputs} comparisons or ${rule.max_days} days, B is adopted when it wins or ties ${pct(rule.not_worse_share)} of them and wins as often as it loses, has no new failure and no 👎, and saves ${pct(rule.min_saving)} of the cost or the time.</p>`;
}

export function Lab({ bot }) {
  const [lab, , err] = useFetch(`/api/lab?bot=${encodeURIComponent(bot.id)}`, ['lab', 'turn'], [bot.id]);
  if (!lab) return html`<p class="bnote">${err ? `Could not read ${bot.name}'s experiments: ${err.message}` : 'Loading…'}</p>`;
  const list = lab.experiments || [];
  return html`<div class="blib bnum">
    ${list.length ? html`<ul class="brout">${list.map((e) => experiment(e, bot, lab.rule))}</ul>`
      : html`<p class="bnote">No experiment yet. The Coach tries one change at a time on a routine of ${bot.name}: its real runs alternate the current version and the change, and Tirith keeps the change only when it is no worse and cheaper or faster.</p>`}
    ${ruleLine(lab.rule)}
  </div>`;
}
