// A lane's avatar and clock, and Stop with a confirm popover (the old stop.js): the lane ends as stopped and its
// worktree is kept.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { ICON, Icon, Span } from '../../common.js';
import { STOPPABLE, itemText, startOf, endOf } from './lanes.js';

export function Avatar({ a, color, size = 36 }) {
  const ch = String(a.agent || a.id || '?').trim().charAt(0).toUpperCase();
  return html`<span class="avatar lava" style=${`width:${size}px;height:${size}px;background:linear-gradient(135deg, ${color}, color-mix(in srgb, ${color} 55%, #1d2433));font-size:${Math.round(size * 0.42)}px`}>${ch}</span>`;
}

export function Clock({ a }) {
  return html`<span class="aclock"><${Icon} d=${ICON.clock} size=${13} w=${1.7} /><${Span} start=${startOf(a)} end=${endOf(a)} /></span>`;
}

function confirm(a, where, stop, toggle) {
  return html`<span class=${`stoppop ${where}`} role="dialog" aria-label="Confirm stop" onClick=${(e) => e.stopPropagation()}>
      <span class="sptxt">Stop <b>${itemText(a.item)}</b>? It ends as stopped and its worktree is kept.</span>
      <span class="spb"><span class="stopgo" role="button" tabindex="0" onClick=${stop}>Stop agent</span>
      <span class="alink" role="button" tabindex="0" onClick=${toggle}>Cancel</span></span></span>`;
}

export function StopBtn({ a, where }) {
  const [ask, setAsk] = useState(false);
  const [pend, setPend] = useState(false);
  const [err, setErr] = useState('');
  if (!STOPPABLE.includes(a.status)) return null;
  if (pend) return html`<span class=${`stopb stopping ${where}`} role="status">Stopping…</span>`;
  const stop = (e) => {
    e.stopPropagation();
    setAsk(false); setPend(true); setErr('');
    api('POST', `/api/lanes/${encodeURIComponent(a.id)}/stop`).catch((x) => { setPend(false); setErr(`Could not stop: ${x.message}`); });
  };
  const toggle = (e) => { e.stopPropagation(); setAsk(!ask); };
  return html`<span class=${`stopb ${where}`} role="button" tabindex="0" title="Stop this lane: it ends as stopped and its worktree is kept"
      onClick=${toggle} onKeyDown=${(e) => (e.key === 'Enter' || e.key === ' ') && toggle(e)}>
      <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><rect x="2.5" y="2.5" width="7" height="7" rx="1.4" fill="currentColor" /></svg><span>Stop</span></span>
    ${ask ? confirm(a, where, stop, toggle) : null}
    ${err ? html`<span class="ierr stoperr">${err}</span>` : null}`;
}
