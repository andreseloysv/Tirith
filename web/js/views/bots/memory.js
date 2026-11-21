// The Library's memories: the shared one of every bot (above) and the bot's own, each read, edited or cleared.
import { html, useState } from '../../../vendor/preact-htm.js';
import { md } from '../../md.js';

/** The heading's actions: Edit, and Clear (asked once). */
function actions({ text, shared, ask, setAsk, setEdit, save }) {
  const clear = () => { setAsk(false); save({ memory: true, shared, delete: true }); };
  return html`<span class="bacts">${ask ? html`<span class="bask">${shared ? 'Forget it for every bot?' : 'Forget everything?'}</span>
        <button type="button" class="bdel" onClick=${clear}>Clear</button><button type="button" onClick=${() => setAsk(false)}>Keep</button>`
      : html`<button type="button" onClick=${() => setEdit({ memory: true, shared, text })}>Edit</button>${text.trim() ? html`<button type="button" class="bdel" onClick=${() => setAsk(true)}>Clear</button>` : null}`}</span>`;
}

/** One memory: its heading, then its editor while it is being edited, its text, or `none`. */
export function Memory({ title, text, shared, edit, setEdit, editor, save, none }) {
  const [ask, setAsk] = useState(false);
  const mine = edit && edit.memory && !!edit.shared === shared;
  const t = (text || '').trim();
  return html`<h4><span>${title}</span>${edit ? null : actions({ text: text || '', shared, ask, setAsk, setEdit, save })}</h4>
    ${mine ? editor(edit) : t ? html`<div class="bmem md" dangerouslySetInnerHTML=${{ __html: md(t) }} />` : html`<p class="bnote">${none}</p>`}`;
}
