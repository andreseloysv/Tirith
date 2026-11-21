// The Library's pieces: one skill (read, edit, delete) and the editor of the memory or of a skill.
import { html, useState } from '../../../vendor/preact-htm.js';
import { md } from '../../md.js';

export const SKILL_START = '---\nname: {name}\ndescription: When to use it, in one sentence.\n---\n\n1. ';
const descOf = (t) => (/^description:\s*(.+)$/m.exec(t || '') || [])[1] || '';
const bodyOf = (t) => String(t || '').replace(/^---\n[\s\S]*?\n---\n?/, '').trim();
/** The section titles of the skill template (factory/skill.md): its `## ` headings. */
export const titlesOf = (t) => [...String(t || '').matchAll(/^##\s+(.+?)\s*$/gm)].map((m) => m[1]);
/** The template's sections a skill lacks, by their titles. */
export const missingIn = (text, titles) => {
  const has = new Set([...String(text || '').matchAll(/^##\s+(.+?)\s*$/gm)].map((m) => m[1].toLowerCase()));
  return titles.filter((t) => !has.has(t.toLowerCase()));
};

export function Editor({ e, onSave, onCancel, err }) {
  const [name, setName] = useState(e.name || '');
  const [text, setText] = useState(e.text || '');
  const [shared, setShared] = useState(!!e.shared);
  const named = !e.fresh || /^[a-z][a-z0-9-]{0,63}$/.test(name);
  const body = e.memory ? { memory: true, shared: !!e.shared, text } : { name: e.fresh ? name : e.name, shared: e.fresh ? shared : e.shared, text: e.fresh ? text.replace('{name}', name) : text };
  return html`<div class="bform bedit">
    ${e.fresh ? html`<label><span class="bcap">Name · lowercase and dashes</span><input value=${name} placeholder="check-job-portal" maxlength="64" onInput=${(x) => setName(x.target.value.trim())} /></label>
      <label class="bchk"><input type="checkbox" checked=${shared} onChange=${(x) => setShared(x.target.checked)} />Shared with every bot</label>` : null}
    <textarea class="bcode" rows=${e.memory ? 12 : 14} value=${text} aria-label=${e.memory ? (e.shared ? 'Shared memory' : 'Memory') : 'Skill'} onInput=${(x) => setText(x.target.value)}></textarea>
    ${err ? html`<p class="berr">${err}</p>` : null}
    <div class="bfoot"><button type="button" class="bbtn" onClick=${onCancel}>Cancel</button>
      <button type="button" class="bbtn pri" disabled=${!named} onClick=${() => onSave(body)}>Save</button></div>
  </div>`;
}

export function Skill({ s, titles = [], open, onOpen, onEdit, onDelete }) {
  const [ask, setAsk] = useState(false);
  const missing = missingIn(s.text, titles);
  return html`<li class="bsk">
    <button type="button" class="bskh" aria-expanded=${open} onClick=${onOpen}><b>${s.name}</b><span class="bmut">${descOf(s.text)}</span></button>
    ${missing.length ? html`<p class="berr bmiss">Missing: ${missing.join(' · ')}</p>` : null}
    ${open ? html`<div class="bskb md" dangerouslySetInnerHTML=${{ __html: md(bodyOf(s.text)) }} />` : null}
    <span class="bacts">${ask ? html`<span class="bask">Delete ${s.shared ? 'this skill for every bot' : 'this skill'}?</span>
        <button type="button" class="bdel" onClick=${onDelete}>Delete</button><button type="button" onClick=${() => setAsk(false)}>Keep</button>`
      : html`<button type="button" onClick=${onOpen}>${open ? 'Hide' : 'Read'}</button><button type="button" onClick=${onEdit}>Edit</button>
        <button type="button" class="bdel" onClick=${() => setAsk(true)}>Delete</button>`}</span>
  </li>`;
}
