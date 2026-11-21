// A small, safe markdown renderer for agent-written text (port of the old page's md.js, same output and classes).
// All HTML in the source is escaped first; the only tags that come out are the ones built here, and links keep
// only http://, https:// and file:// URLs; a bare domain (ejemplo.com/ruta) links to https://… md(text) ->
// block HTML, mdInline(text) -> inline HTML (strings, for
// dangerouslySetInnerHTML). With opts {file(path) -> bool, project}, confirmed paths become file links
// (a.fref); with opts {lane(id) -> bool}, known lane ids become lane links (a.lref, data-key = the lane id).
// Each part lives in its own file under md/; this entry exports what it always did.
import { withRefs } from './md/refs.js';
import { blocks } from './md/blocks.js';
import { inline } from './md/inline.js';

export { esc } from './md/esc.js';
export { splitRef } from './md/files.js';
export { boldLead, ctxHtml } from './md/speakers.js';

/** Block HTML of markdown text, wrapped in <div class="md">; '' for an empty text. */
export function md(text, opts) {
  const h = withRefs(opts, () => blocks(text));
  return h ? '<div class="md">' + h + '</div>' : '';
}

/** Inline HTML of markdown text (one line), wrapped in <span class="md mdi">. */
export function mdInline(text, opts) {
  return '<span class="md mdi">' + withRefs(opts, () => inline(String(text == null ? '' : text).replace(/\s*\n\s*/g, ' '))) + '</span>';
}
