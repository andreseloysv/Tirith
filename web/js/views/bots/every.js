// A routine that runs every so often: the interval's words (2m, 1h, 1d: at least 1 minute) and its presets.
import { html } from '../../../vendor/preact-htm.js';
import { t } from '../../i18n.js';

export const PRESETS = ['2m', '5m', '15m', '30m', '1h', '6h', '1d'];
const UNIT = { s: 1, m: 60, h: 3600, d: 86400 };
/** The seconds of "2m", "90s", "1h", "1d"; 0 when the words are no interval of 1 minute to 7 days. */
export function everySecs(words) {
  const m = /^(\d+)([smhd])$/.exec(String(words || '').trim().toLowerCase());
  const s = m ? Number(m[1]) * UNIT[m[2]] : 0;
  return s >= 60 && s <= 7 * 86400 ? s : 0;
}
/** "Every 2m", for a routine's line. */
export const everyLabel = (words) => t('Every {every}', { every: words });

export function Every({ every, setEvery }) {
  return html`<div class="bfield"><span class="bcap">${t('Every')}</span>
      <div class="bpre">${PRESETS.map((p) => html`<button type="button" key=${p} aria-pressed=${every === p} class=${every === p ? 'on' : ''} onClick=${() => setEvery(p)}>${p}</button>`)}</div>
      <input value=${every} placeholder="2m" aria-label=${t('Every')} maxlength="6" onInput=${(e) => setEvery(e.target.value)} />
      <span class="bmut">${t('How often it runs: 2m, 90s, 1h or 1d, at least 1 minute. It never starts while its last run is still going.')}</span></div>`;
}
