// The owner's page: visitors and views per day, the top pages and where people came from.
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const rows = (list, cols) => list.map((r) => `<tr>${cols.map((c) => `<td>${esc(r[c] || '(direct)')}</td>`).join('')}</tr>`).join('');

export function dashboard(store, n = 30) {
  const table = (title, head, body) => `<h2>${title}</h2><table><tr>${head.map((h) => `<th>${h}</th>`).join('')}</tr>${body}</table>`;
  return `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>OpenBot stats</title><style>body{font:16px/1.5 system-ui,sans-serif;max-width:760px;margin:32px auto;padding:0 16px}
table{border-collapse:collapse;width:100%}td,th{text-align:left;padding:6px 10px;border-bottom:1px solid #e9e7e2}th{color:#6f6f6f;font-weight:500}</style>
<h1>OpenBot site, last ${n} days</h1>
${table('Per day', ['Day', 'Visitors', 'Views'], rows(store.days(n), ['day', 'visitors', 'views']))}
${table('Pages', ['Page', 'Visitors', 'Views'], rows(store.top('path', n), ['name', 'visitors', 'views']))}
${table('From', ['Site', 'Visitors', 'Views'], rows(store.top('ref', n), ['name', 'visitors', 'views']))}
<p style="color:#6f6f6f">No cookies. A visitor is a daily hash that cannot be followed to the next day.</p></html>`;
}
