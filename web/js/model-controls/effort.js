// The efforts of the chosen model: back to the models in the head, then the levels it takes, a check on the chosen one.
import { effortName } from './offer.js';
import { element, icon, row } from './dom.js';
import { t } from '../i18n.js';

export function drawEffort(ctl, root, { value, disabled }, model) {
  const head = element('div', 'head');
  const back = element('button', 'back'); back.type = 'button'; back.setAttribute('aria-label', t('Back to the models'));
  back.append(icon('back'), element('span', '', model.name));
  back.onclick = () => ctl.set({ view: 'models' });
  head.append(back);
  const box = element('div', 'list'); box.setAttribute('role', 'menu'); box.setAttribute('aria-label', t('Effort'));
  box.append(element('p', 'label', t('Effort')));
  for (const e of model.efforts) {
    const action = () => (e === value.effort ? ctl.close() : ctl.change({ effort: e }, true));
    box.append(row({ words: effortName(e), selected: e === value.effort, disabled, action }));
  }
  root.append(head, box);
}
