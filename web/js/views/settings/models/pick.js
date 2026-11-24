// A role card's choosers: the options of a select (with the current value and Other…), and a select that can turn
// into a text field for a value not in the list.
import { html } from '../../../../vendor/preact-htm.js';
import { OTHER } from './roles.js';
import { t } from '../../../i18n.js';

export function Options({ list, cur, blank, other, strict, owner = () => '' }) {
  const all = !strict && cur && !list.includes(cur) ? [cur, ...list] : list;
  return html`${blank != null ? html`<option value="">${blank}</option>` : null}
    ${all.map((o) => html`<option value=${o} selected=${o === cur}>${owner(o) ? `${o} · ${owner(o)}` : o}</option>`)}
    ${other ? html`<option value=${OTHER}>${t('Other…')}</option>` : null}`;
}

/** Settings keeps its original selects and optional custom text fields. */
export function Pick({ label, value, list, blank, set, free, setFree, strict, disabled, owner }) {
  if (!strict && (free || !list.length)) {
    return html`<label class="sf"><span>${label}</span><input type="text" value=${value} placeholder=${blank || label} spellcheck="false"
      aria-label=${label} disabled=${disabled} onInput=${(e) => set(e.target.value)} />
      ${list.length ? html`<button type="button" class="alink" disabled=${disabled} onClick=${() => { setFree(false); set(''); }}>${t('List')}</button>` : null}</label>`;
  }
  return html`<label class="sf"><span>${label}</span><select aria-label=${label} value=${value} disabled=${disabled || (strict && !list.length)} onChange=${(e) => {
    if (e.target.value === OTHER) { setFree(true); set(''); } else set(e.target.value);
  }}><${Options} list=${list} cur=${value} blank=${blank} other=${!strict} strict=${strict} owner=${owner} /></select></label>`;
}
