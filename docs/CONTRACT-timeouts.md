# Timeouts

Every command Tirith starts has a time limit, from one mechanism in `collector/tirith/timeouts.py`.
Plan item: "every command has a timeout by code (collector, stages and the agents' commands): Tirith learns it
from the history, agents may raise or lower it within limits with a reason, and when it runs out the process is
stopped with its children and its locks are cleaned up."

Why: a cut-off `git commit` once left `.git/index.lock` and blocked everything; manager turns waited minutes on
background commands; a test stage ran far longer than usual and nobody noticed.

## Classes and policy

| class | default | min | max | what |
|---|---|---|---|---|
| `git` | 30 s | 5 s | 300 s | git commands Tirith runs to read a repository |
| `git-commit` | 180 s | 30 s | 900 s | a commit, with the checks that run before it |
| `stage` | 1800 s | 60 s | 7200 s | a build or test stage (`stage:<name>` keeps its own history) |
| `agent-bash` | 120 s | 10 s | 600 s | a shell command an agent runs |
| `agent-turn` | 3600 s | 300 s | 21600 s | one run of an agent in a lane (`tirith lane run`) |
| `muse` | 600 s | 60 s | 3600 s | a request to the model |
| `cargo` | 1800 s | 60 s | 7200 s | a cargo build or test |
| `npm` | 900 s | 60 s | 3600 s | an npm install or script |
| `generic` | 120 s | 5 s | 1800 s | everything else (an unknown class is `generic`) |

The value in force for a class, first match wins:

1. **config**: `[timeouts] <class> = <seconds>` in `config.toml` (the owner; agents cannot change it);
2. **set**: `tirith timeout set` by the owner (no expiry) or by an agent (see below);
3. **learned**: `clamp(p95 of the recorded durations x 3, min, max)` once 5 durations are recorded
   (`stage:<name>` uses its own durations, else the class's);
4. **default**.

## History file

`<home>/timeouts.json`: `durations` (class -> last 60 seconds of commands that finished), `timeouts` (class ->
last 12 `{at, seconds, command, lane?}`), `set` (class -> `{seconds, reason, at, lane?}`). A timeout is not a
duration: it says nothing about how long the class normally takes.

## The runner

`timeouts.run(argv, cls, cwd, env, ...)` replaces `subprocess.run` everywhere in the collector (git calls,
review, merge, plan writer, plan gate, launch, actions, the doctor). It returns a `Result` with `returncode`,
`stdout`, `stderr`, `timed_out`, `seconds`, `error`:

* the command starts in its own process group;
* at the deadline the whole group gets SIGTERM, and SIGKILL after 3 s if anything is left;
* the class cleanup runs: for `git` and `git-commit`, the `index.lock` of the repository (also of a linked
  worktree) is removed **only if** it was created since the command started and no process has it open (`lsof`);
* the duration (or the timeout) is recorded;
* `returncode` is 124 and `error` reads `timed out after N s (class X; raise it with: tirith timeout set --class
  X --seconds N --reason "…")`. A program that cannot start gives 127 with `error`; nothing raises.

`timeouts.supervise(proc, cls, ...)` puts a process that streams its output under the same deadline, kill,
cleanup and record: pipeline stages (`runstage.run_stage`, class `stage:<name>`, exit 124 and the message at the
end of the output; the `end` event has `timed_out: true`) and a headless lane's agent (`agent-turn`, the lane
ends `failed` with the message as its report).

Not covered, on purpose: the agent in an interactive lane (the owner is at the terminal), editor launches and
other fire-and-forget detached starts, and the Windows `safegit` shim (it runs without Tirith).

## Agents

```
tirith timeout get [--class X]
tirith timeout set --class X --seconds N --reason "…"
```

`set` is bounded by the class's min and max; beyond them, with no reason, for an unknown class or for a class
fixed in `config.toml` it is refused with the reason in plain words. Each change is a `decision` flow
(`agent:<lane>` to `guard`, with the reason). A value set by an agent belongs to its lane and stops applying
when the lane ends; a value set outside a lane (the owner) stays.

For the `anthropic` alias the `PreToolUse` hook, for a Bash call in a lane, answers with
`hookSpecificOutput.updatedInput`: the same input with `timeout` (ms) from the policy of the command's class
(`cargo`, `npm`, `git commit` else `agent-bash`), keeping a larger value the agent asked for up to the class
maximum. It carries `permissionDecision: "allow"` (the agent CLI only applies `updatedInput` together with a decision);
the gates have already run and blocked what they block. Background commands and other aliases are left alone;
their commands get the runner's timeout where they run through Tirith.

## Settings and self-check

`state.timeouts`: one row per class `{class, label, words, seconds, source (default|learned|agent|owner|config),
default, min, max, reason?, samples, last}`. Settings > Timeouts shows them in plain words (read-only; the
owner fixes a value in `config.toml`).

Self-check `timeouts`: 3 timeouts of one class within an hour put one manager queue item (kind `selfcheck`) with
the commands and times as evidence; the check is red until the timeouts age out.
