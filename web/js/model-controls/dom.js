// The menu's small DOM pieces: an element, an icon, and a row (a check, its words, and a note at its end).

export const element = (tag, cls, text) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
};

const PATHS = {
  check: 'M3.5 8.5l3 3 6-7',
  next: 'M6 3.5L10.5 8 6 12.5',
  back: 'M10 3.5L5.5 8l4.5 4.5',
  search: 'M7 12.2A5.2 5.2 0 107 1.8a5.2 5.2 0 000 10.4zM10.8 10.8l3.4 3.4',
  close: 'M4 4l8 8M12 4l-8 8',
};
/** An inline 16×16 stroke icon. */
export function icon(name) {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 16 16'); svg.setAttribute('width', '16'); svg.setAttribute('height', '16');
  svg.setAttribute('aria-hidden', 'true'); svg.classList.add('i');
  const p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  p.setAttribute('d', PATHS[name]); svg.append(p);
  return svg;
}

/** A menu row: `selected` shows the check (and aria-checked), `note` sits at its end with a chevron when `more`. */
export function row({ words, selected = false, note = '', more = false, disabled = false, action }) {
  const b = element('button', `row${selected ? ' on' : ''}`); b.type = 'button'; b.disabled = disabled; b.onclick = action;
  b.setAttribute('role', 'menuitemradio'); b.setAttribute('aria-checked', String(selected));
  const mark = element('span', 'ck'); if (selected) mark.append(icon('check'));
  b.append(mark, element('span', 'nm', words));
  if (note || more) { const end = element('span', 'end', note); if (more) end.append(icon('next')); b.append(end); }
  return b;
}
