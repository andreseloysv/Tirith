// The extension's worker: keeps the bot's link (the address and token from its tab «Enseñar», pasted once), starts
// and stops the recording with the daemon, and sends it each step the pages record. It stops by itself after 10
// minutes, and follows a Grabar/Parar pressed on Tirith's page.
var MOST_MS = 10 * 60 * 1000;
var lastSync = 0;

function link() {
  return chrome.storage.local.get('link').then(function (got) { return got.link || null; });
}

/** The pasted text, `http://127.0.0.1:<port>/api/teach/<bot>#<token>`, as {base, token}; null when it is not one. */
function parse(text) {
  var m = /^(https?:\/\/(?:127\.0\.0\.1|localhost):\d+\/api\/teach\/[^#\s]+)#([0-9a-f]{32})$/.exec(String(text || '').trim());
  return m ? { base: m[1], token: m[2] } : null;
}

function call(method, path, body) {
  return link().then(function (l) {
    if (!l) throw new Error('Paste the link from the bot\'s tab «Enseñar» first.');
    return fetch(l.base + path, {
      method: method,
      headers: { authorization: 'Bearer ' + l.token, 'content-type': 'application/json' },
      body: body ? JSON.stringify(body) : undefined,
    }).then(function (res) {
      if (res.status === 401) throw new Error('The token is not this bot\'s any more: paste it again.');
      if (!res.ok) throw new Error('Tirith answered ' + res.status);
      return res.json();
    });
  });
}

/** Keeps the recording state the pages read: on, and until when. */
function setState(on, since) {
  var until = on ? (Date.parse(since) || Date.now()) + MOST_MS : 0;
  if (on) chrome.alarms.create('teach-stop', { when: until }); else chrome.alarms.clear('teach-stop');
  return chrome.storage.local.set({ teach: { on: on, until: until } }).then(function () { return { on: on, until: until }; });
}

function sync() {
  lastSync = Date.now();
  return call('GET', '').then(function (r) { return setState(!!r.on, r.since); });
}

function start() { return call('POST', '/start').then(function (r) { return setState(true, r.since); }); }
function stop() { return call('POST', '/stop').then(function () { return setState(false); }); }

function step(s) {
  return call('POST', '/events', s).then(function (r) { if (!r.on) return setState(false); return null; });
}

function save(text) {
  var l = parse(text);
  if (!l) return Promise.reject(new Error('That is not the link from the tab «Enseñar».'));
  return chrome.storage.local.set({ link: l }).then(sync);
}

var handlers = {
  step: function (m) { return step(m.step); },
  sync: function () { return Date.now() - lastSync < 3000 ? Promise.resolve(null) : sync(); },
  start: start,
  stop: stop,
  save: function (m) { return save(m.save); },
  state: function () { return chrome.storage.local.get(['teach', 'link']); },
};

chrome.runtime.onMessage.addListener(function (m, _sender, reply) {
  var name = Object.keys(handlers).find(function (k) { return k in m; });
  if (!name) return false;
  handlers[name](m).then(function (r) { reply({ ok: true, r: r }); }, function (e) { reply({ ok: false, error: e.message }); });
  return true; // the reply comes later
});

chrome.alarms.create('teach-sync', { periodInMinutes: 0.5 });
chrome.alarms.onAlarm.addListener(function (a) {
  if (a.name === 'teach-stop') stop().catch(function () { setState(false); });
  if (a.name === 'teach-sync') sync().catch(function () {});
});
