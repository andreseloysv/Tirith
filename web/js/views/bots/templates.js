// «Desde plantilla»: the factory's bot templates, drawn by the app over any view; choosing one makes the bot.
import { html, useState, useEffect, useRef } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { Face } from './face.js';
import { daysLabel, timeLabel } from './when.js';

const runsWhen = (r) => (r.trigger === 'webhook' ? `On an event · ${r.when}` : `${daysLabel(r.days)} around ${timeLabel(r.at)}`);

/** Focus the dialog; Esc closes it. */
function useDialog(box, onClose) {
  useEffect(() => {
    if (box.current) box.current.focus();
    const key = (e) => { if (e.key === 'Escape') { e.preventDefault(); onClose(); } };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, []);
}

export function BotTemplates({ onClose, onMade }) {
  const [list, , failed] = useFetch('/api/bots/templates');
  const [making, setMaking] = useState(false);
  const [err, setErr] = useState('');
  const box = useRef(null);
  useDialog(box, onClose);
  const make = (template) => {
    setMaking(true);
    setErr('');
    api('POST', '/api/bots/from-template', { template }).then(onMade, (e) => { setErr(e.message); setMaking(false); });
  };
  const problem = err || (failed ? `Could not read the templates: ${failed.message}` : '');
  return html`<div class="bmback" onClick=${onClose}>
    <div ref=${box} tabindex="-1" class="bmodal btpls" role="dialog" aria-modal="true" aria-label="Desde plantilla" onClick=${(e) => e.stopPropagation()}>
      <header><h2>Desde plantilla</h2><button type="button" class="bx" aria-label="Close" onClick=${onClose}><${Icon} d=${ICON.close} size=${14} w=${1.6} /></button></header>
      <div class="btlist">
        ${(list || []).map((t) => html`<button type="button" class="btpl" key=${t.id} disabled=${making} onClick=${() => make(t.id)}>
          <span class="rface"><${Face} shape=${t.shape} color=${t.color} size=${40} still /></span>
          <span class="brtxt"><b>${t.name}</b><span class="bmut">${runsWhen(t.routine)}</span></span></button>`)}
        ${!list && !problem ? html`<p class="bnote">Loading…</p>` : null}
        ${problem ? html`<p class="berr">${problem}</p>` : null}
      </div>
    </div>
  </div>`;
}
