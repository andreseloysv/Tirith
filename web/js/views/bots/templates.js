// From template: the factory's bot templates, drawn by the app over any view; choosing one makes the bot.
import { html, useState, useEffect, useRef } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { Face } from './face.js';
import { daysLabel, timeLabel } from './when.js';
import { t } from '../../i18n.js';

const runsWhen = (r) => (r.trigger === 'webhook' ? t('On an event · {when}', { when: r.when }) : t('{days} around {time}', { days: daysLabel(r.days), time: timeLabel(r.at) }));

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
  const problem = err || (failed ? t('Could not read the templates: {error}', { error: failed.message }) : '');
  return html`<div class="bmback" onClick=${onClose}>
    <div ref=${box} tabindex="-1" class="bmodal btpls" role="dialog" aria-modal="true" aria-label=${t('From template')} onClick=${(e) => e.stopPropagation()}>
      <header><h2>${t('From template')}</h2><button type="button" class="bx" aria-label=${t('Close')} onClick=${onClose}><${Icon} d=${ICON.close} size=${14} w=${1.6} /></button></header>
      <div class="btlist">
        ${(list || []).map((tp) => html`<button type="button" class="btpl" key=${tp.id} disabled=${making} onClick=${() => make(tp.id)}>
          <span class="rface"><${Face} shape=${tp.shape} color=${tp.color} size=${40} still /></span>
          <span class="brtxt"><b>${tp.name}</b><span class="bmut">${runsWhen(tp.routine)}</span></span></button>`)}
        ${!list && !problem ? html`<p class="bnote">${t('Loading…')}</p>` : null}
        ${problem ? html`<p class="berr">${problem}</p>` : null}
      </div>
    </div>
  </div>`;
}
