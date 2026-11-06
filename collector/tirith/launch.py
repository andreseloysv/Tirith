"""``tirith launch``: finish one roadmap step with a sub-agent, started from a ``tirith://`` link.

The web page is a file:// page and cannot run commands, so its button opens
``tirith://launch?project=..&step=..&token=..``. A tiny local handler (``tirith url install``)
turns that into ``tirith url open``, which checks the token and calls ``tirith launch``.
See ``docs/CONTRACT-launch.md``.
"""

from __future__ import annotations

import json
import os
import plistlib
import re
import secrets
import shlex
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import parse_qs, urlencode, urlsplit

from tirith import codemap as codemap_mod
from tirith import timeouts as timeouts_mod
from tirith import manifest as manifest_mod
from tirith import models as models_mod
from tirith import roadmap as roadmap_mod
from tirith import tirithcmd
from tirith.home import Config, now_iso
from tirith.store import read_json, write_json_atomic

LAUNCH_FILE = "launch.json"
SCHEME = "tirith"
APP_NAME = "Tirith Launcher.app"
BUNDLE_ID = "dev.tirith.launcher"
INSTALL_COMMAND = "tirith url install"

# keywords of a step title -> hero class; first match wins, else stonewright
CLASS_BY_KEYWORD = (
    (("test", "prueba", "verific", "regla", "check", "seguridad", "guarda"), "warden"),
    (("doc", "lore", "registro", "decision"), "loremaster"),
    (("web", "visib", "vista", "panel", "plan", "ui", "logs", "app"), "glasswright"),
    (("pipeline", "hook", "ci", "deploy", "release"), "swiftrider"),
)
DEFAULT_CLASS = "stonewright"

PLAN_RULES = (
    "The plan is the only way to start and to finish: never edit `docs/TODO.md` (a gate blocks it). "
    "Missing work goes in with `tirith plan add --project <id> --step <N> --text \"<item>\"`.",
    "Before you finish, report the outcome of your item: `tirith plan report --lane $TIRITH_LANE --status "
    "done|partial|blocked [--add \"<new item>\"]... [--reason \"<why>\"]`. `partial` needs the rest as `--add` "
    "items, `blocked` needs `--reason`. Without a report your work is not merged.",
    "Never merge into or push to the main checkout (a gate blocks it). Finish with the report above: Tirith's "
    "single merge queue brings main in, runs the full suite, merges and pushes one lane at a time, and a "
    "conflict or a red suite comes back to you.",
)

HARD_RULES = (
    "Commit your work on the lane branch (plain `git add` and `git commit`), with no attribution trailer (the "
    "commit guard rejects it). Never merge and never push: the single merge queue reviews the branch with the "
    "full suite and merges it.",
    "Name agents only by their alias (see `docs/CONTRACT-hooks.md` \"Agent aliases\"), never by product or model name.",
    "No servers and no open ports.",
    "The tests must pass before you finish. Run them only with `tirith run --project <id> --stage test` "
    "(`<id>` is the project id of this lane); never call the test runner directly.",
    "Do not touch git hooks, `.git/`, or anything outside this worktree.",
    "Do not start sub-agents except with `tirith lane run`.",
    "A Tirith block is not a stop: use the alternative it gives and continue; never wait for the owner.",
    "Finish with a short report: what changed (files), the test result, and anything left open.",
    *PLAN_RULES,
)


class LaunchError(Exception):
    """A launch that must not start; the message says why and what to do instead."""


# -- token ------------------------------------------------------------------------------------


def launch_token(home: Path, create: bool = True) -> str | None:
    """The owner's launch token from ``<home>/launch.json`` (created on first use)."""
    path = home / LAUNCH_FILE
    data = read_json(path, {})
    token = data.get("launch_token") if isinstance(data, dict) else None
    if isinstance(token, str) and len(token) >= 16:
        return token
    if not create:
        return None
    token = secrets.token_urlsafe(24)
    write_json_atomic(path, {"launch_token": token, "created_at": now_iso()})
    try:
        path.chmod(0o600)
    except OSError:
        pass
    return token


