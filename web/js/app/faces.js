// The bots' faces, at the far left of the sidebar: each in its state, with its unread answers; "+" adds a bot.
import { html } from '../../vendor/preact-htm.js';
import { useStore, setUi } from '../store.js';
import { useUnread } from '../unread.js';
import { go } from '../common.js';
import { stateOf, keyOf, recall } from '../views/bots.js';
import { Face, NewFace } from '../views/bots/face.js';
import { BRAND } from './brand.js';

/* one bot's face button; `n` is its unread answers (none for the bot shown) */
function faceButton(b, sel, n, busy) {
  return html`<button type="button" key=${b.id} class=${`nbot${b.id === sel ? ' on' : ''}`} title=${b.name} aria-label=${n ? `${b.name}, ${n} new` : b.name}
        aria-current=${b.id === sel} onClick=${() => go('bots', b.id)}><${Face} shape=${b.shape} color=${b.color} size=${64} state=${stateOf(b, busy)} />
        ${n ? html`<em class="nunread">${n > 9 ? '9+' : n}</em>` : null}</button>`;
}

export function Faces({ r }) {
  const s = useStore();
  const unread = useUnread();
  const sel = r.view === 'bots' ? r.project || recall() || (unread.bots[0] || {}).id : '';
  return html`<nav class="nbots" aria-label="Bots">
    <div class="nlogo" title="Tirith">${BRAND}</div>
    ${unread.bots.filter((b) => b.show !== false).map((b) => faceButton(b, sel, b.id === sel ? 0 : unread.by[b.id] || 0, !!s.busy[keyOf(b)]))}
    <button type="button" class="nbot nnew" title="New bot" aria-label="New bot" onClick=${() => setUi('botCard', {})}><${NewFace} size=${46} /></button>
  </nav>`;
}
