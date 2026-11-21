// The box to write in: its error line, the files being attached, the text, the clip and Send.
import { html } from '../../../vendor/preact-htm.js';
import { CLIP, SEND } from './icons.js';
import { size } from './util.js';
import { FileChip } from './files.js';

/** The files waiting to go with the next message: pictures as thumbnails, the rest as chips. */
function strip(pics, unpic) {
  if (!pics.length) return null;
  return html`<div class="mc-strip">${pics.map((p, i) => (p.url
    ? html`<span class="mc-th" title=${p.name}><span class="mc-thi"><img src=${p.url} alt=${p.name} />
        <button type="button" aria-label=${`Remove ${p.name}`} title="Remove" onClick=${() => unpic(i)}>×</button></span>
        <span class="mc-thn">${p.name}</span><span class="mc-ths">${size(p.size)}</span></span>`
    : html`<${FileChip} name=${p.name} bytes=${p.size} title=${p.name} onRemove=${() => unpic(i)} />`))}</div>`;
}

/** v: {sp, err, att: useAttach's answer, ta, file, draft, edit, keys: boxKeys' answer, send}. */
export function composer({ sp, err, att, ta, file, draft, edit, keys, send }) {
  const { isProject, whoName, name } = sp;
  return html`<div class="mc-comp">
      ${err ? html`<div class="ierr mc-note">${err}</div>` : null}
      <div class="mc-box">
        ${strip(att.pics, att.unpic)}
        <textarea ref=${ta} rows="1" maxlength="4000" placeholder=${isProject ? `Message the ${whoName} of ${name}…` : `Message ${whoName}…`} aria-label=${`Message to ${whoName}`}
          value=${draft} onInput=${(e) => edit(e.target.value)} onKeyDown=${keys.onKey} onPaste=${keys.onPaste}></textarea>
        <div class="mc-row">
          <button class="mc-clip" type="button" aria-label="Attach files" title="Attach images or files (or paste, or drop them here)" onClick=${() => file.current.click()}>${CLIP}</button>
          <input type="file" ref=${file} multiple hidden onChange=${(e) => { att.addFiles(e.target.files); e.target.value = ''; }} />
          <span class="mc-hint">Enter sends · Shift+Enter new line</span>
          <button class="mc-send" type="button" aria-label="Send" title=${`Send to ${whoName}`} onClick=${send}>${SEND}</button>
        </div>
      </div>
    </div>`;
}