def token_ok(home: Path, given: str | None) -> bool:
    expected = launch_token(home, create=False)
    return bool(expected and given and secrets.compare_digest(expected, given))


# -- url --------------------------------------------------------------------------------------


def parse_url(url: str) -> dict:
    """``tirith://launch?project=a&step=3&token=t`` -> ``{action, project, step, token, agent, lane}``."""
    parts = urlsplit(str(url).strip())
    if parts.scheme.lower() != SCHEME:
        raise LaunchError(f"not a {SCHEME}:// link: {url!r}")
    action = (parts.netloc or parts.path.strip("/")).lower()
    query = {k: v[-1] for k, v in parse_qs(parts.query).items()}
    return {"action": action, "project": query.get("project", ""), "step": query.get("step", ""),
            "token": query.get("token", ""), "agent": query.get("agent") or None,
            "lane": query.get("lane", ""), "item": query.get("item", "")}


def build_review_url(lane: str, token: str) -> str:
    return f"{SCHEME}://review?" + urlencode({"lane": lane, "token": token})


def build_url(project: str, step: str, token: str) -> str:
    return f"{SCHEME}://launch?" + urlencode({"project": project, "step": step, "token": token})


# -- the plan ---------------------------------------------------------------------------------


def find_step(folder: Path, step_id: str) -> tuple[dict, str]:
    """``(step, roadmap intro)`` for a project folder."""
    path = roadmap_mod.find_file(folder, manifest_mod.load(folder).roadmap)
    steps = roadmap_mod._parsed(path) if path else None
    if steps is None:
        raise LaunchError(f"no roadmap found in {folder}")
    step = next((s for s in steps if s["id"] == str(step_id)), None)
    if step is None:
        raise LaunchError(f"step {step_id} is not in {path}")
    return step, roadmap_mod.read_intro(path)


def pick_class(title: str) -> str:
    low = title.lower()
    for words, hero_class in CLASS_BY_KEYWORD:
        if any(re.search(r"\b" + re.escape(w), low) for w in words):
            return hero_class
    return DEFAULT_CLASS


def working_lane(lanes: list[dict], project: str, step_id: str, status_of, item: str | None = None) -> dict | None:
    """The working lane of ``project`` in step ``step_id``; with ``item``, only one working on that same item
    (a lane that names no item holds its whole step). Two lanes on different items of a step are no duplicates."""
    now = datetime.now(timezone.utc)
    want = roadmap_mod.split_status(item)[0] if item else None
    for lane in lanes:
        if (lane.get("project") == project and str(lane.get("step") or "") == str(step_id)
                and status_of(lane, now) == "working"):
            have = roadmap_mod.split_status(str(lane.get("item") or ""))[0]
            if want is None or not have or have == want:
                return lane
    return None


def build_brief(title: str, intro: str, open_items: list[str], step_id: str, item: str | None = None,
                repo: Path | None = None) -> str:
    out = [f"# Lane: {title}", "", "## Why",
           f"Step {step_id} of the project plan: {title}.", ""]
    if intro:
        out += [intro, ""]
    if item:
        out += ["## Your item", f"- {item}", "",
                "Report this item when you finish (see the hard rules); the rest of the step is not yours.", ""]
    out += ["## What to build", *(f"- {text}" for text in (open_items if not item else [item])), ""]
    code_map = codemap_mod.section(repo, item or " ".join(open_items))
    if code_map:
        out += code_map.rstrip("\n").split("\n") + [""]
    out += ["## Hard rules", *(f"- {rule}" for rule in HARD_RULES), "",
            "## Where to read",
            "- `docs/TODO.md` (read only): the plan; do not edit it. Only Tirith writes it.",
            "- `docs/CONTRACT*.md`: the contracts of the parts you touch; update the one you change.",
            "", "## Worker coordination",
            "- Read your local inbox with `tirith task inbox --project $TIRITH_PROJECT --lane $TIRITH_LANE`.",
            "- After handling messages, acknowledge their exact IDs with `tirith task ack --project $TIRITH_PROJECT --lane $TIRITH_LANE <ids>`.",
            "- Send a local handoff with `tirith task send --project $TIRITH_PROJECT --sender $TIRITH_LANE --to <lane> --key <unique-key> \"<message>\"`; reuse the key when retrying delivery.",
            ""]
    return "\n".join(out)


