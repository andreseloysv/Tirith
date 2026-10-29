"""A project's setup: what ``tirith.toml`` and the machine say about build, tests, deploy, hooks, rules.

``build`` is read only (it never installs anything) and feeds ``state.projects[].setup``. The edit
functions append, insert or cut whole ``[[...]]`` blocks of ``tirith.toml`` as text, so comments and
formatting elsewhere stay as they are. See ``docs/CONTRACT-setup.md``.
"""

from __future__ import annotations

import json
import re
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from tirith import gates as gates_mod
from tirith import hooks as hooks_mod
from tirith import inbox as inbox_mod
from tirith import manifest as manifest_mod
from tirith.home import Config

KIND_ORDER = ("build", "test", "deploy")
_ANY_HEADER = re.compile(r"^\s*\[")

# (id, label, enforced_by); the labels match the Rules list of the web page, the descriptions
# come from ``gates.DESCRIPTIONS``
BUILT_IN: tuple[tuple[str, str, list[str]], ...] = (
    ("subagent-tool", "Sub-agent tool", ["hook"]),
    ("agent-cli", "Agent without lane run", ["hook", "shim"]),
    ("no-verify", "Skipped git hooks", ["hook"]),
    ("hooks-path", "Changed hooks path", ["hook"]),
    ("git-hooks", "Edited git hooks", ["hook", "git"]),
    ("settings-edit", "Edited agent settings", ["hook"]),
    ("roadmap-edit", "Edited the plan", ["hook"]),
    ("raw-stage", "Raw pipeline stage", ["hook"]),
    ("lane-brief", "Lane without brief", ["lane run"]),
    ("project-check", "Project check failing", ["lane run"]),
    ("commit-guard", "Commit guard", ["git"]),
    ("workspace-write", "Write outside the lane copy", ["hook"]),
    ("no-secrets", "Read credentials", ["hook"]),
    ("no-servers", "Start a server", ["hook"]),
    ("own-processes", "Stop another process", ["hook"]),
)


# -- reading ----------------------------------------------------------------------------------


@dataclass
class Machine:
    """Whether the rules are active on this machine, with the commands that would fix it."""
    active: bool
    hooks_ok: bool
    shims_ok: bool
    problems: list[str]
    fix: list[str]
    protection: list[dict] = field(default_factory=list)  # hooks.protect rows (public fields only)
    summary: str = ""  # one plain sentence: what is missing and why it matters ("" = all active)
    steps: tuple[tuple[str, str], ...] = ()  # (button label, command) for each owner fix


_settings_has_hooks = hooks_mod.has_hooks


def _names(items: list[str]) -> str:
    return items[0] if len(items) == 1 else ", ".join(items[:-1]) + " and " + items[-1]


def _on_path(alias: str, config: Config, home: Path) -> bool:
    """Whether the CLI of ``alias`` (its ``command``, else the alias itself) is on the lane PATH."""
    command = str((config.agents.get(alias) or {}).get("command") or "").strip() or alias
    return gates_mod.resolve_real(command, home / "shims", gates_mod.lane_path(home)) is not None


