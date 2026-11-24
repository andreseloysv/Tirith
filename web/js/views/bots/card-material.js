// The bot card's material: solid in its colour (the pickers above), a gradient (a preset, or two colours
// and an angle of the owner's), or layered glass — and the colour of its eyes. The preview above wears it
// as it is chosen; the daemon keeps it in the bot's face (look.rs).
import { html } from '../../../vendor/preact-htm.js';
import { GRADIENT_PRESETS, GLASS_PRESETS } from '../../../vendor/botface/materials.js';

const MATERIALS = [['solid', 'Sólido'], ['gradient', 'Gradiente'], ['rainbow-glass', 'Vidrio']];
const row = (label, field) => html`<div class="bfrow"><span class="bcap">${label}</span>${field}</div>`;
const pick = (label, value, options, set) => row(label, html`<select class="bsel" aria-label=${label} value=${value} onChange=${(e) => set(e.target.value)}>
  ${options.map(([v, t]) => html`<option key=${v} value=${v}>${t}</option>`)}</select>`);
const tint = (label, value, set) => row(label, html`<input type="color" aria-label=${label} value=${value} onInput=${(e) => set(e.target.value)} />`);

/** `look`: {material, eye, gradient, gstart, gend, gangle, glass}; `set(key, value)` changes one field. */
export function materialSection(look, set) {
  const grads = [...GRADIENT_PRESETS.map((p) => [p.id, p.label.en]), ['custom', 'Personalizado']];
  return html`<div class="bfactory">
    ${pick('Material', look.material, MATERIALS, (v) => set('material', v))}
    ${tint('Ojos', look.eye, (v) => set('eye', v))}
    ${look.material === 'gradient' ? pick('Gradiente', look.gradient, grads, (v) => set('gradient', v)) : null}
    ${look.material === 'gradient' && look.gradient === 'custom' ? html`
      ${tint('Desde', look.gstart, (v) => set('gstart', v))}
      ${tint('Hasta', look.gend, (v) => set('gend', v))}
      ${row('Ángulo', html`<input class="bsel" type="number" min=${0} max=${360} step=${5} aria-label="Ángulo" value=${look.gangle}
        onInput=${(e) => set('gangle', e.target.value)} />`)}` : null}
    ${look.material === 'rainbow-glass' ? pick('Vidrio', look.glass, GLASS_PRESETS.map((p) => [p.id, p.label.en]), (v) => set('glass', v)) : null}
  </div>`;
}