# -- worktree and command ---------------------------------------------------------------------


def worktree_path(repo: Path, lane_id: str) -> Path:
    return repo.parent / f"{repo.name}-lanes" / lane_id


def branch_exists(repo: Path, branch: str) -> bool:
    return timeouts_mod.run(["git", "-C", str(repo), "rev-parse", "--verify", "-q", f"refs/heads/{branch}"],
                            "git").returncode == 0


RESERVED_PREFIXES = ("manager", "ceo-", "canary-")  # Tirith's own lanes; a work lane named so was taken for one


def lane_id_for(title: str, taken: set[str], repo: Path) -> str:
    base = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")[:32].strip("-") or "lane"
    if base.startswith(RESERVED_PREFIXES):  # "Manager de guardia…" gave "manager-de-guardia…": counted as the manager
        base = ("step-" + base)[:32].strip("-")
    candidate, n = base, 2
    while (candidate in taken or worktree_path(repo, candidate).exists()
           or branch_exists(repo, f"lane/{candidate}")):
        candidate, n = f"{base}-{n}", n + 1
    return candidate


def sparse_patterns(exclude: list[str]) -> str:
    """Non-cone sparse-checkout patterns: everything except each excluded path."""
    return "\n".join(["/*", *(f"!/{p}/" for p in exclude)]) + "\n"


def worktree_add(repo: Path, path: Path, *, branch: str | None = None, base: str | None = None,
                 exclude: list[str] | None = None) -> str | None:
    """``git worktree add`` at ``path`` (new ``branch`` from ``base``, else a detached ``base``/HEAD), with the
    ``exclude`` paths left out of the checkout (sparse, so committing there never deletes them). On failure or
    timeout the half-made folder is removed and worktrees pruned; returns the error text, else ``None``."""
    args = ["git", "-C", str(repo), "worktree", "add", "-q", *(["--no-checkout"] if exclude else []),
            *(["-b", branch] if branch else ["--detach"]), str(path), *([base] if base else [])]
    done = timeouts_mod.run(args, "git-worktree-add", text=True)
    if done.returncode == 0 and exclude:
        done = timeouts_mod.run(["git", "-C", str(path), "sparse-checkout", "set", "--no-cone", "--stdin"], "git",
                                text=True, input=sparse_patterns(exclude))
        if done.returncode == 0:
            done = timeouts_mod.run(["git", "-C", str(path), "checkout", "-q"], "git-worktree-add", text=True)
    if done.returncode == 0:
        return None
    error = (getattr(done, "error", None) or done.stderr or done.stdout or "git failed").strip()
    shutil.rmtree(path, ignore_errors=True)
    timeouts_mod.run(["git", "-C", str(repo), "worktree", "prune"], "git", text=True)
    return error


