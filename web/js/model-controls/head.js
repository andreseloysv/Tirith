// The menu's head: one tab per provider that has models (the chosen model's open first) and the search button.
// While searching, the search field takes the whole head and the list shows every provider's matches.
import { providerName } from './offer.js';
import { element, icon } from './dom.js';
import { t } from '../i18n.js';

const iconButton = (name, label, action) => {
  const b = element('button', 'ib'); b.type = 'button'; b.title = label; b.setAttribute('aria-label', label);
  b.append(icon(name)); b.onclick = action;
  return b;
};

function searchField(ctl) {
  const input = element('input', 'find'); input.type = 'search'; input.placeholder = t('Search models');
  input.setAttribute('aria-label', t('Search models')); input.value = ctl.ui.query;
  input.oninput = () => { ctl.ui.query = input.value; ctl.fillList(); };
  requestAnimationFrame(() => input.focus());
  return input;
}

export function drawHead(ctl, root, agents, tab) {
  const head = element('div', 'head');
  if (ctl.ui.query !== null) {
    head.append(icon('search'), searchField(ctl), iconButton('close', t('Close the search'), () => ctl.set({ query: null })));
  } else {
    const tabs = element('div', 'tabs'); tabs.setAttribute('role', 'tablist');
    for (const agent of agents) {
      const b = element('button', 'tab', providerName(agent)); b.type = 'button';
      b.setAttribute('role', 'tab'); b.setAttribute('aria-selected', String(agent === tab));
      b.onclick = () => ctl.set({ tab: agent, others: false });
      tabs.append(b);
    }
    head.append(tabs, iconButton('search', t('Search models'), () => ctl.set({ query: '' })));
  }
  root.append(head);
}
