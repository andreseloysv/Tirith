"""Which model and effort the manager and the agents it starts use (``[models]`` in ``config.toml``).

``resolve_workers`` merges ``[models.workers]``, its per-project and its per-step override; launch, the
dispatcher and ``lane run --agent`` read it. ``set_models`` edits ``config.toml`` as text so comments and
other tables stay. ``apply_manager`` writes the manager's model into the manager agent's user settings
(only when the owner asks). See ``docs/CONTRACT-models.md``.
"""

from __future__ import annotations

import json
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

from tirith.home import Config
from tirith.store import write_json_atomic, write_text_atomic

EFFORTS = ("low", "medium", "high", "xhigh", "max")
ROLES = ("manager", "workers")
FIELDS = ("agent", "model", "effort")

# alias -> flag that carries the effort on its command line when the preset has none; an alias that is
# not here (and whose preset has no effort flag) records the effort only
EFFORT_FLAGS: dict[str, str] = {"muse": "--reasoning-effort"}
# alias -> key of its user settings file; effort None = the agent has no documented key (noted, not written)
SETTINGS_KEYS: dict[str, dict[str, str | None]] = {
    "anthropic": {"model": "model", "effort": None},
    "muse": {"model": "model", "effort": None},
}
MODEL_FLAGS = ("--model", "-m")
ALL_EFFORT_FLAGS = ("--effort", "--reasoning-effort")
_TOKEN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/+\-\[\]]*$")
_BARE = re.compile(r"^[A-Za-z0-9_-]+$")


class ModelsError(Exception):
    """A models setting that cannot be read, saved or applied; the message says why."""


@dataclass
class Choice:
    """One resolved setting: any field may be None (not configured)."""
    agent: str | None = None
    model: str | None = None
    effort: str | None = None

    def as_dict(self) -> dict:
        return {"agent": self.agent, "model": self.model, "effort": self.effort}


# -- reading -----------------------------------------------------------------------------------


def _table(raw) -> dict:
    return raw if isinstance(raw, dict) else {}


def _pick(table: dict) -> dict:
    out = {k: str(table[k]).strip() for k in FIELDS if isinstance(table.get(k), str) and table[k].strip()}
    if isinstance(table.get("fallback"), list):  # an explicit empty list means "no fallback" here
        out["fallback"] = [str(a).strip() for a in table["fallback"] if str(a).strip()]
    return out


def _strings(raw) -> list[str]:
    return [str(c) for c in raw if str(c).strip()] if isinstance(raw, list) else []


def parse(raw) -> dict:
    """The ``[models]`` table normalised: ``{choices, manager, workers, projects, steps}``."""
    raw = _table(raw)
    workers = _table(raw.get("workers"))
    return {
        "choices": _strings(raw.get("choices")),
        "efforts": _strings(raw.get("efforts")),
        "patterns": _strings(raw.get("out_of_credits_patterns")),
        "manager": _pick(_table(raw.get("manager"))),
        "workers": _pick(workers),
        "projects": {str(k): _pick(_table(v)) for k, v in _table(workers.get("projects")).items()},
        "steps": {str(k): _pick(_table(v)) for k, v in _table(workers.get("steps")).items()},
    }


def resolve_workers(config: Config, project: str | None = None, step: str | None = None) -> Choice:
    """``[models.workers]``, then its ``projects.<project>`` override, then ``steps."<project>/<step>"``."""
    cfg = parse(config.models)
    merged = dict(cfg["workers"])
    merged.update(cfg["projects"].get(str(project or ""), {}))
    if project and step is not None:
        merged.update(cfg["steps"].get(f"{project}/{step}", {}))
    return Choice(merged.get("agent"), merged.get("model"), merged.get("effort"))


def resolve_chain(config: Config, project: str | None = None, step: str | None = None) -> list[str]:
    """The agent aliases to try for a worker, in order: its agent, then ``fallback`` (step override,
    project override, ``[models.workers]``; the first level that sets one wins). Each alias once."""
    cfg = parse(config.models)
    levels = [cfg["workers"]]
    if project:
        levels.append(cfg["projects"].get(str(project), {}))
        if step is not None:
            levels.append(cfg["steps"].get(f"{project}/{step}", {}))
    merged = {}
    for level in levels:
        merged.update(level)
    chain = ([merged["agent"]] if merged.get("agent") else []) + list(merged.get("fallback") or [])
    return list(dict.fromkeys(chain))


