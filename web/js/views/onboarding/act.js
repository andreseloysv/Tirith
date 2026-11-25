// What the welcome's buttons do on the daemon: the first bot called what the owner said, then its opening words
// kept in its chat (so the conversation goes on there) and the owner's choice carried out.
import { api } from '../../store.js';
import { go } from '../../common.js';
import { said, suggestAsk } from './lines.js';

/** The bot called `name` from now on: its card saved with nothing else changed. */
export const rename = (bot, name) => api('POST', '/api/bots', { id: bot.id, name, shape: bot.shape, color: bot.color });

/** The morning email routine, as the email template's bot has it. */
async function emailRoutine(id) {
  const all = await api('GET', '/api/bots/templates');
  const routine = ((all || []).find((x) => x.id === 'resumen-correo') || {}).routine;
  if (routine) await api('POST', `/api/bots/${encodeURIComponent(id)}/routines`, routine);
}

/* the bot's opening words written into its chat; the daemon then owes no welcome (no words: skipped) */
const intro = (id, texts) => api('POST', `/api/bots/${encodeURIComponent(id)}/intro`, { texts });

/** Writes the welcome into the bot's chat, does what was chosen, and opens that chat. `st`: state.js's progress. */
export async function choose(choice, st, owner) {
  const project = `bot-${st.bot}`;
  await intro(st.bot, said(owner, st.was, st.named));
  if (choice === 'suggest') await api('POST', '/api/chat', { text: suggestAsk(), project, attachments: [] });
  if (choice === 'email') await emailRoutine(st.bot);
  go('bots', st.bot);
}

/** The welcome skipped: nothing is written, and the bot's chat opens. */
export async function skip(id) {
  await intro(id, []);
  go('bots', id);
}
