/* The now bar: what merged into main today, what is in progress, what is stuck, and who has it (state.now). */
(function () {
  var T = window.T, esc = T.esc;
  var bar = document.getElementById('nowbar');
  function group(cls, label, items, count) {
    if (!count) return '';
    var shown = items.slice(0, 4).map(function (i) {
      var what = i.item || i.name || i.lane;
      return '<span class="nb-i" title="' + esc((i.why ? i.why + ' · ' : '') + what) + '"><b>' + esc(i.agent) + '</b> ' +
        esc(String(what).slice(0, 48)) + (i.why ? ' <em>' + esc(String(i.why).slice(0, 40)) + '</em>' : '') + '</span>';
    }).join('');
    var more = count > 4 ? '<span class="nb-more">+' + (count - 4) + '</span>' : '';
    return '<div class="nb-g ' + cls + '"><span class="nb-h">' + label + ' <b>' + count + '</b></span>' + shown + more + '</div>';
  }
  function draw(now) {
    if (!bar) return;
    if (!now || !now.counts) { bar.hidden = true; return; }
    var c = now.counts, html = group('merged', 'Merged today', now.merged || [], c.merged) +
      group('doing', 'In progress', now.doing || [], c.doing) + group('stuck', 'Stuck', now.stuck || [], c.stuck);
    bar.innerHTML = html || '<span class="nb-h">Nothing running, nothing stuck.</span>';
    bar.hidden = false;
  }
  var base = T.render;
  T.render = function (fstate, raw) {
    var out = base.apply(this, arguments), s = raw || fstate;
    draw(s && s.now ? s.now : null);
    return out;
  };
})();
