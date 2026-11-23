// The shell: the sidebar, the header and the current view, with the add-project, bot and template cards over any view.
import { html, useState, useEffect } from '../../vendor/preact-htm.js';
import { useStore, setUi } from '../store.js';
import { go } from '../common.js';
import { BotCard, BotTemplates } from '../views/bots.js';
import { AddProject } from '../views/addproject.js';
import { viewOf } from './views.js';
import { useRoute } from './route.js';
import { Nav } from './nav.js';
import { Top } from './top.js';

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

export function App() {
  const r = useRoute();
  const s = useStore();
  const [adding, setAdding] = useState(false);
  useEffect(() => { setUi('q', ''); }, [r.project]); /* a search belongs to the project it was typed in */
  const View = viewOf(r.view)[2];
  const project = r.view === 'settings' ? '' : r.project; /* for the bots view: the bot's id */
  const card = s.botCard;
  return html`<div id="shell">
    <${Nav} r=${r} onAdd=${() => setAdding(true)} />
    <div id="content">
      <${Top} r=${r} />
      ${!s.connected && s.loaded ? html`<div id="stale" class="stale" role="status">OpenBot is not updating — reconnecting…</div>` : null}
      <div id="view" class=${`view v-${r.view}`}>${s.loaded ? html`<${View} project=${project} />` : null}</div>
    </div>
  </div>${adding ? html`<${AddProject} onClose=${() => setAdding(false)} />` : null}
  ${card ? botCard(card) : null}
  ${s.botTemplates ? botTemplates() : null}`;
}
