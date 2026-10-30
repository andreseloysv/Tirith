# Rust port

The collector and the `tirith` CLI are moving from Python (`collector/tirith/`) to Rust. The Python collector
stays live until the Rust one reaches parity; the end state is the desktop app embedding the collector (no
uv/Python) plus one standalone `tirith` binary for agents.

## Layout

```
Cargo.toml                    workspace (members below); one Cargo.lock and one target/ for everything
rust/tirith-core/             library: everything shared
  src/home.rs                 TIRITH_HOME, config.toml -> Config            (home.py)
  src/store.rs                json / jsonl stores, atomic writes, lanes     (store.py)
  src/lock.rs                 collector.lock advisory lock                  (collectlock.py)
  src/timeutil.rs             ISO helpers                                   (home.now_iso / parse_iso)
  src/pytext.rs               Python text semantics (splitlines, strip, truthiness)
  src/manifest.rs             tirith.toml: roadmap / deadline / plans       (manifest.py, partial)
  src/roadmap.rs              checklist parser, state.roadmap, state.plans  (roadmap.py)
  src/projects.rs             registered + auto projects, matching          (projects.py)
  src/state.rs                serde types of state.json
  src/collect.rs              Collector: build_state / cycle / run_once     (collector.py)
rust/tirith-cli/              binary `tirith-rs` (never shadows the live Python `tirith`)
app/src-tauri/                the desktop app, a workspace member that uses tirith-core
scripts/parity/               the parity harness (Python)
```

Build output goes to the shared `CARGO_TARGET_DIR` the stages set; without it, to `target/` at the root.

## Stages

Run from the worktree: `tirith run --project tirith --stage <name>`.

| Stage | What it does |
|---|---|
| `build` | Python compile, `node --check` of the web scripts (the port crates no longer build here) |
| `test` | the Python suite (`pytest`) |
| `rust-test` | `cargo test -p tirith-app` (the desktop app only; the port is paused) |
| `rust-port` pipeline | on demand, not gating: `tirith run --project tirith --stage port-build` / `port-test` build and test `tirith-core` and `tirith-cli`; back in the gate when the port resumes |
| `parity` | builds `tirith-rs`, then runs the parity harness |

## How parity works

`scripts/parity/run.py` takes each fixture home from `scripts/parity/fixtures.py`, copies it twice, runs
`python -m tirith collect --once` on one copy and `tirith-rs collect --once` on the other, and compares the two
`state.json` files with `scripts/parity/compare.py`:

- Fields that may differ between runs are listed once, in `compare.VOLATILE_KEYS` (today: `generated_at`).
- A section key missing from the Rust state (top level, or a key of a `projects[]` row) prints as
  `not ported yet: <section>` and is **not** a failure.
- Everything the Rust state has must equal Python's. Differences print as `path: python X != rust Y`
  (or `missing in rust` / `only in rust` below section level) and fail the run (exit 1). A collector that
  crashes or a missing binary exits 2.

To port a section: emit it from `collect.rs` with the same shape, add (or extend) fixtures that exercise it,
run `--stage parity`. `--show-not-ported` lists what is left; `--only NAME` runs one fixture; `--keep` keeps
the temporary homes for inspection. Fixtures are synthesized (the way the Python tests build theirs) and
reference project folders by absolute path, so both collectors read the same files. The harness only touches
temporary folders and always passes `--home`.

Comparison is on parsed JSON, so key order and `ensure_ascii` escaping do not matter. Python rows that depend
on the clock (`date` tags) use the local day on both sides.

## State sections

Status of `state.json` (owner = the lane that ports it; blank until assigned).

| Section | Python module(s) | Status | Owner |
|---|---|---|---|
| `version`, `generated_at` | collector | ported | |
| `projects[]` id, name, color, folder, auto | home, projects | ported | |
| `projects[].roadmap`, `.plans` | roadmap, manifest | ported | |
| `projects[].guard`, `.setup`, `.env` | guardwrap, setup, projenv, gates | not ported | |
| `projects[].architecture`, `.counts`, `.usage` | collector, brain, agentrun | not ported | |
| `agents`, `projects[].usage` | agentrun, livesteps, logview, review, lanestop | not ported | |
| `pipelines` | runstage, manifest, hooks (git hooks), github | not ported | |
| `waiting_on_you`, `answered_waits` | asks, ownergate, actions | not ported | |
| `manager`, `steward`, `manager_queue`, `steward_queue` | managerd, dispatch, asks | not ported | |
| `inbox`, `says` | inbox, actions, filerefs | not ported | |
| `flows`, `sessions`, `loose_events` | flows, projects (sessions), hooks | not ported | |
| `violations`, `rules` | hooks, manifest (rules) | not ported | |
| `rooms` | rooms, brain | not ported | |
| `autopilot`, `launch`, `disk`, `health` | autopilot, launch, diskdiet, selfcheck, fallback | not ported | |
| `models`, `brain` | models, brain | not ported | |
| `file_refs` | filerefs | not ported | |

## Python modules

`ported` = in `tirith-core` with parity coverage; `partial` = only what the ported sections need.

| Module | Status | Owner |
|---|---|---|
| home, store, collectlock | ported | |
| roadmap | partial (parse/build; `tick`, `edit`, `owner_parts` left) | |
| projects | partial (discovery; config editing and sessions left) | |
| manifest | partial (roadmap fields; pipelines, hooks, rules, `check` left) | |
| collector | partial (`projects[]`) | |
| cli | not ported (`tirith-rs` has only `collect --once`) | |
| actions, activity, agentrun, asks, autopilot, autoreview | not ported | |
| brain, dispatch, diskdiet, fallback, filerefs, flows | not ported | |
| gates, github, guardwrap, hooks, inbox, lanestop, launch | not ported | |
| livesteps, logview, managerd, models, names, ownergate | not ported | |
| planlane, projenv, review, rooms, runstage, safegit | not ported | |
| selfcheck, setup, tirithcmd, verifylane | not ported | |

## Conventions

- **Errors.** Loading input never fails the cycle: a missing or broken file gives the same default Python
  gives (`Option`/empty value), as the Python code does with `try/except`. Writes return `io::Result`;
  the CLI prints one line to stderr and exits non-zero. No `unwrap` on data from disk; `expect` only on
  static invariants (compiled regexes).
- **Writes.** Every file under TIRITH_HOME is written with `store::write_text_atomic` /
  `write_json_atomic` (temp file in the same folder, then rename). Append-only logs use `append_jsonl`.
  One collector per home: hold `lock::hold_collector_lock` for a continuous run.
- **Same behaviour first.** Port the Python behaviour exactly (including its quirks) and let the parity
  harness prove it; improvements come after the section is at parity. Use `pytext` for Python's
  `splitlines`/`strip`/truthiness so edge cases line up.
- **Dependencies.** Prefer std. A new crate needs a reason in the commit message. Current: `serde`,
  `serde_json`, `toml` (`preserve_order`, config order matters), `regex`, `sha1`, `chrono`. No third-party
  code is copied into the repo.
- **No network, no servers, no ports, no OS notifications** in core or the CLI. Subprocesses are limited to
  `git` (and, in later lanes, the commands the Python code already runs).
- **State types.** A section gets serde types in `state.rs` once ported; unported sections stay absent from
  the Rust state rather than being faked.
- **Tests.** Unit tests live next to the code (`#[cfg(test)]`), use `tempfile`, never the real
  `~/.tirith`. Parity fixtures cover behaviour that must match Python byte for byte.
- **Python stays untouched** while it is live: do not change its behaviour to ease the port.