def choices_for(config: Config, alias: str | None = None) -> list[str]:
    """The model ids offered for ``alias``: those starting with one of its ``model_prefixes``; every id
    when the alias declares none (or no alias is given)."""
    choices = parse(config.models)["choices"]
    prefixes = (config.agents.get(alias or "") or {}).get("model_prefixes")
    prefixes = [str(p) for p in prefixes if str(p)] if isinstance(prefixes, list) else []
    return [c for c in choices if any(c.startswith(p) for p in prefixes)] if prefixes else list(choices)


def resolve_manager(config: Config) -> Choice:
    m = parse(config.models)["manager"]
    return Choice(m.get("agent"), m.get("model"), m.get("effort"))


def applies_to(choice: Choice, alias: str | None) -> bool:
    """A choice made for agent X is not used for agent Y (a model id means nothing to another agent)."""
    return not choice.agent or not alias or choice.agent == alias


def effort_flag(alias: str | None, preset: dict | None = None) -> str | None:
    return (preset or {}).get("effort_flag") or EFFORT_FLAGS.get(alias or "")


# -- command lines -----------------------------------------------------------------------------


def _replace_flag(argv: list[str], flags: tuple[str, ...], value: str) -> list[str]:
    out, i, hit = [], 0, False
    while i < len(argv):
        word = argv[i]
        if word in flags and i + 1 < len(argv):
            out += [word, value]
            i, hit = i + 2, True
            continue
        eq = next((f for f in flags if word.startswith(f + "=")), None)
        if eq:
            out.append(f"{eq}={value}")
            hit = True
        else:
            out.append(word)
        i += 1
    return out if hit else argv


def apply_to_command(argv: list[str], choice: Choice, alias: str | None = None) -> list[str]:
    """Replace the model (and effort) flag of an explicit command by the configured one; a flag that is
    not there stays absent. Used for ``[launch] command`` templates."""
    if not applies_to(choice, alias):
        return list(argv)
    out = list(argv)
    if choice.model:
        out = _replace_flag(out, MODEL_FLAGS, choice.model)
    if choice.effort:
        out = _replace_flag(out, ALL_EFFORT_FLAGS, choice.effort)
    return out


def command_choice(argv: list[str] | None) -> dict | None:
    """``{model, effort}`` found in a command line (flags only), or None when it names neither."""
    model = effort = None
    words = [str(w) for w in argv or []]
    for i, word in enumerate(words):
        for flags, which in ((MODEL_FLAGS, "model"), (ALL_EFFORT_FLAGS, "effort")):
            for flag in flags:
                val = None
                if word == flag and i + 1 < len(words):
                    val = words[i + 1]
                elif word.startswith(flag + "="):
                    val = word[len(flag) + 1:]
                if val and not val.startswith("-"):
                    if which == "model":
                        model = model or val
                    else:
                        effort = effort or val
    return {"model": model, "effort": effort} if model or effort else None


def ran_with(lane: dict, usage: dict | None) -> dict | None:
    """What an agent actually ran with: the model from its usage records (else its command), and the
    effort from its command."""
    cmd = lane.get("ran") if isinstance(lane.get("ran"), dict) else {}
    model = (usage or {}).get("model") or cmd.get("model")
    effort = cmd.get("effort")
    return {"model": model, "effort": effort} if model or effort else None


# -- editing config.toml -----------------------------------------------------------------------


def _key(part: str) -> str:
    return part if _BARE.match(part) else json.dumps(part)


def header_for(role: str, project: str | None = None, step: str | None = None) -> str:
    if role == "manager":
        if project or step:
            raise ModelsError("the manager has no per-project or per-step setting")
        return "models.manager"
    if step is not None and not project:
        raise ModelsError("--step needs --project")
    if step is not None:
        return f'models.workers.steps.{json.dumps(f"{project}/{step}")}'
    if project:
        return f"models.workers.projects.{_key(project)}"
    return "models.workers"


def _path_of(header_line: str) -> tuple[str, ...] | None:
    """Key path of a ``[a.b."c"]`` line, or None when the line is not a plain table header."""
    text = header_line.strip()
    if not text.startswith("[") or text.startswith("[["):
        return None
    try:
        node = tomllib.loads(text + "\n")
    except tomllib.TOMLDecodeError:
        return None
    path = []
    while isinstance(node, dict) and len(node) == 1:
        (k, node), = node.items()
        path.append(k)
    return tuple(path)


def _value(text: str) -> str:
    if not _TOKEN.match(text):
        raise ModelsError(f"'{text}' is not a valid model, effort or agent name")
    return json.dumps(text)


