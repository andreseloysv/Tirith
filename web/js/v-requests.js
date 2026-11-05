/* The owner's requests: each chat request with its link (plan step, lanes or answer) and status (state.owner_requests). */
(function () {
  var T = window.T, esc = T.esc;
  var box = document.getElementById('ownerreqs');
  var LABEL = { open: 'Open', answered: 'Answered', done: 'Done' };
  function link(l) {
    if (!l) return '<span class="or-l none">not linked yet</span>';
    if (l.type === 'plan') {
      var lanes = (l.lanes || []).length ? ' · ' + l.lanes.map(esc).join(', ') : '';
      return '<span class="or-l">plan' + (l.step ? ' step ' + esc(l.step) : '') + lanes + '</span>';
    }
    return '<span class="or-l">answered by ' + esc(l.by || '') + '</span>';
  }
  function draw(rows) {
    if (!box) return;
    if (!rows || !rows.length) { box.hidden = true; box.innerHTML = ''; return; }
    var shown = rows.slice(-8).reverse();
    box.innerHTML = '<details><summary>Your requests <b>' + rows.length + '</b></summary>' + shown.map(function (r) {
      return '<div class="or-r ' + esc(r.status) + '"><span class="or-s">' + esc(LABEL[r.status] || r.status) + '</span> ' +
        '<span class="or-t" title="' + esc(r.text) + '">' + esc(String(r.text).slice(0, 80)) + '</span> ' + link(r.link) + '</div>';
    }).join('') + '</details>';
    box.hidden = false;
  }
  var base = T.render;
  T.render = function (fstate, raw) {
    var out = base.apply(this, arguments), s = raw || fstate;
    draw(s && s.owner_requests ? s.owner_requests : null);
    return out;
  };
})();
