// The whole page while no agent has worked here yet (app/shell.js): NeoBot can do nothing without one, so nothing
// else shows, no bots, projects or menu, until one is set up. Once one answers its check, the app opens as usual.
import { html } from '../../../vendor/preact-htm.js';
import { useStore } from '../../store.js';
import { AgentList } from './agents.js';
import { LanguagePicker } from '../settings/language.js';
import { t } from '../../i18n.js';

export function Welcome() {
  const list = useStore().setupAgents || [];
  return html`<div class="hm-page"><div class="home su-welcome">
    <div class="su-lang"><${LanguagePicker} /></div>
    <header class="hm-hello"><h1>${t('Welcome to NeoBot')}</h1>
      <p>${t('NeoBot works through an AI helper you already have. Set one up to begin: it installs and signs in right here.')}</p></header>
    <article class="hm-ask"><${AgentList} list=${list} /></article>
  </div></div>`;
}
