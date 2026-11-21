// The Enseñar tab: the link the Edge extension is given once (address and token), Grabar/Parar, and the steps as
// they arrive, with what the owner wrote in the chat under the step it follows.
import { html, useState } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';

const MOST_MS = 10 * 60 * 1000;

function stepLine(s) {
  if (s.kind === 'said') return html`<span class="tsaid">«${s.text}»</span>`;
  if (s.kind === 'page') return html`<span><b>Page</b> ${s.title || s.url}</span>`;
  if (s.kind === 'click') return html`<span><b>Click</b> ${s.text || s.selector}${s.role ? html` <i>${s.role}</i>` : null}</span>`;
  return html`<span><b>Type</b> ${s.label || 'a field'} = <span class=${s.masked ? 'tmask' : ''}>${s.value}</span></span>`;
}

function Steps({ steps }) {
  if (!steps.length) return html`<p class="bnote">No step yet. Do the task in Edge; each page, click and value typed shows up here.</p>`;
  return html`<ol class="tsteps">${steps.map((s, i) => html`<li key=${i} class=${s.kind === 'said' ? 'said' : ''}>
      <time>${s.at ? new Date(s.at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }) : ''}</time>${stepLine(s)}</li>`)}</ol>`;
}

function Link({ link }) {
  const [copied, setCopied] = useState(false);
  const copy = () => navigator.clipboard.writeText(link).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1500); });
  return html`<div class="tlink"><code>${link}</code>
    <button type="button" class="blink" onClick=${copy}>${copied ? 'Copied' : 'Copy'}</button></div>`;
}

export function Teach({ bot }) {
  const path = `/api/teach/${encodeURIComponent(bot.id)}`;
  const [rec, reload, err] = useFetch(path, ['teach'], [bot.id]);
  const [fail, setFail] = useState('');
  if (!rec) return html`<p class="bnote">${err ? `Could not read the recording: ${err.message}` : 'Loading…'}</p>`;
  const on = !!rec.on;
  const steps = rec.steps || [];
  const press = () => api('POST', `${path}/${on ? 'stop' : 'start'}`).then(() => { setFail(''); reload(); }, (e) => setFail(e.message));
  const until = on ? new Date(Date.parse(rec.since) + MOST_MS).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : '';
  return html`<div class="bteach">
    <p class="bnote">Show ${bot.name} a task once in your Edge, and it writes the skill as a draft and asks what one example cannot show.
      Install <code>tools/teach</code> in Edge (edge://extensions → Load unpacked) and paste this link in it once:</p>
    <${Link} link=${`${location.origin}${path}#${rec.token}`} />
    <div class="trow"><button type="button" class=${`bbtn ${on ? 'bdel' : 'pri'}`} onClick=${press}>${on ? 'Parar' : 'Grabar'}</button>
      <span class="bnote">${on ? `Recording · stops by itself at ${until}. What you write in the chat goes with the steps.`
        : 'Passwords, codes and cards are masked (•••).'}</span></div>
    ${fail ? html`<p class="berr">${fail}</p>` : null}
    <h4><span>${on ? 'Steps so far' : 'Last recording'}</span>${steps.length ? html` <span class="bn">${steps.filter((s) => s.kind !== 'said').length}</span>` : null}</h4>
    <${Steps} steps=${steps} />
  </div>`;
}
