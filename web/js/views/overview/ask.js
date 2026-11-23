// One question on the Home, whole: who asks (its face, its name and project, a link to its chat), when, the question
// in full, and its answer where it is read — Yes / No for an approval, Done for something the owner does, and words
// for anything (Enter sends, Shift+Enter is a new line); × closes one that no longer applies.
import { html, useState } from '../../../vendor/preact-htm.js';
import { setUi } from '../../store.js';
import { ago, go } from '../../common.js';
import { md } from '../../md.js';
import { Face } from '../bots/face.js';
import { useAnswer, WORDS } from '../answer.js';
import { titleOf } from './crew.js';

const UP = html`<svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M12 19V5M5.5 11.5L12 5l6.5 6.5" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
const when = (at) => { const a = ago(at); return !a ? '' : a === 'now' ? 'just now' : `${a} ago`; };
const grow = (el) => { el.style.height = 'auto'; el.style.height = `${Math.min(el.scrollHeight, 200)}px`; };

/* the quick answers: Yes / No for an approval, Done for something the owner does */
function quick(o, words, a, send) {
  if (o.kind === 'approval') return html`<button type="button" class="hm-btn hm-yes" disabled=${a.busy} onClick=${() => send('yes')}>Yes</button><button type="button" class="hm-btn" disabled=${a.busy} onClick=${() => send('no')}>No</button>`;
  return words ? null : html`<button type="button" class="hm-btn hm-yes" disabled=${a.busy} onClick=${() => send('Done')}>Done</button>`;
}

export function Ask({ o, w }) {
  const [draft, setDraft] = useState('');
  const a = useAnswer(o);
  const words = WORDS.includes(o.kind) || !!o.lane;
  const send = async (text) => { if (await a.send(text)) setDraft(''); };
  const key = (e) => { if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); send(draft); } };
  const lane = () => { setUi('openLane', o.lane); go('agents', o.project); };
  return html`<article class="hm-ask">
    <header class="hm-who"><span class="hm-face"><${Face} shape=${w.face.shape} color=${w.face.color} size=${30} state="watch" still /></span>
      <button type="button" class="hm-name" onClick=${w.open} title="Open the chat"><b>${w.name}</b>${w.place ? html`<span>${w.place}</span>` : null}</button>
      <span class="hm-when">${when(o.at)}</span>
      ${o.kind === 'lane' ? null : html`<button type="button" class="hm-x" disabled=${a.busy} onClick=${a.close} title="No longer needed" aria-label="Close: no longer needed">×</button>`}</header>
    <div class="hm-q" dangerouslySetInnerHTML=${{ __html: md(o.question) }}></div>
    ${o.lane && o.item ? html`<button type="button" class="hm-about" onClick=${lane}>About “${titleOf(o.item)}”</button>` : null}
    <div class="hm-reply">${quick(o, words, a, send)}
      <label class="hm-box"><textarea rows="1" placeholder=${words ? 'Your answer…' : 'Or write what you need…'} aria-label="Your answer" value=${draft} disabled=${a.busy}
          onInput=${(e) => { setDraft(e.target.value); grow(e.target); }} onKeyDown=${key}></textarea>
        <button type="button" class="hm-send" disabled=${a.busy || !draft.trim()} onClick=${() => send(draft)} aria-label="Send the answer">${UP}</button></label></div>
    ${a.err ? html`<p class="hm-err" role="alert">${a.err}</p>` : null}
  </article>`;
}
