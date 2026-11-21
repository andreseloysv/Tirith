// The bot's panel beside its chat: the face (asleep, on watch or working), its name and state, and three tabs —
// Routines, Library, Enseñar, Números and Phone.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { Face } from './face.js';
import { stateOf, factoryLine } from './state.js';
import { soonest, whenLabel } from './when.js';
import { RoutineForm, Routine, isHook } from './routines.js';
import { Webhook } from './webhook.js';
import { Library } from './library.js';
import { Phone } from './phone.js';
import { Teach } from './teach.js';
import { Numbers } from './numbers.js';

const TABS = [['routines', 'Routines'], ['library', 'Library'], ['teach', 'Enseñar'], ['numbers', 'Números'], ['phone', 'Phone']];

/** The Routines tab's list, with the form in place of the routine being changed. */
function routineList(bot, list, form, setForm, done) {
  if (list.length) {
    return html`<ul class="brout">${list.map((r) => (form && form.id === r.id
          ? html`<li key=${r.id}><${RoutineForm} bot=${bot} r=${r} onDone=${done} /></li>`
          : html`<${Routine} key=${r.id} bot=${bot} r=${r} onEdit=${() => setForm(r)} />`))}</ul>`;
  }
  return form ? null : html`<p class="bnote">No routine yet. Tell ${bot.name} in the chat what to do and when, as “every weekday at 8, check my inbox for job offers”: it sets the routine up itself. Or add one here.</p>`;
}

function hero(bot, st, status, onEdit) {
  return html`<div class="bhero">
      <${Face} shape=${bot.shape} color=${bot.color} size=${136} state=${st} />
      <h2>${bot.name}</h2>
      <p class="bstat"><i class=${`bdot ${st}`}></i>${status}</p>
      <p class="bcap">${factoryLine(bot)}</p>
      <button type="button" class="blink" onClick=${onEdit}>Edit bot</button>
    </div>`;
}

export function Panel({ bot, busy, onEdit }) {
  const [form, setForm] = useState(null); /* null, {} for a new routine, or the routine being changed */
  const [tab, setTab] = useState('routines');
  useEffect(() => setForm(null), [bot.id]);
  const st = stateOf(bot, busy), next = soonest(bot), list = bot.routines || [];
  const status = st === 'work' ? 'Working' : st === 'watch' ? `On watch${next ? ` · next ${whenLabel(next)}` : ''}` : 'Asleep · no routine on, hears no events';
  const done = () => setForm(null);
  return html`${hero(bot, st, status, onEdit)}
    <section class="bsec">
      <header><div class="btabs" role="tablist">${TABS.map(([k, label]) => html`<button type="button" role="tab" key=${k}
          aria-selected=${tab === k} class=${tab === k ? 'on' : ''} onClick=${() => setTab(k)}>${label}${k === 'routines' && list.length ? html` <span class="bn">${list.length}</span>` : null}</button>`)}</div>
      </header>
      ${form || tab !== 'routines' ? null : html`<div class="baddrow"><button type="button" class="blink" onClick=${() => setForm({})}>+ New routine</button></div>`}
      ${tab === 'library' ? html`<${Library} bot=${bot} />` : tab === 'teach' ? html`<${Teach} key=${bot.id} bot=${bot} />` : tab === 'numbers' ? html`<${Numbers} key=${bot.id} bot=${bot} />` : tab === 'phone' ? html`<${Phone} key=${bot.id} bot=${bot} />` : null}
      ${tab === 'routines' && form && !form.id ? html`<${RoutineForm} bot=${bot} r=${form} onDone=${done} />` : null}
      ${tab !== 'routines' ? null : routineList(bot, list, form, setForm, done)}
      ${tab === 'routines' && !form && list.some(isHook) ? html`<${Webhook} bot=${bot} />` : null}
    </section>`;
}
