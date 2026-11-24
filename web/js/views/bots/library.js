// The Library tab: what the bot learned — the shared memory of every bot, its own memory.md and its skills (its own
// and the shared ones of every bot) — to read, correct or delete.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { SKILL_START, Editor, Skill, titlesOf, activated } from './skills.js';
import { Memory } from './memory.js';
import { MailScript } from './mailscript.js';
import { t } from '../../i18n.js';

export function Library({ bot }) {
  const path = `/api/bots/${encodeURIComponent(bot.id)}/library`;
  const [lib, reload, err] = useFetch(path, ['library', 'busy'], [bot.id]);
  const [edit, setEdit] = useState(null); /* {memory}, a skill, or {fresh} for a new one */
  const [open, setOpen] = useState('');
  const [fail, setFail] = useState('');
  useEffect(() => { setEdit(null); setOpen(''); }, [bot.id]);
  if (!lib) return html`<p class="bnote">${err ? t('Could not read what {name} learned: {error}', { name: bot.name, error: err.message }) : t('Loading…')}</p>`;
  const save = (body) => api('POST', path, body).then(() => { setEdit(null); setFail(''); reload(); }, (x) => setFail(x.message));
  const editor = (e) => html`<${Editor} key=${e.name || (e.shared ? 'shared-memory' : 'memory')} e=${e} err=${fail} onSave=${save} onCancel=${() => { setEdit(null); setFail(''); }} />`;
  const id = (s) => `${s.draft ? 'draft:' : ''}${s.name}${s.shared}`;
  const list = (skills, none) => (skills.length ? html`<ul class="brout">${skills.map((s) => (edit && !edit.memory && edit.name === s.name && edit.shared === s.shared && !!edit.draft === !!s.draft
    ? html`<li key=${(s.draft ? 'd:' : '') + s.name}>${editor(s)}</li>`
    : html`<${Skill} key=${(s.shared ? 's:' : '') + (s.draft ? 'd:' : '') + s.name} s=${s} titles=${titles} open=${open === id(s)} onOpen=${() => setOpen(open === id(s) ? '' : id(s))}
        onEdit=${() => setEdit(s)} onActivate=${() => save({ name: s.name, shared: s.shared, text: activated(s.text) })} onDelete=${() => save({ name: s.name, shared: s.shared, delete: true })} />`))}</ul>` : html`<p class="bnote">${none}</p>`);
  const titles = titlesOf(lib.template);
  const mem = { edit, setEdit, editor, save };
  return html`<div class="blib">
    <${Memory} title=${t('Shared memory · every bot')} text=${lib.shared_memory} shared=${true} ...${mem}
      none=${t('Nothing shared yet. What holds for every bot (your name, time zone, language) goes here, and every bot reads it before its own memory.')} />
    <${Memory} title=${t('Memory')} text=${lib.memory} shared=${false} ...${mem}
      none=${t('Nothing remembered yet. {name} keeps here what you prefer, what you corrected and what worked, and reads it at every turn.', { name: bot.name })} />
    <h4><span>${t('Skills')}</span>${edit ? null : html`<button type="button" class="blink" onClick=${() => setEdit({ fresh: true, text: lib.template || SKILL_START })}>${t('+ New skill')}</button>`}</h4>
    ${edit && edit.fresh ? editor(edit) : null}
    ${list(lib.skills.filter((s) => !s.shared), t('No skill yet. When {name} works out a procedure it will repeat, it writes it down here and uses it next time.', { name: bot.name }))}
    <h4><span>${t('Shared skills · every bot')}</span></h4>
    ${list(lib.skills.filter((s) => s.shared), t("None yet. A skill shared here is in every bot's skills."))}
    <${MailScript} bot=${bot} />
  </div>`;
}
