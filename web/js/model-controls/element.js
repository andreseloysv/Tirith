// The <model-controls> element: configure() sets choices, value and disabled; it redraws only when they change,
// and onChange gets the new value after every pick. It offers only what can run: a model list and its efforts.
import { offered } from './offer.js';
import { element } from './dom.js';
import { drawModels } from './models.js';
import { drawEffort } from './effort.js';

export class ModelControls extends HTMLElement {
  constructor() { super(); this.attachShadow({ mode: 'open' }); }
  configure({ choices = {}, value = {}, disabled = false }) {
    const next = { choices, value: { agent: value.agent || '', model: value.model || '', effort: value.effort || '' }, disabled };
    if (JSON.stringify(next) === this.signature) return;
    this.config = next;
    this.signature = JSON.stringify(next);
    this.draw();
  }
  change(patch) {
    this.config.value = { ...this.config.value, ...patch };
    this.signature = JSON.stringify(this.config);
    this.draw();
    this.onChange?.({ ...this.config.value });
  }
  draw() {
    const list = offered(this.config.choices);
    const sheet = element('link'); sheet.rel = 'stylesheet'; sheet.href = new URL('../model-controls.css', import.meta.url).href;
    const root = element('div', 'controls');
    drawModels(this, root, this.config, list);
    drawEffort(this, root, this.config, list.find(m => m.slug === this.config.value.model));
    this.shadowRoot.replaceChildren(sheet, root);
  }
}
