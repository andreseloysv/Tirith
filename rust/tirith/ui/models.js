// models.js: "Modelos": per role, the agent, model and effort, and the fallback chain for when credits run out.
(() => {
  const box = T.section('models', 'Modelos');
  const NAMES = {manager: 'Manager', workers: 'Lanes', ceo: 'CEO', brain: 'Cerebro'};
  const SRC = {page: 'de la página', config: 'de config.toml', default: 'por defecto'};
  let M = null;

  const opts = (list, cur, empty) => {
    const all = cur && !list.includes(cur) ? [...list, cur] : list;
    return (empty ? `<option value="">${empty}</option>` : '') +
      all.map(v => `<option${v === cur ? ' selected' : ''}>${T.esc(v)}</option>`).join('') +
      '<option value="__other">Otro…</option>';
  };

  function row(role, s) {
    const c = M.choices, brain = role === 'brain';
    const chips = (s.fallback || []).map((f, i) =>
      `<span class="mchip">${T.esc(f)} <a data-mup="${i}" title="Subir">↑</a> <a data-mdel="${i}" title="Quitar">×</a></span>`).join(' ');
    return `<div class="lane mrole" data-role="${role}">
      <div class="top"><b>${NAMES[role]}</b><span class="id">${SRC[s.source] || ''}</span></div>
      <div class="mrow">
        ${brain ? '' : `<select data-f="agent">${opts(c.agents, s.agent)}</select>`}
        <select data-f="model">${opts(c.models, s.model, '(modelo por defecto)')}</select>
        ${brain ? '' : `<select data-f="effort">${opts(c.efforts, s.effort, '(esfuerzo por defecto)')}</select>`}
      </div>
      ${brain ? '' : `<div class="mrow"><span class="quiet">Si se acaban los créditos:</span> <span class="mfb">${chips || '<i class="quiet">nada</i>'}</span>
        <select data-add><option value="">+ añadir</option>${c.agents.map(a => `<option>${T.esc(a)}</option>`).join('')}
          ${c.agents.flatMap(a => c.models.map(m => `<option>${T.esc(a + ':' + m)}</option>`)).join('')}</select></div>`}
      <div class="mrow"><button class="pri" data-save>Guardar</button>
        ${s.source === 'page' ? '<button data-reset>Restablecer</button>' : ''}
        ${brain ? '' : '<button data-preview>Ver comando</button>'}</div>
      <pre class="mprev quiet" hidden></pre>
    </div>`;
  }

  function render() {
    if (!M) return;
    box.innerHTML = '<style>.mrow{display:flex;flex-wrap:wrap;gap:6px;align-items:center;margin-top:6px}' +
      '.mrow select{max-width:180px;padding:3px 6px}.mrow button{margin-left:0;padding:3px 10px}' +
      '.mchip{border:1px solid var(--line);border-radius:12px;padding:1px 8px;font-size:12px}.mchip a{cursor:pointer;color:var(--mute)}' +
      '.mprev{white-space:pre-wrap;word-break:break-all;font-size:11px;margin:6px 0 0}</style>' +
      Object.keys(NAMES).filter(r => M.roles[r]).map(r => row(r, M.roles[r])).join('');
  }

  async function load() { M = await T.api('GET', '/api/models'); render(); }

  const current = el => {
    const s = M.roles[el.dataset.role], out = {fallback: [...(s.fallback || [])]};
    el.querySelectorAll('select[data-f]').forEach(sel => out[sel.dataset.f] = sel.value);
    return out;
  };

  box.addEventListener('change', e => {
    const t = e.target, el = t.closest('.mrole');
    if (!el) return;
    if (t.value === '__other') {
      const v = prompt('Valor:');
      if (v) { t.insertAdjacentHTML('afterbegin', `<option>${T.esc(v.trim())}</option>`); t.value = v.trim(); } else t.selectedIndex = 0;
    } else if (t.dataset.add !== undefined && t.value) {
      const s = M.roles[el.dataset.role];
      s.fallback = [...(s.fallback || []).filter(f => f !== t.value), t.value];
      keep(el); render();
    }
  });

  // Keeps the unsaved selects of a row across a re-render (fallback edits re-render the section).
  function keep(el) {
    const s = M.roles[el.dataset.role];
    el.querySelectorAll('select[data-f]').forEach(sel => s[sel.dataset.f] = sel.value || null);
  }

  box.addEventListener('click', async e => {
    const t = e.target, el = t.closest('.mrole');
    if (!el) return;
    const role = el.dataset.role, s = M.roles[role], fb = s.fallback || [];
    if (t.dataset.mdel !== undefined) { keep(el); fb.splice(+t.dataset.mdel, 1); render(); }
    if (t.dataset.mup !== undefined && +t.dataset.mup > 0) { keep(el); const i = +t.dataset.mup; [fb[i - 1], fb[i]] = [fb[i], fb[i - 1]]; render(); }
    if (t.dataset.save !== undefined) { await T.api('POST', '/api/models/' + role, current(el)); await load(); }
    if (t.dataset.reset !== undefined) { await T.api('DELETE', '/api/models/' + role); await load(); }
    if (t.dataset.preview !== undefined) {
      const p = await T.api('GET', `/api/models/preview?role=${role}&project=${encodeURIComponent(T.state().project || '')}`);
      const pre = el.querySelector('.mprev');
      pre.hidden = false; pre.textContent = p.argv.map(a => /[\s"']/.test(a) ? JSON.stringify(a) : a).join(' ');
    }
  });

  T.on('models', d => { M = d; render(); });
  load();
})();
