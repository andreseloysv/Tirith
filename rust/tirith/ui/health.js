// health.js: the health strip at the top of the right column (green / red metrics against their targets),
// the last CEO reviews and the owner's "Revisar ahora" button.
(() => {
  const box = T.section('health', 'Salud');
  const right = box.parentNode, title = box.previousSibling;
  right.prepend(box); right.prepend(title);
  const css = document.createElement('style');
  css.textContent = `#health .chips{display:flex;flex-wrap:wrap;gap:6px}
#health .chip{font-size:12px;padding:3px 9px;border-radius:12px;border:1px solid var(--ok);color:var(--ok);cursor:default}
#health .chip.red{border-color:var(--bad);color:var(--bad);font-weight:600}
#health .chip.info{border-color:var(--line);color:var(--mute)}
#health .bar{display:flex;gap:8px;align-items:center;margin:8px 0 4px}
#health .bar button{padding:2px 10px;font-size:12px;margin-left:auto}
#health details{font-size:12px;margin:4px 0;border-left:3px solid var(--acc);padding-left:8px}
#health details summary{cursor:pointer;color:var(--mute)}
#health details div{white-space:pre-wrap;margin-top:4px}
.m.ceo{border-left:3px solid var(--acc)}`;
  document.head.appendChild(css);
  let running = false, last = {metrics: [], reviews: []};

  function chip(m) {
    const target = m.target == null ? '' : ` / ${m.at_least ? '≥' : '≤'} ${m.target}`;
    const cls = m.red ? 'red' : (m.target == null ? 'info' : '');
    const tip = (m.since ? `en rojo desde ${T.time(m.since)}\n` : '') + (m.facts || '');
    return `<span class="chip ${cls}" title="${T.esc(tip)}">${T.esc(m.label)}: <b>${T.esc(m.value)}</b>${T.esc(target)}</span>`;
  }

  function render(d) {
    last = d;
    const reds = d.metrics.filter(m => m.red).length;
    const reviews = (d.reviews || []).slice().reverse();
    box.innerHTML = `<div class="chips">${d.metrics.map(chip).join('') || '<span class="quiet">Calculando…</span>'}</div>
      <div class="bar"><span class="quiet">${reds ? reds + ' en rojo' : 'Todo en verde'}${running ? ' · el CEO está revisando…' : ''}</span>
      <button id="ceo-now" ${running ? 'disabled' : ''}>Revisar ahora</button></div>
      ${reviews.map((r, i) => `<details ${i === 0 ? 'open' : ''}><summary>Revisión del CEO · ${T.esc(T.time(r.at))} · ${T.esc(r.metric === 'owner' ? 'pedida por ti' : r.metric)}</summary><div>${T.esc(r.text)}</div></details>`).join('')
        || '<div class="quiet">Ninguna revisión del CEO todavía.</div>'}`;
  }

  async function load() {
    try {
      const d = await T.api('GET', '/api/metrics');
      running = !!d.reviewing;
      render(d);
    } catch (e) { console.error(e); }
  }

  box.addEventListener('click', async e => {
    if (e.target.id !== 'ceo-now') return;
    e.target.disabled = true;
    running = true; render(last);
    await T.api('POST', '/api/ceo/review').catch(() => { running = false; render(last); });
  });
  T.on('metrics', render);
  T.on('ceo', d => { running = !!d.running; render(last); });
  load();
})();
