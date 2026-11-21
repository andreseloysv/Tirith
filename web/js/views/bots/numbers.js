// The Números tab: the bot's numbers from /api/bots/<id>/numbers, in short lines a phone reads (no wide tables).
// By routine: runs, success, time, cost per good run, 👎 and corrections per 10, the judge's mean score and the trend
// (the last 10 runs against the 10 before). By skill: uses, validated uses and its routine's tokens before against
// after. Its memory: what loading it costs, the notes its last runs cited, the ones none cited and the misses.
import { html } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';

const n = (v) => typeof v === 'number' && isFinite(v);
const money = (v) => (n(v) ? `$${v < 0.01 ? v.toFixed(4) : v.toFixed(v < 1 ? 3 : 2)}` : '—');
const secs = (ms) => (!n(ms) ? '—' : ms < 1000 ? 'under 1 s' : ms < 60000 ? `${Math.round(ms / 100) / 10} s` : `${Math.round(ms / 6000) / 10} min`);
const pct = (v) => (n(v) ? `${Math.round(v * 100)}%` : '—');
const int = (v) => (n(v) ? Math.round(v).toLocaleString('en') : '—');

/** One change of the trend: `name` and how much the last 10 moved against the 10 before (a share, or points of success). */
function change(name, last, before, points) {
  if (!n(last) || !n(before)) return null;
  if (points) { const d = Math.round((last - before) * 100); return `${name} ${d > 0 ? '+' : d < 0 ? '−' : '±'}${Math.abs(d)} pts`; }
  if (!before) return null;
  const d = Math.round(((last - before) / before) * 100);
  return `${name} ${d > 0 ? '↑' : d < 0 ? '↓' : '='}${d ? `${Math.abs(d)}%` : ''}`;
}

function trendLine(t) {
  if (!t) return null;
  const parts = [change('cost', t.last.cost_per_good, t.before.cost_per_good), change('time', t.last.ms, t.before.ms), change('ok', t.last.success, t.before.success, true)].filter(Boolean);
  return parts.length ? html`<span class="bcap">last 10 vs the 10 before · ${parts.join(' · ')}</span>` : null;
}

function routine(r) {
  return html`<li key=${r.id || 'chat'} class="bnr"><b>${r.title}</b>
    ${r.runs ? html`<span class="bmut">${r.runs} run${r.runs === 1 ? '' : 's'} · ${pct(r.success)} ok · ${secs(r.ms)} each</span>
      <span class="bmut">${money(r.cost_per_good)} per good run${n(r.score) ? ` · score ${r.score.toFixed(2)}` : ''}</span>
      <span class="bmut">per 10 runs · 👎 ${r.down_per_10} · corrected ${r.corrections_per_10}</span>
      ${trendLine(r.trend)}` : html`<span class="bmut">No run yet</span>`}</li>`;
}

function skill(s) {
  const saved = n(s.saved_percent) ? ` (${s.saved_percent > 0 ? '−' : '+'}${Math.abs(s.saved_percent)}%)` : '';
  return html`<li key=${s.name} class="bnr"><b class="bmono">${s.name}</b>
    <span class="bmut">used ${s.used} · validated ${s.validated} of ${s.used}</span>
    ${n(s.tokens_before) ? html`<span class="bmut">tokens per run ${int(s.tokens_before)} → ${int(s.tokens_after)}${saved}</span>` : html`<span class="bcap">used from its routine's first run</span>`}</li>`;
}

function memory(m) {
  const cited = m.notes.filter((x) => x.cited).sort((a, b) => b.cited - a.cited);
  const unc = m.notes.filter((x) => !x.cited);
  return html`<ul class="bnm">
    <li><span class="bmut">Loading it costs ${int(m.tokens.own)} tokens (its own) and ${int(m.tokens.shared)} (shared)</span></li>
    <li><span class="bmut">${cited.length ? `Cited in its last ${m.runs} runs: ${cited.map((x) => `[${x.id}] ×${x.cited}`).join(' · ')}` : `No note cited in its last ${m.runs} runs`}</span></li>
    ${unc.length ? html`<li><span class="bmut">Never cited, candidates to delete (Tirith never deletes them):</span>
      ${unc.map((x) => html`<span key=${x.id} class="bnn"><b class="bmono">[${x.id}]</b> ${x.text}</span>`)}</li>` : null}
    <li><span class="bmut">${m.misses ? `Memory misses: ${m.misses} (you had to repeat yourself)` : 'No memory miss: you never had to repeat yourself'}</span>
      ${(m.missed || []).map((t, i) => html`<span key=${i} class="bnn">“${t}”</span>`)}</li>
  </ul>`;
}

export function Numbers({ bot }) {
  const [num, , err] = useFetch(`/api/bots/${encodeURIComponent(bot.id)}/numbers`, ['turn', 'bots', 'library'], [bot.id]);
  if (!num) return html`<p class="bnote">${err ? `Could not read ${bot.name}'s numbers: ${err.message}` : 'Loading…'}</p>`;
  return html`<div class="blib bnum">
    ${num.runs ? null : html`<p class="bnote">No run yet. Each run of ${bot.name} (a routine or a chat turn) adds to these numbers.</p>`}
    <h4><span>Routines</span></h4>
    ${num.routines.length ? html`<ul class="brout">${num.routines.map(routine)}</ul>` : html`<p class="bnote">No routine yet.</p>`}
    <h4><span>Skills</span></h4>
    ${num.skills.length ? html`<ul class="brout">${num.skills.map(skill)}</ul>` : html`<p class="bnote">No skill used yet.</p>`}
    <h4><span>Memory</span></h4>
    ${memory(num.memory)}
  </div>`;
}
