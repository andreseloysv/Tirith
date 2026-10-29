/* Lane references in the chat. The lanes the page knows are the agents of the state (an agent's id is its lane
   id); T.chatOpts gives T.md those next to the file references, and md.js links them. A click opens the lane in
   the Agents view: its drawer (or its row in the list) shows the status, the log and the report. */
(function () {
  var T = window.T, ui = T.ui;

  /* the opts for T.md(text, opts) of a chat message of `project`: file and lane references, or null */
  T.chatOpts = function (state, project) {
    var files = T.fileOpts(state, project), ids = {}, n = 0;
    ((state && state.agents) || []).forEach(function (a) { if (a.id) { ids[a.id] = true; n++; } });
    if (!n) return files;
    var o = { project: project || '', lane: function (id) { return ids[id] === true; } };
    if (files) o.file = files.file;
    return o;
  };

  /* the lane's agent, shown even when the sidebar filters or the search would hide it */
  T.openLane = function (id) {
    var raw = T.rawState && T.rawState(), a = T.agentById(raw, id); if (!a) return;
    var f = T.filter, q = document.getElementById('f-q');
    if (f.project && f.project !== a.project) f.project = a.project;
    if (f.session && f.session !== a.session) f.session = '';
    if (f.q) { f.q = ''; if (q) q.value = ''; }
    if (f.mode === 'list') ui.showEarlier = true;
    ui.chatOpen = false; ui.openAgent = id; ui.drawer = 'agent';
    T.setFilter('view', 'agents');
  };

  /* capture: before the view's own click handlers, and the "#" never reaches the hash */
  document.addEventListener('click', function (e) {
    var a = e.target.closest && e.target.closest('a[data-act="golane"]'); if (!a) return;
    e.preventDefault(); e.stopPropagation();
    T.openLane(a.getAttribute('data-key'));
  }, true);
})();
