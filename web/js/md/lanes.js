// Lane references: a known lane id in the text becomes a lane link (a.lref, data-key = the lane id).
import { esc } from './esc.js';
import { ctx } from './refs.js';

export const LANE = /(^|[\s(\[*;])(lane\/)?([A-Za-z0-9][\w.-]*?)(?=[.,:;!?]*(?:$|[\s)\]*&]))/g;
export function laneLink(id, inner) {
  if (!ctx.refs || !ctx.refs.lane || !/^[\w.-]+$/.test(id || '') || !ctx.refs.lane(id)) return null;
  return `<a class="lref" href="#" data-act="golane" data-key="${esc(id)}" title="Open lane ${esc(id)}">${inner}</a>`;
}
/* a bare word is a lane only when it cannot be an ordinary word: an id with - _ or a digit, lane/<id>,
   "lane <id>" or **<id>** */
export function laneWord(s, at, m, pre, slash, id) {
  return /[-_\d]/.test(id) || !!slash || /\blanes?\s$/i.test(s.slice(0, at + pre.length)) || (pre === '*' && s.charAt(at + m.length) === '*');
}
