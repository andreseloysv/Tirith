// The <model-controls> element, the model menu: configure() sets choices, value and disabled (it redraws only when
// they change); show() starts it over as it opens; onChange(value, close) gets each pick, onClose a tap that changes
// nothing. It offers only what can run: provider tabs, the models, and the chosen model's efforts.
import { offered } from './offer.js';
import { element } from './dom.js';
import { drawHead } from './head.js';
import { fillList } from './models.js';
import { drawEffort } from './effort.js';

const FRESH = () => ({ tab: '', query: null, others: false, view: 'models' });

export class ModelControls extends HTMLElement {
  constructor() {
    super(); this.attachShadow({ mode: 'open' }); this.ui = FRESH();
    const sheet = element('link'); sheet.rel = 'stylesheet'; sheet.href = new URL('../model-controls.css', import.meta.url).href;
    this.holder = element('div');   /* redrawn alone: the sheet stays loaded, so no frame is drawn unstyled */
    this.shadowRoot.append(sheet, this.holder);
  }
  configure({ choices = {}, value = {}, disabled = false }) {
    const next = { choices, value: { agent: value.agent || '', model: value.model || '', effort: value.effort || '' }, disabled };
    if (JSON.stringify(next) === this.signature) return;
    this.config = next;
    this.signature = JSON.stringify(next);
    this.draw();
  }
  /** Starts over on the models (or on the efforts when `view` is 'effort'), as the menu opens. */
  show(view = 'models') { this.ui = { ...FRESH(), view }; if (this.config) this.draw(); }
  set(ui) { this.ui = { ...this.ui, ...ui }; this.draw(); }
  close() { this.onClose?.(); }
  change(patch, close = false) {
    this.config.value = { ...this.config.value, ...patch };
    this.signature = JSON.stringify(this.config);
    this.ui = { ...this.ui, view: 'models' };
    this.draw();
    this.onChange?.({ ...this.config.value }, close);
  }
  draw() {
    this.list = offered(this.config.choices);
    const current = this.list.find(m => m.slug === this.config.value.model);
    const agents = [...new Set(this.list.map(m => m.agent))];
    this.tab = agents.includes(this.ui.tab) ? this.ui.tab : current?.agent || agents[0] || '';
    const root = element('div', 'menu');
    if (this.ui.view === 'effort' && current?.efforts.length) drawEffort(this, root, this.config, current);
    else {
      drawHead(this, root, agents, this.tab);
      this.box = element('div', 'list'); this.box.setAttribute('role', 'menu'); this.box.setAttribute('aria-label', 'Models');
      root.append(this.box);
      this.fillList();
    }
    this.holder.replaceChildren(root);
  }
  fillList() { fillList(this, this.box, this.config, this.list, this.tab); }
}
