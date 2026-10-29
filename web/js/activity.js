/* What an agent is doing: goal, status, tests, tools, files, live events. Shared by Flow and the Agents drawer. */
(function () {
  var T = window.T, esc = T.esc, MAXFILES = 12, MAXEVENTS = 12;
  var CH = { added: 'A', modified: 'M', deleted: 'D', renamed: 'R', untracked: 'U' };
  var RES = { passed: 'passed', failed: 'failed', running: 'running' };

  /* "12 s ago", ticking through the .agol class (see main.js tick) */
  T.agoLong = function (iso) { var s = T.ago(iso); return !s ? '' : s === 'now' ? 'just now' : s + ' ago'; };
  var agoLong = function (iso) { return iso ? '<span class="agol" data-at="' + esc(iso) + '">' + esc(T.agoLong(iso)) + '</span>' : ''; };

  /* state of the agent for the dot and the status line: active / idle / done / failed */
  T.agentState = function (a) {
    if (a.status === 'done' || a.status === 'failed') return a.status;
    return a.active ? 'active' : 'idle';
  };

  function sec(title, body, n) {
    return '<section class="asec"><h4>' + esc(title) + (n != null ? ' <span class="n">' + n + '</span>' : '') + '</h4>' + body + '</section>';
  }
  function none(t) { return '<p class="quiet sm">' + esc(t) + '</p>'; }

  function status(a, o) {
    var st = T.agentState(a), h = '<div class="astat"><span class="sbadge ' + st + '">' + (st === 'active' ? '<i class="pdot2"></i>' : '') + T.cap(st) + '</span>';
    if (o.clock) h += '<span class="mut">' + (a.status === 'working' ? 'running ' : 'took ') + T.agentClock(a) + '</span>';
    if (a.last_activity_at && st !== 'done' && st !== 'failed') h += '<span class="mut">last activity ' + agoLong(a.last_activity_at) + '</span>';
    /* the report is the full answer; the one-line summary only shows when it says something else */
    var text = o.sum === false ? '' : a.report || (T.sameText(a.summary, a.report) ? '' : a.summary);
    return h + '</div>' + (text ? '<div class="sum">' + T.mdBox(text, a.id + ':status') + '</div>' : '');
  }
  /* true when one text starts like (or sits inside) the other, ignoring markdown marks and spacing */
  function flat(t) { return String(t || '').toLowerCase().replace(/[`*_#>~\[\]()|-]/g, '').replace(/\s+/g, ' ').trim(); }
  T.sameText = function (a, b) {
    var x = flat(a), y = flat(b);
    if (!x || !y) return false;
    return y.indexOf(x.slice(0, 60)) >= 0 || x.indexOf(y.slice(0, 60)) >= 0;
  };
  /* the owner's request, on the root lane of a tree only (children inherit it from the collector) */
  function asked(a) {
    var s = T.rawState && T.rawState();
    if (!a.request || !a.request.text) return '';
    if (a.parent && s && T.agentById(s, a.parent)) return '';
    return sec('Asked by you', T.mdBox(a.request.text, a.id + ':asked'));
  }
  function tests(a) {
    var l = (a.tests || []).slice().reverse();
    if (!l.length) return none('No tests yet');
    return '<ul class="trows">' + l.map(function (t) {
      var r = RES[t.result] || 'unknown';
      var cnt = t.passed != null ? t.passed + ' passed' + (t.failed ? ', ' + t.failed + ' failed' : '') : '';
      return '<li><span class="rb ' + r + '">' + (r === 'unknown' ? 'no result' : r) + '</span><code title="' + esc(t.full || t.command) + '">' + esc(t.command) + '</code>' +
        (cnt ? '<span class="mut tcnt">' + esc(cnt) + '</span>' : '') + '<span class="mut when">' + esc(T.clockTime(t.at)) + '</span></li>';
    }).join('') + '</ul>';
  }
  function tools(a) {
    var l = a.tools || [];
    if (!l.length) return none('No tools used yet');
    return '<div class="chips">' + l.map(function (t) { return '<span class="chip">' + esc(t.name) + ' <b>' + t.count + '</b></span>'; }).join('') + '</div>';
  }
  function files(a) {
    var l = a.files || [];
    if (!l.length) return none('No files changed yet');
    var h = '<ul class="frows">' + l.slice(0, MAXFILES).map(function (f) {
      var k = CH[f.change] || '?', ins = f.insertions || 0, del = f.deletions || 0, i = f.path.lastIndexOf('/');
      return '<li><span class="cb ' + esc(f.change) + '" title="' + esc(f.change) + '">' + k + '</span><span class="fp" title="' + esc(f.path) + '">' +
        (i >= 0 ? '<span class="mut">' + esc(f.path.slice(0, i + 1)) + '</span>' : '') + esc(f.path.slice(i + 1)) + '</span>' +
        (ins || del ? '<span class="fn"><span class="plus">+' + ins + '</span> <span class="minus">−' + del + '</span></span>' : '') + '</li>';
    }).join('') + '</ul>';
    if (l.length > MAXFILES) h += '<p class="mut sm">and ' + (l.length - MAXFILES) + ' more</p>';
    return h;
  }
  function events(a) {
    var l = (a.events || []).slice(-MAXEVENTS).reverse();
    if (!l.length) return none('No activity yet');
    return '<ul class="erows">' + l.map(function (e) {
      return '<li><span class="mut t">' + esc(T.clockTime(e.ts)) + '</span><span class="tool">' + esc(e.tool || e.event) + '</span><span class="what" title="' + esc(e.summary) + '">' + esc(e.summary) + '</span></li>';
    }).join('') + '</ul>';
  }

  /* opts.goal: show the Goal section (off when the card beside it already shows the same text);
     opts.sum: show the one-line summary (off when the card beside it shows it);
     opts.clock: show the running time (off when the card beside it already shows it) */
  T.activity = function (a, opts) {
    var o = opts || {}, h = '';
    h += asked(a);
    if (o.goal !== false && a.goal) h += sec('Goal', '<div class="agoal">' + T.mdBox(a.goal, a.id + ':goal') + '</div>');
    h += sec('Status', status(a, o));
    if (o.log !== false) h += sec('Log', T.logView(a));
    h += sec('Tests', tests(a), (a.tests || []).length || null);
    h += sec('Tools', tools(a));
    h += sec('Files', files(a), (a.files || []).length || null);
    h += sec('Live activity', events(a));
    return '<div class="activity">' + h + '</div>';
  };
})();
