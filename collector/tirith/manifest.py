"""Per-project ``tirith.toml``: pipelines and hooks, plus the rules ``tirith check`` enforces.

See ``docs/CONTRACT-manifest.md``. Parsing never raises: problems come back as data.
"""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from tirith.guardwrap import verify_guard

FILENAME = "tirith.toml"
KINDS = ("build", "test", "deploy")
HOOK_EVENTS = ("pre-commit", "commit-msg", "pre-push", "post-commit", "post-merge")
GIT_EVENTS = ("pre-commit", "commit-msg", "pre-push")  # the hooks the guard installs and records
GIT_HOOKS_PIPELINE = "git-hooks"
RULE_ACTIONS = ("block", "warn")
BUILT_IN_RULES = ("subagent-tool", "agent-cli", "no-verify", "hooks-path", "git-hooks", "raw-stage",
                  "lane-brief", "project-check", "commit-guard")
_KIND_RE = re.compile(r"build|compile|test|check|lint|deploy|release|publish|ship")
_KIND_ALIASES = {"compile": "build", "check": "test", "lint": "test",
                 "release": "deploy", "publish": "deploy", "ship": "deploy"}
_HEADER_RE = re.compile(r"^\s*\[\[\s*([A-Za-z0-9_.-]+)\s*\]\]")
_PARSE_LINE_RE = re.compile(r"\(at (?:line (\d+)|end of document)")


@dataclass
class Problem:
    file: str
    line: int
    rule: str
    message: str
    fix: str

    def __str__(self) -> str:
        return f"{self.file}:{self.line}: {self.rule}: {self.message}. Fix: {self.fix}"


@dataclass
class Stage:
    name: str
    run: str | None
    kind: str | None


@dataclass
class Pipeline:
    id: str
    name: str
    stages: list[Stage]


@dataclass
class Hook:
    event: str
    run: str


@dataclass
class Rule:
    """A custom rule of ``[[rules]]``: exactly one of ``match`` (command glob), ``regex``, ``paths``."""
    id: str
    action: str
    message: str
    instead: str
    match: str | None = None
    regex: str | None = None
    paths: list[str] = field(default_factory=list)


@dataclass
class Manifest:
    path: Path
    exists: bool = False
    pipelines: list[Pipeline] = field(default_factory=list)
    hooks: list[Hook] = field(default_factory=list)
    rules: list[Rule] = field(default_factory=list)
    deploy_none: bool = False
    roadmap: str | None = None
    deadline: str | None = None
    plans: list[str] = field(default_factory=list)
    problems: list[Problem] = field(default_factory=list)

    def stage_names(self, pipeline_id: str) -> list[str]:
        for pipe in self.pipelines:
            if pipe.id == pipeline_id:
                return [s.name for s in pipe.stages]
        return []


def stage_kind(stage: Stage) -> str | None:
    """build / test / deploy: the explicit ``kind``, else recognised from the stage name."""
    if stage.kind:
        return stage.kind
    found = _KIND_RE.search(stage.name.lower())
    if not found:
        return None
    return _KIND_ALIASES.get(found.group(0), found.group(0))


def _header_lines(text: str) -> dict[str, list[int]]:
    """Line numbers of each ``[[table]]`` header, in file order, keyed by table name."""
    out: dict[str, list[int]] = {}
    for number, line in enumerate(text.splitlines(), 1):
        found = _HEADER_RE.match(line)
        if found:
            out.setdefault(found.group(1), []).append(number)
    return out


def _problem(manifest: Manifest, line: int, rule: str, message: str, fix: str) -> None:
    manifest.problems.append(Problem(str(manifest.path), line, rule, message, fix))


def _parse_error_line(exc: Exception, text: str) -> int:
    found = _PARSE_LINE_RE.search(str(exc))
    if found and found.group(1):
        return int(found.group(1))
    return max(1, len(text.splitlines()))


def _read_stages(manifest: Manifest, row: dict, stage_lines: list[int], pipe_id: str) -> list[Stage]:
    raw = row.get("stages")
    stages: list[Stage] = []
    seen: set[str] = set()
    for index, item in enumerate(raw if isinstance(raw, list) else []):
        line = stage_lines[index] if index < len(stage_lines) else 1
        if not isinstance(item, dict) or not str(item.get("name") or "").strip():
            _problem(manifest, line, "stage-name", "stage has no name",
                     'add name = "build" under [[pipelines.stages]]')
            continue
        name = str(item["name"]).strip()
        kind = item.get("kind")
        if name in seen:
            _problem(manifest, line, "stage-duplicate",
                     f"stage '{name}' appears twice in pipeline '{pipe_id}'",
                     "rename or remove one of them")
            continue
        seen.add(name)
        if kind is not None and kind not in KINDS:
            _problem(manifest, line, "stage-kind",
                     f"stage '{name}' has kind {kind!r}, expected one of {', '.join(KINDS)}",
                     f'set kind = "{KINDS[0]}" (or test / deploy), or remove the kind line')
            kind = None
        run = item.get("run")
        stages.append(Stage(name, str(run) if run is not None else None, kind))
    return stages