def machine_state(config: Config, shim_dir: Path | None = None, home: Path | None = None,
                  protection: list[dict] | None = None) -> Machine:
    """Hooks installed in every configured agent's settings and the shims intact (read only).

    The shims are checked like ``tirith shim verify``: ``<home>/shims`` against the lane PATH (or
    ``shim_dir`` against the plain PATH)."""
    from tirith.home import tirith_home

    home = Path(home or tirith_home())
    problems: list[str] = []
    fix: list[str] = []
    steps: list[tuple[str, str]] = []
    missing: list[str] = []
    unknown: list[str] = []  # no settings path in config.toml: their hooks cannot be installed or checked
    aliases = [a for a in config.agents]
    if not aliases:
        problems.append("no [agents.<alias>] in config.toml: no agent is guarded")
    for alias in aliases:
        try:
            settings = hooks_mod.default_settings(alias, config.agents)
        except ValueError as exc:
            problems.append(str(exc))
            fix.append(f'add settings = "<path to its settings.json>" under [agents.{alias}] in '
                       f"{home / 'config.toml'}, or remove [agents.{alias}] if {alias} is not used here")
            unknown.append(alias)
            continue
        if not _settings_has_hooks(settings, alias):
            problems.append(f"{alias}: tirith hooks are not in {settings}")
            fix.append(f"tirith hook install --agent {alias}")
            steps.append((f"Install the hooks for {alias}", fix[-1]))
            missing.append(alias)
    hooks_ok = bool(aliases) and not problems
    shim_problems = gates_mod.verify_home_shims(config, home, shim_dir) if aliases else []
    if shim_problems:
        problems += shim_problems
        fix.append(f"tirith shim install --dir {shim_dir}" if shim_dir else "tirith shim install")
        steps.append(("Install the agent shims", fix[-1]))
    gates_off = not config.gates_enabled
    if gates_off:
        problems.append("[gates] enabled = false in config.toml")
    shims_ok = not shim_problems
    rows = [{k: r.get(k) for k in ("agent", "ok", "checked_at", "healed_at")} for r in protection or []]
    active = hooks_ok and shims_ok and not gates_off
    parts: list[str] = []
    if not aliases:
        parts.append("No agent is set up in config.toml, so no agent is guarded.")
    if missing:
        parts.append(f"Tirith cannot see what agents do ({', '.join(missing)}), because their hooks are not "
                     "installed.")
    if unknown:
        their = "its" if len(unknown) == 1 else "their"
        parts.append(f"Tirith cannot guard {_names(unknown)}, because config.toml does not say where {their} "
                     "settings are.")
        runnable = [a for a in unknown if _on_path(a, config, home)]
        absent = [a for a in unknown if a not in runnable]
        if runnable:
            parts.append(f"{_names(runnable)} can run on this machine without the rules.")
        if absent:
            parts.append(f"{_names(absent)} {'is' if len(absent) == 1 else 'are'} not installed here, so "
                         f"{'it cannot' if len(absent) == 1 else 'they cannot'} run unguarded yet.")
        parts.append(f'To fix it, add settings = "<path to the agent\'s settings.json>" under '
                     f"{_names([f'[agents.{a}]' for a in unknown])} in config.toml, or remove the section of "
                     "an agent you do not use.")
    if shim_problems:
        parts.append("Agents could start other agents without Tirith seeing them, because the agent "
                     "shims are not installed.")
    if gates_off:
        parts.append("The rules are switched off in config.toml ([gates] enabled = false).")
    return Machine(active, hooks_ok, shims_ok, problems, fix, rows, " ".join(parts), tuple(steps))


def _last(runs: list[dict], project: str, pipeline: str, stage: str) -> dict | None:
    """The newest run of ``stage`` that started (``runs`` are newest first)."""
    for run in runs:
        if run.get("project") != project or run.get("pipeline") != pipeline:
            continue
        for item in run.get("stages") or []:
            if item.get("name") == stage and item.get("status") in ("passed", "failed", "running"):
                return {"status": item["status"], "at": item.get("ended_at") or item.get("started_at"),
                        "duration_s": item.get("duration_s"), "run": run.get("run")}
    return None


def _rules(loaded: manifest_mod.Manifest, machine: Machine, guard_ok: bool) -> list[dict]:
    ok = {"hook": machine.hooks_ok, "shim": machine.shims_ok and machine.hooks_ok, "git": guard_ok,
          "lane run": True, "CI": True}
    rows = [{"id": rid, "label": label, "source": "built-in", "enforced_by": list(by),
             "active": any(ok.get(x, False) for x in by),
             "description": gates_mod.DESCRIPTIONS[rid]}
            for rid, label, by in BUILT_IN]
    for rule in loaded.rules:
        rows.append({"id": rule.id, "label": rule.id, "source": "tirith.toml", "enforced_by": ["hook"],
                     "active": ok["hook"], "action": rule.action, "description": rule.description})
    return rows


def _pending(requests: list[dict], project: str) -> list[dict]:
    return [{"id": r["id"], "at": r["at"], "action": r["setup"]["action"],
             "summary": inbox_mod.setup_summary(r["setup"])}
            for r in requests
            if r["kind"] == "setup" and r["project"] == project and r["status"] == "open" and r.get("setup")]


