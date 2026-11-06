// Markdown-ish text as Preact nodes (no innerHTML): paragraphs, bullet and numbered lists, headings, code blocks,
// `code`, **bold** and *italic*. Same classes as the old md.js (.md, .mdh, .mdpre) so style.css draws it.
import { html } from '../../../vendor/preact-htm.js';

function inline(text) {
  const out = [];
  const re = /(`[^`]+`|\*\*[^*]+\*\*|\*[^*\s][^*]*\*)/g;
  let last = 0, m;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const t = m[0];
    if (t[0] === '`') out.push(html`<code>${t.slice(1, -1)}</code>`);
    else if (t.startsWith('**')) out.push(html`<b>${t.slice(2, -2)}</b>`);
    else out.push(html`<i>${t.slice(1, -1)}</i>`);
    last = m.index + t.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

export function mdBlocks(text) {
  const lines = String(text || '').replace(/\r/g, '').split('\n');
  const out = [];
  let para = [], list = null, code = null;
  const flushPara = () => { if (para.length) out.push(html`<p>${inline(para.join(' '))}</p>`); para = []; };
  const flushList = () => {
    if (list) out.push(list.ordered ? html`<ol>${list.items.map((x) => html`<li>${inline(x)}</li>`)}</ol>` : html`<ul>${list.items.map((x) => html`<li>${inline(x)}</li>`)}</ul>`);
    list = null;
  };
  for (const ln of lines) {
    if (code) {
      if (/^\s*```/.test(ln)) { out.push(html`<pre class="mdpre"><code>${code.join('\n')}</code></pre>`); code = null; } else code.push(ln);
      continue;
    }
    if (/^\s*```/.test(ln)) { flushPara(); flushList(); code = []; continue; }
    const h = /^(#{1,4})\s+(.*)$/.exec(ln);
    const b = /^\s*[-*+]\s+(.*)$/.exec(ln);
    const o = /^\s*\d+[.)]\s+(.*)$/.exec(ln);
    if (h) { flushPara(); flushList(); out.push(html`<div class=${`mdh mdh${h[1].length}`}>${inline(h[2])}</div>`); }
    else if (b || o) {
      flushPara();
      const ordered = !!o;
      if (!list || list.ordered !== ordered) { flushList(); list = { ordered, items: [] }; }
      list.items.push((b || o)[1]);
    } else if (!ln.trim()) { flushPara(); flushList(); }
    else { flushList(); para.push(ln.trim()); }
  }
  if (code) out.push(html`<pre class="mdpre"><code>${code.join('\n')}</code></pre>`);
  flushPara(); flushList();
  return out;
}

export function Md({ text, cls = '' }) {
  return html`<div class=${`md ${cls}`}>${mdBlocks(text)}</div>`;
}

export function MdInline({ text }) {
  return html`<span class="md mdi">${inline(String(text || '').replace(/\s*\n\s*/g, ' '))}</span>`;
}
