// guard.js: the commit guard of each project (hooks that refuse forbidden words) and an Instalar button.
(() => {
  const box = T.section('guard-box', 'Guarda de commits');
  async function load() {
    let list;
    try { list = await (await fetch('/api/guard')).json(); } catch { return; }
    box.innerHTML = list.map(p => `<div class="item"><span class="t">${T.esc(p.name || p.project)}
      ${p.words ? '' : '<span class="quiet">· sin palabras configuradas</span>'}</span>
      ${p.ok ? `<span class="b passed">${p.words ? 'activa' : 'no aplica'}</span>`
        : `<span class="b failed" title="${T.esc(p.reason)}">revisar</span><button data-guard="${T.esc(p.project)}">Instalar</button>`}</div>
      ${p.ok ? '' : `<div class="quiet" style="font-size:12px;padding-left:14px">${T.esc(p.reason)}</div>`}`).join('')
      || '<div class="quiet">Ningún proyecto.</div>';
  }
  box.addEventListener('click', async e => {
    const id = e.target.dataset.guard;
    if (!id) return;
    e.stopPropagation(); e.target.disabled = true;
    try { await T.api('POST', `/api/guard/${encodeURIComponent(id)}/install`); } catch {}
    load();
  });
  T.on('event', d => { if (d.kind === 'guard') load(); });
  load();
})();
