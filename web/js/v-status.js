/* The Status card: Tirith's development at a glance, first on the Overview (state.status, built by code in
   tirith/statuscard.py, no model). Four lines: is it going well, what is being worked on, what just landed, is it
   the owner's turn; plus the sentinel's last look (state.sentinel). */
(function () {
  var T = window.T, esc = T.esc;
  var DOT = { ok: '🟢', warn: '🟡', bad: '🔴', done: '✅' };
  function bar(done, total) {
    var n = total ? Math.round(10 * done / total) : 0;
    return '<span class="stbar" title="' + done + ' of ' + total + ' done">' + '█'.repeat(n) + '<i>' + '░'.repeat(10 - n) + '</i></span>';
  }
  function ago(iso) {
    var ms = T.now() - (T.ms(iso) || T.now()), m = Math.round(ms / 60000);
    return m < 1 ? 'just now' : m < 60 ? m + ' min ago' : Math.floor(m / 60) + ' h ago';
  }
  function sentinelLine(sn) {
    if (!sn) return '';
    if (!sn.enabled) return '<span class="mut">Sentinel off</span>';
    var last = sn.last, wake = sn.wake;
    var txt = 'Sentinel (' + esc(sn.model || '?') + ', every ' + esc(sn.every_min) + ' min, all projects): ' +
      (last ? (last.error ? 'could not look: ' + esc(last.error) : last.wake ? 'woke the CEO' : 'ok') + ' · ' + esc(ago(last.at)) : 'no look yet');
    if (wake && wake.why && (!last || last.at === wake.at)) txt += ' — ' + esc(wake.why);
    return '<span class="mut">' + txt + '</span>';
  }
  /* The page is redrawn on every state poll: a looping animation gets a negative delay from the clock so it goes
     on where it was instead of starting over; the confetti falls once, in the first seconds after "done" appears. */
  var doneSince = 0, CONFETTI_MS = 4200, COLORS = ['#22c55e', '#2f6fed', '#f59e0b', '#ec4899', '#a855f7', '#14b8a6'];
  function phase(ms) { return 'animation-delay:-' + (T.now() % ms) + 'ms'; }
  function confetti() {
    var age = T.now() - doneSince;
    if (age > CONFETTI_MS) return '';
    var h = '';
    for (var i = 0; i < 28; i++) {
      h += '<i style="left:' + ((i * 37) % 100) + '%;background:' + COLORS[i % COLORS.length] +
        ';animation-delay:' + ((i % 7) * 90 - age) + 'ms;--dx:' + (((i * 53) % 60) - 30) + 'px;--r:' + ((i * 97) % 720) + 'deg"></i>';
    }
    return '<div class="stconf" aria-hidden="true">' + h + '</div>';
  }
  function draw(tag, startMs) {  /* the check draws itself once; later redraws show it drawn */
    var age = T.now() - doneSince;
    return '<' + tag + (age < CONFETTI_MS ? ' class="drawing" style="animation-delay:' + (startMs - age) + 'ms"' : '') + '/>';
  }
  function doneCard(s, st) {
    if (!doneSince) doneSince = T.now();
    var plan = st.plan || {}, left = (plan.total || 0) - (plan.done || 0);
    var pct = plan.total ? Math.round(100 * plan.done / plan.total) : 100;
    var owner = st.owner_count ? '<div class="stl stown"><span class="stk">Left for you</span><b>' + st.owner_count + '</b> · ' +
      (st.owner || []).slice(0, 3).map(function (o) { return esc(o.text.length > 60 ? o.text.slice(0, 59) + '…' : o.text); }).join(' · ') + '</div>' : '';
    return '<section class="card stcard done">' + confetti() +
      '<div class="stdone"><div class="stbadge" style="' + phase(3000) + '"><svg viewBox="0 0 52 52" aria-hidden="true">' +
      draw('circle cx="26" cy="26" r="23"', 0) + draw('path d="M15 27l7 7 15-16"', 700) + '</svg></div>' +
      '<div class="stdtxt"><div class="stdtitle">All done</div>' +
      '<div class="stdsub">' + esc(String(st.why || '').replace(/^All done:\s*(.)/, function (m, c) { return c.toUpperCase(); })) + ' · ' + st.merged_today + ' merged today · CI ' + esc(st.ci || 'unknown') + '</div>' +
      '<div class="stdbar" title="' + (plan.done || 0) + ' of ' + (plan.total || 0) + ' plan items done"><i style="width:' + pct + '%;' + phase(2600) + '"></i></div>' +
      '<div class="stdnum mut">' + (plan.done || 0) + ' of ' + (plan.total || 0) + ' plan items done' +
      (left ? ' · the ' + left + ' left are not work for an agent' : '') + '</div></div></div>' +
      owner + '<div class="stl stsen">' + sentinelLine(s.sentinel) + '</div></section>';
  }
  T.statusCard = function (s) {
    var st = s.status;
    if (!st) return '';
    if (st.level === 'done') return doneCard(s, st);
    doneSince = 0;
    var head = '<div class="stl stl1"><b>' + (DOT[st.level] || '') + ' ' + esc(st.why) + '</b> · ' + st.merged_today +
      ' merged today · CI ' + esc(st.ci || 'unknown') + ' · ' + st.working + ' of ' + esc(st.limit || '?') + ' agents working</div>';
    var steps = (st.steps || []).map(function (x) {
      return '<button class="link stp" data-act="goplan" data-key="' + esc(x.project) + '">' + esc(x.title) + '</button> ' +
        bar(x.done || 0, x.total || 0) + ' <span class="mut">' + x.done + '/' + x.total + '</span>';
    }).join(' &nbsp;·&nbsp; ');
    var now = steps ? '<div class="stl"><span class="stk">Now</span>' + steps + '</div>' : '';
    var recent = (st.recent || []).length ? '<div class="stl"><span class="stk">Landed</span>' + st.recent.map(function (r) {
      return '<span title="' + esc(r.text) + '">' + esc(r.text.length > 70 ? r.text.slice(0, 69) + '…' : r.text) + '</span> <span class="mut">(' + esc(ago(r.at)) + ')</span>';
    }).join(' &nbsp;·&nbsp; ') + '</div>' : '';
    var owner = st.owner_count ? '<div class="stl stown"><span class="stk">Your turn</span><b>' + st.owner_count + '</b> · ' +
      (st.owner || []).slice(0, 3).map(function (o) { return esc(o.text.length > 60 ? o.text.slice(0, 59) + '…' : o.text); }).join(' · ') + '</div>' : '';
    return '<section class="card stcard ' + esc(st.level) + '">' + head + now + recent + owner +
      '<div class="stl stsen">' + sentinelLine(s.sentinel) + '</div></section>';
  };
})();
