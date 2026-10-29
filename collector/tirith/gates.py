"""Rules as code: what a ``PreToolUse`` hook blocks, and the shims that guard the agent CLIs.

See ``docs/CONTRACT-gates.md``. Every rule returns a :class:`Violation` whose text tells the agent
exactly what to do instead (``tirith: <rule>: <what was blocked>. Do this instead: <command>``).
Agents are named by alias; the real executable of each alias comes from the owner's
``[agents.<alias>] command`` in ``config.toml``.
"""

from __future__ import annotations

import fnmatch
import os
import re
import shlex
import shutil
import stat
from dataclasses import dataclass
from pathlib import Path

from tirith import manifest as manifest_mod
from tirith.home import Config
from tirith.store import append_jsonl

EVENTS_FILE = "events.jsonl"
SUBAGENT_TOOLS = ("Task", "Agent")
EDIT_TOOLS = ("Edit", "Write", "MultiEdit", "NotebookEdit")
SHIM_MARKER = "# tirith-shim v1"
TOKEN_ENV = "TIRITH_LANE_TOKEN"
LANE_RUN_HINT = ('tirith lane run --project {project} --class {cls} --feature "{feature}" '
                 '--brief {brief} -- {agent} …')
_WRAPPERS = {"env", "command", "exec", "nohup", "time", "sudo", "builtin", "nice", "stdbuf"}
_RUNNERS = {"npx", "bunx", "pnpx"}
_SHELLS = {"sh", "bash", "zsh", "dash", "ksh"}
_MUTATORS = {"rm", "rmdir", "mv", "cp", "chmod", "chflags", "chown", "ln", "tee", "sed", "truncate",
             "install", "touch", "dd", "perl", "unlink", "rsync", "patch"}
_COMMIT_VALUE_FLAGS = "mFCctSu"  # short commit options that take a value: the rest is not flags
_BUILTINS = {"cd", "export", "set", "for", "do", "done", "if", "then", "else", "fi", "exit", "true",
             "while", "case", "esac", "pushd", "popd"}


@dataclass
class Violation:
    rule: str
    what: str
    instead: str
    action: str = "block"  # "warn" is recorded and shown but does not stop the call

    def message(self) -> str:
        return f"tirith: {self.rule}: {self.what}. Do this instead: {self.instead}"


# -- command line parsing ---------------------------------------------------------------------


_HEREDOC = re.compile(r"<<(-?)\s*(['\"]?)([A-Za-z_][A-Za-z0-9_]*)\2")


def strip_heredocs(line: str) -> str:
    """The shell line without here-document bodies: they are data written to a file, not commands."""
    out: list[str] = []
    ends: list[tuple[str, bool]] = []
    for row in str(line or "").split("\n"):
        if ends:
            delim, dash = ends[0]
            if (row.lstrip("\t") if dash else row) == delim:
                ends.pop(0)
            continue
        out.append(row)
        ends.extend((m.group(3), m.group(1) == "-") for m in _HEREDOC.finditer(row))
    return "\n".join(out)


def split_commands(line: str) -> list[list[str]]:
    """Simple commands of a shell line as token lists (split on ``; & | ( )`` and newlines)."""
    out: list[list[str]] = []
    for text in str(line or "").replace("`", ";").replace("$(", ";(").splitlines():
        lexer = shlex.shlex(text, posix=True, punctuation_chars=True)
        lexer.whitespace_split = True
        lexer.commenters = ""
        current: list[str] = []
        try:
            tokens = list(lexer)
        except ValueError:  # unbalanced quote: fall back to a plain split
            tokens = text.split()
        for token in tokens:
            if token and set(token) <= set(";&|()"):
                if current:
                    out.append(current)
                current = []
            else:
                current.append(token)
        if current:
            out.append(current)
    return out


