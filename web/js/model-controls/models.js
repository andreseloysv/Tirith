// The Model section: the agent default, the listed models (searchable when there are more than four), and a
// custom model ID for agents other than Codex.
import { PROVIDERS, ownerOf, pickModel } from './options.js';
import { element, button, section } from './dom.js';

function modelList(ctl, { choices, value, disabled }, { codex, models }) {
  const list = element('div', 'models'); list.setAttribute('role', 'group'); list.setAttribute('aria-label', 'Models');
  list.append(button(disabled, 'Agent default', !value.model, () => ctl.change({ model: '', effort: '' })));
  for (const m of models) {
    const b = button(disabled, m.name || m.slug, m.slug === value.model, () => ctl.change({ ...pickModel(choices, value, m.slug), effort: '' }));
    const owner = ownerOf(choices, m.slug);
    b.title = m.slug; b.dataset.search = `${m.name || ''} ${m.slug} ${owner}`.toLowerCase();
    if (m.name && m.name !== m.slug) b.append(element('small', '', m.slug));
    if (owner && !codex) b.append(element('small', 'owner', PROVIDERS[owner] || owner));
    list.append(b);
  }
  return list;
}

function searchBox(list, disabled) {
  const search = element('input', 'search'); search.type = 'search'; search.placeholder = 'Search models'; search.setAttribute('aria-label', 'Search models'); search.disabled = disabled;
  search.oninput = () => { for (const b of list.children) b.hidden = !`${b.dataset.search || b.textContent.toLowerCase()}`.includes(search.value.toLowerCase()); };
  return search;
}

function customModel(ctl, list, models, { value, disabled }) {
  const custom = element('details', 'custom'); custom.append(element('summary', '', 'Custom model'));
  const input = element('input'); input.type = 'text'; input.value = value.model; input.placeholder = 'Model ID'; input.setAttribute('aria-label', 'Custom model'); input.disabled = disabled;
  input.oninput = () => {
    const model = input.value.trim();
    [...list.children].forEach((b, i) => b.setAttribute('aria-pressed', String((i ? models[i - 1].slug : '') === model)));
    ctl.change({ model }, false);
  };
  custom.append(input);
  return custom;
}

export function drawModels(ctl, root, config, options) {
  const { codex, models } = options;
  const modelField = section(root, 'Model', 'model-field');
  const list = modelList(ctl, config, options);
  const search = searchBox(list, config.disabled);
  if (models.length > 4) modelField.append(search);
  modelField.append(list);
  if (!codex) modelField.append(customModel(ctl, list, models, config));
  else if (!models.length) modelField.append(element('p', 'hint', 'Open Codex on the computer to load its models.'));
}
