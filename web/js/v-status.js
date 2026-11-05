/* The Status card: Tirith's development at a glance, first on the Overview (state.status, built by code in
   tirith/statuscard.py, no model). Four lines: is it going well, what is being worked on, what just landed, is it
   the owner's turn; plus the sentinel's last look (state.sentinel). */
(function () {
  var T = window.T, esc = T.esc;
  var DOT = { ok: '🟢', warn: '🟡', bad: '🔴' };
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
    var txt = 'Sentinel (' + esc(sn.model || '?') + ', every ' + esc(sn.every_min) + ' min): ' +
      (last ? (last.error ? 'could not look: ' + esc(last.error) : last.wake ? 'woke the CEO' : 'ok') + ' · ' + esc(ago(last.at)) : 'no look yet');
    if (wake && wake.why && (!last || last.at === wake.at)) txt += ' — ' + esc(wake.why);
    return '<span class="mut">' + txt + '</span>';
  }
  T.statusCard = function (s) {
    var st = s.status;
    if (!st) return '';
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
