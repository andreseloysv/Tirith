// The bot's panel beside its chat: the face (asleep, on watch or working), its name and state, and its tabs —
// Routines, Library, Teach, Numbers, Improvements (the Coach's experiments), Animation and Phone.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { Face } from './face.js';
import { stateOf, factoryLine, keyOf } from './state.js';
import { useReaction, useWaking } from './face/reaction.js';
import { lookOf } from './face/face-map.js';
import { soonest, whenLabel } from './when.js';
import { RoutineForm, Routine, isHook } from './routines.js';
import { Webhook } from './webhook.js';
import { Library } from './library.js';
import { Phone } from './phone.js';
import { Teach } from './teach.js';
import { Numbers } from './numbers.js';
import { Lab } from './lab.js';
import { AnimLab } from './anim.js';
import { t } from '../../i18n.js';

const TABS = [['routines', t('Routines')], ['library', t('Library')], ['teach', t('Teach')], ['numbers', t('Numbers')], ['lab', t('Improvements')],
  ['anim', t('Animation')], ['phone', t('Phone')]];

/** The Routines tab's list, with the form in place of the routine being changed. */
function routineList(bot, list, form, setForm, done) {
  if (list.length) {
    return html`<ul class="brout">${list.map((r) => (form && form.id === r.id
          ? html`<li key=${r.id}><${RoutineForm} bot=${bot} r=${r} onDone=${done} /></li>`
          : html`<${Routine} key=${r.id} bot=${bot} r=${r} onEdit=${() => setForm(r)} />`))}</ul>`;
  }
  return form ? null : html`<p class="bnote">${t('No routine yet. Tell {name} in the chat what to do and when, as “every weekday at 8, check my inbox for job offers”: it sets the routine up itself. Or add one here.', { name: bot.name })}</p>`;
}

function hero(bot, st, status, onEdit, shown = st) {
  return html`<div class="bhero">
      <${Face} ...${lookOf(bot)} size=${136} state=${shown} />
      <h2>${bot.name}</h2>
      <p class="bstat"><i class=${`bdot ${st}`}></i>${status}</p>
      <p class="bcap">${factoryLine(bot)}</p>
      <button type="button" class="blink" onClick=${onEdit}>${t('Edit bot')}</button>
    </div>`;
}

export function Panel({ bot, busy, onEdit }) {
  const [form, setForm] = useState(null); /* null, {} for a new routine, or the routine being changed */
  const [tab, setTab] = useState('routines');
  useEffect(() => setForm(null), [bot.id]);
  const st = stateOf(bot, busy), next = soonest(bot), list = bot.routines || [];
  const shown = useWaking(st) || useReaction(keyOf(bot), busy) || st; /* the hero's face, reactions included */
  const status = st === 'work' ? t('Working') : st === 'watch' ? t('On watch') + (next ? ` · ${t('next {when}', { when: whenLabel(next) })}` : '') : t('Asleep · no routine on, hears no events');
  const done = () => setForm(null);
  return html`${hero(bot, st, status, onEdit, shown)}
    <section class="bsec">
      <header><div class="btabs" role="tablist">${TABS.map(([k, label]) => html`<button type="button" role="tab" key=${k}
          aria-selected=${tab === k} class=${tab === k ? 'on' : ''} onClick=${() => setTab(k)}>${label}${k === 'routines' && list.length ? html` <span class="bn">${list.length}</span>` : null}</button>`)}</div>
      </header>
      ${form || tab !== 'routines' ? null : html`<div class="baddrow"><button type="button" class="blink" onClick=${() => setForm({})}>${t('+ New routine')}</button></div>`}
      ${tab === 'library' ? html`<${Library} bot=${bot} />` : tab === 'teach' ? html`<${Teach} key=${bot.id} bot=${bot} onRoutine=${(r) => { setTab('routines'); setForm(r); }} />` : tab === 'numbers' ? html`<${Numbers} key=${bot.id} bot=${bot} />` : tab === 'lab' ? html`<${Lab} key=${bot.id} bot=${bot} />` : tab === 'anim' ? html`<${AnimLab} key=${bot.id} bot=${bot} />` : tab === 'phone' ? html`<${Phone} key=${bot.id} bot=${bot} />` : null}
      ${tab === 'routines' && form && !form.id ? html`<${RoutineForm} bot=${bot} r=${form} onDone=${done} />` : null}
      ${tab !== 'routines' ? null : routineList(bot, list, form, setForm, done)}
      ${tab === 'routines' && !form && list.some(isHook) ? html`<${Webhook} bot=${bot} />` : null}
    </section>`;
}