def parse_text(text: str, path: Path) -> Manifest:
    """Parse manifest text; syntax and shape problems land in ``problems``."""
    manifest = Manifest(path=path, exists=True)
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        _problem(manifest, _parse_error_line(exc, text), "toml-syntax",
                 f"cannot parse {FILENAME} ({exc})", f"correct the TOML syntax in {FILENAME}")
        return manifest
    lines = _header_lines(text)
    manifest.deploy_none = str(data.get("deploy") or "").lower() == "none"
    manifest.roadmap = str(data.get("roadmap") or "").strip() or None
    manifest.deadline = str(data.get("deadline") or "").strip() or None
    plans = data.get("plans")
    manifest.plans = [str(p).strip() for p in plans if str(p).strip()] if isinstance(plans, list) else []
    pipes = data.get("pipelines")
    pipe_lines = lines.get("pipelines", [])
    stage_lines = lines.get("pipelines.stages", [])
    seen: set[str] = set()
    consumed = 0
    for index, row in enumerate(pipes if isinstance(pipes, list) else []):
        line = pipe_lines[index] if index < len(pipe_lines) else 1
        nxt = pipe_lines[index + 1] if index + 1 < len(pipe_lines) else 10 ** 9
        mine = [n for n in stage_lines if line < n < nxt]
        consumed += len(mine)
        if not isinstance(row, dict) or not str(row.get("id") or "").strip():
            _problem(manifest, line, "pipeline-id", "pipeline has no id",
                     'add id = "ci" under [[pipelines]]')
            continue
        pipe_id = str(row["id"]).strip()
        if pipe_id in seen:
            _problem(manifest, line, "pipeline-duplicate", f"pipeline id '{pipe_id}' is used twice",
                     "give each [[pipelines]] a unique id")
            continue
        seen.add(pipe_id)
        stages = _read_stages(manifest, row, mine, pipe_id)
        if not stages:
            _problem(manifest, line, "pipeline-stages", f"pipeline '{pipe_id}' has no stages",
                     'add [[pipelines.stages]] with name = "build" and run = "<command>"')
        manifest.pipelines.append(Pipeline(pipe_id, str(row.get("name") or pipe_id), stages))
    for index, row in enumerate(data.get("hooks") if isinstance(data.get("hooks"), list) else []):
        line = lines.get("hooks", [])[index] if index < len(lines.get("hooks", [])) else 1
        event = str(row.get("event") or "") if isinstance(row, dict) else ""
        run = str(row.get("run") or "").strip() if isinstance(row, dict) else ""
        if event not in HOOK_EVENTS:
            _problem(manifest, line, "hook-event",
                     f"hook event {event!r} is not one of {', '.join(HOOK_EVENTS)}",
                     'set event = "pre-commit"')
        elif not run:
            _problem(manifest, line, "hook-run", f"hook '{event}' has no run command",
                     f'add run = "make lint" to the {event} hook')
        else:
            manifest.hooks.append(Hook(event, run))
    rule_rows = data.get("rules") if isinstance(data.get("rules"), list) else []
    rule_lines = lines.get("rules", [])
    rule_ids: set[str] = set()
    for index, row in enumerate(rule_rows):
        line = rule_lines[index] if index < len(rule_lines) else 1
        rule, bad = rule_problems(row, rule_ids)
        for name, message, fix in bad:
            _problem(manifest, line, name, message, fix)
        if rule:
            rule_ids.add(rule.id)
            manifest.rules.append(rule)
    if data.get("rules") is not None and not isinstance(data.get("rules"), list):
        _problem(manifest, 1, "rule-shape", "rules must be written as [[rules]] tables",
                 "use [[rules]] (double brackets) once per rule")
    return manifest


