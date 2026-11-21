// The Library tab: what the bot learned — the shared memory of every bot, its own memory.md and its skills (its own
// and the shared ones of every bot) — to read, correct or delete.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { SKILL_START, Editor, Skill, titlesOf } from './skills.js';
import { Memory } from './memory.js';
import { MailScript } from './mailscript.js';

export function Library({ bot }) {
  const path = `/api/bots/${encodeURIComponent(bot.id)}/library`;
  const [lib, reload, err] = useFetch(path, ['library', 'busy'], [bot.id]);
  const [edit, setEdit] = useState(null); /* {memory}, a skill, or {fresh} for a new one */
  const [open, setOpen] = useState('');
  const [fail, setFail] = useState('');
  useEffect(() => { setEdit(null); setOpen(''); }, [bot.id]);
  if (!lib) return html`<p class="bnote">${err ? `Could not read what ${bot.name} learned: ${err.message}` : 'Loading…'}</p>`;
  const save = (body) => api('POST', path, body).then(() => { setEdit(null); setFail(''); reload(); }, (x) => setFail(x.message));
  const editor = (e) => html`<${Editor} key=${e.name || (e.shared ? 'shared-memory' : 'memory')} e=${e} err=${fail} onSave=${save} onCancel=${() => { setEdit(null); setFail(''); }} />`;
  const list = (skills, none) => (skills.length ? html`<ul class="brout">${skills.map((s) => (edit && !edit.memory && edit.name === s.name && edit.shared === s.shared
    ? html`<li key=${s.name}>${editor(s)}</li>`
    : html`<${Skill} key=${(s.shared ? 's:' : '') + s.name} s=${s} titles=${titles} open=${open === s.name + s.shared} onOpen=${() => setOpen(open === s.name + s.shared ? '' : s.name + s.shared)}
        onEdit=${() => setEdit(s)} onDelete=${() => save({ name: s.name, shared: s.shared, delete: true })} />`))}</ul>` : html`<p class="bnote">${none}</p>`);
  const titles = titlesOf(lib.template);
  const mem = { edit, setEdit, editor, save };
  return html`<div class="blib">
    <${Memory} title="Shared memory · every bot" text=${lib.shared_memory} shared=${true} ...${mem}
      none="Nothing shared yet. What holds for every bot (your name, time zone, language) goes here, and every bot reads it before its own memory." />
    <${Memory} title="Memory" text=${lib.memory} shared=${false} ...${mem}
      none=${`Nothing remembered yet. ${bot.name} keeps here what you prefer, what you corrected and what worked, and reads it at every turn.`} />
    <h4><span>Skills</span>${edit ? null : html`<button type="button" class="blink" onClick=${() => setEdit({ fresh: true, text: lib.template || SKILL_START })}>+ New skill</button>`}</h4>
    ${edit && edit.fresh ? editor(edit) : null}
    ${list(lib.skills.filter((s) => !s.shared), `No skill yet. When ${bot.name} works out a procedure it will repeat, it writes it down here and uses it next time.`)}
    <h4><span>Shared skills · every bot</span></h4>
    ${list(lib.skills.filter((s) => s.shared), 'None yet. A skill shared here is in every bot\'s skills.')}
    <${MailScript} bot=${bot} />
  </div>`;
}