def _program(tokens: list[str]) -> tuple[str, list[str]]:
    """Program name (basename) and its arguments, after env assignments and wrappers."""
    index = 0
    while index < len(tokens):
        word = tokens[index]
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", word):
            index += 1
        elif os.path.basename(word) in _WRAPPERS:
            index += 1
            while index < len(tokens) and (tokens[index].startswith("-")
                                           or re.match(r"^[A-Za-z_]\w*=", tokens[index])):
                index += 1
        else:
            break
    if index >= len(tokens):
        return "", []
    return os.path.basename(tokens[index]), tokens[index + 1:]


def _is_tirith(name: str) -> bool:
    return name == "tirith"


def _starts_agent(tokens: list[str], commands: dict[str, str], depth: int = 0) -> str | None:
    """Alias of the agent CLI this simple command starts, else None."""
    name, args = _program(tokens)
    if not name or _is_tirith(name):
        return None
    if name in commands:
        return commands[name]
    if name in _RUNNERS or (name in ("pnpm", "yarn", "npm", "bun") and args[:1] in (["dlx"], ["exec"], ["x"])):
        rest = args[1:] if name not in _RUNNERS else args
        for word in rest:
            if word.startswith("-"):
                continue
            package = re.sub(r"@[^/@]*$", "", word)  # drop @version
            for candidate in (os.path.basename(package), package):
                if candidate in commands:
                    return commands[candidate]
            break
    if name in _SHELLS and depth < 3 and "-c" in args[:-1]:
        for inner in split_commands(args[args.index("-c") + 1]):
            found = _starts_agent(inner, commands, depth + 1)
            if found:
                return found
    return None


def agent_commands(config: Config) -> dict[str, str]:
    """Basename of each configured agent executable -> its alias."""
    out: dict[str, str] = {}
    for alias, table in config.agents.items():
        command = str(table.get("command") or "").strip()
        if command:
            out[os.path.basename(command)] = alias
    return out


# -- the rules --------------------------------------------------------------------------------


def lane_run_hint(project: str | None, command: str = "<agent cmd>",
                  feature: str = "<what the agent does>") -> str:
    """The ``tirith lane run`` command to use, filled with what is known."""
    return LANE_RUN_HINT.format(project=project or "<project-id>", cls="<class>",
                                feature=feature.replace('"', "'"), brief="<brief-file>",
                                agent=command)


def _subagent(tool_input, project: str | None) -> Violation:
    data = tool_input if isinstance(tool_input, dict) else {}
    feature = " ".join(str(data.get("description") or "").split()) or "<what the sub-agent does>"
    return Violation(
        "subagent-tool", "the agent's internal sub-agent tool is not allowed",
        lane_run_hint(project, feature=feature[:80]) +
        " (write the sub-agent's task into the brief file first)")


def _agent_cli(alias: str, line: str, project: str | None, commands: dict[str, str]) -> Violation:
    command = next((c for c, a in commands.items() if a == alias), alias)
    return Violation(
        "agent-cli", f"agent '{alias}' was started without tirith lane run: {_clip(line)}",
        lane_run_hint(project, command))


def _clip(text: str, limit: int = 100) -> str:
    text = " ".join(str(text).split())
    return text if len(text) <= limit else text[:limit - 1] + "…"


def _hooks_target(tokens: list[str]) -> bool:
    return any(".git/hooks" in token for token in tokens)


def _commit_no_verify(args: list[str]) -> bool:
    """``git commit -n`` (also inside clusters such as ``-an``)."""
    if "commit" not in args:
        return False
    for token in args[args.index("commit") + 1:]:
        if token == "--":
            break
        if re.match(r"^-[A-Za-z]", token):
            for letter in token[1:]:
                if letter == "n":
                    return True
                if letter in _COMMIT_VALUE_FLAGS:
                    break
    return False


