// The controls' small DOM pieces: an element, a choice button and a titled section.

export const element = (tag, cls, text) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
};

export const button = (disabled, words, selected, action) => {
  const b = element('button', 'choice', words); b.type = 'button'; b.disabled = disabled;
  b.setAttribute('aria-pressed', String(selected)); b.onclick = action;
  return b;
};
export const section = (root, label, name) => {
  const field = element('fieldset', name); field.append(element('legend', '', label)); root.append(field); return field;
};