def _check_alias(agent: str) -> None:
    if not re.match(r"^[A-Za-z0-9_-]+$", agent):
        raise ModelsError(f"'{agent}' is not a valid agent alias")


def _line(key: str, value) -> str | None:
    if value is None:
        return None
    if key == "fallback":
        return f"fallback = [{', '.join(_agent(a) for a in value)}]\n"
    return f"{key} = {_agent(value) if key == 'agent' else _value(value)}\n"


def edit_table(text: str, header: str, values: dict[str, str | None]) -> str:
    """``text`` with ``[header]`` holding ``values`` (None removes a key); other lines are untouched."""
    target = _path_of(f"[{header}]")
    lines = text.splitlines(keepends=True)
    start = next((i for i, ln in enumerate(lines) if _path_of(ln) == target), None)
    rendered = {k: _line(k, v) for k, v in values.items()}
    if start is None:
        body = [r for r in rendered.values() if r]
        if not body:
            return text
        sep = "" if not text or text.endswith("\n\n") else ("\n" if text.endswith("\n") else "\n\n")
        return text + sep + f"[{header}]\n" + "".join(body)
    end = len(lines)
    for i in range(start + 1, len(lines)):
        if lines[i].lstrip().startswith("["):
            end = i
            break
    seen = set()
    for i in range(start + 1, end):
        m = re.match(r"^\s*([A-Za-z0-9_-]+)\s*=", lines[i])
        if m and m.group(1) in rendered:
            seen.add(m.group(1))
            lines[i] = rendered[m.group(1)] or ""
    last = end
    while last > start + 1 and not lines[last - 1].strip():
        last -= 1
    if last > start and not lines[last - 1].endswith("\n"):
        lines[last - 1] += "\n"
    add = [r for k, r in rendered.items() if r and k not in seen]
    lines[last:last] = add
    return "".join(lines)


def _agent(alias: str) -> str:
    _check_alias(alias)
    return json.dumps(alias)


def remove_table(text: str, header: str) -> str:
    """``text`` without the ``[header]`` table (its comments inside go with it)."""
    target = _path_of(f"[{header}]")
    lines = text.splitlines(keepends=True)
    start = next((i for i, ln in enumerate(lines) if _path_of(ln) == target), None)
    if start is None:
        return text
    end = next((i for i in range(start + 1, len(lines)) if lines[i].lstrip().startswith("[")), len(lines))
    while end > start + 1 and not lines[end - 1].strip():
        end -= 1
    del lines[start:end]
    return "".join(lines)


def set_models(home: Path, role: str, *, model: str | None = None, effort: str | None = None,
               agent: str | None = None, project: str | None = None, step: str | None = None,
               clear: bool = False, fallback: list[str] | None = None) -> str:
    """Write the setting into ``<home>/config.toml`` and return a one-line description."""
    if role not in ROLES:
        raise ModelsError(f"role must be one of {', '.join(ROLES)}")
    header = header_for(role, project, step)
    path = home / "config.toml"
    text = path.read_text(encoding="utf-8") if path.exists() else ""
    if clear:
        if role == "manager" or not (project or step):
            raise ModelsError("--clear removes a per-project or per-step override only")
        new = remove_table(text, header)
    else:
        if not (model or effort or agent) and fallback is None:
            raise ModelsError("nothing to set: pass --model, --effort, --agent or --fallback")
        if fallback is not None and role != "workers":
            raise ModelsError("only the workers have a fallback chain")
        given = {"agent": agent, "model": model, "effort": effort}
        values = {k: v for k, v in given.items() if v}  # unset keys stay
        if fallback is not None:
            values["fallback"] = list(dict.fromkeys(fallback))
        new = edit_table(text, header, values)
    try:
        tomllib.loads(new)
    except tomllib.TOMLDecodeError as error:
        raise ModelsError(f"the edit would break config.toml: {error}")
    if new != text:
        write_text_atomic(path, new)
    what = ", ".join(f"{k} {v}" for k, v in (("agent", agent), ("model", model), ("effort", effort),
                                             ("fallback", ", ".join(fallback) if fallback else None)) if v)
    return f"{'cleared' if clear else 'set'} [{header}]" + (f": {what}" if what and not clear else "")


# -- the manager's own settings ----------------------------------------------------------------


def manager_settings_path(config: Config, alias: str) -> Path:
    try:
        from tirith import hooks as hooks_mod  # late: hooks -> ... -> launch -> models
        return hooks_mod.default_settings(alias, config.agents)
    except ValueError as error:
        raise ModelsError(str(error))


