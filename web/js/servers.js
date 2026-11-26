// The servers window of the desktop app: this Mac, the team servers added, sign in with Microsoft.
const call = (name, args) => window.__TAURI__.core.invoke(name, args);
const $ = (id) => document.getElementById(id);
const say = (text) => { $('say').textContent = text || ''; };
const button = (label, run) => {
  const b = document.createElement('button');
  b.textContent = label;
  b.onclick = async () => {
    b.disabled = true;
    try { await run(); } catch (e) { say(String(e)); }
    b.disabled = false;
    render();
  };
  return b;
};
const row = (label, note, on, buttons) => {
  const r = document.createElement('div');
  r.className = on ? 'row on' : 'row';
  const t = document.createElement('span');
  t.textContent = label;
  if (note) { const s = document.createElement('small'); s.textContent = note; t.append(s); }
  r.append(t, ...buttons);
  return r;
};
async function render() {
  const s = await call('team_state');
  const list = $('list');
  list.replaceChildren(row('This Mac', 'your own NeoBot on 127.0.0.1', !s.active, [
    button('Show', () => call('team_use', { url: null })),
  ]));
  for (const url of s.servers) {
    const who = s.people[url];
    const note = who ? `signed in as ${who.name} (${who.role})` : 'not signed in';
    list.append(row(url, note, s.active === url, [
      button('Sign in with Microsoft', async () => {
        say('Finish signing in in your browser...');
        const p = await call('team_sign_in', { url });
        say(`Signed in as ${p.name} (${p.role}).`);
      }),
      button('Show', () => call('team_use', { url })),
      button('Remove', () => call('team_remove', { url })),
    ]));
  }
}
$('add').onclick = async () => {
  try { await call('team_add', { url: $('url').value }); $('url').value = ''; say(''); } catch (e) { say(String(e)); }
  render();
};
render();
