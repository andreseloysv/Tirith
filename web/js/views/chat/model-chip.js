// The model chip in a chat's box: the model and effort of who speaks here (a project's chat the Project Manager,
// for every project; a bot's screen that bot), and the model menu it opens above the box. A pick is saved at once.
import { html, useState, useEffect, useLayoutEffect, useRef } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { offered, effortName, providerName, readable } from '../../model-controls.js';
import { t } from '../../i18n.js';

/** Closes the menu on a tap outside `wrap` or on Escape while `open`. */
function useDismiss(open, wrap, close) {
  useEffect(() => {
    if (!open) return undefined;
    const out = (e) => { if (!e.composedPath().includes(wrap.current)) close(); };
    const esc = (e) => { if (e.key === 'Escape') close(); };
    document.addEventListener('pointerdown', out); document.addEventListener('keydown', esc);
    return () => { document.removeEventListener('pointerdown', out); document.removeEventListener('keydown', esc); };
  }, [open]);
}

/** The menu's height: what fits between the box and the edge of what clips it (the chat, or the window). */
function fit(box) {
  const comp = box?.closest('.mc-comp');
  if (!comp) return;
  let clip = comp.parentElement;
  while (clip && getComputedStyle(clip).overflowY === 'visible') clip = clip.parentElement;
  const edge = Math.max(clip ? clip.getBoundingClientRect().top : 0, 0);
  box.style.maxHeight = `${Math.max(160, Math.min(560, comp.getBoundingClientRect().top - edge - 16))}px`;
}

/** The chip's words: [model, effort, runs]; a model no agent runs says so instead of an effort. */
function words(choices, v) {
  const m = offered(choices).find(x => x.slug === v.model);
  if (!v.model) return [t('{provider} default', { provider: providerName(v.agent || '') }), '', true];
  return m ? [m.name, effortName(v.effort), true] : [readable(v.model), t('Not available'), false];
}

export function ModelChip({ role, isProject }) {
  const [M, reload] = useFetch('/api/models', ['models']);
  const [open, setOpen] = useState('');        /* '' closed, else the view it opened on: 'models' or 'effort' */
  const [picked, setPicked] = useState(null);  /* shown while it is saved, until the new state arrives */
  const [error, setError] = useState('');
  const menu = useRef(null), wrap = useRef(null), box = useRef(null);
  useEffect(() => setPicked(null), [M]);
  useDismiss(open, wrap, () => setOpen(''));
  const value = picked || M?.roles[role] || {};
  useLayoutEffect(() => {
    if (!menu.current || !M) return;
    menu.current.onChange = async (v, close) => {
      setPicked(v); setError(''); if (close) setOpen('');
      try { await api('POST', `/api/models/${role}`, v); reload(); } catch (e) { setPicked(null); setError(e.message || String(e)); setOpen('models'); }
    };
    menu.current.onClose = () => setOpen('');
    menu.current.configure({ choices: M.choices, value });
  });
  useLayoutEffect(() => {
    if (!open || !menu.current) return;
    menu.current.show(open); fit(box.current); box.current.scrollTop = 0;
    menu.current.shadowRoot.querySelector('.row.on')?.scrollIntoView({ block: 'nearest' });
  }, [open]);
  if (!M || !M.roles[role]) return null;   /* who speaks here is no factory role: nothing to pick */
  const [name, effort, runs] = words(M.choices, value);
  const toggle = (view) => setOpen(open ? '' : view);
  return html`<span class="mc-pick" ref=${wrap}>
    <span class=${`mc-chip${runs ? '' : ' off'}`}>
      <button type="button" aria-haspopup="menu" aria-expanded=${!!open} title=${t('Choose the model')} onClick=${() => toggle('models')}>${name}</button>
      ${effort ? html`<button type="button" class="mc-chip-e" title=${runs ? t('Choose the effort') : t('Choose another model')}
        onClick=${() => toggle(runs ? 'effort' : 'models')}>${effort}</button>` : null}
    </span>
    <div class="mc-mmenu" ref=${box} hidden=${!open}>
      <model-controls ref=${menu}></model-controls>
      ${error ? html`<p class="ierr mc-mmenu-err">${error}</p>` : null}
      ${isProject ? html`<p class="mc-mmenu-note">${t('Applies to every project.')}</p>` : null}
    </div>
  </span>`;
}
