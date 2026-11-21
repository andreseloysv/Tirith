// The Reasoning effort section: a readout, a slider and one button per level ('' is the agent's default).
import { element, title, button, section } from './dom.js';

function slider(levels, selected, disabled, efforts) {
  const s = element('input', 'slider'); s.type = 'range'; s.min = '0'; s.max = String(levels.length - 1); s.step = '1'; s.value = String(selected);
  s.disabled = disabled || !efforts.length; s.setAttribute('aria-label', 'Reasoning effort'); s.setAttribute('aria-valuetext', title(levels[selected]));
  return s;
}

export function drawEffort(ctl, root, { value, disabled }, { codex, efforts }) {
  const effortField = section(root, 'Reasoning effort', 'effort');
  const levels = ['', ...efforts];
  const selected = Math.max(0, levels.indexOf(value.effort));
  const readout = element('output', 'readout', title(levels[selected]));
  const range = slider(levels, selected, disabled, efforts);
  const ticks = element('div', 'ticks');
  const select = index => {
    range.value = String(index); readout.textContent = title(levels[index]); range.setAttribute('aria-valuetext', title(levels[index]));
    [...ticks.children].forEach((b, i) => b.setAttribute('aria-pressed', String(i === index)));
    ctl.change({ effort: levels[index] }, false);
  };
  levels.forEach((level, i) => ticks.append(button(disabled, title(level), i === selected, () => select(i))));
  range.oninput = () => select(Number(range.value));
  effortField.append(readout, range, ticks);
  if (codex && !value.model) effortField.append(element('p', 'hint', 'Choose a model to adjust its effort.'));
}
