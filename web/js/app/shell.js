// The shell: the sidebar, the header and the current view, with the add-project, bot and template cards over any view.
import { html, useState, useEffect } from '../../vendor/preact-htm.js';
import { useStore, setUi } from '../store.js';
import { go } from '../common.js';
import { BotCard, BotTemplates, BotPackage } from '../views/bots.js';
import { AddProject } from '../views/addproject.js';
import { viewOf } from './views.js';
import { useRoute } from './route.js';
import { Nav } from './nav.js';
import { Top } from './top.js';
import { useAgents, firstRun } from '../views/setup/agents.js';
import { Welcome } from '../views/setup/welcome.js';
import { t } from '../i18n.js';

/* the bot's card (new or edit), drawn here, over any view */
function botCard(card) {
  const saved = (b) => { setUi('botCard', null); go('bots', b ? b.id : ''); };
  return html`<${BotCard} key=${card.id || 'new'} bot=${card} onClose=${() => setUi('botCard', null)} onSaved=${saved} />`;
}

/* the bot templates, drawn here as the card is */
function botTemplates() {
  const made = (b) => { setUi('botTemplates', null); go('bots', b.id); };
  return html`<${BotTemplates} onClose=${() => setUi('botTemplates', null)} onMade=${made} />`;
}

/* a bot package, imported or exported (`bot` set), drawn here too */
function botPackage(ui) {
  const made = (b) => { setUi('botPackage', null); go('bots', b.id); };
  return html`<${BotPackage} bot=${ui.bot} onClose=${() => setUi('botPackage', null)} onMade=${made} />`;
}

export function App() {
  const r = useRoute();
  const s = useStore();
  const [adding, setAdding] = useState(false);
  useEffect(() => { setUi('q', ''); }, [r.project]); /* a search belongs to the project it was typed in */
  const View = viewOf(r.view)[2];
  const project = r.view === 'settings' ? '' : r.project; /* for the bots view: the bot's id; Settings: `sub` (access) */
  const card = s.botCard;
  const agents = useAgents();
  const first = firstRun(agents); /* no agent works yet: the setup alone, and nothing that cannot be used */
  const bare = agents === undefined || first;
  const view = first ? html`<${Welcome} />` : html`<${View} project=${project} sub=${r.project} />`;
  return html`<div id="shell">
    <${Nav} r=${r} onAdd=${() => setAdding(true)} bare=${bare} />
    <div id="content">
      ${bare ? null : html`<${Top} r=${r} />`}
      ${!s.connected && s.loaded ? html`<div id="stale" class="stale" role="status">${t('NeoBot is not updating — reconnecting…')}</div>` : null}
      <div id="view" class=${`view v-${first ? 'overview' : r.view}`}>${s.loaded && agents !== undefined ? view : null}</div>
    </div>
  </div>${adding ? html`<${AddProject} onClose=${() => setAdding(false)} />` : null}
  ${card ? botCard(card) : null}
  ${s.botTemplates ? botTemplates() : null}
  ${s.botPackage ? botPackage(s.botPackage) : null}`;
}
