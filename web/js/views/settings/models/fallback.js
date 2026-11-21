// A role's fallback chain: the agents used in order when its agent is out of credits; reorder, remove, add.
import { html } from '../../../../vendor/preact-htm.js';
import { Options } from './pick.js';

export function Fallback({ list, main, agents, set }) {
  const left = agents.filter((a) => !list.includes(a) && a !== main);
  const move = (i) => { const l = list.slice(); [l[i - 1], l[i]] = [l[i], l[i - 1]]; set(l); };
  return html`<div class="sfallback"><span class="slabel">Fallback</span><span class="mut sm">Used in this order when an agent is out of credits</span>
    <div class="fbrow">${list.length ? list.map((a, i) => html`<span class="fbchip" key=${a}><b>${i + 1}</b> ${a}
        ${i ? html`<button class="alink" aria-label=${`Move ${a} earlier`} onClick=${() => move(i)}>↑</button>` : null}
        <button class="alink" aria-label=${`Remove ${a}`} onClick=${() => set(list.filter((x) => x !== a))}>✕</button></span>`)
      : html`<span class="mut sm">None</span>`}
      ${left.length ? html`<select aria-label="Add a fallback agent" onChange=${(e) => { if (e.target.value) set([...list, e.target.value]); e.target.value = ''; }}>
        <${Options} list=${left} cur="" blank="Add…" /></select>` : null}</div></div>`;
}
