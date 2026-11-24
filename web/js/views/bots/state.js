// A bot's conversation, the bot last shown, what its face shows and its factory shape in one line.
import { isOn } from './when.js';
import { t } from '../../i18n.js';

const SEL_KEY = 'tirith.bot';
export const keyOf = (b) => b.key || `bot-${b.id}`;   /* its conversation: bot-<id>, or a system bot's key (ceo, manager-general…) */

export function recall() { try { return localStorage.getItem(SEL_KEY) || ''; } catch { return ''; } }
export function remember(id) { try { if (id) localStorage.setItem(SEL_KEY, id); else localStorage.removeItem(SEL_KEY); } catch { /* private window */ } }

/** What the face shows: working on a turn, on watch (a routine is on, or it hears factory events) or asleep. */
export const stateOf = (bot, busy) => (busy ? 'work' : (bot.routines || []).some(isOn) || (bot.routes || []).length ? 'watch' : 'sleep');
/** A bot's factory shape in one line: its session, its access and the events it hears. */
export const factoryLine = (bot) => t('{session} · {access} · hears {events}', { session: bot.session || 'chat', access: bot.access === 'read' ? t('read-only') : t('can write'),
  events: (bot.routes || []).map((r) => r.on).join(', ') || t('no events') });
