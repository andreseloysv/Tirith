// The bot card's Factory section: its session, its access and the events it hears, each as a turn or a note,
// with its chat line and phones.
import { html } from '../../../vendor/preact-htm.js';
import { factoryLine } from './state.js';

/** One heard event: which, how, where it shows, and Remove. */
function route(r, i, { EVENTS, setRoute, routes, setRoutes }) {
  return html`<div class="broute" key=${i}>
            <select class="bsel" aria-label="Event" value=${r.on} onChange=${(e) => setRoute(i, { on: e.target.value })}>${EVENTS.map((ev) => html`<option key=${ev} value=${ev}>${ev}</option>`)}</select>
            <select class="bsel" aria-label="Mode" value=${r.mode || 'turn'} onChange=${(e) => setRoute(i, { mode: e.target.value })}>
              <option value="turn">turn — wake it now</option><option value="note">note — with its next turn</option></select>
            <label class="bchk"><input type="checkbox" checked=${!!r.chat} onChange=${(e) => setRoute(i, { chat: e.target.checked })} />chat line</label>
            <label class="bchk"><input type="checkbox" checked=${!!r.phone} onChange=${(e) => setRoute(i, { phone: e.target.checked })} />phone</label>
            ${r.after_minutes ? html`<span class="bcap">after ${r.after_minutes} min</span>` : null}
            <button type="button" class="blink bdel" onClick=${() => setRoutes(routes.filter((_, j) => j !== i))}>Remove</button>
          </div>`;
}

/** f: {bot, session, setSession, access, setAccess, routes, setRoutes, EVENTS}. */
export function factorySection(f) {
  const { bot, session, setSession, access, setAccess, routes, setRoutes, EVENTS } = f;
  const setRoute = (i, patch) => setRoutes(routes.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  return html`<details class="bmodel bfactory"><summary><span class="bcap">Factory</span> ${factoryLine({ session, access, routes })}</summary>
          <p class="bnote">All that makes a bot a CEO or a judge is this configuration${bot.system ? ' — this is a system bot, changed like any other' : ''}. It is saved in config.toml, where its routes replace the factory's for it. Its routines stay its schedule routes, under Routines.</p>
          <label class="bfrow"><span class="bcap">Session</span><select class="bsel" aria-label="Session" value=${session} onChange=${(e) => setSession(e.target.value)}>
            <option value="chat">chat — a long-lived conversation</option><option value="turn">turn — one fresh run, like the CEO</option></select></label>
          <label class="bfrow"><span class="bcap">Access</span><select class="bsel" aria-label="Access" value=${access} onChange=${(e) => setAccess(e.target.value)}>
            <option value="">can write</option><option value="read">read-only, like the CEO and the judge</option></select></label>
          <div class="bfrow"><span class="bcap">Heard events</span></div>
          ${routes.map((r, i) => route(r, i, { EVENTS, setRoute, routes, setRoutes }))}
          <button type="button" class="blink" onClick=${() => setRoutes([...routes, { on: EVENTS.includes('lane.failed') ? 'lane.failed' : EVENTS[0], mode: 'turn', chat: true, phone: false }])}>+ Hear an event</button>
        </details>`;
}
