// The Provider section: one button per agent; picking one clears the model and the effort.
import { PROVIDERS } from './options.js';
import { button, section } from './dom.js';

export function drawProviders(ctl, root, { choices, value, disabled }) {
  const providers = section(root, 'Provider', 'providers');
  for (const agent of choices.agents || []) {
    providers.append(button(disabled, PROVIDERS[agent] || agent, agent === value.agent, () => ctl.change({ agent, model: '', effort: '' })));
  }
}
