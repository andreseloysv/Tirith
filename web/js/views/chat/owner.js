// The owner's text as the daemon stored it: the project tag of old rows and the attached files.
const PIC_EXT = /\.(png|jpe?g|gif|webp)$/i;
const ATTACHED = '\n\nAttached files (open them with your file-reading tool):\n';
const PROJECT_TAG = /^\[project ([\w.-]+)\] /;   /* rows from before every project had its own conversation */

/** {text, project, files: [{path, name, url, pic}]} of an owner message as the daemon stored it. */
export function ownerParts(raw) {
  let text = String(raw || ''), project = '', files = [];
  const at = text.indexOf(ATTACHED);
  if (at >= 0) {
    files = text.slice(at + ATTACHED.length).split('\n').map((l) => l.replace(/^- /, '').trim()).filter(Boolean).map(fileOf);
    text = text.slice(0, at);
  }
  const m = PROJECT_TAG.exec(text);
  if (m) { project = m[1]; text = text.slice(m[0].length); }
  return { text, project, files };
}
/** …/attachments/<id>/<name> -> its URL on the daemon. */
function fileOf(path) {
  const m = /\/attachments\/([0-9a-f]+)\/([^/]+)$/.exec(path), name = path.split('/').pop();
  return { path, name, url: m ? `/api/attachments/${m[1]}/${encodeURIComponent(m[2])}` : '', pic: !!m && PIC_EXT.test(name) };
}
