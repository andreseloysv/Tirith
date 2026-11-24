// What the welcome's buttons do on the daemon: the assistant made with the name the owner gave it, then its
// opening words kept in its chat (so the conversation goes on there) and the owner's choice carried out.
import { api } from '../../store.js';
import { go } from '../../common.js';
import { said, suggestAsk } from './lines.js';

/** The owner's first bot, named as they said: the look a new bot gets in its card. */
export const makeBot = (name) => api('POST', '/api/bots', { name, shape: 'cloud', color: '#ec4899', session: 'chat', access: '' });

/** The morning email routine, as the email template's bot has it. */
async function emailRoutine(id) {
  const all = await api('GET', '/api/bots/templates');
  const routine = ((all || []).find((x) => x.id === 'resumen-correo') || {}).routine;
  if (routine) await api('POST', `/api/bots/${encodeURIComponent(id)}/routines`, routine);
}

/** Writes the welcome into the bot's chat, does what was chosen, and opens that chat. */
export async function choose(choice, st) {
  const { id, name } = st.bot;
  const project = `bot-${id}`;
  await api('POST', `/api/bots/${encodeURIComponent(id)}/intro`, { texts: said(st.owner, name) });
  if (choice === 'suggest') await api('POST', '/api/chat', { text: suggestAsk(), project, attachments: [] });
  if (choice === 'email') await emailRoutine(id);
  go('bots', id);
}
