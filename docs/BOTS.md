# Bots that learn

The owner's plan of 2026-11-21: measure every run of every bot, improve the bots only with proof, give skills a
fixed shape, ask before acting outside, share what every bot should know, let outside events start routines, and
let the owner teach a task by doing it once. Tirith already stamps every lane, event and score with the factory
version and the role's hash; this extends that to the bots' runs, their memory and their skills.

Each part below is one or more plan items (the last section gives the order). The factory rules hold: behaviour
lives in `factory/` (roles, routes, measures, evals, prompts), Rust keeps the mechanisms, tests stay fast and call
no model, and nothing new reaches the owner unasked except what a part says it does, in one line.

## 0. Small files

Every code file that is not a test has 100 lines or less (the owner's decision of 2026-11-21): the daemon's Rust as
`cargo fmt` writes it with rustfmt's default width, and the page's JavaScript and CSS, the app, the tools and the
site the same. Tests (`rust/tirith/checks/`, `*.test.*`, `scripts/checks/`, `tools/perf/`), `docs/` and vendored
code are not counted. `tools/size.sh` is the check; the merge queue runs main's copy of it (`tirith.toml`), so a
branch cannot loosen the rule for itself, and CI runs it too. The old limit on the whole daemon (6000 lines) and the
240-character lines that let it fit are gone: they made the code dense, not small.

How a file is split: by responsibility. A module becomes a folder (`manager/mod.rs` with `talk.rs`, `deliver.rs`,
`events.rs`, ...), each file one job; a function longer than a file is cut into named steps; one large type becomes
small ones composed (a struct that holds the parts it needs, a trait only where there are two implementations),
never one type that knows everything. `mod.rs` re-exports what callers use (`pub use`), so a split changes no
caller outside its folder and no test beyond its imports. A split changes no behaviour: the tests are the proof.

Order: the rule (this branch, which adds no Rust), then rustfmt's defaults applied to the whole crate with no other
change, then one lane per group of files below, in parallel, each touching only its own files (not `tirith.toml`,
`tools/size.sh` or `docs/DAEMON.md`), then one item that empties the list of files still waiting in `tools/size.sh`
and brings `docs/DAEMON.md` up to date with the new folders.

| group | files |
|---|---|
| lanes | `lanes.rs` |
| rooms | `rooms.rs`, `rooms/sealed.rs` |
| manager | `manager.rs` |
| merge | `merge.rs` |
| models | `models.rs` |
| metrics and bots | `metrics.rs`, `bots.rs` |
| sandbox | `sandbox.rs`, `sandbox/proxy.rs` |
| factory and config | `factory.rs`, `config.rs` |
| app, server and main | `app.rs`, `server.rs`, `main.rs` |
| projects, plan and chat | `projects.rs`, `plan.rs`, `chat.rs` |
| agents and rules | `codex.rs`, `rules.rs`, `guard.rs`, `attach.rs` |
| page: chat and bots | `web/js/views/chat.js`, `web/js/views/bots.js`, `web/js/views/bots/face.js` |
| page: the rest | every other `web/js` file over 100 lines |
| styles and the rest | `web/css/*.css`, `site/style.css`, `app/src-tauri/icons/source/glow3d/build.py` |

A page split keeps every view working: `node --check` on each file, `node tools/perf/phone-ui.mjs`, and the page
loaded in a headless browser with no error in its console. A CSS split keeps the page looking the same (a zoomed
screenshot compared with the one before, as for any visual change).

## 1. The run register

Every turn of every chat and turn role (the project managers, the bots, the CEO, the judge) leaves one row in a new
table `turns` (`runs` is the pipelines'):

- **What:** `conv` (its conversation: a project id or a bot's key), `role`, `trigger` (`owner`, `room`, `schedule`,
  `run-now`, `event:<name>`, `webhook`), `routine` (its id when a routine started it), `status` (`ok`, `failed`, or
  `waiting` when an approval card it opened is still open at its end), the skills it read, and the tools it called
  (name and the short summary the chat shows, at most 50).
- **Cost:** `agent`, `model`, `effort`; tokens in, out, cache read and cache written; cost in $; wall time in ms; and
  steps (tool calls). The anthropic agent's `result` line gives `usage`, `total_cost_usd`, `duration_ms` and
  `num_turns`. Muse's exec stream gives `runtime.session` records whose event kind is `model_completed`, each with
  `usage` (`input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_write_tokens`): summed over the turn, and
  read from its session folder's `session.jsonl` when the stream lacks them. Codex's `turn.completed` gives `usage`.
  When an agent gives no cost, `[prices."<model id>"]` in config.toml (dollars per million tokens: `input`,
  `output`, `cached`) gives it; without a price the cost stays empty, never guessed.
- **Versions:** the git blob hash (`git hash-object`) of `memory.md`, of the shared `memory.md` and of each
  `SKILL.md` it read, as they were when it read them: the exact versions every result came from.
- **Quality:** `reaction` (+1 or −1: the owner's 👍/👎 on its answer, on the page or as a message that is only 👍
  or 👎 on the phone, which starts no turn), `corrected` (the owner's next message in that conversation was judged
  `correction` or `repeat` by the `owner-mood` eval), `checks_passed`/`checks_total` (its own validation: the skill
  says how to check its result, and the bot runs `tirith turn checks <passed>/<total> "<names>"`), `score` (the
  judge's grade of the answer, eval `turn-quality` on the new event `turn.done`, with the skill's «Cómo validar» as
  its rubric; `sample` in the factory keeps it cheap), `cited` (the memory notes the answer cites, below).
- **Stamps:** `factory` and `role_hash`, as every other table.

Memory notes are numbered: each note of a `memory.md` is one line `- [m<N>] …`, and an answer that used a note
cites it as `[m<N>]` (the bot prompt says so; Tirith numbers unnumbered lines when it hands the memory over).

What the bot's screen shows, in a tab «Números» (`GET /api/bots/<id>/numbers`):

- **Per routine:** cost per good run (total cost over `ok` runs), mean time, success rate, 👎 and corrections per 10
  runs, and the trend (the last 10 runs against the 10 before).
- **Per skill:** how often it is used, how often its validation passes, and the tokens it saves: the mean tokens of
  that routine's runs before the skill's first use against after.
- **Memory:** what loading it costs per turn (characters / 4, in tokens), how often each note is cited, the notes no
  run cited in the last 30 (candidates to delete, never deleted by Tirith), and memory failures: the owner had to
  correct the same thing twice (`repeat`).

## 2. Skills, approvals and shared memory

- **The template** (`factory/skill.md`): «Cuándo usarla», «Entradas y accesos», «Pasos», «Cómo validar», «Qué
  devuelve» (format and length), «Qué requiere aprobación», «Si algo falla». The Library's new skill starts from it,
  and it marks the sections a skill lacks; the bot prompt tells the bots to write skills with it.
- **Approvals:** a new card kind `approval` (`[cards] kinds`). Before sending, buying, deleting, publishing or
  touching production — and before anything its skill lists under «Qué requiere aprobación» — a bot opens
  `tirith ask-owner --kind approval "<exactly what, to whom>"` and ends its turn; the owner's answer starts its next
  turn, as every card answer does (bots never wait inside a turn). An agent cannot answer it: `tirith answer` takes
  only the owner's own words written after the card, and the daemon knows a caller from inside the sandbox whatever
  it sends. The eval `unapproved` on `turn.done` reads the
  turn's tools and the skill's approval section and labels it `ok` or `unapproved`; the measure `unapproved` (24 h,
  target 0) is routed like any other.
- **Shared memory:** `<tirith home>/bots/memory.md`, what holds for every bot (the owner's name, time zone,
  language). It is handed before each bot's own memory, numbered the same way (`[s<N>]`), and edited in the Library.

## 3. The Coach and the lab

- **Versions in git.** Each bot's folder is a git repository that tracks only `memory.md` and its own skills
  (`.agents/skills`, not the linked shared ones); the shared skills and memory are a repository of their own. Tirith
  commits after every turn that changed them and after every Library edit: the history is the versions.
- **The Coach** is a system bot of the factory (`[roles.coach]`, name Coach, its own face, `session = "turn"`,
  `scope = "own"`, prompt `factory/prompts/coach.md`), routed on a weekly `schedule` (Monday 09:00) and on the new
  event `routine.worse`: a routine's last 5 runs cost 30 % more than the 10 before (tokens or $), or 2 of its last
  5 failed, once per episode. It reads the register and the bot's folder and proposes **one change per experiment**
  with its hypothesis («en gmail-safari, buscar con un filtro en vez de abrir cada correo: −40 % tokens con el mismo
  resultado»; «esta rutina con esfuerzo medio»; «borrar 12 notas que nadie cita»): a branch of the bot's repository
  with one commit, started with `tirith lab start <bot> <branch> --routine <id> --mode replay|live|shadow
  --hypothesis "<text>"`. It writes only in the bots' folders and the lab's worktrees (the rules hold it there).
- **The rule is fixed before the test, by the factory, not by the Coach:** `[lab]` in `factory.toml` —
  `inputs = 8`, `not_worse_share = 0.5`, `min_saving = 0.15`, `max_days = 7`. The variant is adopted when, over
  its comparisons, it is not worse (wins and ties at least half, and wins at least as many as it loses), it has no
  new failure and no 👎, and it saves at least 15 % of cost or of time (in most pairs too: one slow answer, or one tool call too many, moves a mean). Otherwise it is discarded. Tirith decides
  mechanically and merges the branch (adopted) or deletes it (discarded).
- **Three ways to test,** best first:
  1. **Replay.** Every bot turn keeps what it read — the tool results as its agent saw them (the anthropic agent's
     `tool_result` blocks, Muse's `tool.result` texts) and the event that started it — up to 256 KB, for the last 10
     runs of each routine, in `<tirith home>/captures/<turn>.jsonl`. A replay runs the routine on the same last
     `inputs` captures twice, the bot repository's main (A) and the branch (B), each in a throwaway worktree, with
     the network closed to all but the model of the bot's agent (`model_hosts`) and every tool call that reads the
     outside world answered from the capture by Tirith's hooks (a call the capture lacks is refused: «not available
     in a replay»). It needs at least 5 such captures, and two failed runs are no tie. Nothing of it reaches the owner.
  2. **Live.** The bot's real runs of that routine alternate A and B (B in the branch's worktree), for when inputs
     cannot be replayed; each run starts a session of its own and is handed the memory and skills of its side; its answers reach the owner as always. An adopted skill is used from the next run: a changed skill is read again, in a new session.
  3. **Shadow.** B runs beside each real run of A, in its worktree, with live reads and every outward action
     refused (no cards, no `tirith say`, no phones, no `safari open`). It costs double while it lasts.
  Both sides always start alike: a replay warms the cache with an uncounted turn and alternates which side
  goes first; live and shadow runs resume no session.
- **Comparison:** the judge reads the input and the two answers in random order (eval `ab-compare`) and says `a`,
  `b` or `tie`; Tirith maps it back. Two answers of the same text are a tie, without the judge. Tokens, cost and
  time come from the `turns` rows of both.
- **What the owner gets:** one line in the bot's chat, on the phones only for an adoption: «Adopté gmail-safari v3:
  −38 % tokens, misma calidad (8/8)» (events `lab.adopted`, phone true, and `lab.discarded`, phone false). The
  bot's tab «Mejoras» lists every experiment: change, hypothesis, mode, comparisons, the numbers, the decision.

## 4. Routines on outside events

- **The webhook:** each bot gets a secret URL `https://agentsmessenger.com/hook/<secret>` (the platform is already
  on the internet with HTTPS). The platform seals what arrives for the bot's room (Tirith's key) at once and keeps
  only the sealed copy, which it cannot open when it seals once (a key made for that call and thrown away; a platform
  from before that ignores the request and seals with its webhook agent's key, which it holds); Tirith's room watcher turns it into the event `webhook` of that bot, with `source`, `kind`,
  `title`, `url`, `text` and its data (GitHub's `X-GitHub-Event` and `action` give `github:pull_request.opened`).
- **The routine:** a routine may say `on = "webhook"` and `when = "github:pull_request.opened"`; its task may use
  `{title}`, `{url}` and `{text}`. A «Probar» button hands it a sample event without the platform.
- **E-mail:** the Library gives the owner a short Google Apps Script with the bot's URL in it: every 5 minutes, in
  the owner's own Google account, it sends the new mail with the label `tirith/<bot>` to the webhook. No mail
  server, and no bot opening the owner's Safari every 5 minutes.
- **Safety:** what comes from outside is data, never instructions (the bot is handed it fenced as data), and
  approvals still hold.

## 5. Teach it a task

- **Recording:** a small Tirith extension for the owner's Edge (`tools/teach/`, Manifest V3), with the owner's own
  sessions. It records pages (URL, title), clicks (the element's text, role and a short selector) and typing (the
  field's label and value), masks only passwords, one-time codes and cards (`type=password`, `autocomplete`
  `one-time-code` and `cc-*`, fields named like card, cvv, otp, pin), and stops after 10 minutes at most. It sends
  to the daemon with the token the bot's tab «Enseñar» shows.
- **The owner's voice:** what the owner writes in the bot's chat while recording is aligned by time with each step.
- **The draft:** at stop, the bot is handed the steps with the owner's words and writes the skill with the
  template, then asks 3–5 questions in its chat about what one example cannot show: decision rules, failures,
  approvals.
- **The rehearsal:** the bot runs the draft in the owner's Edge through the same extension (open, click, type), one
  screenshot per step, and stops before any step that needs approval. When the owner approves, the skill goes live
  and, if the owner wants, its routine is made.

## Order

0. Small files (above): the rule, rustfmt's defaults, the splits in parallel, the list emptied.
1. The run register: the `turns` rows with cost, versions and the `turn.done` event.
2. The quality signals (reactions, corrections, own checks, `turn-quality`, cited notes) and the numbers API.
3. The tab «Números» and the 👍/👎 on the page.
4. The skill template and the shared memory.
5. Approvals and the `unapproved` measure.
6. Versions in git, the Coach, `routine.worse`, experiments in live mode, the rule, «Mejoras».
7. Captures and replay, with `ab-compare`.
8. Shadow mode.
9. The webhook on agentsmessenger (its own plan).
10. Webhook routines, «Probar» and the e-mail script.
11. Teaching: the extension, the recording, the draft and the questions.
12. The rehearsal and going live.
