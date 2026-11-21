// Icons: the menu's and the buttons' paths, the pipeline stages' paths and colors, and the stroked icon.
import { html } from '../../vendor/preact-htm.js';

export const ICON = {
  overview: 'M4 4h7v7H4z M13 4h7v4h-7z M13 10h7v10h-7z M4 13h7v7H4z',
  pipelines: 'M5 5a2 2 0 100 4 2 2 0 000-4z M19 15a2 2 0 100 4 2 2 0 000-4z M7 7h4a3 3 0 013 3v4a3 3 0 003 3',
  agents: 'M9 11a3.5 3.5 0 100-7 3.5 3.5 0 000 7z M2.5 20c0-3.6 2.8-5.5 6.5-5.5s6.5 1.9 6.5 5.5 M16 4.5a3.5 3.5 0 010 6.5 M18 14.8c2 .6 3.5 2.2 3.5 5.2',
  accesos: 'M12 5C6.5 5 3 12 3 12s3.5 7 9 7 9-7 9-7-3.5-7-9-7z M12 9.5a2.5 2.5 0 100 5 2.5 2.5 0 000-5z',
  settings: 'M4 7h9 M17 7h3 M4 17h3 M11 17h9 M15 5v4 M9 15v4',
  clock: 'M12 21a9 9 0 100-18 9 9 0 000 18z M12 7v5l3 2',
  play: 'M8 5l11 7-11 7z',
  close: 'M6 6l12 12M18 6L6 18',
  owner: 'M12 3a4.5 4.5 0 100 9 4.5 4.5 0 000-9z M3.5 21c0-5.2 3.7-7.5 8.5-7.5s8.5 2.3 8.5 7.5z',
  guard: 'M12 2.5l7.5 3v6.2c0 4.3-3.2 7.4-7.5 9.3-4.3-1.9-7.5-5-7.5-9.3V5.5z',
  rooms: 'M4 4h16v12H10l-5 4v-4H4z',
  bots: 'M12 3.5v3 M7 7h10a4 4 0 014 4v4.5a4 4 0 01-4 4H7a4 4 0 01-4-4V11a4 4 0 014-4z M9.5 12v2 M14.5 12v2',
  check: 'M5 12.5l4.5 4.5L19 7.5',
  retry: 'M4 12a8 8 0 0113.7-5.7L20 8 M20 4v4h-4 M20 12a8 8 0 01-13.7 5.7L4 16 M4 20v-4h4',
  stop: 'M7 7h10v10H7z',
};

export const STAGE_ICON = {
  build: ['M12 3l8 4.5v9L12 21l-8-4.5v-9z M12 12l8-4.5 M12 12v9 M12 12L4 7.5', '#5b8dff', '#6a5cff'],
  tests: ['M9 3h6 M10 3v6l-5 9a2 2 0 002 3h10a2 2 0 002-3l-5-9V3', '#3ccf94', '#12a4a0'],
  lint: ['M4 6h16 M4 12h10 M4 18h13', '#a77bff', '#e46bd0'],
  sign: ['M8 15a4 4 0 100-8 4 4 0 000 8z M11 12l9 9 M17 18l2-2', '#ffc247', '#ff8f2e'],
  device: ['M8 3h8a1 1 0 011 1v16a1 1 0 01-1 1H8a1 1 0 01-1-1V4a1 1 0 011-1z M11 18h2', '#38bdf8', '#3b6bff'],
  store: ['M12 16V4 M7 9l5-5 5 5 M4 20h16', '#ff78a8', '#ff8a5c'],
  review: ['M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z M12 9a3 3 0 100 6 3 3 0 000-6z', '#2dd4bf', '#4ade80'],
  other: ['M12 8a4 4 0 100 8 4 4 0 000-8z M12 2v3 M12 19v3 M2 12h3 M19 12h3', '#aab1be', '#7b8494'],
};
const STAGE_ALIAS = { test: 'tests', upload: 'store', ios: 'device', android: 'device', check: 'review' };
export function stageIcon(name) {
  const k = String(name || '').toLowerCase().split(/\s/)[0];
  return STAGE_ICON[STAGE_ALIAS[k] || k] || STAGE_ICON.other;
}

/** A stroked 24x24 icon from one SVG path. */
export function Icon({ d, size = 16, color = 'currentColor', w = 1.8 }) {
  return html`<svg width=${size} height=${size} viewBox="0 0 24 24" fill="none" stroke=${color} stroke-width=${w}
    stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d=${d} /></svg>`;
}
