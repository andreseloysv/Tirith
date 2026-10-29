"""Writes fixtures/state.planquiet.js: the Plan in every state (done, working, queued, waiting for you,
from a date, testing, review failed, pending request, failed action).
Open web/index.html#state=../fixtures/state.planquiet.js"""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
state = json.loads((ROOT / "fixtures" / "state.demo.json").read_text())
base = next(p for p in state["projects"] if p["id"] == "tirith")
state["projects"] = [base]
state["waiting_on_you"] = []


def item(text, done=False, who="agent", **kw):
    return dict(text=text, done=done, who=who, **kw)


def stp(i, title, status, items, agents=(), until=None):
    s = dict(id=str(i), title=title, status=status, done=sum(1 for x in items if x["done"]),
             total=len(items), items=items, agents=list(agents))
    if until:
        s["until"] = until
    return s


steps = [
    stp(1, "Collector y esquema", "done", [item("Esquema del estado", True), item("Pruebas del recolector", True)]),
    stp(2, "Vista de agentes", "done", [item("Tarjetas", True), item("Detalle", True), item("Registro", True)]),
    stp(3, "Hooks de agentes", "active", [item("Instalar hooks", True), item("Proteger los hooks contra borrado"),
                                          item("Probar en un proyecto limpio")], ["t-world"]),
    stp(4, "Lanzar un sub-agente desde el Plan", "ready", [item("Botón de lanzar", True),
                                                           item("Registrar el manejador de enlaces"), item("Mostrar el estado")]),
    stp(5, "Revisión automática", "ready", [item("Ejecutar las pruebas del carril", True), item("Unir al terminar")]),
    stp(6, "Reglas por proyecto", "ready", [item("Leer reglas", True), item("Avisar de infracciones")]),
    stp(7, "Instalar el manejador", "blocked-owner", [
        item("Hecho en el código", True),
        item("Registrar tirith:// en tu Mac (dueño: instala algo)", who="owner", owner_item="owner-tirith-7-1a2b3c4d")]),
    stp(8, "Publicar la versión", "blocked-date", [
        item("Escribir las notas", True),
        item("Etiquetar la versión (desde 2026-11-02)", who="date", until="2026-11-02")], until="2026-11-02"),
    stp(9, "Informe semanal", "ready", [item("Resumen de la semana"), item("Enviar al correo")]),
    stp(10, "Limpieza de configuración", "ready", [item("Borrar claves viejas"), item("Migrar el archivo")]),
]
base["roadmap"] = dict(file="docs/TODO.md", deadline="2026-11-04T21:00:00-05:00",
                       done=sum(s["done"] for s in steps), total=sum(s["total"] for s in steps), steps=steps)

base.pop("plans", None)  # the plan switcher would show the demo plans


def agent(id, step, status, review, name):
    a = copy.deepcopy(next(x for x in state["agents"] if x["id"] == "t-world"))
    a.update(id=id, step=str(step), status=status, name=name, review=review, summary="")
    return a


state["agents"] = [x for x in state["agents"] if x["project"] != "tirith" or x["id"] == "t-world"]
for x in state["agents"]:
    if x["id"] == "t-world":
        x["summary"] = "Writing the guard that stops hooks being removed"
state["agents"] += [
    agent("lane-merged", 1, "done", dict(status="merged", at="2026-10-28T18:49:00Z", files=3, passed=496), "Periothile"),
    agent("lane-test", 5, "done", dict(status="running", at="2026-10-28T19:36:00Z", run="run-1"), "Quilitara"),
    agent("lane-bad", 6, "done", dict(status="tests-failed", at="2026-10-28T19:30:00Z", run="run-2",
                                      message="2 failed: test_rules_load, test_rules_empty"), "Marrow"),
]
state["autopilot"] = dict(enabled=True, max_parallel=4, started=[], paused_steps=[])


def req(id, kind, step, status, answers=(), lanes=(), text="", at="2026-10-28T18:31:00Z"):
    r = dict(id=id, at=at, kind=kind, project="tirith", step=str(step), text=text, status=status,
             answers=list(answers), lanes=list(lanes))
    r["from"] = "owner"
    return r


def ans(text, at, actions=(), lane=None):
    return dict(at=at, by="manager", auto=False, text=text, lane=lane, actions=list(actions))


def act(id, label, run):
    return dict(id=id, label=label, command="uv run tirith url install", cwd=None, created_by="manager",
                at="2026-10-28T19:00:00Z", last_run=run)


state["inbox"] = [
    req("in-00000001", "finish", 1, "done", [ans("Started lane **periothile** on step 1.", "2026-10-28T18:31:00Z", lane="lane-merged")], ["lane-merged"]),
    req("in-00000002", "ask", 2, "done", [ans("All three items are merged.", "2026-10-28T17:40:00Z")], text="Is this step finished?"),
    req("in-00000003", "finish", 3, "answered", [ans("Marrow already has this step. **No new agent needed.**", "2026-10-28T19:39:25Z", lane="t-world")], ["t-world"]),
    req("in-00000004", "ask", 4, "done", [ans("Install the handler with one click.", "2026-10-28T07:52:00Z", [
        act("ac-pass0001", "Install the link handler", dict(status="passed", at="2026-10-28T08:24:00Z", exit_code=0, log_tail=[]))])],
        text="What is left?"),
    req("in-00000009", "ask", 9, "open", text="Can you start this one?", at="2026-10-28T19:36:00Z"),
    req("in-00000010", "ask", 10, "answered", [ans("Run this once, then I mark the step.", "2026-10-28T19:20:00Z", [
        act("ac-fail0001", "Migrate the config file", dict(status="failed", at="2026-10-28T19:25:00Z", exit_code=2, log_tail=["permission denied"]))])],
        text="How do I finish this?"),
]
(ROOT / "fixtures" / "state.planquiet.js").write_text(
    "window.TIRITH_STATE = " + json.dumps(state, indent=2, ensure_ascii=False) + ";\n")
