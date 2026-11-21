// The <model-controls> element: configure() sets choices, value and disabled; it redraws only when they change,
// and onChange gets the new value after every pick.
import { optionsFor } from './options.js';
import { element } from './dom.js';
import { drawProviders } from './providers.js';
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
  change(patch, redraw = true) {
    this.config.value = { ...this.config.value, ...patch };
    this.signature = JSON.stringify(this.config);
    if (redraw) this.draw();
    this.onChange?.({ ...this.config.value });
  }
  draw() {
    const options = optionsFor(this.config.choices, this.config.value);
    const sheet = element('link'); sheet.rel = 'stylesheet'; sheet.href = new URL('../model-controls.css', import.meta.url).href;
    const root = element('div', 'controls');
    drawProviders(this, root, this.config);
    drawModels(this, root, this.config, options);
    drawEffort(this, root, this.config, options);
    if (!options.valid) root.append(element('p', 'hint', 'Choose a listed Codex model and one of its supported efforts.'));
    this.shadowRoot.replaceChildren(sheet, root);
  }
}