def _git_rules(line: str, segments: list[list[str]]) -> list[Violation]:
    out: list[Violation] = []
    if any(_program(tokens)[0] == "git" and any("core.hookspath" in t.lower() for t in tokens)
           for tokens in segments):
        out.append(Violation(
            "hooks-path", f"changing core.hooksPath bypasses the git hooks: {_clip(line)}",
            "leave core.hooksPath alone; to change what a hook runs, edit the [[hooks]] entries "
            "of tirith.toml and run `tirith check`"))
    for tokens in segments:
        name, args = _program(tokens)
        if name == "git" and ("--no-verify" in args or _commit_no_verify(args)):
            out.append(Violation(
                "no-verify", f"skipping the git hooks is not allowed: {_clip(line)}",
                "run the same git command without --no-verify / -n; if a hook fails, fix what it "
                "reports (`tirith check` lists it)"))
        elif _hooks_target(tokens) and (name in _MUTATORS or any(t.startswith((">", ">>")) for t in tokens)):
            out.append(Violation(
                "git-hooks", f"editing or removing files under .git/hooks is not allowed: {_clip(line)}",
                "leave .git/hooks alone; the owner manages it with `tirith guard install <project>`"))
    return out


def _raw_stage(segments: list[list[str]], folder: Path | None, project: str | None) -> Violation | None:
    if folder is None:
        return None
    manifest = manifest_mod.load(folder)
    if not manifest.exists:
        return None
    runnable = [seg for seg in segments if not _is_tirith(_program(seg)[0])]
    for pipe in manifest.pipelines:
        for stage in pipe.stages:
            if not stage.run:
                continue
            wanted = split_commands(stage.run)
            if not wanted:
                continue
            hit = _contains(runnable, wanted)
            if not hit:
                tail = [seg for seg in wanted if _program(seg)[0] not in _BUILTINS]
                last = tail[-1] if tail else None
                hit = bool(last and len(last) >= 2 and last in runnable)
            if hit:
                same = sum(1 for p in manifest.pipelines for s in p.stages if s.name == stage.name)
                extra = f" --pipeline {pipe.id}" if same > 1 else ""
                flag = f" --project {project}" if project else ""
                return Violation(
                    "raw-stage", f"stage '{stage.name}' of pipeline '{pipe.id}' was run raw "
                    f"({_clip(stage.run)})", f"tirith run{flag}{extra} --stage {stage.name}")
    return None


def _contains(haystack: list[list[str]], needle: list[list[str]]) -> bool:
    for start in range(len(haystack) - len(needle) + 1):
        if haystack[start:start + len(needle)] == needle:
            return True
    return False


def _command_forms(segments: list[list[str]]) -> list[str]:
    """Each simple command as typed and as ``program args`` (env assignments and wrappers dropped)."""
    out: list[str] = []
    for tokens in segments:
        name, args = _program(tokens)
        for form in (" ".join(tokens), " ".join([name, *args]).strip()):
            if form and form not in out:
                out.append(form)
    return out


def _path_forms(path: str, folder: Path, cwd) -> list[str]:
    """The edited path as given, relative to the manifest folder, and relative to ``cwd``."""
    forms = [path]
    candidate = Path(path)
    if not candidate.is_absolute() and cwd:
        candidate = Path(str(cwd)) / candidate
    for base in (folder, Path(str(cwd))) if cwd else (folder,):
        try:
            forms.append(candidate.resolve().relative_to(base.resolve()).as_posix())
        except (ValueError, OSError):
            pass
    return forms


def _glob(pattern: str, text: str) -> bool:
    return fnmatch.fnmatchcase(text, pattern)


def _custom_rules(folder: Path | None, tool: str, data: dict, line: str, segments: list[list[str]],
                  cwd) -> list[Violation]:
    """Violations of the project's ``[[rules]]`` (``tirith.toml``) for one call."""
    if folder is None:
        return []
    rules = manifest_mod.load(folder).rules
    out: list[Violation] = []
    forms = _command_forms(segments) if tool == "Bash" else []
    path = str(data.get("file_path") or data.get("path") or data.get("notebook_path") or "")
    for rule in rules:
        if tool == "Bash":
            if rule.match:
                hit = any(_glob(rule.match, form) for form in forms) or _glob(rule.match, " ".join(line.split()))
            elif rule.regex:
                hit = bool(re.search(rule.regex, line))
            else:
                hit = False
            what = _clip(line)
        else:
            hit = bool(rule.paths and path and any(
                _glob(pattern, form) for pattern in rule.paths for form in _path_forms(path, folder, cwd)))
            what = path
        if hit:
            out.append(Violation(rule.id, f"{rule.message}: {what}", rule.instead, rule.action))
    return out


