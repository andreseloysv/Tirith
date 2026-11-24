// The Home's first lines: a greeting (inside a project, its name) and, in one sentence, what waits for the owner and
// who works.
import { html } from '../../../vendor/preact-htm.js';
import { t } from '../../i18n.js';

const greeting = (h) => (h >= 5 && h < 12 ? t('Good morning') : h >= 12 && h < 18 ? t('Good afternoon') : t('Good evening'));

/** `asks`: open questions; `helpers`: lanes at work; `bosses`: groups at work (a bot alone counts); `line`: false
    when the sleeping agents below say it already. */
export function Hello({ place, asks, helpers, bosses, line: said = true }) {
  const wait = asks === 1 ? t('1 question waits for you') : asks ? t('{n} questions wait for you', { n: asks }) : t('nothing waits for you');
  const work = helpers === 1 ? t('1 helper is working') : helpers ? t('{n} helpers are working', { n: helpers })
    : bosses === 1 ? t('1 agent is working') : bosses ? t('{n} agents are working', { n: bosses }) : t('nobody is working right now');
  const sentence = t('{wait}, and {work}.', { wait, work });
  const line = `${sentence.charAt(0).toUpperCase()}${sentence.slice(1)}`;
  return html`<header class="hm-hello"><h1>${place || greeting(new Date().getHours())}</h1>${said ? html`<p>${line}</p>` : null}</header>`;
}
