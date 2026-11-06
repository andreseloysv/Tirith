// net.js: the "Red" section. Sandbox state, every destination the agents reached through Tirith's proxy (newest
// first) with Bloquear/Desbloquear, and the block list with an input to add a rule.
(() => {
  const T = window.T, box = T.section('net', 'Red');
  const st = document.createElement('style');
  st.textContent = `#net .row{display:flex;gap:8px;align-items:center;font-size:12px;padding:3px 0;border-bottom:1px dashed var(--line)}
#net .host{font-family:ui-monospace,Menlo,monospace;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1}
#net .who{color:var(--mute);max-width:140px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
#net button{padding:1px 8px;font-size:12px}#net .off{color:var(--bad)}#net .on{color:var(--ok)}
#net form{display:flex;gap:6px;margin:6px 0}#net form input{flex:1}#net .list{max-height:320px;overflow:auto}
#net .rule{display:inline-flex;gap:4px;align-items:center;margin:2px 6px 2px 0;font-size:12px}`;
  document.head.appendChild(st);
  box.innerHTML = `<div class="head quiet">…</div><div class="list dests"></div>
<form><input placeholder="Bloquear dominio, IP o rango (ej. example.com, 10.0.0.0/8)"><button>Bloquear</button></form>
<div class="blocks"></div>`;
  const q = s => box.querySelector(s);
  let dests = [], blocks = [];

  function render() {
    q('.dests').innerHTML = dests.length ? dests.map(d => `<div class="row">
<span class="host${d.rule ? ' off' : ''}" title="${T.esc(d.host)}">${T.esc(d.host)}</span>
<span class="quiet">${d.count}${d.refused ? ' · ' + d.refused + ' bloq.' : ''}</span>
<span class="who" title="${T.esc(d.lanes.join(', '))}">${T.esc(d.lanes.join(', '))}</span>
<span class="quiet">${T.time(d.last)}</span>
${d.rule ? `<button data-un="${T.esc(d.rule)}">Desbloquear</button>` : `<button data-block="${T.esc(d.host)}">Bloquear</button>`}
</div>`).join('') : '<div class="quiet">Ningún destino todavía.</div>';
    q('.blocks').innerHTML = blocks.length
      ? 'Bloqueados: ' + blocks.map(r => `<span class="rule"><code>${T.esc(r)}</code><button data-un="${T.esc(r)}">×</button></span>`).join('')
      : '<span class="quiet">Nada bloqueado.</span>';
  }

  async function load() {
    try {
      const [s, d] = await Promise.all([T.api('GET', '/api/net/status'), T.api('GET', '/api/net/destinations')]);
      q('.head').innerHTML = s.sandbox
        ? `<span class="on">Sandbox activo</span> · proxy 127.0.0.1:${s.proxy_port} <span class="quiet">${T.esc(s.proxy)}</span>`
        : `<span class="off">Sandbox apagado</span> <span class="quiet">${T.esc(s.why || '')}</span>`;
      dests = d; blocks = s.blocks; render();
    } catch (e) { q('.head').textContent = 'Red: ' + e.message; }
  }

  async function change(method, rule) {
    try { blocks = (await T.api(method, '/api/net/blocks', {rule})).blocks; load(); }
    catch { /* T.api already showed the reason */ }
  }

  box.addEventListener('click', e => {
    const b = e.target.closest('button');
    if (b && b.dataset.block) change('POST', b.dataset.block);
    else if (b && b.dataset.un) change('DELETE', b.dataset.un);
  });
  q('form').addEventListener('submit', e => {
    e.preventDefault();
    const input = q('form input'), rule = input.value.trim();
    if (rule) { input.value = ''; change('POST', rule); }
  });

  // A new destination or refusal: reload the list, at most once a second.
  let timer = null;
  T.on('event', ev => {
    if ((ev.kind === 'net' || ev.kind === 'net-blocked') && !timer) timer = setTimeout(() => { timer = null; load(); }, 1000);
  });
  T.on('net', d => { blocks = d.blocks; load(); });
  load();
})();