def find_manifest_folder(cwd) -> Path | None:
    """Nearest folder at or above ``cwd`` that has a ``tirith.toml``."""
    if not cwd:
        return None
    path = Path(str(cwd))
    for folder in (path, *path.parents):
        if (folder / manifest_mod.FILENAME).is_file():
            return folder
    return None


def check_call(config: Config, tool: str | None, tool_input, cwd, project: str | None = None) -> list[Violation]:
    """Every rule that blocks one ``PreToolUse`` call (``[]`` = allowed)."""
    data = tool_input if isinstance(tool_input, dict) else {}
    tool = tool or ""
    if tool in SUBAGENT_TOOLS:
        return [_subagent(data, project)]
    if tool in EDIT_TOOLS:
        path = str(data.get("file_path") or data.get("path") or data.get("notebook_path") or "")
        if "/.git/hooks/" in path + "/" or path.endswith("/.git/hooks"):
            return [Violation("git-hooks", f"editing files under .git/hooks is not allowed: {path}",
                              "leave .git/hooks alone; the owner manages it with "
                              "`tirith guard install <project>`")]
        return _custom_rules(find_manifest_folder(cwd), tool, data, "", [], cwd)
    if tool != "Bash":
        return []
    line = strip_heredocs(str(data.get("command") or data.get("cmd") or ""))
    segments = split_commands(line)
    out = _git_rules(line, segments)
    commands = agent_commands(config)
    seen: set[str] = set()
    for tokens in segments:
        alias = _starts_agent(tokens, commands)
        if alias and alias not in seen:
            seen.add(alias)
            out.append(_agent_cli(alias, line, project, commands))
    stage = _raw_stage(segments, find_manifest_folder(cwd), project)
    if stage:
        out.append(stage)
    out += _custom_rules(find_manifest_folder(cwd), tool, data, line, segments, cwd)
    return out


def record_violation(home: Path, violation: Violation, *, ts: str, agent: str | None,
                     lane: str | None, session: str | None, project: str | None,
                     tool: str | None, cwd: str | None) -> dict:
    """Append one ``kind: "violation"`` row to ``events.jsonl``."""
    row = {"kind": "violation", "ts": ts, "rule": violation.rule, "agent": agent, "lane": lane,
           "session": session, "project": project, "tool": tool,
           "message": violation.message(), "cwd": cwd}
    if violation.action != "block":
        row["action"] = violation.action
    append_jsonl(home / EVENTS_FILE, row)
    return row


# -- shims ------------------------------------------------------------------------------------


def shim_script(alias: str, name: str, real: str) -> str:
    """The sh wrapper for one agent: runs the real binary interactively or with a lane token."""
    rule = Violation(
        "agent-cli", f"agent '{alias}' ({name}) must not start outside tirith lane run",
        lane_run_hint(None, name)).message()
    return f"""#!/bin/sh
{SHIM_MARKER} alias={alias}
# Written by `tirith shim install`; do not edit. Runs {real} when a person uses it in a terminal
# or when `tirith lane run` started it (its token matches the lane file).
REAL={shlex.quote(real)}
if [ -t 0 ] && [ -t 1 ]; then exec "$REAL" "$@"; fi
HOME_DIR="${{TIRITH_HOME:-$HOME/.tirith}}"
if [ -n "${{{TOKEN_ENV}:-}}" ] && [ -n "${{TIRITH_LANE:-}}" ] \\
   && grep -F -q "\\"token\\": \\"${TOKEN_ENV}\\"" "$HOME_DIR/lanes/$TIRITH_LANE.json" 2>/dev/null; then
  exec "$REAL" "$@"
fi
printf '%s\\n' {shlex.quote(rule)} >&2
exit 2
"""


