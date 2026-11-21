// The extension's button: Grabar/Parar, how long is left, and where the bot's link is pasted once.
var go = document.getElementById('go');
var say = document.getElementById('say');
var err = document.getElementById('err');
var timer = 0;

function ask(m) {
  return chrome.runtime.sendMessage(m).then(function (r) {
    if (!r || !r.ok) throw new Error((r && r.error) || 'No answer');
    return r.r;
  });
}

function show(st) {
  var teach = st.teach || {};
  var on = !!teach.on && Date.now() < (teach.until || 0);
  go.textContent = on ? 'Parar' : 'Grabar';
  go.className = on ? 'rec' : '';
  go.disabled = !st.link;
  document.getElementById('setup').open = !st.link;
  clearInterval(timer);
  if (!st.link) { say.textContent = 'Primero pega el enlace de la pestaña «Enseñar» de tu bot en Tirith.'; return; }
  if (!on) { say.textContent = 'Haz la tarea una vez. Contraseñas, códigos y tarjetas quedan tapados (•••). Para solo a los 10 minutos.'; return; }
  var tick = function () {
    var left = Math.max(0, Math.round((teach.until - Date.now()) / 1000));
    say.textContent = 'Grabando · quedan ' + Math.floor(left / 60) + ':' + String(left % 60).padStart(2, '0') + '. Lo que escribas en el chat del bot va con los pasos.';
  };
  tick();
  timer = setInterval(tick, 1000);
}

function refresh() {
  return ask({ state: true }).then(show);
}

function fail(e) { err.textContent = e.message; }

go.addEventListener('click', function () {
  err.textContent = '';
  ask(go.textContent === 'Parar' ? { stop: true } : { start: true }).then(refresh, fail);
});

document.getElementById('save').addEventListener('click', function () {
  err.textContent = '';
  ask({ save: document.getElementById('link').value }).then(refresh, fail);
});

ask({ sync: true }).catch(function () {}).then(refresh);
