// The model list: every model that can run, as one menu, under its provider's name when there are several (the
// caller's summary already says "Model"). A pick brings its agent along and keeps the effort when the model takes it.
import { PROVIDERS } from './options.js';
import { keptEffort } from './offer.js';
import { element, button } from './dom.js';

export function drawModels(ctl, root, { value, disabled }, list) {
  const field = element('div', 'model-field'); field.setAttribute('role', 'group'); field.setAttribute('aria-label', 'Model');
  const agents = [...new Set(list.map(m => m.agent))];
  for (const agent of agents) {
    const name = PROVIDERS[agent] || agent;
    if (agents.length > 1) field.append(element('p', 'group', name));
    const menu = element('div', 'models'); menu.setAttribute('role', 'group'); menu.setAttribute('aria-label', name);
    for (const m of list.filter(x => x.agent === agent)) {
      const pick = () => ctl.change({ agent: m.agent, model: m.slug, effort: keptEffort(value.effort, m.efforts) });
      menu.append(button(disabled, m.name, m.slug === value.model, pick));
    }
    field.append(menu);
  }
  if (!list.length) field.append(element('p', 'hint', 'No agent can run a model: connect one in Settings.'));
  root.append(field);
}
