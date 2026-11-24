// The two layouts' pieces: a kanban column of lane cards, a row of the list (open: the lane's detail), and the
// right drawer with one lane's detail.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { useStore } from '../../store.js';
import { ICON, Icon, Span, laneLabel } from '../../common.js';
import { CAP, DOT, startOf, endOf, itemText, questionOf, statusOf } from './lanes.js';
import { Avatar } from './stop.js';
import { LaneCard } from './card.js';
import { LaneDetail } from './detail.js';
import { t } from '../../i18n.js';

export function Drawer({ a, projName, color, onClose }) {
  const q = questionOf(useStore(), a);
  useEffect(() => {
    const key = (e) => { if (e.key === 'Escape' && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) onClose(); };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, [onClose]);
  return html`<aside class="drawer" role="dialog" aria-label=${itemText(a.item)}>
    <header><span class="dhead"><${Avatar} a=${a} color=${color} size=${28} /><span class="dtit"><b>${itemText(a.item)}</b>
      <span class="mut">${a.agent}${a.model ? ` · ${a.model}` : ''} · ${statusOf(a, q)}</span></span></span>
      <button class="x" aria-label=${t('Close')} onClick=${onClose}><${Icon} d=${ICON.close} size=${14} w=${2} /></button></header>
    <div class="dbody" key=${a.id}><${LaneDetail} a=${a} projName=${projName} color=${color} /></div>
  </aside>`;
}

export function Column({ col, lanes, sel, onOpen, colorOf, nameOf, doing, qOf }) {
  const [cap, setCap] = useState(CAP);
  return html`<section class=${`col ${col[2]}`}>
    <header><b>${col[1]}</b><span class="n">${lanes.length}</span></header>
    <div class="cbody">${lanes.slice(0, cap).map((a) => html`<${LaneCard} key=${a.id} a=${a} sel=${sel === a.id} color=${colorOf(a.project)} projName=${nameOf(a.project)} onOpen=${onOpen} doing=${doing[a.id]} q=${qOf(a)} />`)}
      ${lanes.length > cap ? html`<button class="link lshow" onClick=${() => setCap(cap + CAP)}>${t('Show {n} more', { n: Math.min(CAP, lanes.length - cap) })}</button>` : null}</div>
  </section>`;
}

export function ListRow({ a, open, onOpen, color, projName }) {
  const q = questionOf(useStore(), a);
  return html`<li class=${`row${open ? ' open' : ''}`}>
    <button class="head agent" aria-expanded=${open} onClick=${() => onOpen(open ? '' : a.id)}>
      <span class="chv"></span><i class=${`dot ${DOT[a.status] || 'grey'}`} title=${laneLabel(a.status)}></i>
      <span class="name" title=${a.id}>${a.agent} <span class="mut sm">${statusOf(a, q)}${a.attempts ? ` · ${t('try {n}', { n: a.attempts + 1 })}` : ''}</span></span>
      <span class="feat">${itemText(a.item)}</span><span class="mut proj">${projName}</span>
      <span class="time"><${Span} start=${startOf(a)} end=${endOf(a)} /></span></button>
    ${open ? html`<div class="detail"><${LaneDetail} a=${a} projName=${projName} color=${color} /></div>` : null}
  </li>`;
}
