// A stage: its node on the flow (icon, step n of m, state, time) and its log file, reloaded while it runs.
import { html, useState, useEffect, useLayoutEffect, useRef } from '../../../vendor/preact-htm.js';
import { onMessage } from '../../store.js';
import { Icon, Span, dur, elapsed, stageIcon } from '../../common.js';
import { label, sClass } from '../boards/pipe.js';
import { t } from '../../i18n.js';

const PLAY = html`<svg viewBox="0 0 12 12" width="9" height="9" aria-hidden="true"><path d="M3.2 2.1l6.2 3.9-6.2 3.9z" fill="currentColor" /></svg>`;

/** A stage's log file (plain text), reloaded while the stage runs. */
export function StageLog({ st }) {
  const [text, setText] = useState(null);
  const running = st.status === 'running';
  useEffect(() => {
    let alive = true, t = 0;
    const load = () => fetch(`/api/runs/${st.id}/log`).then((r) => r.text()).then((x) => alive && setText(x), () => alive && setText(''));
    setText(null);
    load();
    const off = onMessage((m) => { if (m.t === 'run' && m.d && m.d.id === st.id) { clearTimeout(t); t = setTimeout(load, 300); } });
    return () => { alive = false; off(); clearTimeout(t); };
  }, [st.id, running]);
  const pre = useRef(null);
  useLayoutEffect(() => { if (pre.current) pre.current.scrollTop = pre.current.scrollHeight; }, [text]);
  if (text == null) return html`<p class="quiet">${t('Loading the log…')}</p>`;
  const body = text.replace(/\x1b\[[0-9;]*[A-Za-z]/g, '').replace(/\s+$/, '');
  return body ? html`<pre class="log plog" tabindex="0" ref=${pre}>${body}</pre>` : html`<p class="quiet">${t('No log output yet')}</p>`;
}

export function StageNode({ st, i, n, sel, onOpen }) {
  const s = sClass(st.status), [d, c1, c2] = stageIcon(st.stage);
  const time = st.ended_at ? dur(elapsed(st.started_at, st.ended_at)) : null;
  return html`<button class=${`node snode s-${s}${sel ? ' sel' : ''}`} onClick=${() => onOpen(st.id)}>
    <span class=${`gico ${s}`} style=${`background:linear-gradient(135deg, ${c1}, ${c2})`}><${Icon} d=${d} size=${20} color="#fff" w=${1.9} /></span>
    <span class="nt"><span class="ov">${t('Step {i} of {n}', { i: i + 1, n })}</span><span class="tt">${st.stage}</span>
      <span class=${`ds pds s-${s}`}>${label(st.status)}</span></span>
    ${st.started_at ? html`<span class="dpill">${PLAY}${time || html`<${Span} start=${st.started_at} />`}</span>` : null}
  </button>`;
}
