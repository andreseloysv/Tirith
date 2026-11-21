// The bot card's Model section: the managers' model, or one of its own chosen with Settings' model controls.
import { html, useState, useRef, useLayoutEffect } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { optionsFor } from '../../model-controls.js';
import { keyOf } from './state.js';

/** The model controls of Settings (a custom element), for one bot. */
function ModelControls({ M, value, onChange }) {
  const ref = useRef(null);
  useLayoutEffect(() => {
    ref.current.onChange = onChange;
    ref.current.configure({ choices: M.choices, value: value || {} });
  });
  return html`<model-controls ref=${ref}></model-controls>`;
}
const modelName = (v) => (v ? v.model || v.agent || 'default' : '');

/** The bot's model as the card edits it: {hasOwn, value, valid, setOwn, setModel, persist(saved bot)}. */
export function useCardModel(bot, M) {
  const role = bot.id && M && M.roles[keyOf(bot)];
  const [own, setOwn] = useState(null);      /* null: as it is; true or false: the owner's choice */
  const [model, setModel] = useState(null);  /* the model chosen here */
  const hasOwn = own == null ? !!(role && role.source === 'page') : own;
  const value = model || (role && role.source === 'page' ? role : M && M.roles.manager) || {};
  const valid = !M || !hasOwn || optionsFor(M.choices, value).valid;
  const persist = async (b) => {
    if (own === false && role && role.source === 'page') await api('DELETE', `/api/models/${keyOf(b)}`);
    else if (hasOwn && (model || own)) await api('POST', `/api/models/${keyOf(b)}`, value);
  };
  return { hasOwn, value, valid, setOwn, setModel, persist };
}

/** The Model section (`cm`: useCardModel's answer). */
export function modelSection(M, cm) {
  const { hasOwn, value, setOwn, setModel } = cm;
  return html`<details class="bmodel"><summary><span class="bcap">Model</span> ${hasOwn ? modelName(value) : `the managers' · ${modelName(M.roles.manager)}`}</summary>
          <label class="bchk"><input type="checkbox" checked=${hasOwn} onChange=${(e) => setOwn(e.target.checked)} />A model of its own, not the managers'</label>
          ${hasOwn ? html`<${ModelControls} M=${M} value=${value} onChange=${(v) => { setModel(v); setOwn(true); }} />` : null}</details>`;
}
