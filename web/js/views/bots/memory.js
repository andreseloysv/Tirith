// The Library's memories: the shared one of every bot (above) and the bot's own, each read, edited or cleared.
import { html, useState } from '../../../vendor/preact-htm.js';
import { md } from '../../md.js';
import { t } from '../../i18n.js';

/** The heading's actions: Edit, and Clear (asked once). */
function actions({ text, shared, ask, setAsk, setEdit, save }) {
  const clear = () => { setAsk(false); save({ memory: true, shared, delete: true }); };
  return html`<span class="bacts">${ask ? html`<span class="bask">${shared ? t('Forget it for every bot?') : t('Forget everything?')}</span>
        <button type="button" class="bdel" onClick=${clear}>${t('Clear')}</button><button type="button" onClick=${() => setAsk(false)}>${t('Keep')}</button>`
      : html`<button type="button" onClick=${() => setEdit({ memory: true, shared, text })}>${t('Edit')}</button>${text.trim() ? html`<button type="button" class="bdel" onClick=${() => setAsk(true)}>${t('Clear')}</button>` : null}`}</span>`;
}

/** One memory: its heading, then its editor while it is being edited, its text, or `none`. */
export function Memory({ title, text, shared, edit, setEdit, editor, save, none }) {
  const [ask, setAsk] = useState(false);
  const mine = edit && edit.memory && !!edit.shared === shared;
  const body = (text || '').trim();
  return html`<h4><span>${title}</span>${edit ? null : actions({ text: text || '', shared, ask, setAsk, setEdit, save })}</h4>
    ${mine ? editor(edit) : body ? html`<div class="bmem md" dangerouslySetInnerHTML=${{ __html: md(body) }} />` : html`<p class="bnote">${none}</p>`}`;
}
