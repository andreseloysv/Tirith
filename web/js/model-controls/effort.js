// The Effort section: one row with only the levels the chosen model takes; absent when it takes none.
import { effortName } from './offer.js';
import { element, button, section } from './dom.js';

export function drawEffort(ctl, root, { value, disabled }, model) {
  if (!model || !model.efforts.length) return;
  const field = section(root, 'Effort', 'effort');
  const row = element('div', 'levels'); row.setAttribute('role', 'group'); row.setAttribute('aria-label', 'Effort');
  for (const e of model.efforts) row.append(button(disabled, effortName(e), e === value.effort, () => ctl.change({ effort: e })));
  field.append(row);
}
