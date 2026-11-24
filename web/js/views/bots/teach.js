// The Teach tab: the link the Edge extension is given once (address and token), Record/Stop, and the steps as
// they arrive, with what the owner wrote in the chat under the step it follows; then the bot's rehearsals of the draft.
import { html, useState } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { Rehearsals } from './rehearsals.js';
import { t } from '../../i18n.js';

const MOST_MS = 10 * 60 * 1000;

function stepLine(s) {
  if (s.kind === 'said') return html`<span class="tsaid">«${s.text}»</span>`;
  if (s.kind === 'page') return html`<span><b>${t('Page')}</b> ${s.title || s.url}</span>`;
  if (s.kind === 'click') return html`<span><b>${t('Click')}</b> ${s.text || s.selector}${s.role ? html` <i>${s.role}</i>` : null}</span>`;
  return html`<span><b>${t('Type')}</b> ${s.label || t('a field')} = <span class=${s.masked ? 'tmask' : ''}>${s.value}</span></span>`;
}

function Steps({ steps }) {
  if (!steps.length) return html`<p class="bnote">${t('No step yet. Do the task in Edge; each page, click and value typed shows up here.')}</p>`;
  return html`<ol class="tsteps">${steps.map((s, i) => html`<li key=${i} class=${s.kind === 'said' ? 'said' : ''}>
      <time>${s.at ? new Date(s.at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }) : ''}</time>${stepLine(s)}</li>`)}</ol>`;
}

function Link({ link }) {
  const [copied, setCopied] = useState(false);
  const copy = () => navigator.clipboard.writeText(link).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1500); });
  return html`<div class="tlink"><code>${link}</code>
    <button type="button" class="blink" onClick=${copy}>${copied ? t('Copied') : t('Copy')}</button></div>`;
}

export function Teach({ bot, onRoutine }) {
  const path = `/api/teach/${encodeURIComponent(bot.id)}`;
  const [rec, reload, err] = useFetch(path, ['teach'], [bot.id]);
  const [fail, setFail] = useState('');
  if (!rec) return html`<p class="bnote">${err ? t('Could not read the recording: {error}', { error: err.message }) : t('Loading…')}</p>`;
  const on = !!rec.on;
  const steps = rec.steps || [];
  const press = () => api('POST', `${path}/${on ? 'stop' : 'start'}`).then(() => { setFail(''); reload(); }, (e) => setFail(e.message));
  /* the daemon says how long is left: its clock and this one may differ */
  const until = on ? new Date(Date.now() + (rec.left_ms ?? MOST_MS)).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : '';
  return html`<div class="bteach">
    <p class="bnote">${t('Show {name} a task once in your Edge: it writes the skill as a draft and asks what one example cannot show.', { name: bot.name })}</p>
    <a class="bbtn pri tdl" href="/teach-extension.zip" download>${t('Download the extension')}</a>
    <p class="bnote">${t('Unzip it, open edge://extensions, turn on Developer mode and choose Load unpacked. Then paste this link in it once:')}</p>
    <${Link} link=${`${location.origin}${path}#${rec.token}`} />
    <div class="trow"><button type="button" class=${`bbtn ${on ? 'bdel' : 'pri'}`} onClick=${press}>${on ? t('Stop') : t('Record')}</button>
      <span class="bnote">${on ? t('Recording · stops by itself at {until}. What you write in the chat goes with the steps.', { until })
        : t('Passwords, codes and cards are masked (•••).')}</span></div>
    ${fail ? html`<p class="berr">${fail}</p>` : null}
    <h4><span>${on ? t('Steps so far') : t('Last recording')}</span>${steps.length ? html` <span class="bn">${steps.filter((s) => s.kind !== 'said').length}</span>` : null}</h4>
    <${Steps} steps=${steps} />
    <${Rehearsals} path=${path} list=${rec.rehearsals} onRoutine=${onRoutine} />
  </div>`;
}
