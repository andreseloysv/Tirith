// The faces at the far left of the sidebar (faces/list.js): each agent in its state with its unread answers, in the
// owner's order — drag one up or down to choose its place (faces/drag.js, faces/order.js) — and, while the pointer
// rests on one, its card: the name, then where it works or its last message (faces/hover.js, faces/where.js).
// "+" adds a bot.
import { html, useState } from '../../vendor/preact-htm.js';
import { useStore, setUi } from '../store.js';
import { Face, NewFace } from '../views/bots/face.js';
import { BRAND } from './brand.js';
import { useFaces } from './faces/list.js';
import { whereOf } from './faces/where.js';
import { loadOrder, saveOrder, moved } from './faces/order.js';
import { useDrag } from './faces/drag.js';
import { useHoverCard, HoverCard } from './faces/hover.js';

/* one face: a click opens its conversation; a drag moves it; resting on it shows its card */
function faceButton(f, sel, drag, card) {
  const n = f.unread;
  const show = (e) => card.show(f.id, e.currentTarget);
  return html`<button type="button" key=${f.id} data-face=${f.id} class=${`nbot${f.id === sel ? ' on' : ''}`}
      aria-label=${n ? `${f.name}, ${n} new` : f.name} aria-current=${f.id === sel} aria-describedby=${card.at && card.at.id === f.id ? 'face-card' : null}
      onPointerDown=${(e) => { card.now(); drag.down(e); }} onPointerMove=${drag.move} onPointerUp=${drag.up} onPointerCancel=${drag.cancel}
      onClick=${() => { if (!drag.swallowed()) f.open(); }} onKeyDown=${(e) => drag.key(e, f.id)}
      onPointerEnter=${(e) => { if (!e.buttons && e.pointerType !== 'touch') show(e); }} onPointerLeave=${card.hide}
      onFocus=${(e) => { if (e.currentTarget.matches(':focus-visible')) show(e); }} onBlur=${card.now}>
    <${Face} ...${f.look} size=${64} state=${f.state} />
    ${n ? html`<em class="nunread">${n > 9 ? '9+' : n}</em>` : null}</button>`;
}

export function Faces({ r }) {
  const s = useStore();
  const [order, setOrder] = useState(loadOrder);
  const { faces, sel } = useFaces(r, order);
  const card = useHoverCard();
  const ids = faces.map((f) => f.id);
  const place = (id, to) => { const next = moved(ids, id, to); saveOrder(next); setOrder(next); };
  const drag = useDrag(ids, place, card.now);
  const shown = card.at && faces.find((f) => f.id === card.at.id);
  return html`<nav class="nbots" aria-label="Agents" onScroll=${card.now}>
    <div class="nlogo" title="OpenBot">${BRAND}</div>
    ${faces.map((f) => faceButton(f, sel, drag, card))}
    <button type="button" class="nbot nnew" title="New bot" aria-label="New bot" onClick=${() => setUi('botCard', {})}><${NewFace} size=${46} /></button>
    ${shown ? html`<${HoverCard} face=${shown} at=${card.at} line=${whereOf(shown, s)} />` : null}
  </nav>`;
}
