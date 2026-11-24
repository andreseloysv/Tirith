// One bot (#/bots/<id>): its conversation and its panel, in the content area like any other view.
import { html, useEffect } from '../../../vendor/preact-htm.js';
import { S, useStore, useFetch, setUi } from '../../store.js';
import { Chat } from '../chat.js';
import { markSeen } from '../../unread.js';
import { keyOf, recall, remember } from './state.js';
import { Panel } from './panel.js';
import { Intro } from './intro.js';

/** What the owner has on the screen is read. */
function useSeen(bot, talk) {
  useEffect(() => {
    const read = () => { if (bot && talk && talk.loaded && !document.hidden) markSeen(bot.id, talk.rows); };
    read();
    document.addEventListener('visibilitychange', read);
    return () => document.removeEventListener('visibilitychange', read);
  }, [bot && bot.id, talk && talk.rows.length, talk && talk.loaded]);
}

const pickTemplate = () => setUi('botTemplates', true);
const pickImport = () => setUi('botPackage', {});

/** `project` is the second part of the address: the id of the bot to show (else the last one shown, else the first). */
export function Bots({ project: wanted }) {
  const s = useStore();
  const [bots, , err] = useFetch('/api/bots', ['bots']);
  const bot = bots && (bots.find((b) => b.id === wanted) || bots.find((b) => b.id === recall()) || bots[0]);
  const talk = bot && S.talks[keyOf(bot)];
  useEffect(() => { if (bot) remember(bot.id); }, [bot && bot.id]);
  useSeen(bot, talk);

  if (!bots) {
    return html`<div class="bview bsolo"><p class="bnote">${err ? (/not found/i.test(err.message)
      ? 'The running OpenBot daemon is older than this page and has no bots yet: rebuild and restart it (the page updates on its own).'
      : `Could not read the bots: ${err.message}`) : 'Loading…'}</p></div>`;
  }
  if (!bots.length) return html`<div class="bview bsolo"><${Intro} onNew=${() => setUi('botCard', {})} onTemplate=${pickTemplate} onImport=${pickImport} /></div>`;
  return html`<div class="bview">
    <div class="bchat"><${Chat} key=${keyOf(bot)} project=${keyOf(bot)} who=${bot.name}
      hint=${`Say hello to ${bot.name}. Ask for anything that repeats, as “every weekday at 8, check my inbox for job offers”, and ${bot.name} sets up the routine itself.`} /></div>
    <aside class="bside"><span class="btopen"><button type="button" class="blink" onClick=${pickTemplate}>Desde plantilla</button> <button type="button" class="blink" onClick=${pickImport}>Import</button> <button type="button" class="blink" onClick=${() => setUi('botPackage', { bot })}>Export</button></span>
      <${Panel} bot=${bot} busy=${!!s.busy[keyOf(bot)]} onEdit=${() => setUi('botCard', bot)} /></aside>
  </div>`;
}
