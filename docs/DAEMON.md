# The daemon's modules

The overview each module of `rust/tirith/src` carried in its header, kept here so the code stays short.

## `bots.rs`

Bots: agents of their own, factory roles with `scope = "own"` — the owner's in `~/.tirith/config.toml` (`[roles.bot-<id>]`, extending the manager) and the factory's system bots (`ceo`, `coach`, `judge` with `show = false`, and `manager-general`, the manager of no project), which the page edits the same way, saving the change as a layer in config.toml. Each has a name, a face (shape and
colour) and its own conversation (its key), run like a project's manager with its own room on the phones; the sidebar paints every `own` role with `show = true`. Routines are `[[routes]]` on `schedule`
in that file (their last runs stay in kv as `routine:<id>:<routine>` stamps, not configuration): tasks a bot does by itself at a time of chosen weekdays (the machine's local
time); the lanes scheduler's tick hands a due one to the bot once a day, and the bot makes them itself with `tirith routine add|remove|list`. The page writes config.toml
with `toml_edit`, keeping its comments, and the bots that used to live in kv move there once, at daemon start. A bot learns in its folder (`<tirith home>/manager/<key>`): `memory.md`, which Tirith hands it as it is now at
the start of every turn and routine, and skills in `.agents/skills/<name>/SKILL.md`, its own and the shared ones of `<tirith home>/bots/skills` (linked in before every turn).
Both are git repositories (`bots/versioned.rs`): a bot's folder follows only its `memory.md` and its own skills — its `.gitignore`
says so, and leaves out the links to the shared skills and its working files — and `<tirith home>/bots` (the shared
`memory.md` and `skills/`) is another. Tirith commits, as Tirith, when a turn of the bot's own role in its own
conversation ends having changed them (message «turn <id>», the turn's row id), and after each change from the Library
(«library»); nothing changed, nothing is committed. So the `versions` of a turn (git blob hashes) are blobs of that
history, and `git log` in a bot's folder says which memory and skills each run used.
The bots' agent finds them natively: Muse reads `.agents/skills`, and the agent's own `.<command>/skills` is a link to the same folder. Agents may write in a bot's folder
(never in its hidden folders, where agent settings live, except skills folders) and in the shared skills; the page's Library shows, corrects and deletes all of it.
The shared memory, `<tirith home>/bots/memory.md` (what holds for every bot: the owner's name, time zone, language), is handed before the bot's own, as `[tirith] Shared memory.md as it is now: …`, and the
Library shows it above the bot's memory and edits it (`POST /api/bots/<id>/library` with `memory` and `shared`; `GET` gives it as `shared_memory`). Memory notes are numbered: when it hands a memory over,
Tirith puts `[m<N>]` (the bot's own) or `[s<N>]` (the shared one) before every note line (one that starts with `-` or `*`) that has none yet, the number after the highest already there, and saves
the file so; numbered notes, headings and every other line stay as they are, and the bot cites `[m<N>]` or `[s<N>]` when its answer uses a note. Every skill follows the factory's template,
`factory/skill.md`: its header and seven sections («Cuándo usarla», «Entradas y accesos», «Pasos», «Cómo validar», «Qué devuelve», «Qué requiere aprobación», «Si algo falla»). The bot prompt
gets its text in place of `{skill_template}` (as a role's prompt gets `{name}`); the Library's `GET` gives it as `template`, a new skill starts from it, and every skill is marked with the sections it lacks, by their titles.
A bot may have a model of its own (role `bot-<id>` of `models.rs`, chosen in its card); without one it has the managers'. A routine answer that starts with ∅ (nothing new)
stays on the page and never reaches the phones (`rooms::to_phones`), so only news does.

Routines on an event from outside. A routine is either timed (`schedule`) or waits for its bot's webhook: a `[[routes]]` with `on = "webhook"` and
`when` — the event's `<source>:<kind>` (`github:pull_request.opened`), a pattern of it with `*` (`github:issues.*`), or `*` for every call —
made on the page (Routines: "At a time" or "On an event from outside") or by the bot (`tirith routine hook "<title>" "<when>" "<task>"`); its
task may name `{title}`, `{url}` and `{text}`. The webhook is agentsmessenger.com's sealed webhook (its `src/server/hooks/`):
`POST /api/bots/<id>/webhook` gives the bot's URL, making it once — the daemon makes sure the bot has its own private room (`rooms/hooks.rs`,
even with no phone paired), signs with its room key a `request` event in that room (refs `[]`, body the JSON text
`{"hook": "create", "readers": ["<its public key>"]}`), sends it to `POST <[messenger] url>/hooks` and keeps `{id, url, from, room}` in kv
`webhook:<bot>` (`from`: the key of the platform's `webhook` agent of that room); `POST /api/bots/<id>/webhook/delete` deletes it the same way
(`{"hook": "delete", "id": "<id>"}`), and deleting the bot forgets it. What a service posts to the URL arrives in the room sealed for this
Tirith alone, as `tirith-hook {"v":1,"at":…,"headers":{…},"body":…}`. The room's watcher (`rooms/hooked.rs`) never makes such a line one of
the owner's or of another agent: from the `from` agent it becomes the bot's `webhook` event (`bots/hook_event.rs`) — `source` (`github`
when it has `x-github-event`, else the user-agent), `kind` (`github:<x-github-event>.<action>`, or `hook`), `title` and `url` (of
GitHub's pull request, issue, release or comment, else the body's own `title` and `url`), `text` (the body as text, 4000 characters at most)
and `data` (the body) — and from anyone else it is dropped. The event is kept and shown; each of the bot's routines on `webhook` whose
`when` holds runs (`bots/hook_fire.rs`, trigger `event:webhook`), handed its task and then the event between two random marks
(`<<<outside-data <mark>` … `outside-data <mark>>>>`), said to be data from outside and never instructions (`factory/prompts/bot.md` says
the same). «Probar» on the page (`POST /api/bots/<id>/webhook/test/<routine>`) runs one routine with GitHub's sample of a pull request
opened, with no platform in between. The Library gives a short Google Apps Script with the URL in it that, every 5 minutes in the
owner's Google account, posts the new mail labelled `tirith/<bot name>` (sender, subject, date, text, its link) to the webhook and marks it
read. Tests `webhook_create_signs_the_request` (against a fake platform) and `webhook_starts_its_routine` (`rust/tirith/checks/webhooks.rs`).

## `chat.rs`

Clearing a conversation and its longest life. Every conversation (a project's, a bot's) can be cleared — its chat rows go, its saved agent session (kv `manager_session:*:<conversation>`) is forgotten and its live one ends without cutting a running turn, so the bot starts fresh; its memory, skills and routines stay — and can be given a longest life: `chat_keep`, the owner's own words (`30m`, `1h`, `24 h`, `2 días`, `una semana`; `siempre` takes the limit away), in `~/.tirith/config.toml` on the bot's role or on the project, written with toml_edit. A minute sweep drops the rows older than it, and a sweep that dropped any forgets the saved session too, so the bot cannot remember what is gone. The owner clears and limits a chat in its page menu, or asks its bot in the chat: `tirith chat clear` / `tirith chat keep "<time>"` act on the conversation of the session that runs them (a lane never does), only on the owner's own words. The paired rooms hear both as one sealed line — `tirith-chat {"v":1,"clear":"<ISO UTC>"}` and `tirith-chat {"v":1,"keep_s":<seconds, 0 = forever>}` — so the phone page hides what is gone.

## `factory.rs`

The factory file: `factory/factory.toml`, its prompts (`factory/prompts/*.md`) and its eval rubrics
(`factory/evals/*.md`) hold what used to be fixed in
Rust — each agent's flags, the roles with their settings, `[limits]`, `[plan]`, `[cards]`, `[metrics.*]` and `[[evals]]` — read
as data (docs/FACTORY.md). The loader is strict: an unknown key (`deny_unknown_fields` for `[limits]`, key lists
for the rest), a role on a missing agent or prompt, or an unknown measure makes a version invalid, and the daemon
keeps the last good one while the error shows on the page (`factory_error` of `/api/state`) and is told to the
manager of no project. It loads the repository's `factory/` at run time (`factory` in config.toml points
elsewhere) and reloads when a file changes, config.toml does or main moves (its reflog and ref are part of
`factory::stamp`), with no restart and the embedded copy as the fallback.
`~/.tirith/config.toml` is laid over it key by key and wins; its yesterday's sections are read in memory under
the factory's names (`config.rs`), the file itself untouched except by the page's `toml_edit` writes (the bots'
roles and their schedule routes), and keys nothing reads are an error the owner sees. A role `extends` another (manager-general and the CEO extend
the manager), and an agent the factory does not name runs with the `default` agent's flags. The `[[routes]]` of
the factory say who hears each event the code emits (a closed list in `factory.rs`): which role, as a `turn` now
or a `note` with its next turn, a line in the chat (`chat`), and whether that line and the answer of a turn the
event starts reach the phones (`phone`). A route on a missing event or role is invalid; routes in config.toml
stand next to the factory's. The `[[evals]]` of the factory are judged in the background by their `judge` role
(small text jobs on a cheap, fast model, `access = "read"`, no prompt of its own: each rubric in
`factory/evals/` is its instructions): the event with the conversation before it (and, for a card, the card
itself) goes in, and a `scores` row keeps the label or score, one line why, the event and the judge's stamp;
a share of the events is judged (`sample`, by event id). An eval on a missing event, judge or rubric is
invalid. Nothing of a score reaches the owner: the scores only feed the measures (`satisfaction`,
`needless_cards` and `unapproved` below).

Every version of the factory has a stamp (`Factory::version`): the main commit that last changed `factory/`,
plus a short hash of what config.toml lays over it (`<commit>+<hash>`; `embedded` when no checkout is at hand).
Every lane, chat (turn) row, owner card, event, review and score is born carrying it in its `factory` column,
with `role_hash` next to it: a short hash of the setting of the role it ran with — its resolved table, its
agent's table, its prompt texts and the page's model override for it (`Factory::role_hash`, `App::stamp_for`).
A lane's events and cards carry the lane's stamp; a line no role wrote (the owner's, a room's, Tirith's own)
keeps the version with an empty hash. Comparing two versions, two models or two prompts is then one query
(`GROUP BY factory, role_hash` over `lanes`, `chat`, `owner_items`, `events` or `scores`), with no extra runs.

## `guard.rs`

The commit guard: git hooks in every project that refuse a commit whose message, file names or added lines contain one of the owner's forbidden words (whole word, any case; a word may be a regex). The words come only from
config.toml: the project's `guard_words`, else `[guard] words`; `guard = false` opts a project out.

## `lanes.rs`

Lanes: one agent, one plan item, one worktree slot of its own (`<tirith home>/lanes/<project>/slot-N`) on its own branch; no slot is kept or reused — every lane makes a fresh one.
The autopilot starts a project's open plan items when the machine has room; a lane whose process ends goes to the merge queue, where the checks (never the agent) decide that its item is done. A lane that merges, stops or
fails for good leaves nothing on disk: its uncommitted work is committed on its branch first (a merged lane's branch then goes), its git-ignored files that are no build cache move to the same path in the project (never
overwriting, kept ignored there), and its worktree is removed and pruned; its log, check logs and brief go at the merge, or 24 h after a stop or final failure. A project's shared build cache (`<home>/build/<project>`,
where lanes, stages and every manager, bot and CEO session build through `CARGO_TARGET_DIR`) and its merge folder go after `limits.build_idle_minutes` with no lane, stage or manager of it working; the same pass at
startup applies all of this to whatever already meets the rules.

## `manager.rs`

The managers: one long-lived agent session per project (stream-json in and out), and one for what belongs to no project. A message goes to the session of its project, which starts with its first message and rests after the manager role's quiet
`idle_minutes`; its text, its tool calls and the end of each turn go straight to that project's chat on the page.
Everything else the daemon has to say is an event: `emit_event` keeps its row, shows it on the page and passes it
to the factory's routes. A turn's answer reaches the phones only when the owner started that turn or an event
that started it has `phone = true` in its route.

## `merge.rs`

The merge queue. Up to five lanes of one project ride one train: in the project's merge slot (`<tirith home>/lanes/<project>/merge`) their branches are merged with main, the project's checks and the items' own checks run, and
only a green train moves main (fast-forward), with the plan ticks in the same commits. A red train is split into single lanes; a red single lane is first compared with main, so a broken main never blames a lane. A project's
deploy stages are no checks: they run after main has moved, on that main (`deploy`).

## `metrics.rs`

Health metrics, and the passes of the turn roles (the CEO, the Coach).

Every minute code computes the metrics from the daemon's own data (lanes, runs, plans, kv). The owner's measures
come from the store too: `interruptions` counts, per item merged in 24 h, what reached the owner unasked — phone
messages of turns no owner message started (the events and chat timeline, classified by the factory routes with
`phone = true`), notices whose route has `phone = true`, and cards opened — `needless_cards` is the share of
the cards opened in 24 h and already resolved that were closed without an answer, plus those the `card-needed`
eval judged decidable alone (an answered one too: it bothered the owner needlessly) — and `satisfaction` is,
per day, (approvals − corrections − repeats − noise complaints) / owner messages from the `owner-mood` eval's
scores, judged by the cheap `judge` role from what the owner writes anyway, never by asking the owner (a
question is itself an interruption) — and `unapproved` counts the bots' turns and routines of 24 h that the
`unapproved` eval (on `turn.done` and `routine.done`) labelled `unapproved`: something a skill it read lists under
«Qué requiere aprobación», or a send, purchase, deletion, publication or production change, done without an
answered `approval` card (target 0; red at the first one). Their labels and targets are
`[metrics.interruptions]`, `[metrics.needless_cards]`, `[metrics.satisfaction]` and `[metrics.unapproved]` of the factory; they show with the other metrics on the
page, and a red one is routed as `metric.red` like any other. A metric that
turns red starts an episode: the mechanical fix when there is one, then one note to the manager. Still red
after the delayed `metric.red` route's `after_minutes` (15), one pass runs for that episode: that route's turn
role (the CEO, `access = "read"`) runs one fresh turn like any turn role, and its
answer goes to the CEO's own conversation and, as `review.done`, to the manager of no project. Back to green ends the episode. The
factory's `schedule` route to the CEO also runs it every day at 07:00 with no red measure due — that
when and who are configuration, not a decision in `metrics.rs`, and it is due like a bot's routine (its time
came today and no daily pass ran since; its daily passes are the `ceo_reviews` rows whose `metric` names
`daily`, counted in the same 24 h as its `max_per_day`). Either way its brief (`brief`) carries
the measures, the measures and eval scores grouped by factory version — merged lanes and their first-try
share, the `scores`, the cards and the events per stamp, oldest version first — the last lanes with their
stamp, failing checks, plans and owner questions; the role's prompt (`factory/prompts/ceo.md`) has it
read `factory/` and propose at most one change to it per pass, as an item of step 17 of this repo's plan with
its check and its lane started (`tirith plan add` and `tirith lane start`, which its read access allows). A
factory version whose first 10 merged items (or its first 24 h) went worse than the version before it on
satisfaction, interruptions or first try is reverted by the CEO's next pass, as one more such item;
nothing in the factory needs the owner's yes, and a landed change applies when main moves (the reload above).
While everything is green and the day's pass ran, no model is called.

Running a turn role by its `schedule` route or by an event is the general mechanism, not a second CEO
(`metrics/roles.rs`, `pass.rs`, `review.rs`): every turn role with a prompt runs its pass with the brief its
`brief` key names — `ceo` (the default, the CEO's above) or `bots` (`bots_brief.rs`: why it runs, its own last pass,
and per bot the numbers of `GET /api/bots/<id>/numbers`, its routines, its own skills with the sections of
`factory/skill.md` they lack, the shared skills likewise, and the size of each memory) — within its own
`max_per_day`, one pass of a role at a time, its trigger in `turns` (`schedule`, or `event:<name>`), its answer in
its own conversation and its row in `ceo_reviews` with its `role` (the rows from before that column count as the
CEO's; the page's reviews and the CEO's budget are the CEO's rows only). Its `schedule` route runs it when due (like
the CEO's daily pass, its rows naming `daily`); an event routed to it as a turn with no `after_minutes` runs it at
once (`manager/events.rs` hands it here: a turn role is never handed text in a conversation). Only a `ceo` brief's
answer is told on as `review.done`. The Coach (`coach`, `factory/prompts/coach.md`, `brief = "bots"`) runs weekly
(Mondays 09:00) and on `routine.worse`; until `tirith lab` exists it proposes one experiment in at most 3 lines and
changes nothing, or answers «Sin novedad.». Its `writes = ["bots", "lab"]` confines it (`rules/confined.rs`): its
edits land only in the bots' folders, `<home>/bots` and `<home>/lab`, never in a hidden folder there.

`routine.worse` (`metrics/worse.rs`): every computation looks at each bot's routines with runs in `turns` (the bot's
own role in its own conversation); one whose last 5 runs cost 30 % more than the 10 before (mean tokens, all four
kinds, or mean dollars) or of which 2 failed is emitted once per episode — kv `routine_worse:<bot>:<routine>`, cleared
when it no longer holds — with the bot, the routine and those numbers.

## `models.rs`

Models per role and the fallback chain.

Roles are the factory's: "manager" (a project's chat), "manager-general" (the system bot of no project), "worker" (lanes),
"ceo" (one fresh turn) and "judge" (the evals' cheap turn role, with no prompt of its own, so code never
picks it as a standalone turn role), each with its `session`, `scope`, `extends`, `access`, `idle_minutes`,
`max_per_day` and `runs_on` in `factory/factory.toml`; code finds a role by that shape, never by a fixed
list of names. Each has an agent alias, a model, an effort and a fallback
chain. The effective setting is the owner's override from the page (kv `models:<role>`), else
`[roles.<role>]` of `config.toml` laid over the factory's role. Agents are aliases
(`[agents.<alias>]`); the real command lives only in config.toml. Optional per alias: `model_flag`, `effort_flag` ("" = none) and `resume_args`
(`{session}` and `{message}` are filled in); each alias's flags come from the factory.
Every model knows its agent from the configuration alone: its group in `[models.choices]` (a table
`alias = ["model", …]`), Codex's catalog, or the `model_prefix` its `[agents.<alias>]` declares (by default
its command's program name and `-`). A pair of an agent with another agent's model is refused by
`POST /api/models/<role>`, `tirith models set` and `/model`; a model sent alone brings its agent; a lane on
such a pair fails at once with the reason, without spending a try; and at start a saved one is said in the
general manager's chat. The page lists each model with its agent and switches the agent when a model of
another one is chosen.

## `plan.rs`

The plan: the project's markdown checklist (`## N. Title` steps, `- [ ] item` lines). Its content changes (new item, ticked item) are written to the file at once and committed in batches; which lane works on an item lives in the
daemon, not in the file. The daemon is the only writer. An item may carry its own check: `- [ ] Text (check: `cmd`)`; the merge queue runs it, and ticks a merged item in the merge commit itself.

## `projects.rs`

Adding a project (the page's "+", `tirith project add`): the folder gets a git repository, the commit guard and a
plan with a first step, then a `[[projects]]` block appended to config.toml (its existing text is never rewritten).

## `rooms.rs`

agentsmessenger.com rooms: public rooms Tirith watches (`[[rooms]]` in config.toml), and private `sealed-` rooms where the owner talks to the managers from a phone, end-to-end encrypted, compatible with `am.mjs` and
the agentsmessenger.com chat page. One task per room long-polls it.

Who gives orders: only a phone paired with a one-use QR code (`tirith-pair <code>`) is the owner, in every room made
here (each project gets a room, each room gets every phone). Other agents reach the manager as information. After 30
agent messages in a row with nothing from a paired phone, Tirith stops delivering and says so once, until a phone writes.
A `tirith-hook` line is never a line for people: only the one sealed by the room's webhook agent (kv `webhook:<bot>`)
is handed on, as its bot's `webhook` event (see Routines on an event from outside, above).

## `rooms/sealed.rs`

agentsmessenger.com's protocol, byte-compatible with its `am.mjs` and chat page: ed25519 identities (`{name, secretKey: hex(seed||pub)}`), canonical JSON, signed events, and the sealed envelope (x25519-xsalsa20-poly1305 with the
ed25519 keys converted to curve25519, one wrapped key per reader).

## `sandbox.rs`

Network sandbox. Every process Tirith starts (lane agents, the manager, turn-role reviews, merge-queue checks) runs inside a macOS `sandbox-exec` profile that denies outbound network except loopback and local sockets and denies Apple Events (macOS refuses them from `sandbox-exec` anyway, whatever the profile says), with its proxy
variables pointing at the daemon's own HTTP/CONNECT proxy (`sandbox/proxy.rs`). The proxy authenticates the caller (`who` + token), applies the block list, and records each new destination as an event.

The owner's Safari is read through the daemon instead: `tirith safari open <url>` and `tirith safari read [<url>]` (URL, title and text of the current tab, or of the first tab whose URL starts with the one given; the text is cut at 20,000 characters) ask the daemon — which runs outside the sandbox — to run that one fixed AppleScript, never a script the agent sends, and only for the chat roles (the managers and the bots, never a lane or the CEO).

## `sandbox/proxy.rs`

The daemon's HTTP proxy for sandboxed processes: `CONNECT host:port` and plain-HTTP absolute-form requests. It authenticates the caller, refuses blocked destinations (by name, then by the addresses they resolve to), records each
new destination of each caller, and pipes the bytes.

## `turns/`

The turn log. Every turn of a chat or turn role (the project managers, the bots, the CEO, the judge) leaves one row in
`turns` (`assets/schema.sql`; `runs` is the pipelines'): `conv` (its conversation), `role`, `trigger` (`owner`,
`room`, `schedule`, `run-now` — the page's Run now — or `event:<name>`; whoever hands the message over sets it, and
the first one counts until the turn starts), `routine` (the id of the routine that started it), `started_at`,
`ended_at`, `ms`, `status` (`ok`, `failed`, or `waiting` when an approval card it opened is still open at its end: an `approval` card of its conversation opened since the turn started, `turns/approval.rs`), `agent`, `model`, `effort`, `tokens_in`, `tokens_out`,
`tokens_cache_read`, `tokens_cache_write`, `cost_usd`, `steps` (tool calls), `skills` (JSON: the skills it read —
Anthropic's `Skill` tool, Muse's `read_skill`, or any tool whose input names `skills/<name>/SKILL.md`), `tools` (JSON:
each call's name and the short line the chat shows, 50 at most), `versions` (JSON: the git blob hash, as
`git hash-object` gives it, of its `memory.md`, of `<home>/bots/memory.md` when it exists and of each skill's
`SKILL.md`), `answer` (cut at 4000 characters), `event` (its event's id) and its stamp (`factory`, `role_hash`).

Tokens and cost come from what each agent already prints: Anthropic's `result` line (`usage.input_tokens`,
`output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `total_cost_usd`, `is_error`); Muse's
`runtime.session` records whose `payload.event.kind` is `model_completed` (`payload.event.usage`: `input_tokens`,
`output_tokens`, `cache_read_tokens`, `cache_write_tokens`), summed over the turn, and when its output carries none,
from its session's `session.jsonl` under `~/.local/share/muse/sessions/<year>/<month>/<day>/<id>/` (the records since
the turn started); Codex's `turn.completed` (`usage.input_tokens`, `cached_input_tokens`, `output_tokens`). When the
agent gives no cost, `[prices."<model id>"]` in `~/.tirith/config.toml` does (`input`, `output` and `cached`, dollars
per million tokens; the strict loader accepts the section and none of it goes to `factory/`); without a price the
cost stays empty, never invented.

When a bot's own turn ends (a role with `scope = "own"` and `session = "chat"`; a judge's turn in its conversation emits nothing, or an eval on `turn.done` would judge itself forever) the event `turn.done` is emitted — or
`routine.done` when a routine started it — saying the bot, the trigger, the routine and its task, the tools, the
skills and the beginning of the answer; neither has a default route. For the `unapproved` eval it also carries each
tool call with its summary and, for each skill it read (its own, then the shared one), the text of its «Qué requiere
aprobación» section. `GET /api/turns?conv=<key>&limit=<n>` gives the
rows, the last first. The manager's sessions, exec turns and loose turns, and the bots' routines, only call this
module: `trigger` when a message is handed over, `begin`/`handed`/`line`/`end`/`abandon` around a conversation's
turn, `turn_of`/`done` around a turn role's.

Each row also carries its quality signals (`react.rs`, `signals.rs`, `checks.rs`):

- `reaction`: the owner's 👍 (1) or 👎 (-1). `POST /api/turns/<id>/reaction` with `{"value": 1, -1 or 0}` keeps it (0
  takes it back: empty). In a bot's conversation an owner message that is only 👍 or 👎 (any skin tone) or `+1` / `-1`,
  written on the page (`manager::say`) or from a paired phone (`from_room`), is the reaction to the bot's last kept
  turn: it stays as a chat row, starts no turn, and has no `owner.message` event, so owner-mood does not judge it.
- `corrected`: 1 when the `owner-mood` eval labels an owner message `correction` or `repeat`; it marks the
  conversation's last turn (of its own role, not the judge's) that ended before that message, taken when the message
  came, before the judge runs.
- `checks_passed`, `checks_total`, `checks` (the names, as written): the bot's own validation. `tirith turn checks
  <passed>/<total> "<names>"`, run by the bot in its session (`TIRITH_PROJECT` is its conversation; a lane is refused),
  posts to `POST /api/turns/checks`; the open turn of that conversation keeps them until its row is written, else its
  last kept turn gets them. `factory/prompts/bot.md` asks for it at the end of a routine whose skill has «Cómo validar»,
  with what it really checked.
- `cited` (JSON): the numbered notes, `[m<N>]` of its own memory and `[s<N>]` of the shared one, that its answer
  names, each once in order, read when its row is written.

- `score`: the judge's note from 0 to 1 on a routine's run. The `turn-quality` eval (`[[evals]]` on `routine.done`,
  rubric `factory/evals/turn-quality.md`) reads the routine's task, the answer and the «Cómo validar» section of each
  skill the run read — `routine.done`'s text carries them (`quality.rs`), cut to 2000 characters — and answers a score
  with one line why; the score is copied to the turn whose `event` is that event (`turns::scored`, or `catch_up` when
  the judge answered before the row was written). A judge's turn kept in a bot's conversation emits no event of its own.

`GET /api/turns` gives these columns too; older databases get them by `ALTER TABLE` at start (`add_columns`).

## `numbers/`

A bot's numbers, read from its own rows in `turns` (its role in its conversation; the judge's turns there do not
count) and its memory files: `GET /api/bots/<id>/numbers` gives

- `routines`: per routine of its card (then routines it no longer has, then `Chat` for its turns without one): `runs`,
  `success` (share ok), `ms` (mean), `cost_per_good` (the cost of all its runs over the ones that ended ok),
  `down_per_10` (👎) and `corrections_per_10`, `score` (the mean judge's score, `scored` how many) and `trend`: the last
  10 runs against the 10 before them (`last` and `before`, each with `cost_per_good`, `ms`, `success`), null until there
  are more than 10.
- `skills`: per skill its runs read: `used`, `validated` (runs whose `checks_passed = checks_total`), and the mean tokens
  (in, out and cache) of its routine's runs (the routine of its first use) before that first use against after:
  `tokens_before`, `tokens_after`, `saved_percent`.
- `memory`: `tokens` (`own` and `shared`: characters / 4 of its memory.md and of `bots/memory.md`), `notes` (every
  numbered note with how many of its last 30 runs cited it), `uncited` (none cited them: candidates to delete, which
  Tirith never deletes), `misses` and `missed` (owner messages in its conversation that owner-mood labelled `repeat`).

The page shows them in the bot's «Números» tab (`web/js/views/bots/numbers.js`) in short lines, and every answer of a
bot in its chat carries a discreet 👍 and 👎 (`web/js/views/chat/react.js`) that calls `POST /api/turns/<id>/reaction`
(the same one again takes it back) and stays marked; the answer's turn is found by its words and its time.

## `teach/`

Teaching a bot a task (docs/BOTS.md, part 5). The owner does the task once in their own Edge with Tirith's
extension, `tools/teach/` (Manifest V3, installed with «Load unpacked»; its README says how in two steps): pages (URL
and title), clicks (the element's text, role and a short selector) and typing (the field's label and value), each
with its time. Passwords, one-time codes and cards are masked as `•••` (`type=password`, `autocomplete`
`one-time-code` or `cc-*`, fields whose name or label looks like card, cvv, cvc, otp, pin, tarjeta or código, and
any step marked `sensitive`), first by the extension and again by the daemon; a recording stops by itself after 10
minutes. The recording lives in kv `teach:<bot>` (`on`, `since`, `steps`), the bot's token in `teach-token:<bot>`.

- `GET /api/teach/<id>`: the recording; to the page also the token (the bot's tab «Enseñar» shows the link
  `<daemon>/api/teach/<id>#<token>`, pasted once in the extension), never to the extension.
- `POST /api/teach/<id>/start` and `/stop`: from the page, or from the extension with the token.
- `POST /api/teach/<id>/events`: one step or `{"steps": [...]}`, kept only while recording; it needs
  `Authorization: Bearer <token>` from anyone (401 without it or with another).

The same-origin guard lets the extension's origin (`chrome-extension://bogleilobfephnbnjibgoofoeipkhfef`, fixed by
the `key` in its manifest) reach `/api/teach/` only, and CORS answers that origin only (preflight included). While a
recording is on, what the owner writes in the bot's chat (`manager::say`) is kept with the steps, by time, and starts
no turn. At stop the bot is handed, in one turn (trigger `teach`), the steps with the owner's words under the step
they follow and the skill template (`factory/skill.md`): it writes the skill with `draft: true` in its header and
asks 3 to 5 questions in its chat (decision rules, failures, approvals). No bot uses a draft until the owner
activates it in the Library (factory/prompts/bot.md says so; «Activate» takes the line out).
