// Pipelines: each merge attempt ("merge") and each check of main ("main") with its stages, their logs and a
// retry button on red ones. Uses window.T from index.html.
(() => {
  const box = T.section('pipelines', 'Pipelines');
  const P = {};
  const LABEL = {running: 'en curso', passed: 'ok', failed: 'falló', timeout: 'tiempo'};
  const COLOR = {running: 'var(--run)', passed: 'var(--ok)', failed: 'var(--bad)', timeout: 'var(--bad)'};
  const css = document.createElement('style');
  css.textContent = `#pipelines .pl{border:1px solid var(--line);background:var(--panel);border-radius:10px;padding:7px 10px;margin-bottom:6px}
    #pipelines .top{display:flex;gap:8px;align-items:center;font-size:12px}#pipelines .top button{margin-left:auto;padding:1px 8px;font-size:12px}
    #pipelines .chips{display:flex;flex-wrap:wrap;gap:5px;margin-top:5px}
    #pipelines .chip{font-size:11px;padding:1px 7px;border-radius:9px;border:1px solid currentColor;cursor:pointer;background:none}`;
  document.head.appendChild(css);

  function secs(a, b) {
    if (!a) return '';
    const s = Math.max(0, Math.round(((b ? new Date(b) : new Date()) - new Date(a)) / 1000));
    return s < 60 ? `${s}s` : `${Math.floor(s / 60)}m${String(s % 60).padStart(2, '0')}s`;
  }
  function html(p) {
    const red = p.status === 'failed' || p.status === 'timeout';
    const what = p.kind === 'main' ? 'check de main' : (p.lanes || '').split(',').filter(Boolean).join(', ');
    const chips = (p.stages || []).map(s => `<button class="chip" style="color:${COLOR[s.status] || 'var(--mute)'}"
      data-run="${s.id}" data-name="${T.esc(p.id + ' · ' + s.stage)}">${T.esc(s.stage)} ${secs(s.started_at, s.ended_at)}</button>`).join('');
    return `<div class="top"><span class="b ${T.esc(p.status)}">${T.esc(LABEL[p.status] || p.status)}</span>
      <b>${T.esc(p.id)}</b><span class="quiet">${T.esc(what)} · ${T.esc((p.sha || '').slice(0, 8))} · ${T.time(p.started_at)}</span>
      ${red ? `<button data-retry="${T.esc(p.id)}">Reintentar</button>` : ''}</div>
      <div class="chips">${chips || '<span class="quiet">sin etapas</span>'}</div>`;
  }
  function render(p) {
    if (!p || !p.id) return;
    P[p.id] = p;
    let el = document.getElementById('pl-' + p.id);
    if (!el) {
      el = document.createElement('div'); el.id = 'pl-' + p.id; el.className = 'pl';
      box.prepend(el);
      while (box.children.length > 30) box.lastChild.remove();
    }
    el.innerHTML = html(p);
  }
  async function load() {
    const list = await T.api('GET', '/api/pipelines?limit=30');
    box.innerHTML = list.length ? '' : '<div class="quiet" id="pl-none">Ningún pipeline todavía.</div>';
    list.reverse().forEach(render);
  }
  T.on('pipeline', p => { const n = document.getElementById('pl-none'); if (n) n.remove(); render(p); });
  box.addEventListener('click', async e => {
    const t = e.target.closest('button');
    if (!t) return;
    if (t.dataset.retry) { t.disabled = true; await T.api('POST', `/api/pipelines/${t.dataset.retry}/retry`).catch(() => {}); t.disabled = false; }
    if (t.dataset.run) {
      document.getElementById('mtitle').textContent = t.dataset.name;
      document.getElementById('mbody').textContent = await T.api('GET', `/api/runs/${t.dataset.run}/log`) || '(vacío)';
      document.getElementById('modal').classList.add('on');
    }
  });
  setInterval(() => Object.values(P).filter(p => p.status === 'running').forEach(render), 5000);
  load();
})();