def build(folder: Path | None, project: str, machine: Machine, runs: list[dict], requests: list[dict],
          guard_ok: bool, guard_problems: list[str], words: list[str],
          gates_actions: list[dict] | None = None) -> dict:
    """``state.projects[].setup`` for one project folder."""
    loaded = manifest_mod.load(folder) if folder else None
    exists = bool(loaded and loaded.exists)
    problems: list[dict] = []
    if folder is not None:
        for p in manifest_mod.check(folder, words, ci=True):
            problems.append({"rule": p.rule, "message": p.message, "fix": p.fix, "line": p.line})
    for text in guard_problems if exists and not guard_ok else []:
        problems.append({"rule": "commit-guard", "message": text, "fix": f"tirith guard install {folder}",
                         "line": 1})
    pipelines = []
    for pipe in (loaded.pipelines if exists else []):
        pipelines.append({"id": pipe.id, "name": pipe.name, "stages": [
            {"name": s.name, "kind": manifest_mod.stage_kind(s), "run": s.run,
             "last": _last(runs, project, pipe.id, s.name)} for s in pipe.stages]})
    hooks = [{"event": h.event, "run": h.run,
              "last": _last(runs, project, manifest_mod.GIT_HOOKS_PIPELINE, h.event)}
             for h in (loaded.hooks if exists else [])]
    return {
        "manifest": str(loaded.path) if exists else None,
        "check": {"ok": not problems, "problems": problems},
        "pipelines": pipelines, "hooks": hooks,
        "rules": _rules(loaded or manifest_mod.Manifest(path=Path(manifest_mod.FILENAME)), machine, guard_ok),
        "gates_active": machine.active,
        "gates_problems": machine.problems,
        "gates_fix": machine.fix,
        "protection": machine.protection,
        "gates_summary": machine.summary,
        "gates_actions": list(gates_actions or []),
        "pending": _pending(requests, project),
    }


# -- editing tirith.toml ------------------------------------------------------------------------


class SetupError(Exception):
    """An edit that cannot be made; the message says why and what to do instead."""


def _q(value: str) -> str:
    return json.dumps(str(value), ensure_ascii=False)


def _headers(lines: list[str]) -> list[int]:
    return [i for i, line in enumerate(lines) if _ANY_HEADER.match(line)]


def _block_bounds(lines: list[str], index: int, headers: list[int]) -> tuple[int, int]:
    """``[start, end)`` of the block whose header is at ``index``, comments above it included.

    Comment lines touching the next header belong to that header, so ``end`` stops before them."""
    nxt = next((h for h in headers if h > index), len(lines))
    end = nxt
    while end > index + 1 and lines[end - 1].lstrip().startswith("#") and nxt != len(lines):
        end -= 1
    while end > index + 1 and not lines[end - 1].strip():
        end -= 1
    start = index
    while start > 0 and lines[start - 1].lstrip().startswith("#"):
        start -= 1
    return start, end


def _read(folder: Path) -> tuple[str, list[str], dict]:
    path = Path(folder) / manifest_mod.FILENAME
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise SetupError(f"cannot read {path} ({exc}). Fix: create it, see docs/CONTRACT-manifest.md")
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        raise SetupError(f"{path} does not parse ({exc}). Fix: tirith check, then correct the syntax")
    return text, text.splitlines(), data


def _write(folder: Path, lines: list[str]) -> None:
    path = Path(folder) / manifest_mod.FILENAME
    path.write_text("\n".join(lines).rstrip("\n") + "\n", encoding="utf-8")


def _pipeline_blocks(lines: list[str], data: dict) -> list[dict]:
    """For each ``[[pipelines]]``: header line, the ``[[pipelines.stages]]`` header lines, in order."""
    pipes = [i for i, line in enumerate(lines) if manifest_mod._HEADER_RE.match(line)
             and manifest_mod._HEADER_RE.match(line).group(1) == "pipelines"]
    stages = [i for i, line in enumerate(lines) if manifest_mod._HEADER_RE.match(line)
              and manifest_mod._HEADER_RE.match(line).group(1) == "pipelines.stages"]
    out = []
    for n, head in enumerate(pipes):
        limit = pipes[n + 1] if n + 1 < len(pipes) else len(lines) + 1
        row = (data.get("pipelines") or [])[n] if n < len(data.get("pipelines") or []) else {}
        names = [str(s.get("name")) for s in row.get("stages") or [] if isinstance(s, dict)]
        out.append({"id": str(row.get("id") or ""), "head": head,
                    "stages": [i for i in stages if head < i < limit], "names": names})
    return out


def _insert(lines: list[str], at: int, block: list[str]) -> list[str]:
    """``block`` after line ``at - 1`` with one blank line before it (and after, if text follows)."""
    new = lines[:at] + ([""] if at and lines[at - 1].strip() else []) + block
    rest = lines[at:]
    if rest and rest[0].strip():
        new.append("")
    return new + rest


