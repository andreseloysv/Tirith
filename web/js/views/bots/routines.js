// The Routines tab's pieces: one routine with its switch, Run now, Edit and Delete, and the form for a new or
// changed one.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { DAY, WEEKDAYS, DAILY, isOn, daysLabel, timeLabel, nextRun, whenLabel } from './when.js';

export function RoutineForm({ bot, r, onDone }) {
  const [title, setTitle] = useState(r.title || '');
  const [prompt, setPrompt] = useState(r.prompt || '');
  const [days, setDays] = useState(r.days || WEEKDAYS);
  const [at, setAt] = useState(r.at || '08:00');
  const [err, setErr] = useState('');
  const ok = title.trim() && prompt.trim() && days.length && at;
  const save = (e) => {
    e.preventDefault();
    if (!ok) return;
    api('POST', `/api/bots/${encodeURIComponent(bot.id)}/routines`, { id: r.id, title: title.trim(), prompt: prompt.trim(), days, at }).then(onDone, (x) => setErr(x.message));
  };
  const flip = (n) => setDays(days.includes(n) ? days.filter((d) => d !== n) : [...days, n].sort());
  return html`<form class="bform" onSubmit=${save}>
    <label><span class="bcap">Title</span><input value=${title} placeholder="Daily job offer check" maxlength="80" onInput=${(e) => setTitle(e.target.value)} /></label>
    <label><span class="bcap">What ${bot.name} does each time</span><textarea rows="3" value=${prompt} placeholder="Check my inbox for new job offers and tell me about each one: company, role, link."
      onInput=${(e) => setPrompt(e.target.value)}></textarea></label>
    <div class="bfield"><span class="bcap">Days</span>
      <div class="bdays">${DAY.map((d, i) => html`<button type="button" key=${d} aria-pressed=${days.includes(i + 1)} aria-label=${d} class=${days.includes(i + 1) ? 'on' : ''} onClick=${() => flip(i + 1)}>${d.slice(0, 2)}</button>`)}</div>
      <div class="bpre"><button type="button" onClick=${() => setDays(DAILY)}>Every day</button><button type="button" onClick=${() => setDays(WEEKDAYS)}>Weekdays</button></div></div>
    <label class="btime"><span class="bcap">Around</span><input type="time" value=${at} onInput=${(e) => setAt(e.target.value)} /></label>
    ${err ? html`<p class="berr">${err}</p>` : null}
    <div class="bfoot"><button type="button" class="bbtn" onClick=${onDone}>Cancel</button><button type="submit" class="bbtn pri" disabled=${!ok}>Save routine</button></div>
  </form>`;
}

export function Routine({ bot, r, onEdit }) {
  const [err, setErr] = useState('');
  const [ask, setAsk] = useState(false);
  const [ran, setRan] = useState(false);
  const base = `/api/bots/${encodeURIComponent(bot.id)}`;
  /* resolves to whether it worked */
  const post = (path, body) => api('POST', base + path, body).then(() => { setErr(''); return true; }, (e) => { setErr(e.message); return false; });
  const run = () => post(`/run/${encodeURIComponent(r.id)}`).then((ok) => { if (ok) { setRan(true); setTimeout(() => setRan(false), 4000); } });
  const next = nextRun(r);
  return html`<li class=${`brr${isOn(r) ? '' : ' off'}`}>
    <span class="brico"><${Icon} d=${ICON.clock} size=${18} w=${1.4} /></span>
    <span class="brtxt"><b>${r.title}</b><span class="bmut">${daysLabel(r.days)} around ${timeLabel(r.at)}</span>
      <span class="bcap">${isOn(r) ? (next ? `Next · ${whenLabel(next)}` : '') : 'Paused'}</span>
      ${ask ? html`<span class="bacts"><span class="bask">Delete this routine?</span><button type="button" class="bdel" onClick=${() => post('/routines', { id: r.id, delete: true })}>Delete</button>
          <button type="button" onClick=${() => setAsk(false)}>Keep</button></span>`
        : html`<span class="bacts"><button type="button" onClick=${run} disabled=${ran}>${ran ? 'Started' : 'Run now'}</button>
          <button type="button" onClick=${onEdit}>Edit</button><button type="button" class="bdel" onClick=${() => setAsk(true)}>Delete</button></span>`}
      ${err ? html`<span class="berr">${err}</span>` : null}</span>
    <button type="button" class=${`bswitch${isOn(r) ? ' on' : ''}`} role="switch" aria-checked=${isOn(r)} aria-label=${`${r.title}: ${isOn(r) ? 'on' : 'off'}`}
      title=${isOn(r) ? 'On: switch off' : 'Off: switch on'} onClick=${() => post('/routines', { id: r.id, on: !isOn(r) })}><i></i></button>
  </li>`;
}
