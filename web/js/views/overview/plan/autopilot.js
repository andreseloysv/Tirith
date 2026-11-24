// A project's autopilot line above its plan card: on or off, the agents working and the open items, Pause/Resume.
import { html, useState, useEffect } from '../../../../vendor/preact-htm.js';
import { api } from '../../../store.js';
import { Svg, PAUSE, RESUME } from '../bits.js';
import { t } from '../../../i18n.js';

export function Autopilot({ p, working, open, total }) {
  const [want, setWant] = useState(null);
  const [err, setErr] = useState('');
  const on = want != null && want !== !!p.autopilot ? want : !!p.autopilot;
  useEffect(() => { if (want === !!p.autopilot) setWant(null); }, [p.autopilot]);
  const flip = () => {
    const next = !on;
    setWant(next); setErr('');
    api('POST', `/api/autopilot/${encodeURIComponent(p.id)}`, { on: next }).catch((e) => { setWant(null); setErr(String(e.message || e)); });
  };
  const said = working === 1 ? t('1 agent working') : t('{n} agents working', { n: working });
  const bits = on ? [working ? said : null, open ? t('{n} open', { n: open }) : total ? t('all done') : t('nothing planned yet')].filter(Boolean) : [t('Paused')];
  return html`<div class=${`pauto${on ? ' on' : ''}`}><i class="pdot3"></i>
    <span><b>${on ? t('Autopilot on') : t('Autopilot off')}</b>${bits.map((b) => html` <span class="mut">·</span> ${b}`)}</span>
    <button class="ibtn" onClick=${flip} title=${on ? t('Stop starting lanes for open items; running lanes finish') : t('Start lanes for the open items again')}>
      <${Svg} s=${on ? PAUSE : RESUME} /><span>${on ? t('Pause') : t('Resume')}</span></button>
    ${err ? html`<span class="ierr ovinl">${err}</span>` : null}</div>`;
}