def _find_stage(blocks: list[dict], pipeline: str | None, name: str) -> tuple[dict, int]:
    hits = [(b, k) for b in blocks for k, n in enumerate(b["names"]) if n == name
            and (not pipeline or b["id"] == pipeline) and k < len(b["stages"])]
    if not hits:
        raise SetupError(f"stage '{name}' is not in {manifest_mod.FILENAME}"
                         + (f" pipeline '{pipeline}'" if pipeline else "") + ". Fix: tirith setup show")
    if len(hits) > 1:
        raise SetupError(f"stage '{name}' is in several pipelines ({', '.join(b['id'] for b, _ in hits)}). "
                         "Fix: add --pipeline <id>")
    return hits[0]


def add_stage(folder: Path, pipeline: str, name: str, run: str, kind: str | None = None,
              after: str | None = None) -> str:
    """Add ``[[pipelines.stages]]`` to a pipeline (created at the end of the file if new)."""
    pipeline, name, run = pipeline.strip(), name.strip(), run.strip()
    if not pipeline or not name or not run:
        raise SetupError("add-stage needs a pipeline, a name and a run command. "
                         'Fix: tirith setup add-stage --pipeline ci --name lint --run "make lint"')
    if kind is not None and kind not in manifest_mod.KINDS:
        raise SetupError(f"kind {kind!r} is not one of {', '.join(manifest_mod.KINDS)}. "
                         "Fix: use --kind build, test or deploy, or leave it out")
    _, lines, data = _read(folder)
    blocks = _pipeline_blocks(lines, data)
    block = ["[[pipelines.stages]]", f"name = {_q(name)}", f"run = {_q(run)}"]
    if kind:
        block.insert(2, f"kind = {_q(kind)}")
    mine = next((b for b in blocks if b["id"] == pipeline), None)
    if mine is None:
        if after:
            raise SetupError(f"pipeline '{pipeline}' does not exist, so --after '{after}' has no meaning. "
                             "Fix: drop --after")
        top = ["[[pipelines]]", f"id = {_q(pipeline)}", f"name = {_q(pipeline)}", ""] + block
        new = _insert(lines, len(lines) if not lines else len(_trim_end(lines)), top)
        _write(folder, new)
        return f"added pipeline '{pipeline}' with stage '{name}'"
    if name in mine["names"]:
        raise SetupError(f"stage '{name}' already exists in pipeline '{pipeline}'. "
                         "Fix: pick another name or run tirith setup remove-stage first")
    headers = _headers(lines)
    if after:
        if after not in mine["names"] or mine["names"].index(after) >= len(mine["stages"]):
            raise SetupError(f"--after '{after}' is not a stage of pipeline '{pipeline}' "
                             f"({', '.join(mine['names']) or 'no stages'}). Fix: tirith setup show")
        anchor = mine["stages"][mine["names"].index(after)]
    else:
        anchor = mine["stages"][-1] if mine["stages"] else mine["head"]
    _, end = _block_bounds(lines, anchor, headers)
    _write(folder, _insert(lines, end, block))
    return f"added stage '{name}' to pipeline '{pipeline}'" + (f" after '{after}'" if after else "")


def _trim_end(lines: list[str]) -> list[str]:
    out = list(lines)
    while out and not out[-1].strip():
        out.pop()
    return out


def _tidy(lines: list[str], at: int) -> list[str]:
    """After cutting a block at ``at``: exactly one blank line between what is left on both sides."""
    while at < len(lines) and at and not lines[at - 1].strip() and not lines[at].strip():
        del lines[at]
    if at < len(lines) and at and lines[at - 1].strip() and lines[at].strip():
        lines.insert(at, "")
    return lines


def remove_stage(folder: Path, name: str, pipeline: str | None = None) -> str:
    _, lines, data = _read(folder)
    blocks = _pipeline_blocks(lines, data)
    block, k = _find_stage(blocks, pipeline, name.strip())
    start, end = _block_bounds(lines, block["stages"][k], _headers(lines))
    new = lines[:start] + lines[end:]
    _write(folder, _tidy(new, start))
    return f"removed stage '{name}' from pipeline '{block['id']}'"


def build_rule(rule_id: str, action: str, message: str, instead: str, match: str | None = None,
               regex: str | None = None, paths: list[str] | None = None,
               description: str = "") -> manifest_mod.Rule:
    """Validate the fields of a custom rule like ``tirith check`` does; raises ``SetupError``."""
    row = {"id": rule_id, "action": action, "message": message, "instead": instead,
           "match": match, "regex": regex, "paths": list(paths or []), "description": description}
    rule, bad = manifest_mod.rule_problems({k: v for k, v in row.items() if v not in (None, [])}, set())
    if rule is None:
        raise SetupError("; ".join(f"{message} (Fix: {fix})" for _, message, fix in bad))
    return rule


