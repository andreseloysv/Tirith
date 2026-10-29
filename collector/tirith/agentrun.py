"""Agent presets, per-lane usage and the full diff of a lane, plus ``tirith doctor --agent``.

Presets: ``tirith lane run --agent <alias>`` builds the headless command line from
``[agents.<alias>]`` in ``config.toml`` (and the optional ``[agents.<alias>.projects.<id>]``
override) on top of ``PRESET_DEFAULTS``. The defaults are flags only: the executable always comes
from the config.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

from tirith.home import Config

# -- presets -----------------------------------------------------------------------------------

# alias -> flag names used to assemble a headless command. ``args`` always come first, then the
# model / effort / permission / extra-dir flags, then the prompt as the last positional argument.
PRESET_DEFAULTS: dict[str, dict] = {
    "anthropic": {
        "args": ["-p", "--output-format", "stream-json", "--verbose",
                 "--permission-mode", "acceptEdits"],
        "permissions": ["Read", "Edit", "Write", "Glob", "Grep", "NotebookEdit",
                        "Bash(git status:*)", "Bash(git diff:*)", "Bash(git log:*)",
                        "Bash(ls:*)", "Bash(uv run:*)", "Bash(python3:*)", "Bash(node:*)",
                        "Bash(tirith:*)"],
        "permissions_flag": "--allowedTools",
        "permissions_join": ",",
        "model_flag": "--model",
        "effort_flag": "--effort",
        "add_dir_flag": "--add-dir",
    },
    "muse": {
        "args": ["exec", "--json"],
        "permissions": [],
        "permissions_flag": None,
        "model_flag": "--model",
        "effort_flag": None,
        "add_dir_flag": None,
    },
}

_OVERRIDABLE = ("command", "args", "permissions", "model", "effort", "add_dirs", "prompt_flag")


def agent_settings(config: Config, alias: str, project: str | None = None) -> dict:
    """Preset defaults, then ``[agents.<alias>]``, then ``[agents.<alias>.projects.<project>]``."""
    table = config.agents.get(alias)
    if table is None and alias not in PRESET_DEFAULTS:
        raise SystemExit(f"unknown agent alias '{alias}': add [agents.{alias}] to config.toml")
    merged = dict(PRESET_DEFAULTS.get(alias, {}))
    table = table or {}
    for key, value in table.items():
        if key != "projects":
            merged[key] = value
    override = (table.get("projects") or {}).get(project or "")
    if isinstance(override, dict):
        for key, value in override.items():
            if key in _OVERRIDABLE or key in merged:
                merged[key] = value
    return merged


def build_command(config: Config, alias: str, project: str | None, prompt: str, *,
                  model: str | None = None, effort: str | None = None) -> list[str]:
    """The full headless command for ``alias``; raises SystemExit when it cannot be built."""
    s = agent_settings(config, alias, project)
    exe = s.get("command")
    if not exe:
        raise SystemExit(f"[agents.{alias}] has no command in config.toml")
    cmd = [str(exe)] + [str(a) for a in (s.get("args") or [])]
    model = model or s.get("model")
    if model:
        if not s.get("model_flag"):
            raise SystemExit(f"agent '{alias}' has no model flag configured")
        cmd += [str(s["model_flag"]), str(model)]
    effort = effort or s.get("effort")
    if effort:
        if not s.get("effort_flag"):
            raise SystemExit(f"agent '{alias}' has no effort flag configured")
        cmd += [str(s["effort_flag"]), str(effort)]
    perms = [str(p) for p in (s.get("permissions") or [])]
    if perms and s.get("permissions_flag"):
        sep = s.get("permissions_join")
        if sep:
            cmd += [str(s["permissions_flag"]), sep.join(perms)]
        else:
            cmd += [str(s["permissions_flag"])] + perms
    dirs = [str(Path(str(d)).expanduser()) for d in (s.get("add_dirs") or [])]
    if dirs and s.get("add_dir_flag"):
        for d in dirs:
            cmd += [str(s["add_dir_flag"]), d]
    if s.get("prompt_flag"):
        cmd += [str(s["prompt_flag"]), prompt]
    else:
        cmd += ["--", prompt] if prompt.startswith("-") else [prompt]
    return cmd


# -- usage -------------------------------------------------------------------------------------


def _num(value) -> float:
    return float(value) if isinstance(value, (int, float)) and not isinstance(value, bool) else 0.0


def _tokens(usage: dict) -> tuple[int, int, int]:
    inp = _num(usage.get("input_tokens", usage.get("inputTokens")))
    out = _num(usage.get("output_tokens", usage.get("outputTokens")))
    cache = sum(_num(usage.get(k)) for k in (
        "cache_creation_input_tokens", "cache_read_input_tokens", "cache_tokens",
        "cacheCreationInputTokens", "cacheReadInputTokens"))
    return int(inp), int(out), int(cache)


def parse_usage(path: str | Path | None) -> dict | None:
    """``{input_tokens, output_tokens, cache_tokens, cost_usd, model, turns, duration_s}`` from a
    JSONL lane log, or None when the log has no usage record. ``result`` records win over
    ``run.usage`` records; several records of one kind add up."""
    if not path:
        return None
    try:
        text = Path(path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    results: list[dict] = []
    runs: list[dict] = []
    for line in text.splitlines():
        if '"usage"' not in line and "run.usage" not in line:
            continue
        try:
            rec = json.loads(line)
        except ValueError:
            continue
        if not isinstance(rec, dict):
            continue
        if rec.get("type") == "result" and isinstance(rec.get("usage"), dict):
            results.append(rec)
        elif rec.get("payload_type") == "run.usage" or rec.get("type") == "run.usage":
            runs.append(rec)
    out = {"input_tokens": 0, "output_tokens": 0, "cache_tokens": 0, "cost_usd": None,
           "model": None, "turns": 0, "duration_s": 0}
    cost = 0.0
    best_model, best_cost = None, -1.0
    if results:
        for rec in results:
            i, o, c = _tokens(rec["usage"])
            out["input_tokens"] += i
            out["output_tokens"] += o
            out["cache_tokens"] += c
            if isinstance(rec.get("total_cost_usd"), (int, float)):
                cost += float(rec["total_cost_usd"])
                out["cost_usd"] = 0.0
            out["turns"] += int(_num(rec.get("num_turns")))
            out["duration_s"] += _num(rec.get("duration_ms")) / 1000
            by_model = rec.get("modelUsage")
            if isinstance(by_model, dict):
                for name, row in by_model.items():
                    spent = _num(row.get("costUSD")) if isinstance(row, dict) else 0.0
                    if spent > best_cost:
                        best_model, best_cost = str(name), spent
            elif rec.get("model"):
                best_model = str(rec["model"])
    elif runs:
        for rec in runs:
            body = rec.get("payload") if isinstance(rec.get("payload"), dict) else rec
            usage = body.get("usage") if isinstance(body.get("usage"), dict) else body
            i, o, c = _tokens(usage)
            total = _num(usage.get("total_tokens"))
            if not (i or o) and total:
                i = int(total)
            out["input_tokens"] += i
            out["output_tokens"] += o
            out["cache_tokens"] += c
            spent = body.get("cost_usd", body.get("total_cost_usd"))
            if isinstance(spent, (int, float)) and not isinstance(spent, bool):
                cost += float(spent)
                out["cost_usd"] = 0.0
            out["turns"] += 1
            out["duration_s"] += _num(body.get("duration_ms")) / 1000
            if body.get("model"):
                best_model = str(body["model"])
    else:
        return None
    if out["cost_usd"] is not None:
        out["cost_usd"] = round(cost, 4)
    out["model"] = best_model
    out["duration_s"] = int(round(out["duration_s"]))
    return out


def sum_usage(rows: list[dict | None]) -> dict | None:
    """Totals of several ``usage`` dicts (None when none has usage). Cost stays null only if no
    row knows its cost."""
    rows = [r for r in rows if r]
    if not rows:
        return None
    costs = [r["cost_usd"] for r in rows if r.get("cost_usd") is not None]
    models = sorted({r["model"] for r in rows if r.get("model")})
    return {
        "input_tokens": sum(r["input_tokens"] for r in rows),
        "output_tokens": sum(r["output_tokens"] for r in rows),
        "cache_tokens": sum(r["cache_tokens"] for r in rows),
        "cost_usd": round(sum(costs), 4) if costs else None,
        "model": models[0] if len(models) == 1 else (None if not models else "mixed"),
        "turns": sum(r["turns"] for r in rows),
        "duration_s": sum(r["duration_s"] for r in rows),
    }


# -- diffs -------------------------------------------------------------------------------------

DIFF_MAX = 1024 * 1024
UNTRACKED_FILE_MAX = 256 * 1024
_SAFE = re.compile(r"[^A-Za-z0-9._-]")


def diff_path(home: Path, lane_id: str) -> Path:
    return home / "diffs" / f"{_SAFE.sub('_', lane_id)}.patch"


def _git(workspace: str, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", workspace, *args], capture_output=True, timeout=30)


def _is_binary(data: bytes) -> bool:
    return b"\0" in data[:8192]


def build_patch(workspace: str, start_commit: str) -> str:
    """``git diff <start_commit>`` plus the contents of untracked files as added-file patches.
    Binary files are listed only; the whole text is cut at ``DIFF_MAX`` bytes."""
    tracked = _git(workspace, "diff", "--no-color", start_commit)
    parts = [tracked.stdout.decode("utf-8", "replace")] if tracked.returncode == 0 else []
    listing = _git(workspace, "ls-files", "--others", "--exclude-standard", "-z")
    names = [n for n in listing.stdout.decode("utf-8", "replace").split("\0") if n] \
        if listing.returncode == 0 else []
    for name in names:
        file = Path(workspace) / name
        try:
            data = file.read_bytes()
        except OSError:
            continue
        head = f"diff --git a/{name} b/{name}\nnew file mode 100644\n"
        if _is_binary(data):
            parts.append(head + f"Binary file {name} not shown ({len(data)} bytes)\n")
            continue
        text = data[:UNTRACKED_FILE_MAX].decode("utf-8", "replace")
        lines = text.splitlines()
        body = "".join(f"+{l}\n" for l in lines)
        if len(data) > UNTRACKED_FILE_MAX:
            body += "+… (file cut)\n"
        parts.append(head + f"--- /dev/null\n+++ b/{name}\n@@ -0,0 +1,{len(lines)} @@\n" + body)
    patch = "".join(p if p.endswith("\n") else p + "\n" for p in parts if p)
    raw = patch.encode("utf-8")
    if len(raw) > DIFF_MAX:
        patch = raw[:DIFF_MAX].decode("utf-8", "ignore").rsplit("\n", 1)[0] + "\n… (diff cut at 1 MB)\n"
    return patch


def store_diff(home: Path, lane: dict) -> dict | None:
    """Write the lane's patch to ``<home>/diffs/<lane>.patch`` (and ``.js`` for the page);
    returns ``{path, size}`` or None when the lane has no workspace or start commit."""
    workspace, start = lane.get("workspace"), lane.get("start_commit")
    if not workspace or not start or not Path(workspace).is_dir():
        return None
    try:
        patch = build_patch(str(workspace), str(start))
    except (OSError, subprocess.SubprocessError):
        return None
    target = diff_path(home, str(lane["id"]))
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(patch, encoding="utf-8")
    script = target.with_suffix(".js")  # a file:// page cannot fetch; it loads this script
    script.write_text("window.TIRITH_DIFFS=window.TIRITH_DIFFS||{};window.TIRITH_DIFFS["
                      + json.dumps(str(lane["id"])) + "]=" + json.dumps(patch) + ";\n",
                      encoding="utf-8")
    return {"path": str(target), "size": len(patch.encode("utf-8"))}


# -- doctor ------------------------------------------------------------------------------------


def _which(exe: str, path: str | None = None) -> str | None:
    import shutil
    return shutil.which(exe, path=path)


def doctor_agent(config: Config, alias: str, env_path: str, wrapper_dir: Path) -> list[tuple[str, bool, str]]:
    """``[(check, ok, detail)]`` for ``tirith doctor --agent``. ``env_path`` is the PATH of a lane
    environment."""
    checks: list[tuple[str, bool, str]] = []
    table = config.agents.get(alias)
    exe = (table or {}).get("command") or None
    if not exe:
        checks.append(("executable", False, f"[agents.{alias}] has no command in config.toml"))
    else:
        found = _which(str(exe), env_path)
        checks.append(("executable", bool(found), found or f"'{exe}' not found in PATH"))
    first = env_path.split(os.pathsep)[0] if env_path else ""
    ok = bool(first) and Path(first).resolve() == wrapper_dir.resolve()
    checks.append(("path-order", ok,
                   f"{wrapper_dir} first in lane PATH" if ok else f"lane PATH starts with {first!r}, want {wrapper_dir}"))
    wrapped = wrapper_dir / "git"
    if not wrapped.exists():
        checks.append(("hooks-skip", False, f"no git wrapper at {wrapped}: cannot test (report only)"))
        return checks
    with tempfile.TemporaryDirectory(prefix="tirith-doctor-") as tmp:
        repo = Path(tmp)
        git = _which("git", "/usr/bin:/bin") or "git"
        env = {**os.environ, "PATH": env_path}
        steps = [[git, "init", "-q", str(repo)]]
        for step in steps:
            subprocess.run(step, capture_output=True, env=env, check=False)
        (repo / "f.txt").write_text("x\n", encoding="utf-8")
        subprocess.run([git, "-C", str(repo), "add", "."], capture_output=True, env=env)
        done = subprocess.run(
            ["git", "-C", str(repo), "-c", "user.email=d@example.invalid", "-c", "user.name=doctor",
             "commit", "-q", "--no-verify", "-m", "doctor"],
            capture_output=True, text=True, env=env, timeout=30)
    refused = done.returncode != 0
    checks.append(("hooks-skip", refused,
                   "commit with --no-verify is refused" if refused
                   else "commit with --no-verify went through: the wrapper does not block it"))
    return checks
