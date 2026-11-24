// The Numbers tab: the bot's numbers from /api/bots/<id>/numbers, in short lines a phone reads (no wide tables).
// By routine: runs, success, time, cost per good run, 👎 and corrections per 10, the judge's mean score and the trend
// (the last 10 runs against the 10 before). By skill: uses, validated uses and its routine's tokens before against
// after. Its memory: what loading it costs, the notes its last runs cited, the ones none cited and the misses.
import { html } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';
import { t } from '../../i18n.js';

const n = (v) => typeof v === 'number' && isFinite(v);
const money = (v) => (n(v) ? `$${v < 0.01 ? v.toFixed(4) : v.toFixed(v < 1 ? 3 : 2)}` : '—');
const secs = (ms) => (!n(ms) ? '—' : ms < 1000 ? t('under 1 s') : ms < 60000 ? t('{n} s', { n: Math.round(ms / 100) / 10 }) : t('{n} min', { n: Math.round(ms / 6000) / 10 }));
const pct = (v) => (n(v) ? `${Math.round(v * 100)}%` : '—');
const int = (v) => (n(v) ? Math.round(v).toLocaleString('en') : '—');

/** One change of the trend: `name` and how much the last 10 moved against the 10 before (a share, or points of success). */
function change(name, last, before, points) {
  if (!n(last) || !n(before)) return null;
  if (points) { const d = Math.round((last - before) * 100); return `${name} ${d > 0 ? '+' : d < 0 ? '−' : '±'}${Math.abs(d)} ${t('pts')}`; }
  if (!before) return null;
  const d = Math.round(((last - before) / before) * 100);
  return `${name} ${d > 0 ? '↑' : d < 0 ? '↓' : '='}${d ? `${Math.abs(d)}%` : ''}`;
}

function trendLine(tr) {
  if (!tr) return null;
  const parts = [change(t('cost'), tr.last.cost_per_good, tr.before.cost_per_good), change(t('time'), tr.last.ms, tr.before.ms), change(t('ok'), tr.last.success, tr.before.success, true)].filter(Boolean);
  return parts.length ? html`<span class="bcap">${t('last 10 vs the 10 before')} · ${parts.join(' · ')}</span>` : null;
}

function routine(r) {
  return html`<li key=${r.id || 'chat'} class="bnr"><b>${r.title}</b>
    ${r.runs ? html`<span class="bmut">${t('{runs} · {ok} ok · {time} each', { runs: r.runs === 1 ? t('1 run') : t('{n} runs', { n: r.runs }), ok: pct(r.success), time: secs(r.ms) })}</span>
      <span class="bmut">${t('{cost} per good run', { cost: money(r.cost_per_good) })}${n(r.score) ? ` · ${t('score {score}', { score: r.score.toFixed(2) })}` : ''}</span>
      <span class="bmut">${t('per 10 runs · 👎 {down} · corrected {fixed}', { down: r.down_per_10, fixed: r.corrections_per_10 })}</span>
      ${trendLine(r.trend)}` : html`<span class="bmut">${t('No run yet')}</span>`}</li>`;
}

function skill(s) {
  const saved = n(s.saved_percent) ? ` (${s.saved_percent > 0 ? '−' : '+'}${Math.abs(s.saved_percent)}%)` : '';
  return html`<li key=${s.name} class="bnr"><b class="bmono">${s.name}</b>
    <span class="bmut">${t('used {used} · validated {validated} of {used}', { used: s.used, validated: s.validated })}</span>
    ${n(s.tokens_before) ? html`<span class="bmut">${t('tokens per run {before} → {after}', { before: int(s.tokens_before), after: int(s.tokens_after) })}${saved}</span>` : html`<span class="bcap">${t("used from its routine's first run")}</span>`}</li>`;
}

function memory(m) {
  const cited = m.notes.filter((x) => x.cited).sort((a, b) => b.cited - a.cited);
  const unc = m.notes.filter((x) => !x.cited);
  return html`<ul class="bnm">
    <li><span class="bmut">${t('Loading it costs {own} tokens (its own) and {shared} (shared)', { own: int(m.tokens.own), shared: int(m.tokens.shared) })}</span></li>
    <li><span class="bmut">${cited.length ? t('Cited in its last {runs} runs: {notes}', { runs: m.runs, notes: cited.map((x) => `[${x.id}] ×${x.cited}`).join(' · ') }) : t('No note cited in its last {runs} runs', { runs: m.runs })}</span></li>
    ${unc.length ? html`<li><span class="bmut">${t('Never cited, candidates to delete (NeoBot never deletes them):')}</span>
      ${unc.map((x) => html`<span key=${x.id} class="bnn"><b class="bmono">[${x.id}]</b> ${x.text}</span>`)}</li>` : null}
    <li><span class="bmut">${m.misses ? t('Memory misses: {n} (you had to repeat yourself)', { n: m.misses }) : t('No memory miss: you never had to repeat yourself')}</span>
      ${(m.missed || []).map((x, i) => html`<span key=${i} class="bnn">“${x}”</span>`)}</li>
  </ul>`;
}

export function Numbers({ bot }) {
  const [num, , err] = useFetch(`/api/bots/${encodeURIComponent(bot.id)}/numbers`, ['turn', 'bots', 'library'], [bot.id]);
  if (!num) return html`<p class="bnote">${err ? t("Could not read {name}'s numbers: {error}", { name: bot.name, error: err.message }) : t('Loading…')}</p>`;
  return html`<div class="blib bnum">
    ${num.runs ? null : html`<p class="bnote">${t('No run yet. Each run of {name} (a routine or a chat turn) adds to these numbers.', { name: bot.name })}</p>`}
    <h4><span>${t('Routines')}</span></h4>
    ${num.routines.length ? html`<ul class="brout">${num.routines.map(routine)}</ul>` : html`<p class="bnote">${t('No routine yet.')}</p>`}
    <h4><span>${t('Skills')}</span></h4>
    ${num.skills.length ? html`<ul class="brout">${num.skills.map(skill)}</ul>` : html`<p class="bnote">${t('No skill used yet.')}</p>`}
    <h4><span>${t('Memory')}</span></h4>
    ${memory(num.memory)}
  </div>`;
}