def add_rule(folder: Path, rule: manifest_mod.Rule) -> str:
    _, lines, data = _read(folder)
    if any(isinstance(r, dict) and r.get("id") == rule.id for r in data.get("rules") or []):
        raise SetupError(f"rule '{rule.id}' already exists. Fix: pick another id or run "
                         "tirith setup remove-rule first")
    block = ["[[rules]]", f"id = {_q(rule.id)}"]
    if rule.match:
        block.append(f"match = {_q(rule.match)}")
    if rule.regex:
        block.append(f"regex = {_q(rule.regex)}")
    if rule.paths:
        block.append("paths = [" + ", ".join(_q(p) for p in rule.paths) + "]")
    block += [f"action = {_q(rule.action)}", f"message = {_q(rule.message)}", f"instead = {_q(rule.instead)}",
              f"description = {_q(rule.description)}",
              f"described = {_q(manifest_mod.match_hash(rule.action, rule.match, rule.regex, rule.paths))}"]
    base = _trim_end(lines)
    _write(folder, _insert(base, len(base), block))
    return f"added {rule.action} rule '{rule.id}'"


def remove_rule(folder: Path, rule_id: str) -> str:
    _, lines, data = _read(folder)
    rows = data.get("rules") or []
    heads = [i for i, line in enumerate(lines) if manifest_mod._HEADER_RE.match(line)
             and manifest_mod._HEADER_RE.match(line).group(1) == "rules"]
    for n, row in enumerate(rows):
        if isinstance(row, dict) and row.get("id") == rule_id and n < len(heads):
            start, end = _block_bounds(lines, heads[n], _headers(lines))
            _write(folder, _tidy(lines[:start] + lines[end:], start))
            return f"removed rule '{rule_id}'"
    raise SetupError(f"rule '{rule_id}' is not in {manifest_mod.FILENAME}. Fix: tirith setup show")


def describe_rule(folder: Path, rule_id: str, description: str) -> str:
    """Set the description of a custom rule and store the hash of its match fields as ``described``."""
    description = description.strip()
    if not description:
        raise SetupError("a rule needs a description. Fix: pass --description with one plain sentence")
    _, lines, data = _read(folder)
    rows = data.get("rules") or []
    heads = [i for i, line in enumerate(lines) if manifest_mod._HEADER_RE.match(line)
             and manifest_mod._HEADER_RE.match(line).group(1) == "rules"]
    for n, row in enumerate(rows):
        if isinstance(row, dict) and row.get("id") == rule_id and n < len(heads):
            start, end = _block_bounds(lines, heads[n], _headers(lines))
            paths = row.get("paths") if isinstance(row.get("paths"), list) else []
            stamp = manifest_mod.match_hash(str(row.get("action") or ""), row.get("match") or None,
                                            row.get("regex") or None, paths)
            keep = [x for x in lines[start:end] if not re.match(r"\s*(description|described)\s*=", x)]
            tail = 0
            while keep and not keep[-1].strip():
                keep.pop()
                tail += 1
            keep += [f"description = {_q(description)}", f"described = {_q(stamp)}"] + [""] * tail
            _write(folder, lines[:start] + keep + lines[end:])
            return f"described rule '{rule_id}'"
    raise SetupError(f"rule '{rule_id}' is not in {manifest_mod.FILENAME}. Fix: tirith setup show")


def apply_request(folder: Path, setup: dict) -> str:
    """Apply the edit a ``setup`` inbox request asks for (not ``run-stage``)."""
    action = setup.get("action")
    if action == "add-stage":
        return add_stage(folder, str(setup.get("pipeline") or ""), str(setup.get("name") or ""),
                         str(setup.get("run") or ""), setup.get("kind") or None, setup.get("after") or None)
    if action == "remove-stage":
        return remove_stage(folder, str(setup.get("name") or ""), setup.get("pipeline") or None)
    if action == "add-rule":
        paths = setup.get("paths") or []
        if isinstance(paths, str):
            paths = [p.strip() for p in paths.split(",") if p.strip()]
        rule = build_rule(str(setup.get("id") or ""), str(setup.get("effect") or "block"),
                          str(setup.get("message") or ""), str(setup.get("instead") or ""),
                          setup.get("match") or None, setup.get("regex") or None, list(paths),
                          str(setup.get("description") or ""))
        return add_rule(folder, rule)
    if action == "remove-rule":
        return remove_rule(folder, str(setup.get("id") or ""))
    raise SetupError(f"cannot apply {action!r} here")
