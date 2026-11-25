// The owner's 👍 / 👎 under each answer of a bot: the answer's turn (its row in /api/turns) is found by its words
// and time, and a click calls POST /api/turns/<id>/reaction (the same one again takes it back); the choice stays marked.
// Beside them, how the answer was made (`Made`): the model and effort that turn really ran with and how long it
// took. The chip in the box says what the next turn will use; a turn already running keeps what it started with.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { readable, effortName } from '../../model-controls.js';
import { t } from '../../i18n.js';

/** The turns of a bot's conversation (`on`: it is a bot's), under its own role, the last first; none elsewhere. */
export function useBotTurns(project, on) {
  const [rows] = useFetch(on ? `/api/turns?conv=${encodeURIComponent(project)}&limit=300` : '', ['turn'], [project, on]);
  return on && rows ? rows.filter((t) => t.role === project) : null;
}

const words = (t) => String(t || '').replace(/\s+/g, ' ').trim();
const time = (at) => { const t = Date.parse(at || ''); return isNaN(t) ? 0 : t; };

/** The turn whose answer an ended turn's last answer row `r` is: one whose answer holds its first words, else the one
    that ended nearest after it (within two minutes); the nearest in time wins. */
export function turnOf(turns, r) {
  if (!turns || !r) return null;
  const start = words(r.text).slice(0, 80), at = time(r.at);
  const near = (t) => Math.abs(time(t.ended_at) - at);
  const said = turns.filter((t) => start && words(t.answer).includes(start));
  const pool = said.length ? said : turns.filter((t) => time(t.ended_at) >= at - 5000 && near(t) < 120000);
  return pool.reduce((best, t) => (!best || near(t) < near(best) ? t : best), null);
}

/** Two discreet buttons; the chosen one stays marked until it is clicked again (taken back). */
export function Reactions({ turn }) {
  const [mine, setMine] = useState(turn.reaction || 0);
  const [err, setErr] = useState('');
  useEffect(() => setMine(turn.reaction || 0), [turn.id, turn.reaction]);
  const set = (v) => {
    const next = mine === v ? 0 : v, was = mine;
    setMine(next);
    api('POST', `/api/turns/${turn.id}/reaction`, { value: next }).then(() => setErr(''), (e) => { setMine(was); setErr(e.message); });
  };
  const btn = (v, face, label) => html`<button type="button" class=${mine === v ? 'on' : ''} aria-pressed=${mine === v}
      aria-label=${label} title=${err || label} onClick=${() => set(v)}>${face}</button>`;
  return html`<span class="mc-react">${btn(1, '👍', t('Good answer'))}${btn(-1, '👎', t('Bad answer'))}</span>`;
}

const took = (ms) => (ms < 1000 ? t('under 1 s') : ms < 60000 ? t('{n} s', { n: Math.round(ms / 1000) }) : t('{n} min', { n: Math.round(ms / 6000) / 10 }));

/** The model, the effort and the time of the turn that gave this answer, in a few quiet words. */
export function Made({ turn }) {
  const parts = [turn.model ? readable(turn.model) : '', effortName(turn.effort), turn.ms ? took(turn.ms) : ''].filter(Boolean);
  return parts.length ? html`<span class="mc-made" title=${t('The model and effort this answer was made with, and how long it took')}>${parts.join(' · ')}</span>` : null;
}