def create_worktree(repo: Path, lane_id: str, base: str | None = None) -> Path:
    """The worktree ``<repo>-lanes/<lane-id>`` on a new branch ``lane/<lane-id>`` (from ``base``, else HEAD).
    The manifest's ``lane_exclude`` paths are not checked out."""
    path = worktree_path(repo, lane_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    error = worktree_add(repo, path, branch=f"lane/{lane_id}", base=base,
                         exclude=manifest_mod.load(repo).lane_exclude)
    if error is not None:
        raise LaunchError(f"git worktree add failed: {error}")
    link_node_modules(repo, path)
    return path


def link_node_modules(repo: Path, worktree: Path) -> bool:
    """A Node project's worktree gets ``node_modules`` as a link to the main checkout's, so the lane and
    its review can import the dependencies (no ERR_MODULE_NOT_FOUND). False when nothing was linked."""
    source, target = repo / "node_modules", worktree / "node_modules"
    if not (repo / "package.json").is_file() or not (worktree / "package.json").is_file():
        return False
    if target.exists() or target.is_symlink():
        return False
    if not source.is_dir() and not install_node_modules(repo):
        return False
    try:
        target.symlink_to(source.resolve(), target_is_directory=True)
    except OSError:
        return False
    exclude_from_status(worktree, "/node_modules")
    return True


def install_node_modules(repo: Path) -> bool:
    """The main checkout declares dependencies but has no ``node_modules`` (never installed), so every lane
    worktree stayed without the link. Install them there once (``npm ci`` from the lockfile, else
    ``npm install``; no scripts) so the lanes can link it. True when ``node_modules`` exists afterwards."""
    try:
        package = json.loads((repo / "package.json").read_text())
    except (OSError, ValueError):
        return False
    if not isinstance(package, dict) or not (package.get("dependencies") or package.get("devDependencies")):
        return False
    verb = "ci" if (repo / "package-lock.json").is_file() else "install"
    timeouts_mod.run(["npm", verb, "--ignore-scripts", "--no-audit", "--no-fund", "--prefer-offline"], "npm",
                     cwd=repo, text=True)
    return (repo / "node_modules").is_dir()


def stage_all(worktree: Path, env: dict | None = None) -> "timeouts_mod.Result":
    """``git add -A`` for a lane's worktree, minus the ``node_modules`` link to main's. A committed link would
    replace main's real ``node_modules`` when the lane merges, and the next commit would delete it."""
    done = timeouts_mod.run(["git", "-C", str(worktree), "add", "-A"], "git", text=True, **({"env": env} if env else {}))
    if (worktree / "node_modules").is_symlink():
        timeouts_mod.run(["git", "-C", str(worktree), "reset", "-q", "--", "node_modules"], "git", text=True,
                         **({"env": env} if env else {}))
    return done


def exclude_from_status(worktree: Path, pattern: str) -> None:
    """Add ``pattern`` to the repository's ``info/exclude`` so it is not an uncommitted change. A
    ``node_modules/`` line in .gitignore does not match a symlink, so the link needs this."""
    done = timeouts_mod.run(["git", "-C", str(worktree), "rev-parse", "--path-format=absolute", "--git-path",
                             "info/exclude"], "git", text=True)
    if done.returncode != 0 or not done.stdout.strip():
        return
    exclude = Path(done.stdout.strip())
    try:
        current = exclude.read_text() if exclude.is_file() else ""
        if pattern in current.splitlines():
            return
        exclude.parent.mkdir(parents=True, exist_ok=True)
        with exclude.open("a") as handle:
            handle.write(("" if not current or current.endswith("\n") else "\n") + pattern + "\n")
    except OSError:
        return


def agent_command(config: Config, agent: str | None, brief: Path, workspace: Path,
                  project: str | None = None, step: str | None = None) -> list[str]:
    """The agent command: the agent's ``launch`` template, else ``[launch] command``. The model and
    effort flags in it are replaced by ``[models.workers]`` (with its overrides for ``project`` and
    ``step``) when those are configured."""
    template = None
    chosen = models_mod.resolve_workers(config, project, step)
    alias = agent or (chosen.agent if chosen.agent in config.agents else None)
    if alias:
        table = config.agents.get(alias)
        if table is None:
            raise LaunchError(f"unknown agent '{alias}' (no [agents.{alias}] in config.toml)")
        template = table.get("launch")
    template = template or config.launch_command
    if not template:
        raise LaunchError('no agent command: add [launch] command = "<agent> ... {brief}" '
                          "(placeholders {brief} and {workspace}) to config.toml")
    words = shlex.split(template) if isinstance(template, str) else [str(w) for w in template]
    words = [w.replace("{brief}", str(brief)).replace("{workspace}", str(workspace)) for w in words]
    return models_mod.apply_to_command(words, chosen, alias)


def tirith_argv(home: Path | None = None) -> list[str]:
    """The argv that runs ``tirith`` from anywhere (``tirithcmd.tirith_command``), not the bare name."""
    return shlex.split(tirithcmd.resolved(home))


def lane_run_argv(project: str, step_id: str, hero_class: str, title: str, brief: Path,
                  workspace: Path, lane_id: str, command: list[str],
                  request_id: str | None = None, alias: str | None = None,
                  fallback_from: str | None = None, tried: list[str] | None = None,
                  by_tirith: bool = False, item: str | None = None, item_step: str | None = None,
                  retry_of: str | None = None) -> list[str]:
    extra = ["--request", request_id] if request_id else []
    if item:
        extra += ["--item", item]
    if item_step and str(item_step) != str(step_id):
        extra += ["--item-step", str(item_step)]
    if retry_of:
        extra += ["--retry-of", retry_of]
    if by_tirith:
        extra += ["--by-tirith"]
    if alias:
        extra += ["--as-agent", alias]
    if fallback_from:
        extra += ["--fallback-from", fallback_from]
    if tried:
        extra += ["--tried", ",".join(tried)]
    return [*tirith_argv(), "lane", "run", "--id", lane_id, "--step", str(step_id), "--project", project,
            "--class", hero_class, "--feature", title, "--brief", str(brief),
            "--workspace", str(workspace), *extra, "--", *command]


def spawn_detached(argv: list[str], home: Path, log: Path) -> int:
    """Start ``argv`` detached (own session / process group, stdin from /dev/null) so it survives the handler,
    the terminal and the app that started it."""
    log.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, TIRITH_HOME=str(home), PYTHONPATH=str(Path(__file__).resolve().parent.parent))
    extra = ({"creationflags": getattr(subprocess, "DETACHED_PROCESS", 0)
              | getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)} if os.name == "nt"
             else {"start_new_session": True})
    with log.open("ab") as handle:
        proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=handle, stderr=subprocess.STDOUT,
                                env=env, **extra)
    return proc.pid