def is_shim(path: Path) -> bool:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            handle.readline()
            return handle.readline().startswith(SHIM_MARKER)
    except OSError:
        return False


def _path_without(shim_dir: Path, path_env: str | None) -> str:
    keep = []
    for part in (path_env if path_env is not None else os.environ.get("PATH", "")).split(os.pathsep):
        if part and Path(part).expanduser().resolve() != shim_dir.resolve():
            keep.append(part)
    return os.pathsep.join(keep)


def resolve_real(command: str, shim_dir: Path, path_env: str | None = None) -> str | None:
    """The real executable of ``command`` on PATH, never one inside the shim dir."""
    if os.sep in command:
        path = Path(command).expanduser()
        return str(path) if path.is_file() and os.access(path, os.X_OK) \
            and not is_shim(path) else None
    found = shutil.which(command, path=_path_without(shim_dir, path_env))
    return found if found and not is_shim(Path(found)) else None


def install_shims(config: Config, shim_dir: Path, path_env: str | None = None) -> list[tuple[str, str, str]]:
    """Write one shim per configured agent; returns ``(alias, status, detail)`` rows."""
    rows: list[tuple[str, str, str]] = []
    for alias, table in config.agents.items():
        command = str(table.get("command") or "").strip()
        if not command:
            continue
        name = os.path.basename(command)
        target = shim_dir / name
        real = resolve_real(command, shim_dir, path_env)
        if real is None:
            rows.append((alias, "skipped", f"'{command}' is not on PATH (outside {shim_dir})"))
        elif target.exists() and not is_shim(target):
            rows.append((alias, "skipped", f"{target} exists and is not a Tirith shim"))
        else:
            shim_dir.mkdir(parents=True, exist_ok=True)
            target.write_text(shim_script(alias, name, real), encoding="utf-8")
            target.chmod(target.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
            rows.append((alias, "installed", str(target)))
    return rows


def uninstall_shims(config: Config, shim_dir: Path) -> list[tuple[str, str, str]]:
    rows: list[tuple[str, str, str]] = []
    for alias, table in config.agents.items():
        command = str(table.get("command") or "").strip()
        if not command:
            continue
        target = shim_dir / os.path.basename(command)
        if target.exists() and is_shim(target):
            target.unlink()
            rows.append((alias, "removed", str(target)))
        elif target.exists():
            rows.append((alias, "skipped", f"{target} is not a Tirith shim"))
        else:
            rows.append((alias, "absent", str(target)))
    return rows


def verify_shims(config: Config, shim_dir: Path, path_env: str | None = None) -> list[str]:
    """Problems with the installed shims (``[]`` = all intact)."""
    problems: list[str] = []
    for alias, table in config.agents.items():
        command = str(table.get("command") or "").strip()
        if not command:
            continue
        target = shim_dir / os.path.basename(command)
        fix = f"Fix: tirith shim install --dir {shim_dir}"
        if not target.exists():
            problems.append(f"{alias}: no shim at {target}. {fix}")
            continue
        if not is_shim(target):
            problems.append(f"{alias}: {target} is not a Tirith shim; move it away. {fix}")
            continue
        if not os.access(target, os.X_OK):
            problems.append(f"{alias}: {target} is not executable. {fix}")
        match = re.search(r"^REAL=(.*)$", target.read_text(encoding="utf-8"), re.M)
        real = shlex.split(match.group(1))[0] if match else ""
        if not real or not os.access(real, os.X_OK):
            problems.append(f"{alias}: the real binary '{real}' is gone. {fix}")
        first = shutil.which(os.path.basename(command),
                             path=path_env if path_env is not None else os.environ.get("PATH", ""))
        if first is None or Path(first).resolve() != target.resolve():
            problems.append(f"{alias}: {shim_dir} is not first for '{os.path.basename(command)}' "
                            f"in PATH. Fix: put {shim_dir} before the real binary's folder in PATH")
    return problems
