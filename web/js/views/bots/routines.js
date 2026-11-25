// The Routines tab's pieces: one routine with its switch, Run now (or, on an event, Test), Edit and Delete, and the
// form for a new or changed one: at a time on some days, or on an event from outside that reaches its webhook, and
// with the bot's model or one of its own (routine-model.js).
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { DAY, WEEKDAYS, DAILY, isOn, daysLabel, timeLabel, nextRun, whenLabel } from './when.js';
import { Webhook } from './webhook.js';
import { RoutineModel, ownWords } from './routine-model.js';
import { t } from '../../i18n.js';

const TRIGGERS = [['schedule', t('At a time')], ['webhook', t('On an event from outside')]];
export const isHook = (r) => r.trigger === 'webhook';

function Timed({ days, setDays, at, setAt }) {
  const flip = (n) => setDays(days.includes(n) ? days.filter((d) => d !== n) : [...days, n].sort());
  return html`<div class="bfield"><span class="bcap">${t('Days')}</span>
      <div class="bdays">${DAY.map((d, i) => html`<button type="button" key=${d} aria-pressed=${days.includes(i + 1)} aria-label=${d} class=${days.includes(i + 1) ? 'on' : ''} onClick=${() => flip(i + 1)}>${d.slice(0, 2)}</button>`)}</div>
      <div class="bpre"><button type="button" onClick=${() => setDays(DAILY)}>${t('Every day')}</button><button type="button" onClick=${() => setDays(WEEKDAYS)}>${t('Weekdays')}</button></div></div>
    <label class="btime"><span class="bcap">${t('Around')}</span><input type="time" value=${at} onInput=${(e) => setAt(e.target.value)} /></label>`;
}

function OnEvent({ bot, when, setWhen }) {
  return html`<label><span class="bcap">${t('When')}</span><input value=${when} placeholder="github:pull_request.opened" onInput=${(e) => setWhen(e.target.value)} />
      <span class="bmut">${t('The event it waits for: github:pull_request.opened, github:issues.*, or * for every call. Its task may name {title}, {url} and {text}.')}</span></label>
    <${Webhook} bot=${bot} />`;
}

export function RoutineForm({ bot, r, onDone }) {
  const [title, setTitle] = useState(r.title || '');
  const [prompt, setPrompt] = useState(r.prompt || '');
  const [trigger, setTrigger] = useState(r.trigger || 'schedule');
  const [when, setWhen] = useState(r.when || 'github:pull_request.opened');
  const [days, setDays] = useState(r.days && r.days.length ? r.days : WEEKDAYS);
  const [at, setAt] = useState(r.at || '08:00');
  const [model, setModel] = useState(r.model || '');
  const [effort, setEffort] = useState(r.effort || '');
  const [err, setErr] = useState('');
  const hook = trigger === 'webhook';
  const ok = title.trim() && prompt.trim() && (hook ? when.trim() : days.length && at);
  const save = (e) => {
    e.preventDefault();
    if (!ok) return;
    const body = { id: r.id, title: title.trim(), prompt: prompt.trim(), trigger, model, effort, ...(hook ? { when: when.trim() } : { days, at }) };
    api('POST', `/api/bots/${encodeURIComponent(bot.id)}/routines`, body).then(onDone, (x) => setErr(x.message));
  };
  return html`<form class="bform" onSubmit=${save}>
    <label><span class="bcap">${t('Title')}</span><input value=${title} placeholder=${hook ? t('Review new pull requests') : t('Daily job offer check')} maxlength="80" onInput=${(e) => setTitle(e.target.value)} /></label>
    <label><span class="bcap">${t('What {name} does each time', { name: bot.name })}</span><textarea rows="3" value=${prompt} placeholder=${hook ? t('Read the pull request {title} at {url} and tell me what it changes and what looks risky.') : t('Check my inbox for new job offers and tell me about each one: company, role, link.')}
      onInput=${(e) => setPrompt(e.target.value)}></textarea></label>
    <div class="bfield"><span class="bcap">${t('Runs')}</span><div class="btrig" role="radiogroup">${TRIGGERS.map(([k, label]) => html`<button type="button" key=${k} role="radio"
      aria-checked=${trigger === k} class=${trigger === k ? 'on' : ''} onClick=${() => setTrigger(k)}>${label}</button>`)}</div></div>
    ${hook ? html`<${OnEvent} bot=${bot} when=${when} setWhen=${setWhen} />` : html`<${Timed} days=${days} setDays=${setDays} at=${at} setAt=${setAt} />`}
    <${RoutineModel} bot=${bot} model=${model} effort=${effort} setModel=${setModel} setEffort=${setEffort} />
    ${err ? html`<p class="berr">${err}</p>` : null}
    <div class="bfoot"><button type="button" class="bbtn" onClick=${onDone}>${t('Cancel')}</button><button type="submit" class="bbtn pri" disabled=${!ok}>${t('Save routine')}</button></div>
  </form>`;
}