# -- the macOS url handler --------------------------------------------------------------------


def apps_dir() -> Path:
    return Path.home() / "Applications"


def app_path(base: Path | None = None) -> Path:
    return (base or apps_dir()) / APP_NAME


def handler_installed(base: Path | None = None) -> bool:
    return (app_path(base) / "Contents" / "Info.plist").is_file()


HANDLER_PATH = "/opt/homebrew/bin:/usr/local/bin:$HOME/.local/bin:$HOME/.cargo/bin"  # where uv lives
HANDLER_LOG = "url-handler.log"


def handler_log(home_env: str | None = None) -> Path:
    return (Path(home_env).expanduser() if home_env else Path.home() / ".tirith") / "logs" / HANDLER_LOG


def log_handler(home: Path, text: str) -> None:
    """One timestamped line in ``<home>/logs/url-handler.log`` (a log that cannot be written is ignored)."""
    try:
        log = Path(home) / "logs" / HANDLER_LOG
        log.parent.mkdir(parents=True, exist_ok=True)
        with log.open("a", encoding="utf-8") as handle:
            handle.write(f"{now_iso()} {text}\n")
    except OSError:
        pass


def redact_url(url: str) -> str:
    return re.sub(r"(token=|env=)[^&\s]*", r"\1***", str(url))  # env= holds a project's variable values


