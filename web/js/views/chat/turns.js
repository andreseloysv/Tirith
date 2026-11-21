// Grouping a conversation's rows: a run of manager / tool rows is one turn, and once it ended only its answer
// shows. An answer that starts with ∅ (nothing new) is a quiet line. While a turn runs, one tiny line says what
// it is doing in plain words (intent.js).
import { html } from '../../../vendor/preact-htm.js';
import { intentOf } from '../../intent.js';

/** [{kind: 'msg', m} | {kind: 'turn', key, rows}] in order; a turn is a run of manager / tool messages. */
export function group(rows) {
  const out = [];
  let turn = null;
  for (const m of rows) {
    if (m.role === 'manager' || m.role === 'tool') {
      if (!turn) { turn = { kind: 'turn', key: 't' + m.id, rows: [] }; out.push(turn); }
      turn.rows.push(m);
    } else {
      turn = null;
      out.push({ kind: 'msg', key: 'm' + m.id, m });
    }
  }
  return out;
}

/** What a turn shows once it ended: its texts after the last tool call, else its last text (so words said before a
    final step are not lost); nothing while it runs. */
export function answerOf(rows, running) {
  if (running) return [];
  let last = -1;
  rows.forEach((r, i) => { if (r.role === 'tool') last = i; });
  const after = rows.slice(last + 1);
  return after.length ? after : rows.filter((r) => r.role === 'manager').slice(-1);
}
/** An answer with nothing new: one quiet line (it stays here and never reaches the phone). */
export const quiet = (text) => /^\s*∅/.test(String(text || ''));

/** The rows a conversation shows as messages of its agent (answers of ended turns, Tirith's notes), quiet ones left
    out: what the unread badges count. `running`: its last turn still runs. */
export function shownAnswers(rows, running) {
  const items = group(rows);
  return items.flatMap((it, i) => (it.kind === 'turn' ? answerOf(it.rows, running && i === items.length - 1) : it.m.role === 'tirith' ? [it.m] : []))
    .filter((r) => !quiet(r.text));
}

/** While a turn runs: one tiny line with what it is doing, in plain words. */
export function Intent({ who, step }) {
  return html`<div class="mc-intent" role="status" aria-live="polite"><i class="mc-pulse" aria-hidden="true"></i>${who ? html`<b>${who}</b>` : null}<span>${intentOf(step)}…</span></div>`;
}
