// Shared with AgentsMessenger's phone chat. Keep model-controls.js and .css identical in both apps.
export const PROVIDERS = { codex: 'OpenAI · Codex', anthropic: 'Anthropic', muse: 'Meta · Muse', copilot: 'GitHub · Copilot', '': 'Meta API' };
export function optionsFor(choices, value) {
  const { agent = '', model = '', effort = '' } = value;
  const codex = agent === 'codex';
  const models = codex ? choices.codex || [] : (choices.models || []).map(slug => ({ slug, name: slug }));
  const efforts = codex ? models.find(m => m.slug === model)?.efforts || [] : choices.efforts || [];
  const valid = !codex || ((!model || models.some(m => m.slug === model)) && (!effort || efforts.includes(effort)));
  return { codex, models, efforts, valid };
}
const element = (tag, cls, text) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
};
const title = value => value ? value[0].toUpperCase() + value.slice(1) : 'Default';

export class ModelControls extends HTMLElement {
  constructor() { super(); this.attachShadow({ mode: 'open' }); }
  configure({ choices = {}, value = {}, brain = false, disabled = false }) {
    const next = { choices, value: { agent: value.agent || '', model: value.model || '', effort: value.effort || '' }, brain, disabled };
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
    const { choices, value, brain, disabled } = this.config;
    const { codex, models, efforts, valid } = optionsFor(choices, value);
    const sheet = element('link'); sheet.rel = 'stylesheet'; sheet.href = new URL('./model-controls.css', import.meta.url).href;
    const root = element('div', 'controls');
    const button = (words, selected, action) => {
      const b = element('button', 'choice', words); b.type = 'button'; b.disabled = disabled;
      b.setAttribute('aria-pressed', String(selected)); b.onclick = action;
      return b;
    };
    const section = (label, name) => {
      const field = element('fieldset', name); field.append(element('legend', '', label)); root.append(field); return field;
    };
    const providers = section('Provider', 'providers');
    for (const agent of brain ? ['', 'codex'] : choices.agents || []) {
      providers.append(button(PROVIDERS[agent] || agent, agent === value.agent, () => this.change({ agent, model: '', effort: '' })));
    }
    const modelField = section('Model', 'model-field');
    const search = element('input', 'search'); search.type = 'search'; search.placeholder = 'Search models'; search.setAttribute('aria-label', 'Search models'); search.disabled = disabled;
    const list = element('div', 'models'); list.setAttribute('role', 'group'); list.setAttribute('aria-label', 'Models');
    list.append(button('Agent default', !value.model, () => this.change({ model: '', effort: '' })));
    for (const m of models) {
      const b = button(m.name || m.slug, m.slug === value.model, () => this.change({ model: m.slug, effort: '' }));
      b.title = m.slug; b.dataset.search = `${m.name || ''} ${m.slug}`.toLowerCase();
      if (m.name && m.name !== m.slug) b.append(element('small', '', m.slug));
      list.append(b);
    }
    search.oninput = () => { for (const b of list.children) b.hidden = !`${b.dataset.search || b.textContent.toLowerCase()}`.includes(search.value.toLowerCase()); };
    if (models.length > 4) modelField.append(search);
    modelField.append(list);
    if (!codex) {
      const custom = element('details', 'custom'); custom.append(element('summary', '', 'Custom model'));
      const input = element('input'); input.type = 'text'; input.value = value.model; input.placeholder = 'Model ID'; input.setAttribute('aria-label', 'Custom model'); input.disabled = disabled;
      input.oninput = () => {
        const model = input.value.trim();
        [...list.children].forEach((b, i) => b.setAttribute('aria-pressed', String((i ? models[i - 1].slug : '') === model)));
        this.change({ model }, false);
      };
      custom.append(input); modelField.append(custom);
    } else if (!models.length) modelField.append(element('p', 'hint', 'Open Codex on the computer to load its models.'));
    if (!brain || codex) {
      const effortField = section('Reasoning effort', 'effort');
      const levels = ['', ...efforts];
      const selected = Math.max(0, levels.indexOf(value.effort));
      const readout = element('output', 'readout', title(levels[selected]));
      const slider = element('input', 'slider'); slider.type = 'range'; slider.min = '0'; slider.max = String(levels.length - 1); slider.step = '1'; slider.value = String(selected);
      slider.disabled = disabled || !efforts.length; slider.setAttribute('aria-label', 'Reasoning effort'); slider.setAttribute('aria-valuetext', title(levels[selected]));
      const ticks = element('div', 'ticks');
      const select = index => {
        slider.value = String(index); readout.textContent = title(levels[index]); slider.setAttribute('aria-valuetext', title(levels[index]));
        [...ticks.children].forEach((b, i) => b.setAttribute('aria-pressed', String(i === index)));
        this.change({ effort: levels[index] }, false);
      };
      levels.forEach((level, i) => ticks.append(button(title(level), i === selected, () => select(i))));
      slider.oninput = () => select(Number(slider.value));
      effortField.append(readout, slider, ticks);
      if (codex && !value.model) effortField.append(element('p', 'hint', 'Choose a model to adjust its effort.'));
    }
    if (!valid) root.append(element('p', 'hint', 'Choose a listed Codex model and one of its supported efforts.'));
    this.shadowRoot.replaceChildren(sheet, root);
  }
}
if (!customElements.get('model-controls')) customElements.define('model-controls', ModelControls);
