// The Library tab: what the bot learned — its memory.md and its skills (its own and the shared ones of every
// bot) — to read, correct or delete.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { md } from '../../md.js';
import { SKILL_START, Editor, Skill } from './skills.js';

/** The Memory heading's actions: Edit, and Clear (asked once). */
function memoryActions({ lib, memory, ask, setAsk, setEdit, save }) {
  return html`<span class="bacts">${ask ? html`<span class="bask">Forget everything?</span>
        <button type="button" class="bdel" onClick=${() => save({ memory: true, delete: true })}>Clear</button><button type="button" onClick=${() => setAsk(false)}>Keep</button>`
      : html`<button type="button" onClick=${() => setEdit({ memory: true, text: lib.memory })}>Edit</button>${memory ? html`<button type="button" class="bdel" onClick=${() => setAsk(true)}>Clear</button>` : null}`}</span>`;
}

export function Library({ bot }) {
  const path = `/api/bots/${encodeURIComponent(bot.id)}/library`;
  const [lib, reload, err] = useFetch(path, ['library', 'busy'], [bot.id]);
  const [edit, setEdit] = useState(null); /* {memory}, a skill, or {fresh} for a new one */
  const [open, setOpen] = useState('');
  const [ask, setAsk] = useState(false);
  const [fail, setFail] = useState('');
  useEffect(() => { setEdit(null); setOpen(''); }, [bot.id]);
  if (!lib) return html`<p class="bnote">${err ? `Could not read what ${bot.name} learned: ${err.message}` : 'Loading…'}</p>`;
  const save = (body) => api('POST', path, body).then(() => { setEdit(null); setAsk(false); setFail(''); reload(); }, (x) => setFail(x.message));
  const editor = (e) => html`<${Editor} key=${e.name || 'memory'} e=${e} err=${fail} onSave=${save} onCancel=${() => { setEdit(null); setFail(''); }} />`;
  const list = (skills, none) => (skills.length ? html`<ul class="brout">${skills.map((s) => (edit && !edit.memory && edit.name === s.name && edit.shared === s.shared
    ? html`<li key=${s.name}>${editor(s)}</li>`
    : html`<${Skill} key=${(s.shared ? 's:' : '') + s.name} s=${s} open=${open === s.name + s.shared} onOpen=${() => setOpen(open === s.name + s.shared ? '' : s.name + s.shared)}
        onEdit=${() => setEdit(s)} onDelete=${() => save({ name: s.name, shared: s.shared, delete: true })} />`))}</ul>` : html`<p class="bnote">${none}</p>`);
  const memory = lib.memory.trim();
  return html`<div class="blib">
    <h4><span>Memory</span>${edit ? null : memoryActions({ lib, memory, ask, setAsk, setEdit, save })}</h4>
    ${edit && edit.memory ? editor(edit) : memory ? html`<div class="bmem md" dangerouslySetInnerHTML=${{ __html: md(memory) }} />`
      : html`<p class="bnote">Nothing remembered yet. ${bot.name} keeps here what you prefer, what you corrected and what worked, and reads it at every turn.</p>`}
    <h4><span>Skills</span>${edit ? null : html`<button type="button" class="blink" onClick=${() => setEdit({ fresh: true, text: SKILL_START })}>+ New skill</button>`}</h4>
    ${edit && edit.fresh ? editor(edit) : null}
    ${list(lib.skills.filter((s) => !s.shared), `No skill yet. When ${bot.name} works out a procedure it will repeat, it writes it down here and uses it next time.`)}
    <h4><span>Shared skills · every bot</span></h4>
    ${list(lib.skills.filter((s) => s.shared), 'None yet. A skill shared here is in every bot\'s skills.')}
  </div>`;
}