def read_manager_settings(config: Config, alias: str | None) -> dict | None:
    """``{path, model, effort}`` the manager's settings file holds now, or None when unreadable."""
    if not alias:
        return None
    try:
        path = manager_settings_path(config, alias)
        data = json.loads(path.read_text(encoding="utf-8") or "{}") if path.exists() else {}
    except (ModelsError, OSError, ValueError):
        return None
    if not isinstance(data, dict):
        return None
    keys = SETTINGS_KEYS.get(alias, {"model": "model", "effort": None})
    return {"path": str(path), "model": data.get(keys["model"]) if keys["model"] else None,
            "effort": data.get(keys["effort"]) if keys["effort"] else None}


def apply_manager(config: Config) -> list[str]:
    """Write ``[models.manager]`` into the manager agent's user settings, keeping every other key.
    Returns the lines to print."""
    choice = resolve_manager(config)
    if not choice.agent:
        raise ModelsError("no manager agent: run `tirith models set manager --agent <alias> --model <id>`")
    if not choice.model:
        raise ModelsError("no manager model: run `tirith models set manager --model <id>`")
    path = manager_settings_path(config, choice.agent)
    try:
        data = json.loads(path.read_text(encoding="utf-8") or "{}") if path.exists() else {}
    except (OSError, ValueError) as error:
        raise ModelsError(f"{path} is unreadable: {error}")
    if not isinstance(data, dict):
        raise ModelsError(f"{path} is not a JSON object")
    keys = SETTINGS_KEYS.get(choice.agent, {"model": "model", "effort": None})
    data[keys["model"] or "model"] = choice.model
    notes = [f"manager model {choice.model} written to {path}"]
    if choice.effort:
        if keys["effort"]:
            data[keys["effort"]] = choice.effort
            notes.append(f"manager effort {choice.effort} written to {path}")
        else:
            notes.append(f"effort {choice.effort} recorded in config.toml only: {choice.agent} has no "
                         "effort key in its settings file")
    write_json_atomic(path, data)
    return notes


# -- state and show ----------------------------------------------------------------------------


def state(config: Config, pending: list[str] | None = None, status: dict | None = None) -> dict:
    """``state.models``: the settings, what the manager's file holds, and the lists the page offers."""
    cfg = parse(config.models)
    manager = resolve_manager(config).as_dict()
    held = read_manager_settings(config, manager["agent"])
    synced = bool(held) and held["model"] == manager["model"] and (
        not manager["effort"] or not SETTINGS_KEYS.get(manager["agent"], {}).get("effort")
        or held["effort"] == manager["effort"]) if manager["model"] else None
    aliases = sorted(set(config.agents) | {"anthropic", "muse"})
    return {
        "manager": {**manager, "settings": held, "in_sync": synced,
                    "effort_key": bool(SETTINGS_KEYS.get(manager["agent"] or "", {}).get("effort"))},
        "workers": {**{k: cfg["workers"].get(k) for k in FIELDS}, "fallback": cfg["workers"].get("fallback") or []},
        "projects": cfg["projects"], "steps": cfg["steps"],
        "choices": cfg["choices"], "choices_by_agent": {a: choices_for(config, a) for a in aliases},
        "efforts": cfg["efforts"] or list(EFFORTS), "agents": aliases,
        "agent_status": status or {},
        "pending": pending or [],
    }


def show_lines(config: Config) -> list[str]:
    cfg = parse(config.models)

    def fmt(d: dict) -> str:
        return "  ".join(f"{k}={d[k]}" for k in FIELDS if d.get(k)) or "(not set)"
    out = [f"manager: {fmt(cfg['manager'])}", f"workers: {fmt(cfg['workers'])}"]
    if cfg["workers"].get("fallback"):
        out.append("workers fallback: " + " -> ".join(cfg["workers"]["fallback"]))
    out += [f"workers project {k}: {fmt(v)}" for k, v in sorted(cfg["projects"].items())]
    out += [f"workers step {k}: {fmt(v)}" for k, v in sorted(cfg["steps"].items())]
    if cfg["choices"]:
        out.append("choices: " + ", ".join(cfg["choices"]))
    held = read_manager_settings(config, cfg["manager"].get("agent"))
    if held:
        out.append(f"manager settings file {held['path']}: model={held['model'] or '-'}"
                   + (f" effort={held['effort']}" if held["effort"] else ""))
    return out
