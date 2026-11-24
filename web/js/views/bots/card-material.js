// The bot card's material: solid in its colour (the pickers above), a gradient (a preset, or two colours
// and an angle of the owner's), or layered glass — and the colour of its eyes. The preview above wears it
// as it is chosen; the daemon keeps it in the bot's face (look.rs).
import { html } from '../../../vendor/preact-htm.js';
import { GRADIENT_PRESETS, GLASS_PRESETS } from '../../../vendor/botface/materials.js';
import { t } from '../../i18n.js';

const MATERIALS = [['solid', t('Solid')], ['gradient', t('Gradient')], ['rainbow-glass', t('Glass')]];
const row = (label, field) => html`<div class="bfrow"><span class="bcap">${label}</span>${field}</div>`;
const pick = (label, value, options, set) => row(label, html`<select class="bsel" aria-label=${label} value=${value} onChange=${(e) => set(e.target.value)}>
  ${options.map(([v, name]) => html`<option key=${v} value=${v}>${name}</option>`)}</select>`);
const tint = (label, value, set) => row(label, html`<input type="color" aria-label=${label} value=${value} onInput=${(e) => set(e.target.value)} />`);

/** `look`: {material, eye, gradient, gstart, gend, gangle, glass}; `set(key, value)` changes one field. */
export function materialSection(look, set) {
  const grads = [...GRADIENT_PRESETS.map((p) => [p.id, p.label.en]), ['custom', t('Custom')]];
  return html`<div class="bfactory">
    ${pick(t('Material'), look.material, MATERIALS, (v) => set('material', v))}
    ${tint(t('Eyes'), look.eye, (v) => set('eye', v))}
    ${look.material === 'gradient' ? pick(t('Gradient'), look.gradient, grads, (v) => set('gradient', v)) : null}
    ${look.material === 'gradient' && look.gradient === 'custom' ? html`
      ${tint(t('From'), look.gstart, (v) => set('gstart', v))}
      ${tint(t('To'), look.gend, (v) => set('gend', v))}
      ${row(t('Angle'), html`<input class="bsel" type="number" min=${0} max=${360} step=${5} aria-label=${t('Angle')} value=${look.gangle}
        onInput=${(e) => set('gangle', e.target.value)} />`)}` : null}
    ${look.material === 'rainbow-glass' ? pick(t('Glass'), look.glass, GLASS_PRESETS.map((p) => [p.id, p.label.en]), (v) => set('glass', v)) : null}
  </div>`;
}
