// A routine's own model and effort, in its form: the bot's own unless the owner picks another for this routine
// alone (a quick, cheap one for a simple greeting; a careful one for a long search). Only the models the bot's
// agent runs are offered. The daemon keeps the choice in the bot's routines.toml (bots/routine_model.rs).
import { html } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';
import { offered, effortName, readable } from '../../model-controls.js';
import { keyOf } from './state.js';
import { t } from '../../i18n.js';

/** What the bot itself runs with: its own setting, else the managers'. */
const settingOf = (M, bot) => { const own = M.roles[keyOf(bot)]; return (own && own.agent ? own : M.roles.manager) || {}; };

export function RoutineModel({ bot, model, effort, setModel, setEffort }) {
  const [M] = useFetch('/api/models', ['models']);
  if (!M) return null;
  const own = settingOf(M, bot);
  const models = offered(M.choices).filter((m) => m.agent === own.agent);
  const effortsOf = (slug) => (models.find((m) => m.slug === (slug || own.model)) || {}).efforts || M.choices.efforts || [];
  /* another model keeps the effort only when it takes it */
  const pick = (slug) => { setModel(slug); if (effort && !effortsOf(slug).includes(effort)) setEffort(''); };
  return html`<div class="bfield"><span class="bcap">${t('Runs with')}</span><div class="bown">
      <select aria-label=${t('Model')} value=${model} onChange=${(e) => pick(e.target.value)}>
        <option value="">${t("{name}'s model · {model}", { name: bot.name, model: own.model ? readable(own.model) : t('default') })}</option>
        ${models.map((m) => html`<option key=${m.slug} value=${m.slug}>${m.name}</option>`)}</select>
      <select aria-label=${t('Effort')} value=${effort} onChange=${(e) => setEffort(e.target.value)}>
        <option value="">${t("{name}'s effort · {effort}", { name: bot.name, effort: effortName(own.effort) || t('default') })}</option>
        ${effortsOf(model).map((e) => html`<option key=${e} value=${e}>${effortName(e)}</option>`)}</select></div>
    <span class="bmut">${t('For this routine alone: a quick model for a simple task, a careful one for a hard one.')}</span></div>`;
}

/** A routine's own model and effort as its line says them ('' when it has neither). */
export const ownWords = (r) => [r.model ? readable(r.model) : '', effortName(r.effort)].filter(Boolean).join(' · ');
