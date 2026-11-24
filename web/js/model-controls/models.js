// The menu's list: the models of the open tab (or every match while searching), a check on the chosen one with its
// effort at its end (a tap opens the efforts), and the tab's less-used models folded under "Other models".
import { effortName, keptEffort, providerName } from './offer.js';
import { element, icon, row } from './dom.js';
import { t } from '../i18n.js';

const FIRST = 5;   /* models of a tab shown before "Other models" (the chosen one is always shown) */

const matches = (m, q) => `${m.name} ${m.slug} ${providerName(m.agent)}`.toLowerCase().includes(q.trim().toLowerCase());

function modelRow(ctl, { value, disabled }, m, searching) {
  const on = m.slug === value.model;
  const more = on && m.efforts.length > 0;
  const pick = () => ctl.change({ agent: m.agent, model: m.slug, effort: keptEffort(value.effort, m.efforts) }, true);
  const again = () => (more ? ctl.set({ view: 'effort' }) : ctl.close());
  const note = on ? effortName(value.effort) : searching ? providerName(m.agent) : '';
  return row({ words: m.name, selected: on, note, more, disabled, action: on ? again : pick });
}

function fold(ctl) {
  const b = element('button', `fold${ctl.ui.others ? ' open' : ''}`); b.type = 'button';
  b.setAttribute('aria-expanded', String(ctl.ui.others));
  b.append(icon('next'), element('span', '', t('Other models')));
  b.onclick = () => ctl.set({ others: !ctl.ui.others });
  return b;
}

export function fillList(ctl, box, config, list, tab) {
  const searching = ctl.ui.query !== null;
  const shown = searching ? list.filter(m => matches(m, ctl.ui.query)) : list.filter(m => m.agent === tab);
  const first = searching ? shown : shown.filter((m, i) => i < FIRST || m.slug === config.value.model);
  const rest = shown.filter(m => !first.includes(m));
  const draw = m => modelRow(ctl, config, m, searching);
  box.replaceChildren(...first.map(draw));
  if (rest.length) box.append(fold(ctl), ...(ctl.ui.others ? rest.map(draw) : []));
  if (!shown.length) box.append(element('p', 'hint', searching ? t('No model matches.') : t('No agent can run a model: connect one in Settings.')));
}
