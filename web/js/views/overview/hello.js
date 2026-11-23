// The Home's first lines: a greeting (inside a project, its name) and, in one sentence, what waits for the owner and
// who works.
import { html } from '../../../vendor/preact-htm.js';

const greeting = (h) => (h >= 5 && h < 12 ? 'Good morning' : h >= 12 && h < 18 ? 'Good afternoon' : 'Good evening');
const count = (n, one, many) => `${n} ${n === 1 ? one : many}`;

/** `asks`: open questions; `helpers`: lanes at work; `bosses`: groups at work (a bot alone counts); `line`: false
    when the sleeping agents below say it already. */
export function Hello({ place, asks, helpers, bosses, line: said = true }) {
  const wait = asks ? `${count(asks, 'question waits', 'questions wait')} for you` : 'nothing waits for you';
  const work = helpers ? `${count(helpers, 'helper is', 'helpers are')} working`
    : bosses ? `${count(bosses, 'agent is', 'agents are')} working` : 'nobody is working right now';
  const line = `${wait.charAt(0).toUpperCase()}${wait.slice(1)}, and ${work}.`;
  return html`<header class="hm-hello"><h1>${place || greeting(new Date().getHours())}</h1>${said ? html`<p>${line}</p>` : null}</header>`;
}
