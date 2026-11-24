// The ⋯ menu of one chat.
import { html, useState, useRef } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { t } from '../../i18n.js';

/** The menu of one chat: clear it (asked once, in the panel), and how long it lives — the owner's own words
    ("24h", "2 days", "one week"; "always" takes the limit away), the current one shown in the field.
    Clearing deletes the conversation's messages and its bot starts fresh; its memory and skills stay. */
export function ChatMenu({ project }) {
  const [keep, reloadKeep] = useFetch(`/api/chat/keep?project=${encodeURIComponent(project)}`, ['chat-keep'], [project]);
  const [val, setVal] = useState(null);   /* null: not being edited, the field shows what is saved */
  const [ask, setAsk] = useState(false);
  const [err, setErr] = useState(null);
  const box = useRef(null);
  const close = () => { if (box.current) box.current.open = false; setAsk(false); };
  const doClear = () => api('POST', '/api/chat/clear', { project })
    .then(() => { setErr(null); close(); }, (e) => setErr(t('Could not clear: {error}', { error: e.message })));
  const doKeep = () => api('POST', '/api/chat/keep', { project, keep: (val || '').trim() })
    .then(() => { setVal(null); setErr(null); reloadKeep(); close(); }, (e) => setErr(e.message));
  return html`<details class="mc-menu" ref=${box}>
    <summary aria-label=${t('Chat options')} title=${t('Chat options')}>⋯</summary>
    <div class="mc-mpanel">
      <label class="mc-mlab" for=${`keep-${project}`}>${t('Chat duration')}</label>
      <div class="mc-mrow">
        <input id=${`keep-${project}`} type="text" value=${val != null ? val : (keep && keep.keep) || ''} placeholder=${t('30m, 1h, 24 h, 2 days…')}
          onInput=${(e) => setVal(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') { e.preventDefault(); doKeep(); } }} />
        <button type="button" onClick=${doKeep}>${t('Save')}</button>
      </div>
      <p class="mc-mhint">${keep && keep.keep_s
        ? t('Messages older than this are deleted and the bot forgets them.')
        : t('No limit: the chat keeps everything.')} ${t('Write “always” for no limit.')}</p>
      ${ask ? html`<p class="mc-mhint">${t('Clear this chat? Its messages are deleted and the bot starts fresh. Its memory, skills and routines stay.')}</p>
        <div class="mc-mrow"><button type="button" class="mc-danger" onClick=${doClear}>${t('Clear chat')}</button>
          <button type="button" onClick=${() => setAsk(false)}>${t('Cancel')}</button></div>`
        : html`<button type="button" class="mc-mlink" onClick=${() => { setErr(null); setAsk(true); }}>${t('Clear chat…')}</button>`}
      ${err ? html`<div class="ierr">${err}</div>` : null}
    </div>
  </details>`;
}