export function Routine({ bot, r, onEdit }) {
  const [err, setErr] = useState('');
  const [ask, setAsk] = useState(false);
  const [ran, setRan] = useState(false);
  const base = `/api/bots/${encodeURIComponent(bot.id)}`;
  /* resolves to whether it worked */
  const post = (path, body) => api('POST', base + path, body).then(() => { setErr(''); return true; }, (e) => { setErr(e.message); return false; });
  const hook = isHook(r);
  /* Test: a sample GitHub pull request opened, straight to this routine */
  const run = () => post(hook ? `/webhook/test/${encodeURIComponent(r.id)}` : `/run/${encodeURIComponent(r.id)}`).then((ok) => { if (ok) { setRan(true); setTimeout(() => setRan(false), 4000); } });
  const next = hook ? null : nextRun(r);
  const timed = hook ? t('On an event · {when}', { when: r.when }) : t('{days} around {time}', { days: daysLabel(r.days), time: timeLabel(r.at) });
  const said = [timed, ownWords(r)].filter(Boolean).join(' · '); /* and its own model or effort, when it has one */
  const state = hook ? t('Waits for its webhook') : next ? t('Next · {when}', { when: whenLabel(next) }) : '';
  return html`<li class=${`brr${isOn(r) ? '' : ' off'}`}>
    <span class="brico"><${Icon} d=${hook ? ICON.rooms : ICON.clock} size=${18} w=${1.4} /></span>
    <span class="brtxt"><b>${r.title}</b><span class="bmut">${said}</span>
      <span class="bcap">${isOn(r) ? state : t('Paused')}</span>
      ${ask ? html`<span class="bacts"><span class="bask">${t('Delete this routine?')}</span><button type="button" class="bdel" onClick=${() => post('/routines', { id: r.id, delete: true })}>${t('Delete')}</button>
          <button type="button" onClick=${() => setAsk(false)}>${t('Keep')}</button></span>`
        : html`<span class="bacts"><button type="button" onClick=${run} disabled=${ran}
            title=${hook ? t('Sends it a sample GitHub event: a pull request opened') : ''}>${ran ? t('Started') : hook ? t('Test') : t('Run now')}</button>
          <button type="button" onClick=${onEdit}>${t('Edit')}</button><button type="button" class="bdel" onClick=${() => setAsk(true)}>${t('Delete')}</button></span>`}
      ${err ? html`<span class="berr">${err}</span>` : null}</span>
    <button type="button" class=${`bswitch${isOn(r) ? ' on' : ''}`} role="switch" aria-checked=${isOn(r)} aria-label=${`${r.title}: ${isOn(r) ? t('on') : t('off')}`}
      title=${isOn(r) ? t('On: switch off') : t('Off: switch on')} onClick=${() => post('/routines', { id: r.id, on: !isOn(r) })}><i></i></button>
  </li>`;
}