def applescript(home_env: str | None = None, tirith: str | None = None) -> str:
    """The ``on open location`` script; it runs ``tirith url open`` detached, by the absolute command.

    ``tirith`` is the shell-ready command (default ``tirithcmd.resolved``). Its output goes to the
    handler log, where ``tirith url open`` also writes each link it received and the result."""
    tirith = tirith or tirithcmd.resolved(Path(home_env) if home_env else None)
    prefix = f"PATH={HANDLER_PATH}:$PATH PYTHONPATH={shlex.quote(str(Path(__file__).resolve().parent.parent))} "
    if home_env:
        prefix += f"TIRITH_HOME={shlex.quote(home_env)} "
    base = prefix + tirith + " url open "
    escaped = base.replace("\\", "\\\\").replace('"', '\\"')
    log = shlex.quote(str(handler_log(home_env))).replace("\\", "\\\\").replace('"', '\\"')
    return ("on open location theURL\n"
            f'\tdo shell script "{escaped}" & quoted form of theURL & " >>{log} 2>&1 &"\n'
            "end open location\n")


def install_handler(base: Path | None = None, home_env: str | None = None, register: bool = True,
                    tirith: str | None = None) -> Path:
    if sys.platform != "darwin" or not shutil.which("osacompile"):
        raise LaunchError("the url handler is macOS only for now (needs osacompile)")
    app = app_path(base)
    if app.exists():
        shutil.rmtree(app)
    app.parent.mkdir(parents=True, exist_ok=True)
    source = app.parent / f".{APP_NAME}.applescript"
    tirith = tirith or tirithcmd.resolved(Path(home_env) if home_env else None)
    source.write_text(applescript(home_env, tirith), encoding="utf-8")
    try:
        done = timeouts_mod.run(["osacompile", "-o", str(app), str(source)], "generic", text=True)
    finally:
        source.unlink(missing_ok=True)
    if done.returncode != 0:
        raise LaunchError(f"osacompile failed: {done.stderr.strip()}")
    plist_path = app / "Contents" / "Info.plist"
    info = plistlib.loads(plist_path.read_bytes())
    info.update({"CFBundleIdentifier": BUNDLE_ID, "CFBundleName": "Tirith Launcher", "LSUIElement": True, "TirithCommand": tirith,
                 "CFBundleURLTypes": [{"CFBundleURLName": "Tirith link", "CFBundleURLSchemes": [SCHEME]}]})
    plist_path.write_bytes(plistlib.dumps(info))
    if register:
        register_app(app)
    return app


def register_app(app: Path, unregister: bool = False) -> None:
    lsregister = ("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework"
                  "/Support/lsregister")
    if Path(lsregister).is_file():
        timeouts_mod.run([lsregister, "-u" if unregister else "-f", str(app)], "generic")


def uninstall_handler(base: Path | None = None, register: bool = True) -> bool:
    app = app_path(base)
    if not app.exists():
        return False
    if register:
        register_app(app, unregister=True)
    shutil.rmtree(app)
    return True


def verify_handler(base: Path | None = None) -> list[str]:
    """Problems with the installed handler (empty = fine)."""
    app = app_path(base)
    plist_path = app / "Contents" / "Info.plist"
    if not plist_path.is_file():
        return [f"{app} is not installed. Fix: {INSTALL_COMMAND}"]
    try:
        info = plistlib.loads(plist_path.read_bytes())
    except Exception:
        return [f"{plist_path} is unreadable. Fix: {INSTALL_COMMAND}"]
    schemes = [s for t in info.get("CFBundleURLTypes", []) for s in t.get("CFBundleURLSchemes", [])]
    if SCHEME not in schemes:
        return [f"{app} does not handle {SCHEME}://. Fix: {INSTALL_COMMAND}"]
    return command_problems(str(info.get("TirithCommand") or ""))


def command_problems(command: str) -> list[str]:
    """Problems with the command the handler runs: it must exist (empty = fine)."""
    words = shlex.split(command) if command.strip() else []
    if not words:
        return [f"the handler does not name the tirith command it runs. Fix: {INSTALL_COMMAND}"]
    exe = words[0]
    found = Path(exe).is_file() and os.access(exe, os.X_OK) if "/" in exe else \
        shutil.which(exe, path=os.environ.get("PATH", "") + ":" + os.path.expandvars(HANDLER_PATH)) is not None
    return [] if found else [f"the handler runs {exe}, which does not exist. Fix: {INSTALL_COMMAND}"]
