// Small components: a duration that ticks and an empty view's text.
import { html, useState, useEffect } from '../../vendor/preact-htm.js';
import { dur, elapsed } from './time.js';

/** A duration that ticks every second while it has no end. */
export function Span({ start, end }) {
  const [, tick] = useState(0);
  useEffect(() => {
    if (end || !start) return undefined;
    const t = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [start, end]);
  return html`<span>${dur(elapsed(start, end))}</span>`;
}

export function Empty({ text }) {
  return html`<div class="empty">${text}</div>`;
}
