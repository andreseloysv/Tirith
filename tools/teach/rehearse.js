// The rehearsal, in the worker: while the bot rehearses a draft, ask the daemon for its next step (a long poll with the
// bot's token), do it in the rehearsal's own tab (open a page, or let act.js click or type there), and answer with the
// result and a screenshot of the tab (chrome.tabs.captureVisibleTab). It ends with the rehearsal, at 10 minutes at most.
/* global call, link */
/* exported rehearseLoop, rehearsing */
var looping = false;

function rehearsal() { return chrome.storage.local.get('rehearse').then(function (g) { return g.rehearse || {}; }); }
function pause(ms) { return new Promise(function (done) { setTimeout(done, ms); }); }

/** Waits until the tab has loaded, 15 s at most; `go` starts the load and resolves to the tab. */
function loaded(go) {
  return new Promise(function (done) {
    var id = null;
    var timer = setTimeout(finish, 15000);
    function finish() { clearTimeout(timer); chrome.tabs.onUpdated.removeListener(on); done(id); }
    function on(tid, info) { if (tid === id && info.status === 'complete') finish(); }
    chrome.tabs.onUpdated.addListener(on);
    go().then(function (tab) { id = tab.id; if (tab.status === 'complete' && tab.url !== 'about:blank') finish(); }, finish);
  });
}

function open(st, url) {
  var fresh = function () { return chrome.tabs.create({ url: url, active: true }); };
  var go = st.tab ? function () { return chrome.tabs.update(st.tab, { url: url, active: true }).catch(fresh); } : fresh;
  return loaded(go).then(function (id) {
    st.tab = id;
    return chrome.storage.local.set({ rehearse: st }).then(function () { return { ok: !!id, result: 'opened ' + url }; });
  });
}

/** A click or a value typed, done by act.js in the tab, never on Tirith's own page; then the page a click may have opened gets time to load. */
function act(st, cmd) {
  if (!st.tab) return Promise.resolve({ ok: false, result: 'no page is open in the rehearsal yet: open one first' });
  return Promise.all([chrome.tabs.get(st.tab), link()]).then(function (got) {
    var at = new URL(got[0].url || 'about:blank');
    var own = at.port === new URL(got[1].base).port && /^(localhost\.?|127\..*|\[.*\])$/.test(at.hostname);
    if (own) return { ok: false, refused: true, result: 'that is Tirith\'s own page, where a click is the owner\'s yes' };
    return chrome.tabs.sendMessage(st.tab, { rehearse: cmd });
  })
    .catch(function () { return { ok: false, result: 'the rehearsal\'s tab is closed or still loading' }; })
    .then(function (r) {
      return pause(800).then(function () { return loaded(function () { return chrome.tabs.get(st.tab); }); })
        .then(function () { return r || { ok: false, result: 'the page did not answer' }; });
    });
}

function shot(st) {
  if (!st.tab) return Promise.resolve('');
  return chrome.tabs.update(st.tab, { active: true })
    .then(function (tab) { return chrome.tabs.captureVisibleTab(tab.windowId, { format: 'jpeg', quality: 60 }); })
    .catch(function () { return ''; });
}

function run(cmd) {
  return rehearsal().then(function (st) {
    st.until = Date.now() + (cmd.left_ms || 0); // the daemon says how long is left: this machine's clock may differ
    var did = cmd.kind === 'open' ? open(st, cmd.target) : act(st, cmd);
    return did.then(function (r) {
      if (st.tab) chrome.tabs.sendMessage(st.tab, { banner: st.until }).catch(function () {});
      return shot(st).then(function (s) { return { id: cmd.id, ok: !!r.ok, refused: !!r.refused, result: r.result, shot: s }; });
    });
  });
}

function over() {
  return rehearsal().then(function (st) {
    if (st.tab) chrome.tabs.sendMessage(st.tab, { banner: 0 }).catch(function () {});
    return chrome.storage.local.set({ rehearse: {} });
  });
}

/** Asks for steps until the rehearsal is over; one loop at a time (the alarm and each page may start it). */
function rehearseLoop() {
  if (looping) return Promise.resolve();
  looping = true;
  var turn = function () {
    return call('GET', '/next').then(function (r) {
      if (!r.on) return over();
      return chrome.storage.local.get('rehearse').then(function (g) { // an extension call: keeps the worker awake
        return chrome.storage.local.set({ rehearse: Object.assign(g.rehearse || {}, { until: Date.now() + (r.left_ms || 0) }) });
      }).then(function () {
        if (r.cmd) return run(r.cmd).then(function (answer) { return call('POST', '/done', answer); }).then(turn);
        return turn();
      });
    });
  };
  return turn().catch(function () {}).then(function () { looping = false; });
}

/** For a page of the rehearsal's tab, when its banner shows until; 0 for any other page. */
function rehearsing(sender) {
  return rehearsal().then(function (st) {
    var ours = sender && sender.tab && sender.tab.id === st.tab && Date.now() < (st.until || 0);
    if (ours) rehearseLoop();
    return ours ? st.until : 0;
  });
}