def rule_problems(row, ids: set[str]) -> tuple[Rule | None, list[tuple[str, str, str]]]:
    """Validate one ``[[rules]]`` row: ``(rule or None, [(rule-name, message, fix)])``."""
    if not isinstance(row, dict):
        return None, [("rule-shape", "a [[rules]] entry is not a table", 'write key = "value" lines under [[rules]]')]
    rid = str(row.get("id") or "").strip()
    bad: list[tuple[str, str, str]] = []
    if not rid:
        bad.append(("rule-id", "rule has no id", 'add id = "no-force-push" under [[rules]]'))
    elif rid in BUILT_IN_RULES:
        bad.append(("rule-id", f"rule id '{rid}' is a built-in rule", "pick another id for your rule"))
    elif rid in ids:
        bad.append(("rule-duplicate", f"rule id '{rid}' is used twice", "give each [[rules]] a unique id"))
    label = f"rule '{rid}'" if rid else "rule"
    how = [k for k in ("match", "regex", "paths") if row.get(k) not in (None, "", [])]
    match, regex, paths = row.get("match"), row.get("regex"), row.get("paths")
    if len(how) != 1:
        bad.append(("rule-match", f"{label} needs exactly one of match, regex, paths (found {len(how)})",
                    'set match = "git push --force*" (command glob), or regex = "...", '
                    'or paths = ["harness/**"] for edits'))
    elif match is not None and not isinstance(match, str):
        bad.append(("rule-match", f"{label}: match must be a string", 'write match = "git push --force*"'))
    elif regex is not None:
        try:
            if not isinstance(regex, str):
                raise re.error("not a string")
            re.compile(regex)
        except re.error as exc:
            bad.append(("rule-regex", f"{label}: regex does not compile ({exc})",
                        "fix the regular expression (Python syntax)"))
    elif paths is not None and not (isinstance(paths, list) and all(isinstance(x, str) and x for x in paths)):
        bad.append(("rule-paths", f"{label}: paths must be a list of globs",
                    'write paths = ["harness/**"]'))
    action = row.get("action")
    if action not in RULE_ACTIONS:
        bad.append(("rule-action", f"{label} has action {action!r}, expected block or warn",
                    'set action = "block" (or "warn")'))
    message, instead = str(row.get("message") or "").strip(), str(row.get("instead") or "").strip()
    if not message:
        bad.append(("rule-message", f"{label} has no message", 'add message = "what is wrong with this action"'))
    if not instead:
        bad.append(("rule-instead", f"{label} has no instead",
                    'add instead = "the command or step the agent should use"'))
    if bad:
        return None, bad
    return Rule(rid, action, message, instead, match if isinstance(match, str) else None,
                regex if isinstance(regex, str) else None,
                list(paths) if isinstance(paths, list) else []), []


def load(folder: str | Path) -> Manifest:
    """Read ``<folder>/tirith.toml``; a missing file gives ``exists=False`` and no problems."""
    path = Path(folder) / FILENAME
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return Manifest(path=path)
    except (OSError, UnicodeDecodeError) as exc:
        manifest = Manifest(path=path, exists=True)
        _problem(manifest, 1, "toml-syntax", f"cannot read {FILENAME} ({exc})",
                 f"make {FILENAME} readable UTF-8 text")
        return manifest
    return parse_text(text, path)


# -- tirith check ---------------------------------------------------------------------------


def check(folder: str | Path, words: list[str], ci: bool = False) -> list[Problem]:
    """Run the ``tirith check`` rules for a project folder; [] = all pass.

    ``ci`` marks a run outside a local clone with hooks (a CI runner): the commit-guard rule
    is not applicable there and is skipped."""
    manifest = load(folder)
    if not manifest.exists:
        return [Problem(str(manifest.path), 1, "manifest-missing", f"{FILENAME} not found",
                        f'create {manifest.path} with [[pipelines]] id = "ci" and build, test '
                        "and deploy [[pipelines.stages]]")]
    problems = list(manifest.problems)
    syntax_broken = any(p.rule == "toml-syntax" for p in problems)
    if not syntax_broken:
        problems += _coverage_problems(manifest, need_deploy=not manifest.deploy_none)
    if not ci:
        problems += _guard_problems(manifest, folder, words)
    return problems


def _coverage_problems(manifest: Manifest, need_deploy: bool = True) -> list[Problem]:
    needed = ("build", "test", "deploy") if need_deploy else ("build", "test")
    best: set[str] = set()
    for pipe in manifest.pipelines:
        kinds = {stage_kind(s) for s in pipe.stages}
        if len(kinds & set(needed)) > len(best):
            best = kinds & set(needed)
    missing = [k for k in needed if k not in best]
    if not missing:
        return []
    fixes = []
    for kind in missing:
        fixes.append(f'[[pipelines.stages]] name = "{kind}" run = "<command>"'
                     if kind != "deploy" else
                     'deploy stage, or put deploy = "none" at the top of the file')
    return [Problem(str(manifest.path), 1, "pipeline-coverage",
                    f"no single pipeline declares {', '.join(missing)} stage"
                    f"{'s' if len(missing) > 1 else ''}",
                    "add to one pipeline: " + "; ".join(fixes))]


def _guard_problems(manifest: Manifest, folder: str | Path, words: list[str]) -> list[Problem]:
    repo = Path(folder)
    problems = verify_guard(repo, words)
    return [Problem(str(manifest.path), 1, "commit-guard", text,
                    f"tirith guard install {repo}") for text in problems]
