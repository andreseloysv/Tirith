// Who speaks, bold for fast reading.
import { esc } from './esc.js';

/* a leading "CEO proposal:", "CEO directive d-1:" or "Autopilot:" of a text */
const LEAD = /^(CEO proposal:|CEO directive [^\s:]+:|CEO review:|Review:|Autopilot:)/;
export function boldLead(text) { return String(text == null ? '' : text).replace(LEAD, '**$1**'); }

/** The speaker of a message line, bold: "Manager · tirith" -> <b>Manager</b> · tirith (escaped). */
export function ctxHtml(ctx) {
  const s = String(ctx || ''), at = s.indexOf(' · ');
  return at < 0 ? '<b>' + esc(s) + '</b>' : '<b>' + esc(s.slice(0, at)) + '</b>' + esc(s.slice(at));
}
