# The daemon's modules

What each folder of `rust/tirith/src` does, and one line per file. The code carries no longer overview of its own;
this is it. Every file has 100 lines or less (`tools/size.sh`, docs/BOTS.md part 0); a folder's `mod.rs` only
declares its files and re-exports what the other modules use.

## `access/`

The connectors on the page's Access view (`web/js/views/access-connectors.js`), beside the network destinations, and
the owner's alone (`/api/connectors/` is refused to every agent in `server/scope.rs` and again by the routes' own
layer). `GET /api/connectors/calls` gives the gateway's `connector_calls` newest first (300 at most), each as `at`,
`bot`, `call` (`connector.tool`) and `verdict`, narrowed by `?bot=<id>` and `?connector=<id>`; the page refetches it on
the `connectors` feed message every recorded call sends. The proxy rows of a remote connector's call are the bot's too:
its run's `<role>@bot-<id>` is named `bot-<id>` in the `net` events and the destinations (`sandbox/owner.rs`), never
the manager role it runs on nor the daemon.

`GET /api/connectors/blocks` gives each connector with `blocked` (for all bots) and its `bots` (`{bot, blocked}`: each
granted one, and each one blocked alone). `POST` blocks and `DELETE` unblocks, with `{connector, bot}`, `bot` empty for
all bots. Blocking one bot takes exactly its `[roles.bot-<id>.connectors.<connector>]` table out of config.toml and
keeps it in the store (`kv` `connector-block:<bot>:<connector>`: its text, comments included, and its place in the
file); its unblock lays that table back where it was (every table after it moves one place down), so the file reads
as before, byte for byte when nothing else changed; a bot that is gone has nothing to get back and is forgotten.
Blocking all bots sets `blocked = true` on the `[[connectors]]` row; its unblock takes that key off. Every change goes
through the config writer (`projects::rewrite`, toml_edit, every comment kept), which reloads the config, so the next
call of a gateway already open is refused (`refused by block`, the one-bot block too, not `refused by scope`) with no
daemon restart; each is a `connector-block` event. Tests `connector_calls_show_the_bot` and
`blocked_connector_refuses_next_call` (`rust/tirith/checks/connector_access.rs`) against the gateway checks' fake MCP
servers: the rows, their filters and the proxy's bot rows; both scopes, the next call refused and running again, the
exact file restored, comments kept.

Files:

- `block.rs`: The edits of a block in config.toml's text: one bot's grant taken out (with what and where it was) or laid back, `blocked` set or taken off a `[[connectors]]` row.
- `calls.rs`: What the page shows: the recorded calls, newest first and narrowed, and each connector's block state.
- `change.rs`: Block and Unblock, written by the config writer so the next call is refused or runs again, with no restart.
- `kept.rs`: The grants a one-bot block took out, kept in the store until its unblock lays them back.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `place.rs`: Making room for a table laid back at its old place.
- `routes.rs`: The owner's routes: the calls, the block state, Block and Unblock (the Connectors page's too, `connector_page/`).

## `app/`

The daemon's shared state (`Shared`): the config, SQLite, the lanes in memory and the change feed the page listens to, and the small helpers every module uses (git, time, log text, the PATH).

Files:

- `config.rs`: The config and the projects' manifests, re-read only when they changed on disk.
- `conv.rs`: Which conversation and which factory role a row belongs to, and the stamp it carries.
- `feed.rs`: The change feed the page listens to, and the event and chat rows it shows.
- `git.rs`: Running git.
- `keys.rs`: What proves who calls, 0600 in the home and hidden from the sandbox: `owner.token`, `net.secret`, `owner.html`.
- `lane.rs`: One lane as the daemon keeps it in memory and in SQLite.
- `lanes.rs`: The lanes in memory, each change written to SQLite and shown on the page.
- `migrate.rs`: Moves kept from older names.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `open.rs`: Opening the shared state at an NeoBot home: its folders, its database and the lanes it kept.
- `path.rs`: The PATH agents and checks run with.
- `process.rs`: Signalling processes.
- `query.rs`: The store's reads: rows mapped by a function, or as JSON objects.
- `reply.rs`: What a JSON API handler answers.
- `state.rs`: The shared state itself: what every module reaches through `Shared`.
- `store.rs`: The store's writes: a row, a key's value, and keeping the event table bounded.
- `text.rs`: Log text: its secrets hidden, its last lines.
- `time.rs`: The time now, as the rows and the lanes keep it.
- `worktree.rs`: A lane's worktree: added, its excluded paths left out, its ignored local files copied in.

## `attach/`

Chat attachments: files and images the owner drops, pastes or picks in the chat, saved under `$TIRITH_HOME/attachments/<id>/<name>` so the manager can open them by path, and served back never as a page on this origin.

Files:

- `describe.rs`: The attachments as the manager reads them in the owner's message.
- `fetch.rs`: Serving a saved attachment, never as a page on this origin.
- `kind.rs`: The Content-Type a file is served with.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `name.rs`: An attachment's file name.
- `routes.rs`: The attachments' HTTP API.
- `upload.rs`: Saving an uploaded attachment in its own folder.

## `bots/`

Bots: agents of their own, factory roles with `scope = "own"` — the owner's in `~/.tirith/config.toml` (`[roles.bot-<id>]`, extending the manager) and the factory's system bots (`ceo`, `coach`, `judge` with `show = false`, and `manager-general`, the manager of no project), which the page edits the same way, saving the change as a layer in config.toml. Each has a name, a face (shape and
colour) and its own conversation (its key), run like a project's manager with its own room on the phones; the sidebar paints every `own` role with `show = true`. Routines are `[[routes]]` on `schedule`
in that file (their last runs stay in kv as `routine:<id>:<routine>` stamps, not configuration): tasks a bot does by itself at a time of chosen weekdays (the machine's local
time); the lanes scheduler's tick hands a due one to the bot once for each of its times. Times missed (the daemon was down, the machine slept)
follow the route's `catch_up`: `"once"`, the default, runs the routine one time, however many of its times passed since its last run, and tells the bot how late
that is, counted from the latest of them; `"skip"` waits for the next time. Up to 5 minutes after its time is on time, and a routine that never ran only has
today's time to catch up. `POST /api/bots/<id>/routines` takes `catch_up` too, and the bot makes them itself with `neobot routine add|remove|list`. The page writes config.toml
with `toml_edit`, keeping its comments, and the bots that used to live in kv move there once, at daemon start. A bot learns in its folder (`<tirith home>/manager/<key>`): `memory.md`, which NeoBot hands it as it is now at
the start of every turn and routine, and skills in `.agents/skills/<name>/SKILL.md`, its own and the shared ones of `<tirith home>/bots/skills` (linked in before every turn).
Both are git repositories (`bots/versioned.rs`): a bot's folder follows only its `memory.md` and its own skills — its `.gitignore`
says so, and leaves out the links to the shared skills and its working files — and `<tirith home>/bots` (the shared
`memory.md` and `skills/`) is another. NeoBot commits, as NeoBot, when a turn of the bot's own role in its own
conversation ends having changed them (message «turn <id>», the turn's row id), and after each change from the Library
(«library»); nothing changed, nothing is committed. So the `versions` of a turn (git blob hashes) are blobs of that
history, and `git log` in a bot's folder says which memory and skills each run used.
A routine's own effort (and model): the bot's folder may also hold `routines.toml`, a table per routine id (`[r1]`, `effort = "medium"`, optionally `model`), tracked in the same history so a branch of the Coach's can change it. It holds for that routine's turns only (`tuning.rs`): when the run starts, a live session ends so a fresh one starts with it, `models::role` gives its turns the routine's values, and the end of the turn ends the session again; an exec agent takes it per turn, and a replay's or a shadow's B reads the branch's file from its worktree.
The bots' agent finds them natively: Muse reads `.agents/skills`, and the agent's own `.<command>/skills` is a link to the same folder. Agents may write in a bot's folder
(never in its hidden folders, where agent settings live, except skills folders) and in the shared skills; the page's Library shows, corrects and deletes all of it.
The shared memory, `<tirith home>/bots/memory.md` (what holds for every bot: the owner's name, time zone, language), is handed before the bot's own, as `[tirith] Shared memory.md as it is now: …`, and the
Library shows it above the bot's memory and edits it (`POST /api/bots/<id>/library` with `memory` and `shared`; `GET` gives it as `shared_memory`). Memory notes are numbered: when it hands a memory over,
NeoBot puts `[m<N>]` (the bot's own) or `[s<N>]` (the shared one) before every note line (one that starts with `-` or `*`) that has none yet, the number after the highest already there, and saves
the file so; numbered notes, headings and every other line stay as they are, and the bot cites `[m<N>]` or `[s<N>]` when its answer uses a note. The chat and turns keep
those marks (the `cited` count needs them); the page draws each as a small raised number on the word before it that shows its note on hover or tap and is left out of a copy
(`web/js/views/chat/cite.js`), and what goes to the phones and answers a room has them taken out with the spaces before them (`rooms/uncite.rs`). Every skill follows the factory's template,
`factory/skill.md`: its header and seven sections («Cuándo usarla», «Entradas y accesos», «Pasos», «Cómo validar», «Qué devuelve», «Qué requiere aprobación», «Si algo falla»). The bot prompt
gets its text in place of `{skill_template}` (as a role's prompt gets `{name}`); the Library's `GET` gives it as `template`, a new skill starts from it, and every skill is marked with the sections it lacks, by their titles.
A bot may have a model of its own (role `bot-<id>` of `models/`, chosen in its card); without one it has the managers'. A routine answer that starts with ∅ (nothing new)
stays on the page and never reaches the phones (`rooms::to_phones`), so only news does.

Routines on an event from outside. A routine is either timed (`schedule`) or waits for its bot's webhook: a `[[routes]]` with `on = "webhook"` and
`when` — the event's `<source>:<kind>` (`github:pull_request.opened`), a pattern of it with `*` (`github:issues.*`), or `*` for every call —
made on the page (Routines: "At a time" or "On an event from outside") or by the bot (`neobot routine hook "<title>" "<when>" "<task>"`); its
task may name `{title}`, `{url}` and `{text}`. The webhook is agentsmessenger.com's sealed webhook (its `src/server/hooks/`):
`POST /api/bots/<id>/webhook` gives the bot's URL, making it once — the daemon makes sure the bot has its own private room (`rooms/hooks.rs`,
even with no phone paired), signs with its room key a `request` event in that room (refs `[]`, body the JSON text
`{"hook": "create", "readers": ["<its public key>"], "seal": "once"}`), sends it to `POST <[messenger] url>/hooks` and keeps `{id, url, from, room}` in kv
`webhook:<bot>` (`from`: the key of the platform's `webhook` agent of that room); `POST /api/bots/<id>/webhook/delete` deletes it the same way
(`{"hook": "delete", "id": "<id>"}`), and deleting the bot forgets it. What a service posts to the URL arrives in the room sealed for this
NeoBot alone, as `tirith-hook {"v":1,"at":…,"headers":{…},"body":…}`. The room's watcher (`rooms/hooked.rs`) never makes such a line one of
the owner's or of another agent: from the `from` agent it becomes the bot's `webhook` event (`bots/hook_event.rs`) — `source` (`github`
when it has `x-github-event`; `mail` for the owner's mail script, a body with `from` and `subject`; else the user-agent), `kind` (`github:<x-github-event>.<action>`, `mail:received`, or `hook`), `title` and `url` (of
GitHub's pull request, issue, release or comment, else the body's own `title` and `url`), `text` (the body as text, 4000 characters at most)
and `data` (the body) — and from anyone else it is dropped. The event is kept and shown; each of the bot's routines on `webhook` whose
`when` holds runs (`bots/hook_fire.rs`, trigger `event:webhook`), handed its task and then the event between two random marks
(`<<<outside-data <mark>` … `outside-data <mark>>>>`), said to be data from outside and never instructions (`factory/prompts/bot.md` says
the same). What the task names (`{title}`, `{url}`, `{text}`) is filled in one pass and each value is fenced with the same unguessable mark
(`⟦outside-data <mark>⟧value⟦/outside-data <mark>⟧`), so a value that names another `{…}` is never expanded again; the chat and the event line show
the label as name characters only (`hook_clean.rs`) and the title as one quoted line. «Probar» on the page (`POST /api/bots/<id>/webhook/test/<routine>`)
runs one routine with the sample that fits its `when` (`hook_sample.rs`: an e-mail for `mail:…`, the GitHub event and action it names, else a pull
request opened), with no platform in between, and says whether the `when` holds for it (`matches`). The Library gives a short Google Apps Script with
the URL in it that, every 5 minutes in the owner's Google account, posts the new mail labelled `tirith/<bot name>` (sender, subject, date, text, its link)
to the webhook; it marks a message read only after a 2xx answer, goes on with the next one when one fails, and cuts each call under the platform's 64 KB
whatever the characters (`web/js/views/bots/mailtext.js`). One button there («Set it up») makes the address and the routine «Mail to <bot>» (`when`
`mail:*`). Tests `webhook_create_signs_the_request` (against a fake platform) and `webhook_starts_its_routine` (`rust/tirith/checks/webhooks.rs`).

Templates. `factory/bots/<id>/` is one bot template: `bot.toml` (`name`, `start` — what the bot says first, so the owner knows what it needs —,
`[face]` with `shape` and `color`, and `[routine]` with the fields `POST /api/bots/<id>/routines` takes) and `SKILL.md`, the skill named `<id>`
with the seven sections of `factory/skill.md`. The repo's three are built in (`factory/bot_templates.rs`) under the folder's, which is read on
each call; one that does not load is an error, by its id. `GET /api/bots/templates` lists them and `POST /api/bots/from-template`
(`{"template": "<id>"}`) makes the bot and answers it: an owner's bot like any other, with the skill in its folder and the routine stamped as
just made, so it first runs at its next time. «Desde plantilla» in the Bots view shows them. Test `bot_templates_are_complete`
(`rust/tirith/checks/bot_templates.rs`).

Files:

- `api.rs`: The bots' HTTP API: the list and its cards, a bot deleted (its secrets with it), its routines, a routine run now, its Library.
- `card.rs`: A bot's card saved: a new bot, or a change to one.
- `connect.rs`: A bot card's Permissions → Connectors (`GET`/`POST /api/bots/<id>/connectors`, the owner's alone): each connector granted or not, blocked, and its narrowing; a grant given, narrowed or removed through the bot store (`store::with`), which writes `[roles.bot-<id>.connectors.<connector>]` (`grants.rs`), never a list of bots on the connector.
- `cli.rs`: A bot's own commands, in its conversation.
- `context.rs`: What a bot is handed at the start of every turn and routine, and the skill links laid before it.
- `drafts.rs`: Draft skills (a taught task not yet approved): kept in the bot's `drafts/` folder (shared ones in `<home>/bots/drafts`), outside every folder an agent loads by itself; one found among the skills is moved there (by `context`, `versioned` and the Library's read). One activated in the Library is told to its bot with its next message.
- `folder.rs`: A bot's folder and skills: where its sessions run (the lab's worktree during a B run), where agents may write, and the skills a folder holds.
- `grants.rs`: A bot's connector grants through a rewrite of config.toml: kept under each bot still there on a card save, gone with a deleted bot's role (`connectors/`); the one a card's Connectors changed (`connect.rs`) laid in, its keys set or taken off in place.
- `fragment.rs`: The bots as a TOML fragment: the owner's bots' roles, and every written bot's routes and routines.
- `handed.rs`: What a bot is handed with its next message, from the folder that turn runs in (an experiment's B: the branch's); a skill that changed is read again in a session of its own.
- `from_template.rs`: A bot made from a template: its card, its routine, its skill and its first words.
- `hook.rs`: A bot's webhook as NeoBot keeps it (kv `webhook:<bot>`: the platform's id, url and from, and the room), and what arrives through it.
- `hook_api.rs`: A bot's webhook from the page: its URL (made once on the platform), deleted, and «Probar».
- `hook_clean.rs`: What a webhook event shows outside its data block: a label of name characters only, a title as one quoted line, and a value fenced inside a task.
- `hook_event.rs`: What a webhook call brought, as a bot's `webhook` event: where from, what kind, its title, link and text, and the body.
- `hook_fire.rs`: A webhook event reaches its bot: the bot's `webhook` routines whose `when` holds run with it, the event inside a block marked as data from outside.
- `hook_sample.rs`: The sample events «Probar» sends a routine: GitHub's (the event and action the `when` names) and the mail script's.
- `layer.rs`: A changed system bot's layer over its factory role, in config.toml.
- `library.rs`: The page's Library: a bot's memory, its own skills and the shared ones, drafts included (marked `draft`), read and changed; an agent's call may not activate a draft.
- `list.rs`: The bots as the page sees them, read from the effective factory table, and how one is found.
- `migrate.rs`: At start: every bot's folder, and the bots that used to live in kv moved to config.toml once.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `notes.rs`: Numbered memory notes: `[m<N>]` and `[s<N>]` put before every note line that has none.
- `routine.rs`: A routine made, changed or removed from a bot's card or its own commands.
- `run.rs`: Routines run: one now, or every one due on the scheduler's tick (which also decides the lab's experiments that are due); each run is handed to the bot as a turn of its own (`manager::run_routine`).
- `schedule.rs`: When a routine runs: its days of the week, whether it is due now, and how late.
- `store.rs`: Keeping the bots: one change at a time, written into config.toml and reloaded.
- `sweep.rs`: What goes out of config.toml before the bots are written again: their roles and routes (not a timed route with no task: that is a turn role's pass), then the sections left empty.
- `template.rs`: A bot template read and checked: its name, face, routine, first words and skill.
- `told.rs`: What a bot is handed when one of its routines is due.
- `tuning.rs`: A routine's own effort and model: `routines.toml` in the bot's folder, read when its run starts and applied to its turns only.
- `versioned.rs`: The bots' folders and `<home>/bots` as git repositories: only the memories and the own skills followed, a commit after each turn, Library change or bot made from a template that changed them.
- `written.rs`: The text of config.toml with the bots laid into it.

## `bundle/`

What the binary carries so that it runs with no checkout beside it (`rust/tirith/build.rs` builds it in, and `rust/tirith/build/version.rs` its version, which follows each commit): the page
(`web/`), the teach extension (`tools/teach/`, downloaded from the page as `/teach-extension.zip`) and the browser
launcher, plus the version (`neobot --version`). A development build still reads its checkout (the page shows on
reload, the factory follows main); one built with `TIRITH_DIST` (`scripts/release/build.sh`) never looks for one
and holds no folder of the machine that built it.

Files:

- `checkout.rs`: The checkout a development build was built from, and none in a build to hand out.
- `files.rs`: The carried files, by their path in the repository.
- `launcher.rs`: The browser MCP's launcher: the checkout's, else the carried one written into NeoBot's home.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `zip.rs`: A zip with its files stored as they are.

## `chat/`

Clearing a conversation and its longest life. Every conversation (a project's, a bot's) can be cleared — its chat rows go, its saved agent session (kv `manager_session:*:<conversation>`) is forgotten and its live one ends without cutting a running turn, so the bot starts fresh; its memory, skills and routines stay — and can be given a longest life: `chat_keep`, the owner's own words (`30m`, `1h`, `24 h`, `2 días`, `una semana`; `siempre` takes the limit away), in `~/.tirith/config.toml` on the bot's role or on the project, written with toml_edit. A minute sweep drops the rows older than it, and a sweep that dropped any forgets the saved session too, so the bot cannot remember what is gone. The owner clears and limits a chat in its page menu, or asks its bot in the chat: `neobot chat clear` / `neobot chat keep "<time>"` act on the conversation of the session that runs them (a lane never does), only on the owner's own words. The paired rooms hear both as one sealed line — `tirith-chat {"v":1,"clear":"<ISO UTC>"}` and `tirith-chat {"v":1,"keep_s":<seconds, 0 = forever>}` — so the phone page hides what is gone.

Files:

- `clear.rs`: Clearing a conversation so its bot starts from zero.
- `cli.rs`: `neobot chat clear|keep`.
- `keep.rs`: A conversation's longest life: reading it and setting it.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `routes.rs`: The chat's HTTP routes: clearing a conversation and its longest life.
- `sweep.rs`: Dropping the chat rows older than their conversation's longest life.
- `words.rs`: The owner's words for how long a chat lives.
- `written.rs`: Writing a conversation's `chat_keep` into config.toml.

## `cli/`

The command line. `main.rs` is the crate root: its `mod` lines and a `main` that hands the arguments to `cli/`. `neobot serve` is the daemon; every other command but the guide's (`setup`, `connect`, `doctor`, `service`) (see `service/`) is one HTTP call to it on 127.0.0.1 (`client.rs`), so it starts in milliseconds. A module's own commands live in that module (`guard/cli.rs`, `rooms/cli.rs`, ...); `cli/dispatch.rs` asks them first, but `neobot secret`, which reads its raw argv (`secrets/cli.rs`) and so comes before it.

Files:

- `../main.rs`: The crate root: its `mod` lines and a short `main` that hands the work to `cli/`.
- `../client.rs`: The CLI side: one HTTP/1.1 request to the daemon on 127.0.0.1 per command, with the caller's token.
- `../client/ancestry.rs`: A variable from the environment this process's own ancestors started with.
- `../client/token.rs`: The token this process calls with: its agent's, else the owner's; a hook's is never the owner's.
- `args.rs`: Reading the arguments: a flag's value, the project a command is about, and the positional words.
- `ask.rs`: Questions and answers: a lane asks the manager, the manager answers a lane or a card, the manager asks the owner.
- `dispatch.rs`: The command the positional words name: a module's own first, then the daemon's.
- `hook.rs`: `neobot hook`: the command-hook bridge for agents without HTTP hooks.
- `hooks.rs`: `neobot hooks install`: the agent's command hooks become HTTP hooks on the daemon.
- `hooks_token.rs`: The HTTP hook with `X-Tirith-Token`, and an older install's given it as the daemon starts.
- `open.rs`: `neobot open [link]`: the page opened as the owner.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `plan.rs`: `neobot plan show` (each step with its open items and their lanes) and `neobot plan step`; `neobot plan step delete` is in `dispatch.rs`.
- `serve.rs`: `neobot serve`: the daemon.
- `status.rs`: `neobot status`: the lanes working now and the last ones that ended.

## `codex/`

Codex's local catalog and the JSONL exec turns of the agents without a streaming chat mode (Codex, GitHub Copilot, Muse: `kind.rs`). Conversations resume their native sessions; text jobs run read-only. A turn's small files
(Muse's prompt, Copilot's usage) carry that turn's own name, so a role's turns can run at the same time.

Files:

- `argv.rs`: The command line of one exec turn of Muse and of Codex.
- `catalog.rs`: The models Codex lists in its local cache, with the reasoning efforts each one supports.
- `command.rs`: The command line that starts one exec turn, and how the agent gets its message and gives its usage.
- `copilot.rs`: The command line of one Copilot turn, and what its stream says of the answer.
- `kind.rs`: Which agents run exec turns, and how each is named.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `options.rs`: The options every Codex command line gets from config.toml: its profile and the reasoning effort.
- `reading.rs`: Reading an exec turn's JSONL events: the answer's text, whether the turn completed, and why it failed.
- `scratch.rs`: The small files of one exec turn, named for it alone and gone with it.
- `setup.rs`: What one exec turn is: which agent runs it, in which folder, and how its role's factory shape sets it up.
- `turn.rs`: One exec turn: set it up, start the agent, read its answer within the turn's time, and clean up.

## `connector_page/`

The Connectors page (`web/js/views/connectors.js`, with `connectors/` beside it; in the menu under Overview) and the
Permissions → Connectors section of each owner's bot's card (`web/js/views/bots/card-connectors.js`), the owner's
alone: `/api/connectors/` is refused to every agent in `server/scope.rs`, and these routes and `/api/bots/<id>/connectors`
refuse any agent's token again by their own layer (`routes.rs`). What is defined, granted and blocked is read only from
config.toml's tables (`[[connectors]]`, `[roles.bot-<id>.connectors.<connector>]`) and the Access APIs' one-bot blocks
(`access/`); the page's Block / Unblock is `access/`'s `POST`/`DELETE /api/connectors/blocks` and its Login is
`oauth/`'s.

`GET /api/connectors/page` gives `connectors` and the owner's `bots` (`id`, `name`). Each connector has `id`, `kind`
(`local` or `remote`), `host` (a remote one's host and port alone — never its command, its url's path or query, or a
secret), `blocked` (for all bots), `last` (its newest `connector_calls.at`), `tools` (the names its last open listed)
and `bots`: each owner's bot granted it or blocked for it alone, with `bot`, `name`, `blocked` (alone), `last`, its
`status` and `why`, and `tools`, each `{name, mark}` marked by the gateway's own verdict (`connectors::verdict`) for
that bot's grant: `allowed`, `approval` or `denied` (all denied when it is blocked for the bot). The status (`status.rs`):
`error` when its last open or sign-in failed; `needs login` when a secret the definition names is not set for the bot
(names only, read from its store), or for a remote connector that names none, when it is neither signed in
(`TIRITH_OAUTH_<ID>_ACCESS`) nor ever opened; otherwise `ok`.

What the gateway last saw is kept in kv (`seen.rs`): `connector-tools:<id>` (the tool names of the last open) and
`connector-state:<bot>:<id>` (`{at, ok}` or `{at, error}`), written when the gateway opens a connector for a bot
(`gateway/open.rs`) and when a sign-in begins or ends (`oauth/api.rs`, `oauth/callback.rs`). An error's why is scrubbed
before it is kept or said anywhere (`scrub.rs`): each value the bot holds, a Bearer or Basic credential, a url's query
and the value of any pair whose key names a secret (token, secret, password, key, auth, cookie, credential, code)
become `(hidden)`; the gateway's `error` event names the same scrubbed why.

`POST /api/connectors/<id>/remove` (`remove.rs`), after the page asks once ("Remove <id> and every bot's grant of
it?" — Remove or Keep, like a bot's Delete): the config writer (`projects::rewrite`, toml_edit, every other line and
comment kept) takes the `[[connectors]]` row and every role's grant of it out (a role's `connectors` table left empty
goes too); then the one-bot blocks kept of it, what was seen of it and each owner's bot's sign-in to it (its reserved
names) go; a `connector-remove` event. An unknown id is a 400.

A bot's card grants or removes a connector with a checkbox and narrows a grant with its tools (empty: all the
connector allows), deny and approval globs, saved at once: `POST /api/bots/<id>/connectors` with `{connector, grant}`,
`grant` an object (`tools` null or a list, `deny`, `approval`) or null to remove it. It goes through the bot store
(`bots/connect.rs`, `store::with`, one change at a time): the bot carries the change to the writer, which lays it into
that bot's `[roles.bot-<id>.connectors.<connector>]` (`bots/grants.rs`) — keys set in place, `tools` taken off when
null and `deny` or `approval` when empty — never a bots list on the connector. An unknown connector, an empty or
quoted glob, a system bot or a connector blocked for that bot alone (unblock it first) is refused. `GET` gives each
connector with `granted`, `blocked` and the grant's `tools`, `deny` and `approval`. Test `connectors_page_state`
(`rust/tirith/checks/connectors_page.rs`), against the gateway checks' fake MCP servers: the status of each bot, the
real tools marked, a grant round-trip through the card's API (kept by a card save), narrowing, removal of one grant,
Remove, a login error and a scrubbed why, agents refused, and no secret value in any answer.

Files:

- `marks.rs`: A connector's real tools marked for one bot by the gateway's own verdict: allowed, approval or denied.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `remove.rs`: Remove: the connector's row and every grant out of config.toml, its kept blocks, sign-ins and seen state with them.
- `routes.rs`: The owner's routes: the page's state and Remove, and the owner-only layer the card's route uses too.
- `scrub.rs`: An error's text with no secret in it.
- `seen.rs`: What the gateway last saw of a connector: its tools, and by bot whether its last open or sign-in worked or why not.
- `state.rs`: What the page shows: each connector, its bots with their status and last use, and its tools marked for each.
- `status.rs`: One bot's status with one connector: needs login, ok or error.

## `connectors/`

Connectors are MCP servers a bot may use, defined once in `~/.tirith/config.toml` — never in a repo: a `[[connectors]]`
in a factory folder's file is an unknown section. What each bot may do with one lives under that bot's own role. The exact shape:

```toml
[[connectors]]
id = "gmail"                          # lowercase letters, digits, - and _; unique
command = ["node", "gmail-server.js"] # a local server: its argv (or url, never both)
# url = "https://mcp.example.com/mcp" # a remote server: an http(s) address
blocked = false                       # optional, default false: true refuses every tool for every bot
deny = ["delete_*"]                   # optional globs (* any run, ? one character)
approval = ["send_*"]                 # optional globs: a call asks the owner first
secrets = ["GMAIL_TOKEN"]             # optional: variable names only (A-Z, 0-9, _), never values, never a TIRITH_OAUTH_ one
# a remote server that signs in with OAuth needs no secret: neobot connector login <id> --bot <bot> (see oauth/)

[roles.bot-<id>.connectors.gmail]     # the grant: without this table the bot has no gmail
tools = ["read_*", "search"]          # optional: when set, all the bot may use
deny = ["read_spam"]                  # optional: more refused
approval = ["read_*"]                 # optional: more that asks first
```

An empty grant (`[roles.bot-<id>.connectors.gmail]` alone) is the connector as defined. A grant can only narrow:
one verdict (`verdict.rs`) decides both what a bot is listed and what a call does, in this order — connector
`blocked`; connector `deny`; bot `deny`; outside the bot's `tools` when it set them; connector `approval`; bot
`approval`; otherwise run — so a bot's `tools = ["*"]` or empty `approval` reaches nothing the connector refuses or asks
for. A refusal names the connector, the tool, why (`refused by block`, `refused by deny`, `refused by scope`) and the
tools the bot may use, never a secret or the command. The loader is strict (`check.rs`, from `factory/parse.rs`): an
unknown key in a connector or a grant, a missing or malformed `id`, `command` and `url` both or neither, an argv that is
not a list of strings, a url that is not http(s), a secret that is not a variable name, an id defined twice, a grant of
an unknown connector or a `connectors` table on a role that is not a `bot-<id>` makes the version invalid, like any
factory error. The bots' writer keeps a bot's grants when its card is saved and drops them with its role when the bot
is deleted (`bots/grants.rs`). The connector gateway (`gateway/`) is the verdict's caller. Tests `bot_rules_cannot_loosen_the_connector` (`rust/tirith/checks/connectors.rs`) and
`deleting_a_bot_drops_its_grants` (`rust/tirith/checks/bots.rs`), on fake tool catalogs and config fixtures, with no model.

Files:

- `check.rs`: The strict check of `[[connectors]]` and of the grants that name them: a typo, an unknown connector or a half-defined one is an error, never silence.
- `glob.rs`: The globs of `tools`, `deny` and `approval`: `*` is any run of characters, `?` one; anything else matches itself.
- `grants.rs`: The strict check of the grants under the roles: only a bot's role holds one, only of a defined connector, with only the keys a grant has.
- `mod.rs`: The connector and grant types; declares the folder's files and re-exports what other modules use.
- `read.rs`: Reading connectors and grants out of TOML that has already passed the checks.
- `row.rs`: The strict check of one `[[connectors]]` row.
- `verdict.rs`: The one verdict on a bot's use of a connector's tool: listing and calling both ask it, so a tool that is listed is one a call may reach.

## `config/`

`~/.tirith/config.toml` (projects, agent aliases, this machine's overlay on the factory) and each project's `tirith.toml` (plan file, checks, lane excludes, extra rules). Unknown keys are an error the owner sees, never silently ignored; yesterday's sections are read in memory under the factory's names, the file itself untouched.

Files:

- `file.rs`: The config of one NeoBot home, as read from its config.toml.
- `known.rs`: The sections of config.toml something reads (`connectors` among them, see `connectors/`), and the keys nothing does.
- `load.rs`: Loading config.toml: parse it, move yesterday's sections, lay it over the factory and name what is wrong.
- `manifest.rs`: A project's `tirith.toml`: its plan file, lane excludes, pipeline stages and extra rules.
- `migrate.rs`: Yesterday's sections of config.toml read under today's names, in memory only.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `paths.rs`: The owner's home, NeoBot's home, and paths written with `~/`.
- `project.rs`: One `[[projects]]` entry of config.toml.
- `table.rs`: Reading one typed value out of a TOML table by its path.

## `expected/`

What was due and how it ended. Every run of a routine and every scheduled pass of a turn role is written down when it is due
(table `expected_runs`), before anything starts, and ends in a row of `turns` whether it ran or not. One that failed, was lost or
never started says why, counts as a failure in the numbers the Coach reads, and is one line in the chat and on the phones, sent
by code, so it needs no agent and no quota. «∅» means "I looked and nothing is new". Code fails a run only where it is certain;
a "could not" in other words is told by the judge that grades the run (`turns/`, `score`), so a good run's news reach the phones
once it is judged, or when no word of its judge can come any more (26 minutes): a failure is the one line.

Files:

- `admits.rs`: An answer that says in the first person that the task could not be done: not someone's words that it quotes or tells, and not the failure of someone or something that the task reports. Of a scheduled pass, only where it begins or of what it tried: proposing nothing is no failure.
- `admitted.rs`: A run that code found good and that its judge read as "I could not", in words no list holds: it ends as a failure, once. Where no judge could read it, plain words of trouble do the same, so it is never kept as a success in silence. A run that asked for the owner's yes is none of these.
- `calendar.rs`: When something scheduled was due, in this machine's time, kept as UTC instants.
- `changed.rs`: What the conversation is told when a routine's schedule changes: nobody waits for it on a day, or at a time, it no longer has.
- `claims.rs`: What an answer claims. «∅» is "I looked and nothing is new"; «✗» is "I could not"; a «∅» that says it could not is a «✗».
- `found.rs`: Which occurrence a question is about: a scheduled one by when it was due, or a routine's oldest in a state.
- `gaps.rs`: When NeoBot was away: it keeps that it is alive, so what was missed says whether NeoBot was stopped or the machine slept.
- `life.rs`: An expected run's life: it starts, and its turn ends it; or it never becomes a turn and ends without one (`lost.rs`).
- `lost.rs`: The row in `turns` of a run that never was a turn: failed, with no agent, no time and no tokens (the numbers count the failure and nothing else), and why.
- `missed.rs`: Occurrences that nothing ever wrote down and that will not run: NeoBot was stopped, or the machine slept, across them. The one `catch_up` still runs is not lost; with "skip" one is lost as soon as it is late. Each gets its row, and each routine one line.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `news.rs`: The news of a good run wait for its judge, so a "could not" that it finds is one line; never long, and in the store.
- `notice.rs`: The one line the owner gets of a run that did not work, written by code and kept until the phones have it.
- `overdue.rs`: Runs that wait, or work, too long (`[limits] routine_minutes`), counted on a clock that stops while the machine sleeps; and a pass whose day ended while it waited and that `catch_up` runs no more.
- `passes.rs`: A turn role's scheduled pass as an expected run: written down when due, waiting while something keeps it, and never twice.
- `reasons.rs`: Why a turn failed, in a few words for the owner: the kind of failure and the agent's own last line.
- `restart.rs`: What a start finds open: a run NeoBot was stopped in the middle of failed and is not done again (it may have acted); one that had not started is handed again on the day it was due or caught up, and else is lost.
- `stands.rs`: Where one occurrence stands, for whoever reports it (`/status`): the register's word, and its turn's where it left one, so it is the truth the numbers and the notice have.
- `store.rs`: The expected runs as kept (`expected_runs`): one row for each occurrence, waiting, running, then ok or failed.
- `timed.rs`: Everything that has a time of day: the bots' routines and the turn roles' scheduled passes.
- `told.rs`: Whose words an answer holds: what it quotes, and what it tells of someone's, are theirs. What it tells of its own tool or helper is its own.
- `unjudged.rs`: A good run that no judge will read any more: code's plain words decide, as when a judge gave no verdict; and whether an experiment's run still waits for its judge, before the experiment is decided.
- `verdict.rs`: Whether a routine's turn did what it was started for, as far as code is certain: its agent ended well, no «✗», no tool's error proves it could not, and a «∅» says no "could not" on its line and comes from having looked.
- `watch.rs`: The scheduler's tick: NeoBot says it is alive, what was missed is written down, what takes too long is ended, news that waited too long for a judge go out, and a run no judge will read is looked at by code.
- `words.rs`: Times, days and texts as the owner reads them in one line.

## `factory/`

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
the factory's names (`config/migrate.rs`), the file itself untouched except by the page's `toml_edit` writes (the bots'
roles and their schedule routes), and keys nothing reads are an error the owner sees. A role `extends` another (manager-general and the CEO extend
the manager), and an agent the factory does not name runs with the `default` agent's flags. The `[[routes]]` of
the factory say who hears each event the code emits (a closed list in `factory/lists.rs`): which role, as a `turn` now
or a `note` with its next turn, a line in the chat (`chat`), and whether that line and the answer of a turn the
event starts reach the phones (`phone`). A route on a missing event or role is invalid; routes in config.toml
stand next to the factory's. The `[[evals]]` of the factory are judged in the background by their `judge` role
(small text jobs on a cheap, fast model, `access = "read"`, no prompt of its own: each rubric in
`factory/evals/` is its instructions): the event with the conversation before it (and, for a card, the card
itself) goes in, and a `scores` row keeps the label or score, one line why, the event and the judge's stamp;
a share of the events is judged (`sample`, by event id). An eval on a missing event, judge or rubric is
invalid. `[lab]` is the rule the lab decides experiments by (`lab/` below), read with `deny_unknown_fields` and
its shares checked (from 0 to 1, `inputs` at least 1). Nothing of a score reaches the owner: the scores only feed the measures (`satisfaction`,
`needless_cards` and `unapproved` below).

Every version of the factory has a stamp (`Factory::version`): the main commit that last changed `factory/`,
plus a short hash of what config.toml lays over it (`<commit>+<hash>`; `embedded` when no checkout is at hand).
Every lane, chat (turn) row, owner card, event, review and score is born carrying it in its `factory` column,
with `role_hash` next to it: a short hash of the setting of the role it ran with — its resolved table, its
agent's table, its prompt texts and the page's model override for it (`Factory::role_hash`, `App::stamp_for`).
A lane's events and cards carry the lane's stamp; a line no role wrote (the owner's, a room's, NeoBot's own)
keeps the version with an empty hash. Comparing two versions, two models or two prompts is then one query
(`GROUP BY factory, role_hash` over `lanes`, `chat`, `owner_items`, `events` or `scores`), with no extra runs.

Files:

- `agents.rs`: Reading a factory's agents: their keys and their commands.
- `bot_templates.rs`: The bot templates' texts: `factory/bots/<id>/`, the repo's built in under the folder's.
- `connectors.rs`: A factory version's connectors (config.toml's `[[connectors]]`, checked on load) and the ones a bot is granted (`connectors/`).
- `current.rs`: The factory version the daemon runs now, reloaded under config.toml's overlay; a bad version keeps the last good one.
- `embedded.rs`: The repo's factory, built into the binary: what runs when no factory folder is found, and what every folder's file is laid over.
- `evals.rs`: Reading a factory's evals, and the hash of the setting a role runs with (what an eval's verdict is tied to).
- `extends.rs`: Roles that extend another role: each is laid over its parent, all the way up.
- `keys.rs`: The keys a factory section may hold.
- `layers.rs`: The layers of one factory version: the embedded text, the folder's file, config.toml's sections and routes, and the `default` agent under every other.
- `lab.rs`: The factory's `[lab]`: the rule an experiment is decided by (`inputs`, `not_worse_share`, `min_saving`, `max_days`), strictly read.
- `limits.rs`: The factory's `[limits]`: load, time and size limits the daemon runs under.
- `lists.rs`: The closed lists a factory is checked against: its sections, its measures and the events the code emits.
- `loaded.rs`: One loaded factory version.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `parse.rs`: Parsing one factory version, layer by layer, and validating it.
- `paths.rs`: Where the factory folder is, and the texts it holds.
- `refs.rs`: The checks that a factory's names point at something: protocols, parents, agents, prompts and measures.
- `roles.rs`: Reading a factory's roles: their keys, who runs a session, and the prompts they run with.
- `routes.rs`: Reading a factory's routes: who hears an event, and the timed passes.
- `rowrefs.rs`: The checks that a factory's routes and evals name a real event, role, mode, rubric and sample.
- `stamp.rs`: A factory version's stamps: its name (commit plus overlay hash) and the newest change time of its files.
- `unknown.rs`: The checks that name the first key or section a factory should not hold.
- `validate.rs`: The strict check of one factory version: the first thing wrong in it, in a fixed order, makes it invalid.
- `values.rs`: Small readers and writers of TOML values the factory uses everywhere.
- `words.rs`: Reading a factory's measures and its word lists (plan words, card kinds).

## `gateway/`

The connector gateway: `neobot mcp --bot <id>`, the one MCP server a bot's run carries for its connectors
(`connectors/`). `models/gateway_args.rs` adds it, as `tirith_connectors`, to the runs of a `bot-<id>` role granted at
least one connector, in the anthropic (`--mcp-config=` and `--allowedTools=mcp__tirith_connectors__*`), Codex
(`-c mcp_servers.tirith_connectors={…}`, `env_vars = ["TIRITH_TOKEN"]`) and Muse (`-c mcpServers.tirith_connectors=…`,
optional) dialects, where the browser's flags go (`models/browser_args.rs`): stream-json chat sessions and turns
(`models/chat_argv.rs`) and exec turns (`codex/command.rs`). A lane, a manager, the CEO and every other role get none.
Its env names only `TIRITH_HOME` and the run's own `TIRITH_TOKEN`; no secret is in it or in its arguments.

The process the agent starts holds nothing. It opens a session at the daemon (`POST /api/mcp/open` with its bot) with
the caller token of its run, and the daemon proves it from that token alone: the token must be a run of that very bot
(`<role>@bot-<id>`, no lane), else 403 and the gateway exits before serving anything — another bot's run, a lane, a
manager (a system bot holds no grant) or the owner. It answers `initialize` and `ping` itself and hands `tools/list`
and every `tools/call` to the daemon (`/api/mcp/<session>/tools`, `/api/mcp/<session>/call`; `DELETE
/api/mcp/<session>` when the agent closes it), where only the run that opened a session finds it again.

The daemon keeps the session's connectors open: each one the bot's role is granted and not `blocked`. A local one
(`command`) is started through `sandbox::wrap` as the bot's run, in its sandbox, with the bot's values of the
connector's `secrets` added to that process's environment alone; a remote one (`url`, MCP's streamable HTTP: a POST
per message, the answer JSON or an event stream, `Mcp-Session-Id` kept) is reached always through NeoBot's proxy with
the bot's run's credentials (`sandbox/proxy_url.rs`), so the proxy counts it as the bot's, never the daemon's; its
first secret goes as `Authorization: Bearer <value>` and any other as a header of its own name (`_` as `-`), marked
sensitive; a bot signed in to it (`neobot connector login`, `oauth/`) sends its access as that Bearer first, renewed by the daemon when it ends or the server answers 401. The values come from the per-bot store (`secrets::value`) at open time and stay in the daemon. A connector
that does not open is the owner's `error` event (with why) and missing from the list, never the end of the others.

The agent is listed each open connector's tools as `<connector>__<tool>`, only those the verdict lets this bot reach
(one that asks first says "(asks the owner first)"), by the grants of now. Every `tools/call` asks the verdict again,
with config.toml re-read when it changed, never trusting the list it gave: a refusal (`refused by deny`, `refused by
scope` — a connector the bot is not granted too — or `refused by block`, also for a connector the owner blocked for this bot alone, `access/`) answers the refusal's line as a failed tool
result; an approval match runs only on the owner's permit for that very call (below); only `ran` and `approved and
ran` reach the connector. Every
attempt, each repeated one too, is a row of `connector_calls` (`assets/schema.sql`): `at`, `bot` (its id), `conv`
(its key), `turn` (the start in µs of the conversation's turn open then, 0 for none), `connector`, `tool`,
`args_sha` (sha256 of the arguments as canonical JSON: keys sorted, no spaces), `result_sha` (of what ran answered;
empty when nothing ran) and `verdict` — never the arguments or the result themselves. A session whose gateway
process is gone is closed, its connectors' processes killed, at the next open. Tests `mcp_gateway_scopes_tools`,
`gateway_hides_secrets` and `refused_calls_are_recorded` (`rust/tirith/checks/gateway.rs`), with a fake stdio and a
fake HTTP MCP server (`checks/gateway_fakes.rs`), the daemon's proxy and no model.

A call that matches `approval` (the connector's or the bot's) does not run on its own (`approve.rs`, `permit.rs`). Its
key is the sha256 of the canonical JSON of `[connector id, tool name, arguments]`, in the bot's conversation. Without a
granted permit for that key the daemon opens an `approval` card in the bot's conversation (`manager::ask_owner`, so on
the page, in the chat and on the phones; one still open for the same call is reused, not doubled) naming the bot, the
connector and the tool with a short summary of the arguments (`summary.rs`: top-level keys and short values; a value
under a key that names a secret — token, secret, password, key, auth, cookie, credential — or holding one of the
connector's secret values for this bot is `(hidden)`), keeps a `connector_permits` row `asked` for it, records
`waiting for approval`, and answers the bot a failed result naming the card and telling it to end its turn: the turn is
kept `waiting` while the card is open (`turns/approval.rs`), and the owner's answer starts the bot's next one. Only the
owner's own answer on that card grants the permit: `card_done` (`manager/cards.rs`) settles it — `granted` when the
answer's first word is one of the factory's `[cards] yes` (the page's Yes button sends `yes`; `o12: yes` from a paired
phone), `refused` for any other answer or a closed card. An agent's `neobot answer` of such a card is refused outright
(`manager/relayed.rs`), even with the owner's words in the chat, and nothing a caller sends to the gateway (a role, a
card id, an `approved` flag) counts. The next call with the same key takes the permit (`used`, under the database lock,
so only one call can) and runs once: its result carries `_meta["tirith/approval"] = {verdict: "approved and ran",
card}`, the owner gets a `gateway` event, and the record says `approved and ran`, never the plain `ran`. Changed
arguments, another tool or connector, or a second identical call find no permit and ask again with a new card; a no
leaves none. Test `approval_is_bound_to_the_call` (`rust/tirith/checks/gateway_approval.rs`): the fake stdio server,
no run before yes, one after, no reuse, changed arguments and another tool ask again, a no runs nothing, and the bot's
own caller token cannot answer the card (as itself, in the owner's chat words, or with a forged gateway request).

Files:

- `api.rs`: The gateway's routes, for the bot run whose token opened it alone: open, the tools of now, one call, close.
- `auth.rs`: A remote connector's secrets as its auth headers, each marked sensitive.
- `approve.rs`: A call that asks the owner first: run once on its permit, said `approved and ran`, else a card and `waiting for approval`.
- `call.rs`: One `tools/call`: the verdict asked again with the grants of now, a refusal answered without running, an approval match handed to its permit, and every attempt recorded.
- `canon.rs`: Canonical JSON (keys sorted at every depth, no spaces) and its sha256.
- `cli.rs`: `neobot mcp --bot <id>`: its stdout is the protocol's alone; why it stops goes to stderr.
- `door.rs`: The gateway process's way to the daemon: its port and its run's caller token, nothing else.
- `given.rs`: The bot's values of a connector's secrets, read from its store for that connector alone.
- `link.rs`: One connector of an open gateway: its connection, local or remote, and the tools it listed.
- `list.rs`: What the agent is listed: each open connector's tools the verdict lets this bot reach now.
- `local.rs`: A local connector: started in the bot's sandbox with its secrets in its own environment, JSON-RPC one line each way.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `names.rs`: The names the agent sees: `<connector>__<tool>`.
- `open.rs`: Opening a bot's gateway: the caller proven by its token alone, then every connector its role is granted.
- `permit.rs`: One-use permits keyed by the sha256 of connector, tool and canonical arguments, bound to their card: granted by the owner's yes alone, used by one call.
- `record.rs`: Every attempt as a row of `connector_calls`: hashes and the verdict, never the content.
- `remote.rs`: A remote connector: MCP's streamable HTTP through NeoBot's proxy as the bot.
- `renew.rs`: A signed-in remote connector's renewal on a 401: the daemon renews the sign-in, the connection sends the new bearer.
- `rpc.rs`: JSON-RPC 2.0 as MCP speaks it, both ways, and an HTTP answer's messages.
- `session.rs`: The gateways open now, each found again only by the run that opened it.
- `stdio.rs`: The gateway's side facing the agent: MCP over stdio, every tools message handed to the daemon.
- `summary.rs`: The short, secret-free summary of a call's arguments an approval card shows.

## `guard/`

The commit guard: git hooks in every project that refuse a commit whose message, file names or added lines contain one of the owner's forbidden words (whole word, any case; a word may be a regex). The words come only from
config.toml: the project's `guard_words`, else `[guard] words`; `guard = false` opts a project out.

Files:

- `cli.rs`: `neobot guard …` commands.
- `git_dir.rs`: Where a repository keeps its hooks.
- `immutable.rs`: The immutable flag on installed hooks.
- `install.rs`: Installing a project's guard hooks.
- `intact.rs`: Whether a project's guard is installed, current and not skipped by git.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `render.rs`: The text of the guard's hooks, with the words in it.
- `routes.rs`: The guard's HTTP API: every project's state, and installing one.
- `start.rs`: The guard at the daemon's start.
- `words.rs`: A project's forbidden words, from config.toml.

## `lab/`

The lab (docs/BOTS.md, part 3): the Coach's experiments on a bot's routine, one change each, decided by NeoBot alone
with the factory's `[lab]` rule. An experiment is a branch of the bot's repository (`bots/versioned.rs`) with exactly
one commit over its main; the Coach makes it in a draft worktree under `<home>/lab` and removes that worktree
(`factory/prompts/coach.md`), then starts it:

- `neobot lab start <bot> <branch> --routine <id> --mode live|replay|shadow --hypothesis "<text>"` → `POST /api/lab/start`:
  the bot (key, id or name), a routine of its own with no experiment running (one at a time per routine, either
  mode), the mode (`live`, `replay` or `shadow`, as `factory/prompts/coach.md` chooses: replay when the captures are enough, live when not and the change is small, shadow when not and the change touches what is sent), a hypothesis, and a branch that exists, has one
  commit over the bot's main and is checked out nowhere. A row in `experiments` (its `change` is the commit's subject);
  live or shadow, a worktree `<home>/lab/<bot>/<experiment>` on the branch; replay, its pairs start running.
- `neobot lab list [<bot>]` → `GET /api/lab?bot=<id or key>`: the experiments, newest first (change, hypothesis, mode,
  status, the comparisons judged, the numbers once decided), and the rule; the bot's tab «Mejoras» shows them.

**Live.** Each real run of that routine is the variant after the last one (kv `lab:last:<experiment>`; A first), chosen
when the run's turn starts, not when it came due (`manager/jobs.rs` calls `lab::fired` as the run is handed over, so a
run waiting its turn holds no seat): A in the bot's own folder (its main), B in the worktree, which is the bot's folder
for that turn — `lab::seat` maps `<home>/manager/<key>` to it for the session's folder (`manager/spawn.rs`,
`codex/setup.rs`), the memory and skills it is handed (`bots/context.rs`), the versions its row keeps and the commit
of what it changed (on the branch). Each run, A or B, starts a session of its own (`fresh.rs`: nothing is resumed, so what
the chat gathered weighs on neither side), in its folder, and B's session is never resumed. The seat is always freed: at the turn's end, or, when the run ends without a row (its session
ended, it was abandoned, or its agent never started), by `lab::released` (`manager::release_run`). The answer reaches the owner as always. At the turn's end (`turns/save.rs`) its row gets
`experiment` and `variant`; a B run is compared with the A run before it: the event `lab.compare` (no route) carries the
routine's task and the two answers as «Answer a» and «Answer b» in an order drawn at random, the letter B was shown
under kept in `lab_comparisons` (`b_is`), and the eval `ab-compare` (rubric `factory/evals/ab-compare.md`, judge
`judge`; the answers stand between marks drawn for that pair alone, as data) answers `a`, `b` or `tie`, mapped back to B's `win`, `lose` or `tie` (`manager/evals.rs` calls `lab::judged`).
A pair whose two answers are the same words is a `tie` at once, with no event and no judge.

**Captures.** Every turn of a bot (`turns/capture*.rs`) keeps what it read in `<home>/captures/<turn>.jsonl`: a first
line with what started it — the event or task handed over, the notes and the owner's words, never the memory NeoBot
lays before them (`manager/deliver.rs` calls `turns::told`) — with its conversation, routine and trigger, then one line
per tool call in order: the tool as its agent named it, its input and its result as the agent saw it (Anthropic's
`tool_use` blocks matched with the `tool_result` blocks of its `user` lines; Muse's `tool.result`, its command as the
input). A capture stops at 256 KB (a result is cut on a character, and `full` says so). The captures of the last 10
runs of each routine of the bot are kept (its turns outside a routine count as one routine more); older ones go.

**Replay.** `--mode replay` (`replay.rs`, `replay_side.rs`) commits the bot's folder and takes the routine's last
`inputs` captured runs whose outside reads all kept a result (`replay_runs.rs`; fewer than min(5, `inputs`): refused,
saying to use live or shadow). For each, oldest first, A (a detached worktree of the bot's main) and then B
(one of the branch) under `<home>/lab/<bot>/<experiment>/<a|b>-<turn>` do the capture's start again as a loose turn
of the bot's role (`manager::run_turn`), handed that worktree's memory and skills. A tokio task-local marks the turn:
`lab::seat` resolves every folder of the bot to the worktree (the session's folder, its context, Muse's workspace),
and `sandbox::wrap` gives its processes the proxy caller `replay:<experiment>`, to which the proxy refuses every
destination but, over TLS, the `model_hosts` of the bot's agent (factory.toml, plus the agents' catalog: `reach.rs`),
since a cloud agent cannot think without them; an agent with none is not replayed. Its `replay_args` are added
(Codex's own web search and apps, which no hook hears, are off). In NeoBot's `PreToolUse` hook (`server/hook.rs`, Anthropic's and Muse's alike) a call from the
worktree that reads outside — the web, a browser or any MCP tool, `neobot safari` or `neobot teach`, shell commands
with network such as curl, ssh or git fetch (`outside.rs`) — is refused with the next unanswered call of the same
call in the capture that asks for the same thing as its reason, «Recorded result (replay): <its result>»
(`answers.rs`; what a call asks for is its tool and its URL, query or command with its spaces normalised, never its
descriptive fields, `call_key.rs`); one the capture has no such call for is refused as «not available in a replay» and
counted as missed. A call that would change something outside the worktree (`outward.rs`) is refused. Local calls (its worktree, which the rules let it write in as its
own folder, and its skills) run as always. Nothing of a replay reaches the owner: its turns run in a conversation of
their own, `replay-<experiment>-<turn>-<A|B>`, which no chat, phone, room or card knows (`neobot say` and
`ask-owner` refuse it as an unknown project), and the hook refuses `neobot say`, `ask`, `ask-owner` and `chat` in it.
Each turn's row gets `experiment`, `variant`, the routine and the trigger `replay`; the worktrees go after each turn,
and an uncounted first turn warms what both sides share (`replay_warm.rs`, variant `W`), the pairs alternate
which side goes first, and each B is compared with its A like a live run (`lab.compare`, `ab-compare`), unless a side missed a call, used tools no hook heard, or both sides failed (that last
holds in live and shadow too): that pair proves nothing, is kept as a comparison with the verdict `void` (`void.rs`)
and is never a tie, a win or a loss (the decision line says how many). Under min(5, `inputs`) comparisons nothing
is adopted. A replay is refused, with the reason, when the network cannot be closed for it
(`sandbox::closes_network`: the sandbox is off or cannot run here, or the daemon itself runs in one). A replay the daemon was
restarted in the middle of (its pairs ran in memory) is decided at the next start when all its pairs ran and were
judged, and discarded with a line saying so otherwise (`interrupted.rs`), never later on the pairs it happened to have.
Once every pair has run (kv `lab:replayed:<experiment>`) and been judged, it is due.

**Shadow.** `--mode shadow` (`shadow.rs`) keeps every real run of the routine as it is — A, in the bot's own folder,
its answer to the owner as always — and starts its B beside it at the same moment (`lab::fired`, called as the run is
handed over by `manager/jobs.rs`, handed the same task): a loose turn of the bot's role (`manager::run_turn`) in the
experiment's worktree, with the same instructions and recent chat as A's fresh session, handed its memory and skills, in a conversation of its own, `shadow-<experiment>-<pair>`, that
no chat, phone, room or card knows (kv `lab:pairs:<experiment>` counts the pairs; while a B is still on its way, a
new real run is no pair). A tokio task-local makes the worktree every folder of the bot for B's turn (`lab::seat`),
and the rules let it write there (`lab::in_replay`). Its reads are live (the web, a browser, `neobot safari read`),
but NeoBot's `PreToolUse` hook refuses every outward action from that worktree (`shadow_open.rs`) as «not
available in a shadow run»: a `neobot` command that is not a plain read — `say`, `ask`, `ask-owner`, `chat`,
`safari open`, `teach`, rooms, cards, lanes — and shell commands that send, buy, delete or publish (curl or wget
with a writing method, data or a form, `gh` writes, a push or a publish, mail, ssh and rsync, `open`, AppleScript, a
deletion outside the worktree: `outward.rs`); in a browser a click on a control that sends, buys, deletes or
publishes (English or Spanish), a submitted text, Enter, a form, an upload, an accepted dialog or code in the page,
and any other MCP tool unless its name says that it reads and none of its words sends, writes or deletes (`outward_browser.rs`). The run in the bot's own folder
is never touched by it. B's row gets `experiment`, `variant` B, the routine and the trigger `shadow`; A's row
`experiment` and `variant` A at its end (`turn.rs`). Each pair — the same input at the same time — is compared once
both have ended, in either order (`shadow_pair.rs`, then `lab.compare` and `ab-compare` as above), and decided by
the same `[lab]` rule, never while a run of the pair is on its way. The «Mejoras» tab says, while it runs, that the
shadow costs double.

**The decision.** The rule is the factory's `[lab]` as it was when the experiment started (kept in `experiments.rule`,
`fixed.rs`): a change of the factory never moves the goal of a test under way. With `inputs` comparisons judged, a
replay's pairs all run and judged, or `max_days` after its start (the scheduler's tick), and no run of the bot on its
way, B is adopted; one whose days ran out with fewer than min(5, `inputs`) comparisons proves nothing and is discarded,
saying so (`due.rs`). B is adopted when it is not worse (wins and ties at least `not_worse_share` of the comparisons, and
wins at least as many as it loses), has no new failure (a B run failed where its A did not) and no 👎, and saves at
least `min_saving` of A's mean tokens, dollars or time (each only when most pairs, one by one, or four of every five for time, saved
that much too: `saved.rs`). Adopted, the bot's folder is committed and the branch is merged
into its main (the live folder is that checkout, so the next run uses it: a changed skill is read again, in a session of its own, `bots/handed.rs`; where main moved on the same lines, B's
tested text wins, but a note main added to `memory.md` meanwhile that the merge lost is written back as a new note,
`adopt.rs`; a skill or other file of the branch that main changed meanwhile was never tested like that, and discards
it, as a merge that still fails does); in both
cases the worktree and the branch go. The row keeps the status, the end and the numbers (A's and B's runs, mean
tokens, dollars and time, the tally, the savings and the line), and one line is emitted in the bot's conversation:
`lab.adopted` (routed to its chat and the phones) or `lab.discarded` (its chat only), as «Adopté gmail-safari v3:
−38 % tokens, misma calidad (8/8)» or «Descarté gmail-safari v4: peor calidad (3/8), −70 % tokens».

Files:

- `adopt.rs`: Adopting a branch: merged into the bot's main without losing a note written in memory.md meanwhile.
- `answers.rs`: A replay's tool calls in the hook: an outside read answered from the capture's call that asked for the same, one it lacks (counted) or one for the owner refused.
- `call_key.rs`: What a tool call asks for (the tool and its URL, query or command), so a replay answers only a call that asks for the same thing.
- `own_folder.rs`: What a run of an experiment is to the rest of NeoBot: a live B's worktree stands for the bot's own folder, and no history is handed to either side.
- `reach.rs`: What a replay's processes may reach (its agent's model and nothing else), and what its agent is also started with.
- `recorded.rs`: A capture's calls that read outside, as a replay answers them, and whether each has an answer.
- `replay_runs.rs`: The captured runs a replay does again: enough of them, each answerable.
- `due.rs`: Whether an experiment is ready to be decided by the rule it started with: due, not yet, or expired with too few comparisons.
- `fixed.rs`: The `[lab]` rule an experiment started with, kept in its row and read back to decide it.
- `fresh.rs`: A run of a live or shadow experiment starts a session of its own, as the run it is compared with does.
- `interrupted.rs`: A replay the daemon was restarted in the middle of: decided if all its pairs were judged, ended honestly otherwise.
- `scope.rs`: Inside a replay's or a shadow B run's turn, its worktree and routine, where the routine's `routines.toml` is read from.
- `void.rs`: A pair that proves nothing (both failed, a read the capture lacks, calls no hook heard): kept with the verdict `void`.
- `api.rs`: The lab's commands and routes: `neobot lab start` and `neobot lab list`, `POST /api/lab/start` and `GET /api/lab`.
- `blind.rs`: What the judge reads of a pair and what its answer means for B; when two answers are the same, so none is needed.
- `compare.rs`: A B run against the A run before it (or against the A of its shadow pair), judged blind by `ab-compare`, and the judge's answer mapped back to B's verdict.
- `decide.rs`: The decision once an experiment is due and none of its runs is on its way or waits for its judge: the numbers read, the rule it started with applied, the branch merged or deleted, the line emitted.
- `git.rs`: Git in a bot's repository as NeoBot: the branch's one commit checked, a branch merged (its text winning where main moved on the same lines), the worktree and the branch removed.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `numbers.rs`: An experiment's numbers from the store: each side's runs (mean tokens, dollars, time; left out those of a void pair or of one whose two runs failed, also late), B's comparisons, new failures and 👎, the void pairs, and the comparisons the judge has not answered.
- `outside.rs`: Which tool calls read outside (web, browser, MCP, `neobot safari`, network commands) and which would reach the owner.
- `replay.rs`: Replay mode: the task-local that marks a replay's turn, the proxy's closed caller, and the pairs run one after another.
- `replay_side.rs`: One side of a replayed run: its throwaway worktree, its turn on the capture's start, its row's experiment and variant.
- `replay_warm.rs`: A replay's uncounted first turn, so the side that runs first pays no cold cache for both.
- `saved.rs`: What B saves against A, and which saving counts (each measure: only saved in most pairs too; time, in four of every five).
- `rule.rs`: The `[lab]` rule applied to A's and B's numbers and the comparisons, and the one line that says the decision.
- `outward.rs`: Which shell calls of a shadow run act outward: `neobot` commands that are not plain reads, and commands that send, buy, delete or publish; a replay refuses them too.
- `outward_browser.rs`: Which browser and MCP calls of a shadow run act outward: a click on «Send» or «Comprar», a submit, a form, an upload, code in the page; an MCP tool that does not say it reads.
- `outward_words.rs`: The words in an MCP tool's name that say whether a shadow run may call it.
- `seat.rs`: Which folder a bot's turn runs in during an experiment, taken when its run starts and freed when it ends or never starts: live A, its own; live B, the branch's worktree; a replay's or a shadow B's turn, its worktree; a shadow run is A, its B started beside it.
- `shadow.rs`: Shadow mode: B of each real run started beside it in the worktree, in its own conversation, its row marked.
- `shadow_open.rs`: The worktrees shadow B runs are in now, the task-local of B's turn, and the hook's refusal of an outward action from one.
- `shadow_pair.rs`: A shadow pair's two ends, in either order; once both ended, B is compared with that A.
- `start.rs`: `neobot lab start`: the request checked, the experiment's row, and its worktree (live, shadow) or the captured runs of `replay_runs.rs` (replay).
- `store.rs`: The experiments' rows: one, the running ones, their worktree and the list the page shows.
- `turn.rs`: A bot's turn ended: a run of an experiment keeps its variant, a B run is compared, the experiment may be decided.
- `workfiles.rs`: The bot's working files in a worktree, which git does not hold: the bot's own for a live B run, a copy that does not stay for a shadow or replayed one.

## `lanes/`

Lanes: one agent, one plan item, one worktree slot of its own (`<tirith home>/lanes/<project>/slot-N`) on its own branch; no slot is kept or reused — every lane makes a fresh one.
The autopilot starts a project's open plan items when the machine has room; a lane whose process ends goes to the merge queue, where the checks (never the agent) decide that its item is done. A lane that merges, stops or
fails for good leaves nothing on disk: its uncommitted work is committed on its branch first (a merged lane's branch then goes), its git-ignored files that are no build cache move to the same path in the project (never
overwriting, kept ignored there), and its worktree is removed and pruned; its log, check logs and brief go at the merge, or 24 h after a stop or final failure. A project's shared build cache (`<home>/build/<project>`,
where lanes, stages and every manager, bot and CEO session build through `CARGO_TARGET_DIR`) and its merge folder go after `limits.build_idle_minutes` with no lane, stage or manager of it working; the same pass at
startup applies all of this to whatever already meets the rules.

Files:

- `ask.rs`: A lane asks the manager and waits; the answer reaches it live or in a resumed run.
- `autopilot.rs`: The autopilot: in round-robin over the projects, the next open plan item that no lane works on becomes a lane.
- `blocked.rs`: Why a project cannot start a lane now, and what fixes it.
- `brief.rs`: A lane's brief: the factory's `brief` prompt with its placeholders filled from the lane, its step and its project.
- `brief_parts.rs`: The parts of a lane's brief taken from the plan, the other lanes and the project's rules.
- `builds.rs`: The shared cargo build folders: one per project, removed for projects with nothing under way when the disk runs low.
- `cleanup.rs`: A lane that ended for good leaves nothing behind: its work saved, its ignored files kept, its worktree and (merged) branch and logs gone.
- `delete.rs`: `neobot lane delete`: a lane thrown away with its branch, its worktree and its open plan item.
- `disk.rs`: Free disk space: how much is left, and one card to the owner when it runs low.
- `ended.rs`: The agent's process is gone: where the lane goes next.
- `exclude.rs`: Keeping what a lane moved into a project out of git there, through `.git/info/exclude`.
- `failed.rs`: A lane whose agent ended out of credits, or without commits: the next agent of the chain, or failed.
- `heal.rs`: What earlier cleanups moved into a project but git still sees as new gets its folder excluded.
- `idle.rs`: An idle project's build cache and merge folder go once it has been long enough without work.
- `ignored.rs`: A lane's git-ignored files: build caches stay behind, the rest moves to the project without overwriting anything.
- `keep.rs`: Before a worktree goes: its git-ignored files that are no build cache move to the project, kept out of git there.
- `launch.rs`: Launching a lane's agent in its slot: a new slot for a lane without one, the brief or a resume of its session.
- `live.rs`: Live messages for a lane whose agent still runs: told while it works, taken once.
- `logscan.rs`: Reading a lane's agent log: how long it has been quiet, whether its run ended, and its last commands (an end is judged by the current run's lines alone: never by an earlier agent's «out of credits»).
- `long.rs`: A lane running for long is reported to the manager once, with its last commands.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `pick.rs`: Starting the next lane: the oldest queued one, or else the autopilot's next item, in a project not blocked now.
- `scheduler.rs`: The scheduler loop: every few seconds (or when woken) it keeps the house, watches the running lanes and starts the next one when there is room.
- `send_back.rs`: Sending a lane's work back to its agent with the reason, twice at most.
- `session.rs`: A codex lane's session: the thread id its current run started, kept on the lane so it can resume.
- `skip.rs`: Plan items the autopilot leaves alone: role items, items waiting for the owner, and items waiting for another open item.
- `spawn.rs`: Starting the agent's process for a lane, its output appended to the lane's log, and ending the lane when it exits.
- `stamp.rs`: Time stamps of lanes and runs as seconds since the epoch.
- `start.rs`: Starting a lane: an open plan item no lane works on becomes a queued lane with an id of its own.
- `stop.rs`: Stopping a lane: its agent ends, the lane is stopped and cleaned up.
- `sweep.rs`: The pass at startup and then every few minutes that applies the cleanup rules to everything that already meets them.
- `watch.rs`: Watching a lane whose agent runs: its run ended in the log, it runs long, it went silent, or its process is gone.
- `worktree.rs`: A lane's worktree slot: a fresh one on its branch when it starts, gone in full when it ends.

## `manager/`

The managers: one long-lived agent session per project (stream-json in and out), and one for what belongs to no project. A message goes to the session of its project, which starts with its first message and rests after the manager role's quiet
`idle_minutes`; its text, its tool calls and the end of each turn go straight to that project's chat on the page.
Everything else the daemon has to say is an event: `emit_event` keeps its row, shows it on the page and passes it
to the factory's routes. A turn's answer reaches the phones only when the owner started that turn or an event
that started it has `phone = true` in its route.

A routine's run (due on its schedule, the page's Run now, a webhook: `emit_event` for `schedule` or `webhook` through
`run_routine`) is a turn of its own with its own trigger and routine, never gathered with another event or message: it
waits in the conversation's queue of runs (`jobs.rs`), in the order they came due, and is handed over alone when the
conversation is free (no turn running or waiting to start, no message on its way, no report waiting) — at once, or when
the turn before it ends (`stream.rs`, `exec_end.rs`). Starting its turn is what seats it in a live experiment and applies
its own effort (`bots/tuning.rs`). What the owner or a room writes meanwhile is handed over as soon as it comes, in a
turn of its own, ahead of the runs still waiting; several messages handed to one live session are several turns, each
with the trigger of its own message (`turns/live.rs`). Reports waiting for the next turn are never dropped silently: the
newest 30 are kept and the message says how many older ones were left out. A trigger belongs to the message it was
composed with (`compose.rs`): a message that starts no turn (`/model`, words kept while a task is taught) sets none.
What a paired phone writes is the owner's words: `owner-mood` judges it like a line typed on the page (never another
agent's line in the room).

Files:

- `answer.rs`: The owner's answer to an open card or to a waiting lane.
- `ask.rs`: `neobot ask-owner`: a new card for the owner.
- `blocks.rs`: An assistant message of a stream-json session: its text and its tool calls, in the chat and on the phones.
- `held.rs`: What a routine's run says waits for its end: on the phones only when the run was good and has news. A failed one is the one line of `expected/`.
- `relayed.rs`: An agent's answer of a card; an approval or a payment takes the owner's own words written after it was asked, and is kept in those words; a card asking for a connector call is never the agent's to answer.
- `owner_words.rs`: What the owner last wrote in each conversation (kv `owner_said:<conversation>`), kept as it comes in, never read back from the chat.
- `cards.rs`: A card's end: its final state (an answered or closed approval ends the wait of the turns that waited for it, and settles a gateway call's permit), the line that says so, and closing one that is no longer needed.
- `compose.rs`: The message a conversation is handed: the memory, the waiting reports and the words of the owner, a room, an event or a routine, with what starts its turn.
- `deliver.rs`: Handing a conversation its messages: to its live session, or waiting for the session that starts for them; with nothing to say, the next routine's run when the conversation is free.
- `evals.rs`: The evals: an event judged in the background by its judge role, kept as a score.
- `events.rs`: Events and their routes: who hears an event, as a chat line, a turn now, a routine's run of its own or a note with the next turn.
- `exec.rs`: The exec session: one exec turn of Codex, Copilot or Muse per hand-over (a routine's run alone), resumed under its kept session id.
- `exec_copilot.rs`: What a Copilot turn says while it runs: its messages and its tools' calls.
- `exec_end.rs`: The two ends of an exec turn: its process runs, and its answer goes to the chat, the phones and its rooms.
- `exec_stream.rs`: What an exec turn says while it runs: Muse's stream, Codex's and Copilot's events, in the chat and on the phones.
- `interrupt.rs`: Stopping the turn a conversation runs now.
- `jobs.rs`: The runs of routines waiting for their conversation: queued, handed over alone when it is free (nothing joins a run: `waiting.rs`), their seat and effort taken when they start and released when they end without a turn.
- `judged.rs`: Reading what a judge answered, and which card an event's line names.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `open.rs`: The open questions: the cards still open and the lanes that wait for an answer.
- `progress.rs`: The short line a tool call shows on the phones.
- `prompt.rs`: What a conversation's agent is told besides its messages: its role's instructions and the recent chat.
- `queue.rs`: Reports and events for a conversation that wait for its next turn: the newest 30 kept, the older ones counted.
- `unheard.rs`: A note for a conversation's next message, in the store until a message hands it over, so a restart loses none.
- `run.rs`: A conversation's sessions, one after another while messages wait, started again more slowly when they fail young.
- `commands.rs`: The owner's commands that NeoBot answers itself, with no turn (`/status`, `/models`, `/model`).
- `say.rs`: What the owner, a room or an agent writes to a conversation (a paired phone's words are judged as the owner's).
- `session.rs`: The stream-json session of a conversation: started with what waits for it, read until it ends or rests.
- `session_end.rs`: The end of a stream-json session: what its dead turn left, its process, and what waits again when it never got ready.
- `spawn.rs`: Starting a streaming chat agent, and the answers to its permission requests.
- `stale.rs`: Open questions that no longer apply, closed by NeoBot.
- `stream.rs`: Reading a stream-json session's output: its start, its words and tools, its turn ends and permission requests.
- `talk.rs`: One conversation's state, kept under its project's id (or "" for the one of no project): its session, its messages on their way, its waiting reports and routine runs, and what lists or ends its sessions.
- `turn.rs`: The loose turn: one fresh turn of a `session = "turn"` role, its text the answer.
- `turn_read.rs`: Reading a loose turn of a stream-json agent: its text, its usage and its permission requests.
- `unstarted.rs`: A routine's run whose agent does not start is tried a few times and then has failed (the owner's own words keep being tried), and one handed to a session that died without a word has failed too.
- `verdict.rs`: A judge's verdict: the last line it wrote in its rubric's shape (none: it is asked once more).
- `waiting.rs`: A routine's run has its agent's turn to itself: what comes meanwhile waits and is handed when that turn ends.
- `typing.rs`: A turn's rooms and its typing on the phones: where it shows its progress, when it starts and stops.

## `merge/`

The merge queue. Up to five lanes of one project ride one train: in the project's merge slot (`<tirith home>/lanes/<project>/merge`) their branches are merged with main, the project's checks and the items' own checks run, and
only a green train moves main (fast-forward), with the plan ticks in the same commits. A red train is split into single lanes; a red single lane is first compared with main, so a broken main never blames a lane. A project's
deploy stages are no checks: they run after main has moved, on that main (`deploy`).

Files:

- `batch.rs`: Which lanes ride the next train.
- `deploy.rs`: The deploy stages run on main as it is now, after a merge, retried with a growing wait when they fail.
- `env.rs`: The environment an item's check borrows from the pipeline stage it belongs to.
- `land.rs`: A green train lands: pending plan edits committed, items ticked in the merge slot, main fast-forwarded.
- `landed.rs`: After main moved: the deploy is owed, main is green, the push goes out, the lanes are merged and gone, and everyone hears it.
- `limits.rs`: A stage's time limit: the manager's, else the one learned from past timeouts, else the default.
- `locks.rs`: The locks of the merge queue: the merge slot, the main checks already run and one stage at a time per project.
- `main_check.rs`: One stage on a main commit in the merge slot, remembered per commit.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `news.rs`: What a landed merge tells: the lanes still working, the manager, and the page.
- `paths.rs`: Which stages a change asks for: `paths` globs, and deploy stages that publish main instead of checking a lane.
- `pipeline.rs`: Pipelines: one row per train, main check or deploy, with its runs as stages.
- `prepare.rs`: Readying the merge slot on main's head for a train, and what happens to its lanes when that fails.
- `red.rs`: A red main: `main_red:<project>` is "<stage> at <sha8>"; the queue tests it again when main moves and clears it when green.
- `red_train.rs`: A red train: several lanes are merged alone next; one lane goes back, unless main itself is red.
- `retry.rs`: Retry from the page: a merge pipeline's lanes go back to the queue, a main or deploy pipeline runs again on main.
- `riders.rs`: Who rides a train: lanes with commits are merged, lanes without are ticked if their check passes on main.
- `slot.rs`: The project's merge slot: one worktree where trains, main checks and deploys run.
- `stage.rs`: One stage of a pipeline, run in a folder under the sandbox, one at a time per project, within its time limit.
- `train.rs`: A train: up to five lanes of one project merged together in the merge slot, checked once, then landed on main.
- `train_checks.rs`: What checks a train: the files it changes, the stages they ask for and the merged lanes' item checks.
- `worker.rs`: The queue's loop: it rechecks a red main, rides the next train, or deploys what is owed.

## `metrics/`

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
when and who are configuration, not a decision in `metrics/`, and it is due like a bot's routine (one of its
times came and no daily pass ran since, with the route's `catch_up`; its daily passes are the `ceo_reviews` rows whose `metric` names
`daily`, counted in the same 24 h as its `max_per_day`). Either way its brief (`brief`) carries
the measures, the measures and eval scores grouped by factory version — merged lanes and their first-try
share, the `scores`, the cards and the events per stamp, oldest version first — the last lanes with their
stamp, failing checks, plans and owner questions; the role's prompt (`factory/prompts/ceo.md`) has it
read `factory/` and propose at most one change to it per pass, as an item of step 17 of this repo's plan with
its check and its lane started (`neobot plan add` and `neobot lane start`, which its read access allows). A
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
(Mondays 09:00) and on `routine.worse`; until `neobot lab` exists it proposes one experiment in at most 3 lines and
changes nothing, or answers «Sin novedad.». Its `writes = ["bots", "lab"]` confines it (`rules/confined.rs`): its
edits land only in the bots' folders, `<home>/bots` and `<home>/lab`, never in a hidden folder there. An event heard while a pass of its role runs (a second `routine.worse`) is kept and run, with the others heard meanwhile, as one pass when it ends (`queued.rs`), never lost.

`routine.worse` (`metrics/worse.rs`): every computation looks at each bot's routines with runs in `turns` (the bot's
own role in its own conversation); one whose last 5 runs cost 30 % more than the 10 before (mean tokens, all four
kinds, or mean dollars) or of which 2 failed (a run that `waiting` for the owner's yes is not a failure; the B runs of experiments are not runs of the routine) is emitted once per episode — kv `routine_worse:<bot>:<routine>`, cleared
when it no longer holds — with the bot, the routine and those numbers.

Files:

- `api.rs`: The metrics' clock and the page's API: the computation every minute, the last result and the reviews.
- `book.rs`: What the metrics keep between computations: the last result the page gets, since when each project is idle, and the turn roles whose pass runs now.
- `bots_brief.rs`: The "bots" brief a pass reads (the Coach's): each bot's numbers, routines, skills with the sections they lack and memory sizes, and the role's own last pass.
- `brief.rs`: The facts a CEO pass reads: red measures, measures and scores by factory version, last lanes, failing checks, plans, questions.
- `cards.rs`: Needless cards: the share of the cards resolved in 24 h that were closed without an answer or judged decidable alone.
- `compute.rs`: One computation of every measure, in the order the page shows them, from what one moment of the daemon holds.
- `episode.rs`: A measure's red episode, kept across restarts.
- `facts.rs`: Lines the measures' facts share: the owner's open questions, a paused project's reason and fix, an empty plan's fix.
- `flow.rs`: How the merged lanes flowed: the median minutes from start to main, and the share merged at the first try.
- `idle.rs`: Work standing still: projects with open items and nothing running, and running lanes gone quiet.
- `interruptions.rs`: What reached the owner unasked: phone messages of turns no owner message started, notices on the phones, cards opened.
- `metric.rs`: One health measure: how it is built from the factory's description of its id, and the line a CEO pass reads.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `mood.rs`: Satisfaction from the owner-mood scores of the last 24 h, per owner message; judged from what the owner writes anyway, never asked.
- `owed.rs`: Which time of its schedule a turn role's pass is owed for.
- `pass.rs`: When a turn role's pass may run by itself: its daily budget, and its scheduled pass (none is owed from before the home first started, kv `born`; until it is a day old «En main (24 h)» is never red).
- `queued.rs`: The passes an event asked for while a pass of the same role ran: kept, run as one when it ends.
- `plans.rs`: The plans as a CEO pass reads them: each project's steps with their open and done items, its autopilot and its main.
- `review.rs`: The pass of a turn role: who reviews a long red stretch and when, the pass turn itself with the brief its role names, and the page's "CEO now".
- `roles.rs`: Any turn role's pass: the brief its `brief` key names, its `schedule` route and the events routed to it.
- `tick.rs`: One computation: episodes opened, fixed, noted and reviewed, the other turn roles' scheduled passes and the routines gone worse; the page gets the result.
- `trouble.rs`: What went wrong or waits: projects whose main is red, lanes failed in 24 h, questions open for the owner.
- `unapproved.rs`: The `unapproved` measure: bots' turns of 24 h the eval says acted without an answered approval card.
- `worse.rs`: A bot's routine gone worse (its last 5 runs 30 % dearer than the 10 before, or 2 failed): `routine.worse`, once per episode.
- `work.rs`: The work there is and what of it reached main: the autopilot projects with and without startable items, and the lanes merged in 24 h.

## `models/`

Models per role and the fallback chain.

Roles are the factory's: "manager" (a project's chat), "manager-general" (the system bot of no project), "worker" (lanes),
"ceo" (one fresh turn) and "judge" (the evals' cheap turn role, with no prompt of its own, so code never
picks it as a standalone turn role), each with its `session`, `scope`, `extends`, `access`, `idle_minutes`,
`max_per_day` and `runs_on` in `factory/factory.toml`; code finds a role by that shape, never by a fixed
list of names. Each has an agent alias, a model, an effort and a fallback
chain. The effective setting is the owner's override from the page (kv `models:<role>`), else
`[roles.<role>]` of `config.toml` laid over the factory's role. Agents are aliases
(`[agents.<alias>]`); the real command lives only in config.toml. Copilot's enterprise host is asked by the guide and kept in `<home>/auth` (`setup/host.rs`). Optional per alias: `model_hosts` and `replay_args` (for the lab's replays), `model_flag`, `effort_flag` ("" = none) and `resume_args`
(`{session}` and `{message}` are filled in); each alias's flags come from the factory.
Every model knows its agent from the configuration alone: its group in `[models.choices]` (a table
`alias = ["model", …]`), Codex's catalog, or the `model_prefix` its `[agents.<alias>]` declares (by default
its command's program name and `-`). A pair of an agent with another agent's model is refused by
`POST /api/models/<role>`, `neobot models set` and `/model`; a model sent alone brings its agent; a lane on
such a pair fails at once with the reason, without spending a try; and at start a saved one is said in the
general manager's chat. The page lists each model with its agent and switches the agent when a model of
another one is chosen.

Files:

- `browser.rs`: Where the browser MCP's server runs from: node, the launcher of this repo and its profile mode.
- `browser_args.rs`: The browser MCP every agent run carries, in its own dialect.
- `gateway_args.rs`: The connector gateway (`neobot mcp --bot <id>`) a bot's run carries, in its agent's own dialect; a lane or a manager never gets it.
- `chat_argv.rs`: The argv of a streaming chat session for a role.
- `command.rs`: The owner's `/model` and `/models` chat commands.
- `credits.rs`: Running out of credits, and the next choice of a role's fallback chain.
- `flags.rs`: An agent's command line flags from the factory's `[agents.<alias>]`, and setting them in an argv.
- `http.rs`: The daemon's /api/models routes and the `neobot models` command that calls them.
- `lane_argv.rs`: The argv of a lane run.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `owners.rs`: Which agent a model belongs to, and why an agent cannot run a model.
- `phone.rs`: A paired phone's model requests.
- `roles.rs`: A role's effective setting and where it comes from.
- `save.rs`: Saving the owner's setting of a role, and its one-line form.
- `start.rs`: At start, saved settings that cannot run are said in the general manager's chat.
- `state.rs`: What GET /api/models gives the page.
- `updated.rs`: A role's setting with the owner's changes applied, checked before it is saved.
- `validate.rs`: The checks a role's setting passes before it is saved.

## `numbers/`

A bot's numbers, read from its own rows in `turns` (its role in its conversation; the judge's turns there and the B
runs of experiments do not count; a replay's or a shadow's run is in a conversation of its own) and its memory files: `GET /api/bots/<id>/numbers` gives

- `routines`: per routine of its card (then routines it no longer has, then `Chat` for its turns without one): `runs`,
  `success` (share ok of the runs that ended: one that `waiting` for the owner's yes is neither ok nor failed, and
  `waiting` counts them), `ms` (mean), `cost_per_good` (the cost of all its runs over the ones that ended ok),
  `down_per_10` (👎) and `corrections_per_10`, `score` (the mean judge's score, `scored` how many) and `trend`: the last
  10 runs against the 10 before them (`last` and `before`, each with `cost_per_good`, `ms`, `success`), null until there
  are more than 10.
- `skills`: per skill its runs read: `used`, `validated` (runs whose `checks_passed = checks_total`), and the mean tokens
  (in, out and cache) of its routine's runs (the routine of its first use) before that first use against after:
  `tokens_before`, `tokens_after`, `saved_percent`.
- `memory`: `tokens` (`own` and `shared`: characters / 4 of its memory.md and of `bots/memory.md`), `notes` (every
  numbered note with how many of its last 30 runs cited it), `uncited` (the bot's own notes none of those runs cited
  and written at least 30 runs ago, by the history of its memory.md: candidates to delete, which NeoBot never deletes;
  a shared note is never proposed from one bot's runs), `misses` and `missed` (owner messages in its conversation that owner-mood labelled `repeat`).

The page shows them in the bot's «Números» tab (`web/js/views/bots/numbers.js`) in short lines, and every answer of a
bot in its chat carries a discreet 👍 and 👎 (`web/js/views/chat/react.js`) that calls `POST /api/turns/<id>/reaction`
(the same one again takes it back) and stays marked; the answer's turn is found by its words and its time.

Files:

- `age.rs`: How old a note of a bot's memory is, in runs, by the git history of its memory.md.
- `memory.rs`: A bot's memory in numbers: its loading cost in tokens, each note's citations, its own notes uncited for 30 runs and the misses.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `routines.rs`: A bot's numbers by routine: its runs, success, time, cost per good run, 👎 and corrections per 10, mean score and trend.
- `rows.rs`: A bot's own turns as the numbers read them, the first first.
- `skills.rs`: A bot's numbers by skill: uses, validated uses, and its routine's tokens before its first use against after.

## `oauth/`

Signing a bot in to a remote connector (`url`): `neobot connector login <id> --bot <bot>`, the owner's in their own
terminal (an agent is refused, and its routes `/api/connectors/…` are the owner's alone by `server/scope.rs` and by a
check of their own). A local connector (`command`) needs no login and is refused one: it is given the secrets its
definition names, as before.

The daemon (`start.rs`) discovers the server (`discover.rs`): its protected-resource metadata (RFC 9728) at
`/.well-known/oauth-protected-resource<path>`, then at the origin; its first `authorization_servers` entry's metadata
(RFC 8414 `/.well-known/oauth-authorization-server<path>`, then OpenID's `openid-configuration`); without either the
server's origin with `/authorize`, `/token` and `/register`. A server that lists PKCE methods without `S256` is
refused. NeoBot registers itself (`register.rs`, RFC 7591 dynamic registration, a public client: `token_endpoint_auth_method
= "none"`) at the `registration_endpoint`, with the daemon's own callback `http://127.0.0.1:<port>/oauth/callback` as
its one redirect; a server that offers no registration cannot be signed in to. The sign-in address carries `S256`
PKCE (`pkce.rs`, a 32-byte verifier that never leaves the daemon), a random state, the connector's url as `resource`
(RFC 8707) and the resource's `scopes_supported`. The CLI opens it in the owner's browser (`open`, `xdg-open`; over
SSH it prints it) and waits on a ticket (`GET /api/connectors/logins/<ticket>`: waiting, done or failed with why), ten
minutes at most. The Connectors page's Login (`connector_page/`) begins the same sign-in with
`POST /api/connectors/<id>/login` (`{bot}`), opens the address in a new tab and waits on the same ticket. A failed
discovery, registration or callback is kept as that bot's `error` with the connector (`connector_page/seen.rs`), and
the why the reply, the ticket and the page say is scrubbed of every secret first (`connector_page/scrub.rs`).

The code comes only to `GET /oauth/callback` on 127.0.0.1 (`callback.rs`), the one route without a token
(`server/admit.rs`): a sandboxed caller and an agent's token are refused; its state must be a sign-in under way
(`pending.rs`, in memory, taken once, ten minutes); the code is exchanged with its verifier at the token endpoint
(`token.rs`). The tokens go in that bot's secret store (`secrets::keep`) under reserved names (`names.rs`):
`TIRITH_OAUTH_<ID>_ACCESS`, `TIRITH_OAUTH_<ID>_REFRESH` and `TIRITH_OAUTH_<ID>_GRANT` (the token address, the
client, the resource and the expiry), `<ID>` the connector's id in capitals with `-` as `_`. The owner cannot set a
`TIRITH_OAUTH_` name (`neobot secret set` and its route refuse it) and no connector's `secrets` may name one
(`connectors/row.rs`), so the token reaches its own connector alone; `neobot secret list` shows the names,
`neobot secret remove` signs out, and deleting the bot removes them with its other secrets. No route answers a token.

The gateway (`fresh.rs`, `gateway/renew.rs`) reads the bot's access when it opens that remote connector and sends it
as `Authorization: Bearer`, before the connector's own secrets; one that ends within a minute is renewed first with
the refresh token, and a 401 from the server renews it once and sends the new one. Renewals go one at a time (a
server may rotate the refresh token) and are kept in the same names. Another bot granted the same connector uses its
own sign-in or none: nothing of this bot's store or account reaches it.

Manual acceptance servers (never in the automated checks): Asana `https://mcp.asana.com/v2/mcp` and Calendly
`https://mcp.calendly.com`, each as a `[[connectors]]` `url`, granted to a bot, then `neobot connector login
asana --bot <bot>` and a bot turn that calls one of its tools. Test `connector_oauth_flow`
(`rust/tirith/checks/connector_oauth.rs`) runs offline against one fake server that is the discovery, registration,
authorization, token, refresh and MCP endpoints: the owner alone begins it, a local connector is refused, the PKCE
verifier matches, a forged or reused state is refused, the tokens are kept under the reserved names in that bot's
store alone, the gateway renews an access that is ending and one the server revoked, a second bot granted the same
connector sends no token and gets none of its tools, and no token is in anything an agent or a route was given.

Files:

- `api.rs`: The owner's routes of a sign-in: begin one, ask how it stands; and the callback's.
- `callback.rs`: The daemon's 127.0.0.1 callback, the one place a code comes in.
- `cli.rs`: `neobot connector login <id> --bot <bot>`: the owner's, waiting until the callback took the code.
- `discover.rs`: OAuth discovery of a remote MCP server from its `.well-known` metadata.
- `fresh.rs`: The gateway's bearer for a remote connector, renewed when it ends or was refused.
- `keep.rs`: A sign-in's tokens in the bot's store under the reserved names, and read back.
- `mod.rs`: Declares the folder's files and re-exports what other modules use; the daemon's OAuth client.
- `names.rs`: The reserved names a connector's sign-in is kept under.
- `pending.rs`: Sign-ins under way by their state, and how each ended by its ticket.
- `pkce.rs`: PKCE (S256) and the state.
- `register.rs`: Dynamic client registration, when the server offers it.
- `start.rs`: A sign-in begun: discovery, registration and the browser's address.
- `token.rs`: The token endpoint: a code exchanged and a refresh.

## `plan/`

The plan: the project's markdown checklist (`## N. Title` steps, `- [ ] item` lines). Its content changes (new step, new item, ticked item, deleted step) are written to the file at once and committed in batches; which lane works on an item lives in the
daemon, not in the file. The daemon is the only writer. An item may carry its own check: `- [ ] Text (check: `cmd`)`; the merge queue runs it, and ticks a merged item in the merge commit itself.

Files:

- `add.rs`: Adding an item to a step of the plan.
- `change.rs`: Changing a plan's lines under the writer lock.
- `check.rs`: Item checks that can never pass.
- `delete.rs`: Removing an old step from the plan: its heading, prose and items, never renumbering the others. `neobot plan step delete <project> <number>` (`POST /api/plan/{project}/steps/{num}/delete`) refuses, without touching the file, a number that is not all digits, a step the plan lacks, or a step with a lane that is not merged (it names them); merged lanes do not block it.
- `flush.rs`: Committing the plan files NeoBot changed.
- `item.rs`: A plan's steps and their items, with the line each one is on.
- `lookup.rs`: Finding a project's plan and its items.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `parse.rs`: Reading a plan's markdown into its steps and items.
- `step.rs`: Creating the plan's next step.

## `power/`

`[power] keep_awake = true` in config.toml (false by default), read when the daemon starts: on macOS it runs
`caffeinate -i -s -w <its pid>`, so the machine does not sleep while the daemon lives and may again once it ends,
however it ends. `-s` holds on mains power only. On Linux nothing runs.

Files:

- `argv.rs`: The command that keeps the machine awake, when there is one to run.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `start.rs`: Keeping the machine awake from the daemon's start.

## `projects/`

Adding a project (the page's "+", `neobot project add`): the folder gets a git repository, the commit guard and a
plan with a first step, then a `[[projects]]` block appended to config.toml (its existing text is never rewritten).

Files:

- `add.rs`: Adding a folder as a project, step by step.
- `add_plan.rs`: The plan of a project being added: what it has, and its start when it has none.
- `block.rs`: The `[[projects]]` block appended to config.toml.
- `cli.rs`: `neobot project add`.
- `colors.rs`: The colours a project is shown with.
- `config_file.rs`: Config.toml, written one change at a time.
- `folder.rs`: The folder a project is added from.
- `folders.rs`: Git repositories that could be added as projects.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `pick.rs`: The macOS folder picker.
- `project.rs`: The project as it will be written: its name, its colour and its id.
- `refusal.rs`: Why a real folder cannot be a project, the first reason that holds.
- `register.rs`: Writing a new project into config.toml.
- `repo.rs`: The git repository a folder to add is in, if any.
- `routes.rs`: The projects' HTTP routes and their list.

## `rooms/`

agentsmessenger.com rooms: public rooms NeoBot watches (`[[rooms]]` in config.toml), and private `sealed-` rooms where the owner talks to the managers from a phone, end-to-end encrypted, compatible with `am.mjs` and
the agentsmessenger.com chat page. One task per room long-polls it.

Who gives orders: only a phone paired with a one-use QR code (`tirith-pair <code>`) is the owner, in every room made
here (each project gets a room, each room gets every phone). Other agents reach the manager as information. After 30
agent messages in a row with nothing from a paired phone, NeoBot stops delivering and says so once, until a phone writes.
A `tirith-hook` line is never a line for people: only the one sealed by the room's webhook agent (kv `webhook:<bot>`)
is handed on, as its bot's `webhook` event (see Routines on an event from outside, above).

Files:

- `agents.rs`: Another agent's line: the owner told when it names them, a brake on long runs of agents, the manager given it as information.
- `api.rs`: The page's changes to a room: a new QR, a code for another manager, leaving it.
- `cli.rs`: `neobot room …` commands.
- `create.rs`: A private room this manager makes on the platform for a conversation, where its phones are paired.
- `deliver.rs`: What a poll decided is done: lines sent, the phones' model controls and commands answered, cards answered, the rest handed to the manager.
- `files.rs`: The pictures the phones send, kept as files for a day.
- `forget.rs`: The owner has NeoBot forget a phone.
- `handle.rs`: The rules for one message of a room, in order: only genuine events, who wrote it, what it says, pairing, history, and who it is for.
- `history.rs`: The room's last messages, as the page shows them.
- `hooked.rs`: What a bot's sealed webhook posts in its room: `tirith-hook <JSON>`, sealed by the platform's agent of that room.
- `hooks.rs`: A bot's sealed webhook on the platform: its own private room and the signed requests that make or delete it.
- `join.rs`: This manager joins another manager's room with the one-use code that manager gave (`sealed-<room>:<key id>:<code>`).
- `keeper.rs`: The rooms' start, and the loop that keeps them as the config and the phones need them.
- `listing.rs`: The list of the owner's rooms the phones get (`tirith-rooms [{"r": room, "t": name, "f": face}]`), one chat per project.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `outbox.rs`: What NeoBot says in rooms by itself, sent in order by one task.
- `pair.rs`: Pairing: a phone scans the one-use QR (`tirith-pair <code>`), another manager joins with its one-use code (`tirith-join <code>`).
- `poll.rs`: One long poll of a room: read, handle each new message once (the cursor moves past it), answer.
- `provide.rs`: A phone paired in one room is the owner's in all: every project gets a room and every room every phone.
- `public.rs`: The public rooms NeoBot watches, as config.toml lists them.
- `record.rs`: What is kept of a room and of the people in it (SQLite table `am_rooms`, one JSON row per room). Pairing codes live only here.
- `roster.rs`: Each paired phone is told who else is in the room (`tirith-roster <ids>`) when that changed.
- `routes.rs`: The rooms' HTTP API for the page and the CLI.
- `say.rs`: The owner writes in a room (page or CLI).
- `settings.rs`: What the config says about the rooms: the platform, the owner's names, this manager's name and the public rooms.
- `share.rs`: Every room this manager made gets every paired phone, and the phones are told the list of rooms when it changed.
- `show.rs`: What `neobot room …` prints: a link with its QR, the rooms, a room's messages.
- `store.rs`: The rooms in memory and on disk: one lock for all of them, one row per room, and the page told of each change.
- `svg.rs`: A room's link as a QR picture for the page: the one-use pairing QR (`?kind=invite`) or the permanent link.
- `tophones.rs`: What a project's conversation says reaches the phones in its rooms.
- `turns.rs`: The manager's turns for rooms: what a turn received stops being owed only when it ends, and what no turn answered is handed again.
- `typing.rs`: The phones see a turn typing, and its tools, while it runs.
- `uncite.rs`: A bot's memory citations (`[m<N>]`, `[s<N>]`) stay off the phones and the rooms: they are for NeoBot's count of the notes that serve (`turns::cited`), noise for the owner. The chat and the turns keep them.
- `unsent.rs`: Lines that must reach the phones (a routine that failed): kept (kv `rooms:unsent`) until sent, so a restart while the platform is away loses none.
- `view.rs`: The page's view of a room (no secret but the current one-use QR, which the page shows), and its chat links.
- `watch.rs`: One task per room listens to it, polling again and again, slower while the platform fails.
- `words.rs`: What a message says: its words (unsealed, a picture saved as a file), and whether it is a control rather than a line for people.

## `rooms/sealed/`

agentsmessenger.com's protocol, byte-compatible with its `am.mjs` and chat page: ed25519 identities (`{name, secretKey: hex(seed||pub)}`), canonical JSON, signed events, and the sealed envelope (x25519-xsalsa20-poly1305 with the
ed25519 keys converted to curve25519, one wrapped key per reader).

Files:

- `envelope.rs`: The sealed envelope: a message only its readers can open, signed inside by its author (sealed once: by a key thrown away, so not even for its author).
- `events.rs`: Posting signed events, registering this NeoBot's key when the platform does not know it yet.
- `http.rs`: Requests to the platform.
- `identity.rs`: This NeoBot's identity in the rooms: its ed25519 key in `<home>/am/<name>.json`.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `reading.rs`: Reading from the platform: its key, a room's pages and keys, and a new private room.
- `signed.rs`: Signatures and event ids: an event is genuine when its id, its author's signature and the platform's stamp all hold.
- `text.rs`: Hex, key ids, names and canonical JSON, as am.mjs writes them.

## `rules/`

Rules as code, checked in memory on every `PreToolUse` hook call: a lane writes only in its own copy, never touches main and never starts agents; commit checks are never skipped; no agent edits NeoBot's own files or the agents' settings; a project's `tirith.toml` adds its own. A refusal says what to do instead.

Files:

- `call.rs`: A normalized tool call as the rules read it.
- `check.rs`: One tool call against every rule, in order: the first refusal wins.
- `confined.rs`: A role confined by its `writes` key (the Coach): its edits land only in the bots' folders and the folders `writes` names; of the hidden folders only a skill's (`.agents/skills`).
- `hooks.rs`: The rules about the turn itself: commit checks are never skipped, and a chat's turn never waits.
- `judge.rs`: A judge (an eval's read-only turn role that nobody talks to) uses no tool but its agent's own book-keeping: what it is handed is data.
- `lane.rs`: A lane's rules: it writes only in its own copy, never touches main and never starts agents.
- `manifest.rs`: The rules a project's tirith.toml brings: its plan is written only by NeoBot, and its own `[[rules]]`.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `normalize.rs`: Every agent's tool call in the names the rules know.
- `owned.rs`: NeoBot's own files and the agents' settings: no agent edits them, except in the parts of NeoBot's home agents write in.
- `patterns.rs`: The shell command shapes the rules look for.
- `role_at.rs`: The chat role a hook belongs to, read from its folder.
- `summary.rs`: A tool call in one short line, for logs and the chat.

## `sandbox/`

Network sandbox. Every process NeoBot starts (lane agents, the manager, turn-role reviews, merge-queue checks) runs confined (`confine.rs`): on Linux in a network namespace of its own (`sandbox/linux/`, below), on macOS inside a `sandbox-exec` profile that denies outbound network except loopback and local sockets and denies Apple Events (macOS refuses them from `sandbox-exec` anyway, whatever the profile says), with its proxy
variables pointing at the daemon's own HTTP/CONNECT proxy (`sandbox/proxy/`). The proxy authenticates the caller by its token (the user name is for people to read; the same token is its `TIRITH_TOKEN` at the daemon's door, see `server/`), applies the block list, and records each new destination as an event.

The owner's Safari is read through the daemon instead: `neobot safari open <url>` and `neobot safari read [<url>]` (URL, title and text of the current tab, or of the first tab whose URL starts with the one given; the text is cut at 20,000 characters) ask the daemon — which runs outside the sandbox — to run that one fixed AppleScript, never a script the agent sends, and only for the chat roles (the managers and the bots, never a lane or the CEO). Since 2026-11-23 the owner lets the chat roles act there too, without asking, and no Safari setting is needed: `neobot safari click <target> [<url>]` and `neobot safari type <target> <text> [<url>]` send only words — the words on the element or its label, a list's option for typing — and the daemon runs its own fixed script (`assets/safari-ax.js`, JavaScript for Automation) through macOS Accessibility, the words as the script's arguments: it finds the element the page shows, presses it, or focuses a field and sets its value (a list opens and its option is pressed); a URL first brings that tab forward with Safari's own scripting. The first time, macOS asks the person to let NeoBot use Accessibility. It never reads cookies, storage or passwords. Which browser the agents act in is the owner's choice, said to a bot: `neobot browser` (which one and how), `neobot browser use neobot|safari` (kv `browser`, NeoBot's own until they choose) and `neobot browser login <url>`, which shows that page in NeoBot's own browser on their screen (its profile, with the agents' keychain switches) so they sign in once. In a shadow run these count as acting outward (`lab::outward`: every `tirith` command but a read).

Files:

- `blocks.rs`: The owner's block list: in memory (mirroring the `net_blocks` table), checked for every destination.
- `check.rs`: The block list for agent sessions NOT started by NeoBot (the owner's own sessions), checked in the hook.
- `cli/commands.rs`: Which command runs for the words given.
- `cli/list.rs`: `neobot net list`: the destinations seen, then the rules blocking.
- `cli/mod.rs`: Declares the folder's files and re-exports what other modules use.
- `cli/status.rs`: `neobot net status`.
- `caller.rs`: Whether a connection to the daemon comes from inside the sandbox, as the kernel tells it.
- `git.rs`: The git the daemon itself runs, confined like the rest: what a repository agents write in makes git run is an agent's.
- `confine.rs`: How this system confines (macOS's profile, Linux's namespace), and the launcher's way in from the command line.
- `env.rs`: The environment of a sandboxed process: the proxy for every tool that reads one, ssh through the proxy and the tool wrappers first on PATH.
- `handover.rs`: Handing descriptors to another process over a Unix socket (SCM_RIGHTS): Linux's launcher gets its relay's sockets, macOS's broker its caller's stdio.
- `jvm.rs`: Gradle and the JVM ignore HTTPS_PROXY; the JDK refuses Basic proxy auth on tunnels unless re-enabled.
- `launch.rs`: What a process NeoBot starts runs as: the program, its arguments and the environment the sandbox adds.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `owner.rs`: Who a sandboxed process belongs to.
- `mac/mod.rs`: macOS: whether a process runs under NeoBot's own profile (a browser's sandbox is told apart).
- `mac/socket.rs`: macOS: which processes of this user hold a loopback connection (libproc).
- `profile.rs`: The sandbox profile, the proxy's port and whether processes run inside the sandbox.
- `proxy_url.rs`: The proxy's address with a caller's credentials, for what the daemon reaches on that caller's behalf (a bot's remote connector).
- `quiet.rs`: `neobot quiet`: an agent that was given private values (a token, an enterprise's host) starts through it, and they are hidden in all it writes before that reaches a log, a chat or an event.
- `replay.rs`: Whether a replay's turn can run with the network closed: the sandbox must be in force for the processes NeoBot starts.
- `routes.rs`: The sandbox's HTTP routes: its status, the block list, the destinations seen and Safari.
- `rules.rs`: The shape of a block rule: a domain, an address or a CIDR range.
- `safari/act.rs`: `neobot safari click <target> [<url>]` / `type <target> <text> [<url>]`: osascript's arguments for the fixed Accessibility script (`assets/safari-ax.js`), and the AppleScript that brings a tab forward.
- `safari/choice.rs`: `neobot browser [use neobot|safari | login <url>]`: the browser the owner chose for the agents (kv `browser`).
- `safari/login.rs`: `neobot browser login <url>`: NeoBot's own browser shown on the owner's screen at a page, to sign in once.
- `safari/mod.rs`: Declares the folder's files and re-exports what other modules use.
- `safari/route.rs`: `POST /api/safari`: the lane and role come from the caller's token.
- `safari/run.rs`: Running the Safari scripts outside the sandbox, and the browser commands.
- `safari/script.rs`: The fixed AppleScript of `neobot safari open <url>` / `read [<url>]`.
- `start.rs`: Starting the sandbox's part of the daemon.
- `tags.rs`: Access-audit tags of one tool call, recorded by the hook handler as `access` events.
- `hidden.rs`: macOS: the profile's rules that keep the owner's token, the secret and the page's file from everyone confined.
- `token.rs`: The token of each who, its proxy password and its proof at the daemon's door: derived from `<home>/net.secret`, findable from itself alone.
- `tunnel.rs`: `neobot net tunnel <host> <port>`: stdin/stdout through the proxy (ssh's ProxyCommand inside the sandbox).
- `wrap.rs`: Putting a process NeoBot starts inside the sandbox.
- `wrappers.rs`: The wrappers in `<home>/net/bin` that turn off the inner sandbox of tools that start their own (on macOS Codex's is `sandbox/nested/`).

## `sandbox/linux/`

Linux's confinement, with no root and nothing to install: `neobot net run <port…> -- <program>` forks, the child
moves into a user, a mount (`hide.rs`) and a network namespace of its own (the same uid and gid, a loopback and no other device), binds
the forwarded ports there (the daemon's, the proxy's and `[network] loopback = [5432, …]`), hands the listeners
to the launcher that stayed outside and becomes the program; the launcher copies their bytes to the same ports of
the machine's loopback, passes signals on and exits as the program did. The program dies with its launcher. Where
the kernel refuses user namespaces, `neobot net status` says so with the fix (on Ubuntu 23.10 and later, once:
`sudo neobot net allow`, an AppArmor profile for this binary). Like macOS's profile it is a guard of the
network: local Unix sockets stay reachable.

Files:

- `allow.rs`: `sudo neobot net allow`: the AppArmor profile that lets this binary make user namespaces.
- `child.rs`: The confined side: into its own network, its ports handed out, then the program.
- `mod.rs`: Declares the folder's files, and the launcher's command line.
- `source.rs`: The address the relay calls from (127.0.0.2) at the daemon's port alone, by which the daemon knows a confined caller.
- `hide.rs`: The files that prove who calls bound over with /dev/null, in the confined process's own mounts.
- `namespace.rs`: The namespaces entered (user, network, mounts): the same user, a loopback that is up; root one further in, where the mounts are locked.
- `ready.rs`: Whether this Linux lets NeoBot confine, by trying it, and what to do when it does not.
- `relay.rs`: A forwarded port's connections copied to the machine's loopback.
- `run.rs`: The launcher: fork, take the listeners, relay, wait.
- `supervise.rs`: Signals passed on to the program, and its exit code.

## `sandbox/nested/`

macOS alone: Codex inside NeoBot's sandbox. macOS refuses a sandbox inside another ("sandbox_apply: Operation not permitted"),
so a `codex exec` or `codex review` found by its name on the sandboxed PATH asks the daemon, which starts it as two processes under
NeoBot's profile plus the limit it was asked: Codex's own, which alone keeps its state and renews its sign-in and writes in no
folder, and its exec-server, which alone runs the model's commands and patches. On Linux Codex's sandbox nests by itself.

What differs from Codex by itself on macOS: its commands get nothing of NeoBot but, for a role of NeoBot's, its API; MCP servers
that are programs run under the commands' limit; its other subcommands that run commands do not get a sandbox there, and the caller
is told; started by its whole path, so round the wrapper, it fails loudly; one message between the two parts holds 16 MiB at most.
It leans on the exec-server of Codex 0.165: a Codex without it is refused a limit, and says so.
With danger-full-access there is no limit to keep: it runs under NeoBot's profile alone. A permission profile or an
administrator's managed settings count as read-only, and a limited Codex's tool calls leave no `access` events.

Files:

- `answers.rs`: The bridge's way back: what runs a limited Codex's commands says it, a message a line.
- `around.rs`: What the daemon adds to a request and takes from no caller, and the request as the kernel names its paths.
- `ask.rs`: The client's talk with the broker: its own stdio and the request, then the exit code of what ran. Nothing is handed to a socket a confined process put in the daemon's place: what is handed holds the caller's environment.
- `asked.rs`: What a `codex exec` or `codex review` command line asks of Codex's sandbox, and that line without the words that ask it. `codex review` is `codex exec review`, which takes the limit.
- `bridge.rs`: The one way to the process that runs a limited Codex's commands and patches (`exec-server --listen stdio`): a WebSocket on loopback that takes one connection, of that Codex's own process as the kernel tells, and hands each message over that process's pipes. Nobody else reaches it: it listens nowhere. The bridge keeps its port for as long as the run lives, so nobody can stand in its place when Codex comes back for another connection.
- `broken.rs`: When a limited Codex is stopped while it runs: what would run its commands is gone, or was never its.
- `broker.rs`: The daemon's side: it starts what a caller under NeoBot's own profile asks, under one at least as tight, waits, and gives its exit code. Anyone else gets nothing: it is no way out of another sandbox. A caller that is gone takes it along, and so does a daemon that stops (`watch.rs`).
- `capable.rs`: Whether a Codex will hand its commands and patches to the process that has their limit. One that would not is never started under a limit: they would run in the process that holds its state.
- `carry.rs`: The bridge's two ways, once Codex's own process is on it. Whichever end is gone first says whether the run is lost.
- `client.rs`: `codex` inside NeoBot's sandbox (the wrapper of `<home>/net/bin`): its limit is read and the daemon is asked to apply it.
- `commands.rs`: What Codex's own sandbox refuses a command besides writing and the network; and the Codex program is no deputy of its commands.
- `doors.rs`: The daemon's two ports as a limit's profile knows them.
- `ending.rs`: Ending what the broker started. Codex gives each command a session of its own, so its process group is not all of it: every descendant goes.
- `flags.rs`: Codex's options as a command line is read: which take a value, and one option's name and value.
- `frames.rs`: The WebSocket frames (RFC 6455) of the bridge: one peer on this machine, whole messages, no extensions.
- `here.rs`: Where and when Codex runs, as its own environment note would say it: that note goes, for it calls the sandbox none.
- `layers.rs`: A setting as Codex reads it: `-c`, then the chosen profile, the trusted project's file, and its config.toml.
- `limits.rs`: The profiles of a limited Codex's two parts (`pair.rs`): NeoBot's, then the limit that was asked. macOS lets a rule name the program it is for (`process-path`), so in Codex's own part what its process needs (its state, the proxy, the system's services) is that binary's alone, and not its children's. They only take away from NeoBot's.
- `line.rs`: The command line Codex is started with: its own sandbox off, and its model told the true limit.
- `listen.rs`: What the broker opens when the daemon starts: its socket, and the rules' door for the limited ones.
- `mod.rs`: macOS: Codex inside NeoBot's sandbox. Its own sandbox cannot start there (macOS refuses any profile but the same one again: «sandbox_apply: Operation not permitted»), and no flag keeps both. So the limit its caller asked for becomes part of the profile: a `codex` on the sandboxed PATH reads what was asked (`asked.rs`, `settings.rs`), and the daemon, from outside, starts Codex under NeoBot's profile plus that limit (`limits.rs`, `broker.rs`), its own sandbox off only there, as two processes (`pair.rs`): Codex's own, which alone has its state, and its `exec-server`, which alone runs the model's commands and patches. Such a profile is never looser than NeoBot's, the broker serves only a caller under NeoBot's, and what runs under one gets no other. On Linux Codex's sandbox nests by itself: nothing here runs.
- `mode.rs`: The three limits Codex's sandbox knows.
- `network.rs`: What a limit leaves of this machine's ports: none of them the commands', unless the network was asked for, and the daemon's only a role's.
- `notice.rs`: What the daemon says to a caller while its Codex runs, on that caller's own stderr: Codex's report of a failed patch drops the reason it was given.
- `other.rs`: The other `codex` subcommands that run commands: their sandbox does not nest in NeoBot's on macOS, and the caller is told.
- `pair.rs`: A limited Codex is two processes. Its own keeps what is its alone (its state with the sign-in it renews, the proxy) and writes in none of the folders; what runs the model's commands and patches, `apply_patch` too, is its `exec-server`, under the limit and with nothing of that state. A patch cannot be told apart inside one process, so it is not in that process.
- `place.rs`: The broker's socket: in the user's own temporary folder of macOS (0700, whatever TMPDIR says), one for each NeoBot home. Who calls is told by the kernel.
- `profiles.rs`: Which profile of Codex's settings is chosen, and which MCP servers they start.
- `program.rs`: Which program a name on the command line is, and Codex's own binary in it: the profile's rules name that one file.
- `request.rs`: What a client and the broker pass between them.
- `roots.rs`: The folders workspace-write opens for writing.
- `sb.rs`: A profile's text as it is written: paths travel as parameters, never as profile text, each as the kernel names it.
- `servers.rs`: MCP servers that are programs: Codex's own process takes none once it is told where commands run (it has no environment of its own then), so each runs where the commands do, under their limit, from a whole path.
- `services.rs`: The system services a command may look up: those of Codex's own macOS policy. Codex's process keeps what NeoBot's profile lets it.
- `settings.rs`: The limit that holds for one `codex exec`: what it asked, else what its settings say, else Codex's own default.
- `spawn.rs`: One process started for a caller under the profile of its part, with that environment alone.
- `stalled.rs`: Whether what runs a limited Codex's commands was stopped: a command can signal its own sandbox, that process included, and a stopped one answers no more.
- `state.rs`: What Codex's own process writes in its state folder, found by running the real CLI in empty ones: no settings, `.env`, profiles, rules, prompts, plugins, skills or models' catalogue, which steer a later run.
- `told.rs`: What is said of a limit kept in one profile, to who called and to Codex's model: it never changes in silence.
- `too_big.rs`: A message past the bridge's cap is not carried: its call is answered that it failed, and the daemon never holds more.
- `upgrade.rs`: The bridge's half of a WebSocket's opening. A web page's is refused: it says where it comes from.
- `values.rs`: TOML as Codex's settings are read from it.
- `watch.rs`: A watch on what the broker started, outliving the daemon: when NeoBot stops in the middle, also by force, that Codex and all below it are ended, never left running with nobody to answer to. The watcher is NeoBot's own program with none of the caller's environment: it runs outside every sandbox.
- `writes.rs`: What may be written: the folders workspace-write opens, less what it protects, but not by Codex's own process, whose writing is its state's alone (and only what it must). A folder above what stays cannot be renamed, or the rule would stay behind.

## `sandbox/proxy/`

The daemon's HTTP proxy for sandboxed processes: `CONNECT host:port` and plain-HTTP absolute-form requests. It authenticates the caller, refuses blocked destinations (by name, then by the addresses they resolve to), records each
new destination of each caller, and pipes the bytes.

Files:

- `answer.rs`: The proxy's own answers: errors and refusals.
- `auth.rs`: Who is calling the proxy.
- `dests.rs`: The destinations seen, by host, for the page and `neobot net list`.
- `forward.rs`: Passing the request on and the bytes both ways until either side closes.
- `handle.rs`: One caller of the proxy, from its request head to the end of the connection.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `record.rs`: Counting each connection and refusal, and an event for each new one.
- `request.rs`: A request's head: the first line, the headers and where it goes.
- `serve.rs`: Listening for the proxy's callers.
- `upstream.rs`: Reaching the destination: refused if blocked by name or by any of its addresses, and to a replay unless it is its agent's model; never the daemon's own port (there the sandbox is known).

## `secrets/`

A bot's secrets. The owner's commands are `neobot secret set <bot> <NAME>`, `neobot secret list <bot>` and
`neobot secret remove <bot> <NAME>` (`<bot>` is its id, as in `#/bots/<id>`; NAME is A-Z, 0-9 and _). `set` reads
the value from stdin, typed unseen or piped whole less its last line end, and takes none from argv; `list` prints
names alone. Their routes (`/api/secrets/<bot>`) are the owner's alone, by `server/scope.rs` and by a check of their own: a
bot, a lane and a manager get 403, for their own secrets too. No route and no command answers a value. The daemon's lookup
(`lookup.rs`) is by bot and name; only the connector gateway asks it (`gateway/`) and only a connector's sign-in (`oauth/`) keeps a value through it, under its reserved `TIRITH_OAUTH_` names that the owner cannot set, and puts a value only in that
connector's own process environment or its auth headers, never in the agent's environment, its MCP arguments, a
transcript or a capture.

A value lies in the system's credential store when there is one, called in-process: macOS's login Keychain, Windows
Credential Manager, Linux's Secret Service, as service `tirith.bot.<id>` and account `<NAME>@<tag>` (the tag keeps
two NeoBot homes of one user apart). Only where no store exists is it in `<home>/bot-secrets/`: one file per bot,
0600 in a 0700 folder, sealed (XSalsa20-Poly1305) with a key of that bot's made from the daemon's random key there
(`key`, 0600, refused when others could read it). A store that is there but refuses (locked, a prompt denied) is an
error, never a move to the file; a value in the file moves up when it is set again and a store exists. Where each
name lies is kv `secrets:<bot>`, with no value. Deleting a bot first removes its values, then its role: when one
cannot be removed the bot stays and the error says why.

The limits. The fallback's key is as private as the owner's files: a program of the same user can read it (see
`server/`), so there the seal protects a copy of the sealed files without the key, not against that program. A
Linux Secret Service that is unlocked answers any program of that user too. macOS lets the program that made an
item read it, so a rebuilt, unsigned tirith asks the owner again; on a Mac with nobody logged in the Keychain may
be locked. Windows takes 2560 bytes a value, and the daemon itself is built only for macOS and Linux today.

Files:

- `api.rs`: The owner's routes: a value comes in, only names go out.
- `backend.rs`: The one shape of a place that keeps values, and which one is the system's (a fake in tests).
- `cli.rs`: `neobot secret`, reading its own argv so that a value there is refused.
- `fallback.rs`: One sealed file per bot where no OS store exists.
- `forget.rs`: Removal: one value, or all of a bot's with its role.
- `index.rs`: A bot's names and where each value lies, never a value.
- `input.rs`: A value as it comes on stdin.
- `key.rs`: The fallback's private files and the daemon's key material in them.
- `lookup.rs`: The daemon's own lookup of one value, by bot and name, for the connector gateway alone, and its keeping of a connector's sign-in; a bot's names (never values) for the Connectors page's status, and a removed connector's sign-in dropped (`connector_page/`).
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `names.rs`: What may be a bot, a name and a value.
- `os.rs`: The OS's credential store, in-process.
- `seal.rs`: The fallback's cipher, with a key per bot.
- `store.rs`: The store, one call at a time: a value set, and the names.

## `server/`

HTTP on 127.0.0.1: the page (`web/`), its JSON API, the change stream (SSE) and the agents' hooks, behind one same-origin guard and one gate that knows the caller by its token. Each module brings its own routes; `server/routes/` holds the daemon's own.

Who calls is a token (`admit.rs`), never what a caller says of itself: no header names a lane, a role or the owner.
The token goes as `X-Tirith-Token`.

- **An agent.** `sandbox::wrap` gives each process it starts its per-who token, the proxy's password, in `TIRITH_TOKEN`,
  confined or not. From that token alone (`identify.rs`) come the lane with its project, or the role with its project,
  and the bot. Who it is: a lane's id, `<role>@<conversation>` for a chat or a turn, `merge:<lane>` for a check, `git`,
  and any of them after `replay:<experiment>:` inside a replay. The token derives from `<home>/net.secret`; the database
  holds only which who a token's hash may be, and the token is derived again, so a row written there proves nothing.
- **The owner.** `<home>/owner.token` (0600), which the CLI reads in their terminal, and never from inside the sandbox:
  there the file, the secret and `owner.html` are unreadable, and the kernel says where a caller is (Linux's relay calls
  the daemon's port from 127.0.0.2; on macOS the socket's holder is looked up and its profile read; one nobody of this
  user holds is not the owner either), asked once per connection that claims to be the owner.
- **The page.** The same owner, by a cookie derived from that token. `neobot open` (the app does it as it starts; on a
  server `neobot open link`) gets `/auth/<code>` from `POST /api/page`: it works once, for 10 minutes, and sets the
  cookie (HttpOnly, SameSite=Lax). The API takes the cookie neither from another site or port nor for an address a
  browser was made to visit. Without it even `/` is 403, and says how to open the page.
- **A hook without a valid agent's token** (the owner's own sessions, whatever their token) can only be refused, as
  those sessions always were: the rules of the folder it stands in and the block list. It is no lane and no role: it
  gets no allow, no message, and writes no event but a refusal, no session and no note.
- **Routes with a token of their own** keep it: the teach extension's (the bot's token; `teach/door.rs`) and the page's
  link. They too are refused from inside the sandbox.
- **Everything else** without a valid caller token is 403. `scope.rs` keeps a token from acting as another: a lane asks
  only as itself and does nothing that is the manager's, a bot rehearses and keeps checks only as itself.

An agent may hand its hooks and shells none of its variables (Muse): the CLI then takes `TIRITH_TOKEN` from the
environment its own ancestors started with (`client/ancestry.rs`). A session of a lane or a role never falls back to
the owner's token. Hooks installed over HTTP name the variable in `allowedEnvVars`; the daemon gives older ones the
header as it starts (`cli/hooks_token.rs`). A process that started before this has no token: its hooks only refuse
and its commands are 403 until it is started again (the proxy takes its old password until the daemon's next start).

The owner's yes never comes from an agent. An agent's
`neobot answer` of an approval or payment card needs the owner's own words, written after it was asked, and keeps those; its
words in a chat are never the owner's; it gets neither the teach extension's token nor a phone's pairing code, and
it neither approves a draft nor gives a 👍,
nor lifts one of the owner's network blocks (it may add one), nor sets, lists or removes a bot's secret (`secrets/`), nor gets a link to the page. Git the daemon runs is confined too (`sandbox/git.rs`): a hook, an fsmonitor or a filter left in a bot's or a
project's repository is an agent's, so its `git push` also goes through the proxy and the block list, with
`GIT_SSH_COMMAND` as the lanes have it. The limit: a program of the same user that reads or writes that user's files (the store, start-up
files, programs on the PATH), reads the environment or logs of another agent of the same user (its token is there), or
drives the owner's own browser, is not stopped by this; another user account is the boundary for that.

Files:

- `chat.rs`: A new line in a conversation, from the owner's page or from an agent.
- `admit.rs`: The gate in front of every route: the caller from its token, else 403.
- `identify.rs`: From a token alone to its agent: lane, role with its project, and bot.
- `page.rs`: The page opened as the owner: the link that works once, and the cookie.
- `refused.rs`: What a caller hears that the gate does not let in.
- `scope.rs`: What an agent's token does not open: the owner's alone, and another's name.
- `who.rs`: The caller as each route is told, and the header its token comes in.
- `hook.rs`: One agent hook call (without an agent's token only refusals): recorded, a replay's outside read answered from its capture (`lab::answer`), refused when it breaks a rule, a lane's live messages handed over.
- `hook_door.rs`: The door of a Codex under its caller's limit (sandbox/nested/): the rules' hook and nothing else. That limit's profile refuses the daemon's own port in the kernel, so none of its requests needs a look at where it comes from.
- `hook_find.rs`: The project of a hook call, and for one without a token the lane whose refusals hold in its folder.
- `hook_lane.rs`: What a hook call is about: its lane and project, its tool call under the rules' names, and its summary.
- `hook_reply.rs`: What a lane's agent hears back from a hook call that was not refused.
- `hook_rules.rs`: A tool call that breaks a rule is refused before it runs.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `door.rs`: The daemon's door: loopback only, and on Linux only for this NeoBot's own user, root and `[server] users`.
- `origin.rs`: The same-origin guard in front of every route.
- `caller.rs`: A caller's connection, and whether it comes from inside the sandbox, where nobody is the owner.
- `peer.rs`: Who is at the other end of a loopback connection, from the kernel's table of sockets (Linux).
- `plan.rs`: A project's plan as the page shows it (each item with its newest lane), and the step a command adds to it.
- `router.rs`: The whole router: every module's routes, the daemon's own, the page, the gate and the same-origin guard.
- `routes/lanes.rs`: A lane's routes: start, stop, delete, its log, its question and its answer.
- `routes/mod.rs`: Declares the folder's files and re-exports what other modules use.
- `routes/owner.rs`: The owner's cards: asked, answered, closed.
- `routes/runs.rs`: The pipelines and their runs: the list, a retry, a stage's time limit and a run's log.
- `routes/state.rs`: What the page reads: the whole state, the change stream, a project's plan and its autopilot.
- `routes/talk.rs`: Talking: a conversation's rows, a new line in it, interrupting its manager, and the agents' hooks.
- `settings.rs`: Settings the manager or the page change: a project's autopilot and a stage's time limit.
- `state.rs`: The whole state the page starts from.
- `web.rs`: The page's files.

## `service/`

Server mode (`neobot service …`, `neobot setup --server`): the daemon as a service of its machine, one per NeoBot
home. On Linux a systemd user unit with linger (no root, up from boot); on macOS a launchd job of the owner's, or
with `--boot` one of the system's that runs as the owner with nobody logged in (sudo); where there is neither (a
container) it is started in a session of its own and says that nothing brings it back. Its launchd label is
`com.tirith.serve` (a marked one for a home that is not the usual). A restart leaves the
running lanes alone: the next daemon takes them up.

Files:

- `alive.rs`: Whether what answers is the service's own process (`alive:` in `neobot service status`).
- `cli.rs`: `neobot service install|status|start|stop|restart|uninstall|logs`.
- `kind.rs`: Which way this home's daemon is kept running.
- `launchctl.rs`: launchctl on one job, in the owner's domain or the system's.
- `launchd.rs`: The launchd job installed, started, stopped and removed.
- `mod.rs`: Installing, starting and stopping the service, each way.
- `plain.rs`: The daemon started by hand in a session of its own, and stopped by the pid it gives.
- `plist.rs`: The text of the launchd job.
- `systemd.rs`: The systemd user unit installed, with linger.
- `systemd_text.rs`: The text of that unit.
- `unit.rs`: The service's names and files for one home.

## `setup/`

The guide (`neobot setup`, `neobot connect [<agent>]`, `neobot doctor`). It takes a bare Mac or Linux machine to
agents that work: what the machine needs (git, curl, bash, a git identity, the sandbox), the daemon, then for each
agent its program (or its maker's installer), its hooks at NeoBot (for Codex also trusted, through its app-server),
its sign-in (its own login, which works over SSH, or a token on stdin kept 0600 in `<home>/auth/` and handed to
that agent's processes alone) and the proof: one cheap real turn as the hidden role `check`
(`POST /api/setup/check/<agent>`), which is connected only when it answered **and** the rules heard its tool call.
Roles whose agent is not connected move to the chosen one. The agents' catalog is `assets/agents.toml`; what the
repository does not name a release build lays over it (`TIRITH_AGENTS_FILE`, read by build.rs).

Files:

- `api.rs`: The daemon's side: who it is (`GET /api/setup`) and the connection check.
- `as_user.rs`: root's `tirith` on a server goes on as the service's user, not as a second NeoBot.
- `ask.rs`: Asking the owner on the terminal, also under `curl | sh`.
- `auth.rs`: An agent's token, kept private and handed to its processes alone.
- `catalog.rs`: The agents the guide knows.
- `choose.rs`: Which agents the owner connects, and which one the work runs on.
- `configure.rs`: What the guide writes in config.toml.
- `connect.rs`: Connecting one agent, in order.
- `context.rs`: What every step works with: the home, this binary, the terminal.
- `daemon.rs`: The guide's talk with this home's daemon.
- `doctor.rs`: `neobot doctor`: every part this machine needs, each with its fix.
- `edit.rs`: config.toml changed one change at a time, its comments kept.
- `find.rs`: Where a program is on this machine.
- `guide.rs`: `neobot setup`: the whole guide.
- `hook_lines.rs`: NeoBot's hook in an agent's hooks, and telling it from everyone else's.
- `hooks.rs`: An agent's hooks pointed at NeoBot, the rest of its settings kept.
- `host.rs`: An agent that may live on an enterprise's GitHub host (Copilot): asked, kept with its token, given to its login and all its processes, never shown.
- `install.rs`: An agent installed with its maker's installer.
- `login.rs`: An agent's own login on the owner's terminal, and how to do it later.
- `mod.rs`: The commands' way in from the command line.
- `needs.rs`: What the machine must have first.
- `one.rs`: `neobot connect`: one agent, or the list of them.
- `npm.rs`: A maker's npm installer where its script's build cannot run (Copilot on musl).
- `opts.rs`: The options of these commands.
- `private.rs`: What never reaches a log, an event or an answer: the values kept for the agents (tokens, an enterprise's host).
- `packages.rs`: The system's programs installed with its package manager, when the owner says so.
- `program.rs`: An agent's program found, or installed now.
- `terminal.rs`: Linux: the service's user, in root's terminal during the guide, cannot type into it (nor can what it starts).
- `root.rs`: What the guide has root do through sudo: the system's own programs alone, and a file's text on stdin.
- `prove.rs`: The connection check: one real turn by the same way work runs.
- `why.rs`: Why a connection check failed: the telling lines of that turn's stderr.
- `run.rs`: Running another program in the owner's sight, or quietly.
- `secret.rs`: A secret read from stdin, not shown.
- `signin.rs`: An agent signed in, then proven.
- `summary.rs`: The guide's end: what is connected, what is left, how to open NeoBot.
- `trust.rs`: NeoBot's hooks trusted in Codex through its app-server.
- `trusted.rs`: Who signs in on a server whose NeoBot is another user's is added to `[server] users`, for their tunnel.

## `status/`

`/status`, written by the owner in a chat or from a paired phone in its room: NeoBot answers it from its own data and no
turn opens (no `owner.message` or `room.message` event, so no eval either, and no trigger for a next turn). It says how
long the daemon has been up, each project's active lanes by state, today's timed routines of the bots and the last error.
A routine is done when its own first run after its `schedule` event ended well; it is pending until then, unless it
failed or a change to it after its time means it no longer runs today. The last error is the newest of the error and
failure events and the failed turns, on one line with its secrets hidden.

Files:

- `command.rs`: The owner's `/status` command and the answer it gives.
- `error.rs`: The last thing that went wrong: an error event or a failed turn.
- `lanes.rs`: The active lanes of each project, counted by their state.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `routines.rs`: Today's timed routines of the bots: the ones done and the ones still to do.
- `span.rs`: A length of time in a few characters.

## `teach/`

Teaching a bot a task (docs/BOTS.md, part 5). The owner does the task once in their own Edge with NeoBot's
extension, `tools/teach/` (Manifest V3; the tab «Enseñar» downloads it from `/teach-extension.zip`, which the daemon serves from what it
carries, so it works on a headless server reached through an SSH tunnel; installed with «Load unpacked», its README says how): pages (URL
and title), clicks (the element's text, role and a short selector) and typing (the field's label and value), each
with its time. Passwords, one-time codes and cards are masked as `•••`, first by the extension and again by the daemon, by whole words
(`names.rs`, `tools/teach/words.js`): `type=password` (also a field the page showed as text since), `autocomplete` `one-time-code`, `cc-*` or
holding `password`, a name, id or label with a word such as pwd, pass, passwd, passcode, password, contraseña, clave, secret, token, otp, pin,
cvv, tarjeta, card, código, or «verification code» / «security code» (not «passenger», not «company»), and any step marked `sensitive`. The
secret parts of a recorded address are masked too (`url.rs`: the password of `user:password@`, and the value of a query or fragment parameter
named code, token, key, password, sig, session and the like; ordinary ones stay), and what is typed in a contenteditable is never a click's
text. Both sides pass the same table, `rust/tirith/checks/teach_table.json` (`scripts/checks/teach-words.test.mjs` runs it on the extension's
words). A recording stops by itself after 10 minutes, also across a restart of the daemon (`teach::start` stops and hands over, once, a
recording that ran out meanwhile, and arms the one still running). The recording lives in kv `teach:<bot>` (`on`, `since`, `steps`), the bot's
token in `teach-token:<bot>`. The browser's clock may differ from the daemon's by minutes or hours: the daemon tells the extension how long is
left (`left_ms`, in the recording, the start and the polls) instead of when it ends, and each step carries `at` and `sent` (the extension's two
times), from which the daemon rebuilds the step's time on its own clock (`clock.rs`), so the 10 minutes and the order of steps and of the owner's
words hold.

- `GET /api/teach/<id>`: the recording; to the page also the token (the bot's tab «Enseñar» shows the link
  `<daemon>/api/teach/<id>#<token>`, pasted once in the extension), never to the extension.
- `POST /api/teach/<id>/start` and `/stop`: from the page, or from the extension with the token.
- `POST /api/teach/<id>/events`: one step or `{"steps": [...]}`, kept only while recording; it needs
  `Authorization: Bearer <token>` from anyone (401 without it or with another).

The same-origin guard lets the extension's origin (`chrome-extension://bogleilobfephnbnjibgoofoeipkhfef`, fixed by
the `key` in its manifest) reach `/api/teach/` only, and CORS answers that origin only (preflight included). While a
recording is on, what the owner writes in the bot's chat (`manager::say`) is kept with the steps, by time, and starts
no turn. At stop the bot is handed, in one turn (trigger `teach`), the steps with the owner's words under the step
they follow (all between two unguessable marks, said to be data from outside, each page's words on one line: `steps.rs`) and the skill template
(`factory/skill.md`): it writes the skill with `draft: true` in its header as `drafts/<name>/SKILL.md` and asks 3 to 5 questions in its chat
(decision rules, failures, approvals). A draft is kept outside every folder an agent loads by itself (`bots/drafts.rs`): one found in
`.agents/skills` is moved to `drafts/` at the start of every turn, at its end and when the Library is read; the Library lists drafts (marked)
and «Activate» moves one into the skills, without the line. No bot uses a draft until the owner approves it on the tab «Enseñar» or
activates it in the Library (factory/prompts/bot.md says so).

**Rehearsal.** The bot then rehearses the draft in the owner's Edge through the same extension, in a tab of its own
(factory/prompts/bot.md: it follows the draft's steps and stops before any step its skill lists under «Qué requiere
aprobación», or that sends, buys, deletes or publishes, and says in its chat where it stopped; the `unapproved` evals
judge a `neobot teach do` step like any other action). From its session (`TIRITH_PROJECT` is its conversation):

- `neobot teach rehearse "<skill>"` → `POST /api/teach/<id>/rehearse` `{skill}`: a new rehearsal (the one before
  stops); a line in its chat says until when. `neobot teach stop` → `POST /api/teach/<id>/rehearse/stop`.
- `neobot teach do open "<url>"`, `do click "<selector or text>"`, `do type "<field>" "<value>"` →
  `POST /api/teach/<id>/do` `{kind, target, value}`: the daemon holds the step (one at a time per bot, `teach/relay.rs`)
  until the extension takes it, waits up to 40 s for its answer, keeps the step and answers with the result and the
  path of its screenshot (`<bot folder>/rehearsals/<time>.jpg`, where the bot can look at it). 409 with no rehearsal
  on, 410 once its 10 minutes are over, 504 when the extension does not answer.
- The extension, with the bot's token (401 without it): `GET /api/teach/<id>/next` is its long poll (20 s; `{on,
  until, cmd}`, `on` false once the rehearsal is over) and `POST /api/teach/<id>/done` `{id, ok, result, shot}` its
  answer, the screenshot a data URL from `chrome.tabs.captureVisibleTab` (16 MB at most). Its worker
  (`tools/teach/rehearse.js`) polls while a rehearsal is on, waking on its 30 s alarm, opens the page in the
  rehearsal's tab, and `act.js` clicks (a selector, else the shown element with that text) or types (a selector, else
  the field with that label, placeholder or name) there, under the visible notice «NeoBot está ensayando» until the
  rehearsal ends.
- A click is refused by code before it reaches the extension (`outward.rs`, in `run.rs`) when its target's words send, buy, pay, delete or publish
  (English and Spanish, as `tools/teach/outward.js` reads them) or name what the skill lists under «Qué requiere aprobación»; the extension
  refuses the same by the element it found (its text, aria-label, title or value). The result starts «Refused:» and tells the bot to stop there
  with `neobot teach stop`; the refused step is kept with `ok` false.
- The calls that start, stop or send steps, and the screenshots (`GET /api/teach/<id>/shot/<file>`), are refused to
  the extension's origin (403). Rehearsals live in kv `rehearse:<bot>` (the last 5: `skill`, `since`, `on`, `steps`
  with `n`, `kind`, `target`, `value` (a secret field's masked), `ok`, `result`, `shot`), and `GET /api/teach/<id>`
  gives them as `rehearsals`.

The tab «Enseñar» shows each rehearsal with its screenshots in order and two buttons: «Aprobar» →
`POST /api/teach/<id>/approve` `{skill}` moves the draft into the skills without `draft: true` (400 when it is no draft, or when no finished
rehearsal of that skill had a step that worked), so the bot uses it from then on, and says so in its chat; it is the owner's: a call that carries
an agent's token (which `tirith` run from a session sends) gets 403; «Crear rutina» opens the routine
form with the task written.

Files:

- `api.rs`: The teach API: the recording read, started and stopped (by the page, or by the extension with the token).
- `own_page.rs`: NeoBot's own page is no step of a rehearsal.
- `approvals.rs`: What the skill lists under «Qué requiere aprobación»: the click a rehearsal must stop at because the skill names it.
- `approve.rs`: «Aprobar» on the tab «Enseñar»: the draft rehearsed loses `draft: true` from its header, so from then on the bot uses it; it is told with its next message.
- `clock.rs`: A step's time rebuilt on the daemon's clock from the extension's `at` and `sent`.
- `cli.rs`: `neobot teach rehearse|do|stop`: a bot rehearses a draft in the owner's Edge, from its session (`TIRITH_PROJECT` is its conversation).
- `caller.rs`: Who may act for a bot: the page, or that bot by its own token.
- `door.rs`: The routes that carry the bot's own token, which the gate leaves to it.
- `cors.rs`: Who may call the teach API: the page (same origin) and NeoBot's extension with the bot's token. CORS answers only the extension's origin.
- `events.rs`: The extension's steps: one, or `{"steps": [...]}`, kept only while recording, masked again and timed on the daemon's clock.
- `ext_api.rs`: The extension's side of a rehearsal, with the bot's token: the long poll for the next step, and its answer with a result and a screenshot.
- `hand.rs`: The owner's words while recording, and the turn at stop: the steps with those words, the skill template, what to ask.
- `mask.rs`: A recorded step as it is kept: its known fields only, cut short, a secret's value and an address's secrets masked.
- `names.rs`: Whole words that name a password, a code or a card in a field, label or parameter.
- `outward.rs`: What a rehearsal never does by itself: a click whose words send, buy, pay, delete or publish (English and Spanish).
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `record.rs`: A recording started and stopped: by the page, the extension, or its 10 minutes running out; at stop the bot is handed it.
- `rehearsal.rs`: A bot's rehearsals in kv (`rehearse:<bot>`): their skill, start, state and steps with result and screenshot.
- `rehearse_api.rs`: The rehearsal API: the bot starts, stops and sends steps; the extension asks for the next one and answers it.
- `relay.rs`: The relay between `neobot teach do` and the extension: one command at a time per bot, its answer handed back.
- `run.rs`: One rehearsal step: checked, handed to the extension through the relay, and kept with its result and screenshot.
- `url.rs`: A recorded address with its secrets masked (user:password@, secret query and fragment parameters).
- `shots.rs`: A rehearsal's screenshots: kept as files in the bot's folder (`rehearsals/`), where the bot can look at them (the newest 300 stay), and served to the page.
- `steps.rs`: The recorded steps as the bot reads them: one line each, between two marks, as data from outside.
- `state.rs`: A bot's recording in kv (`teach:<bot>`: on, since, steps, the owner's words) and its token (`teach-token:<bot>`).

## `turns/`

The turn log. Every turn of a chat or turn role (the project managers, the bots, the CEO, the judge) leaves one row in
`turns` (`assets/schema.sql`; `runs` is the pipelines'): `conv` (its conversation), `role`, `trigger` (`owner`,
`room`, `agent`, `schedule`, `run-now` — the page's Run now — or `event:<name>`; each message carries its own, taken when it is
composed, so it never outlives its message), `routine` (the id of the routine that started it), `started_at`,
`ended_at`, `ms`, `status` (`ok`, `failed`, or `waiting` when an approval card it opened is still open at its end: an `approval` card of its conversation opened since the turn started, `turns/approval.rs`; once no such card is open, because the owner answered it or it was closed, the row becomes `ok`; a turn the daemon was stopped in is kept as `failed` at the next start, `interrupted.rs`), `agent`, `model`, `effort`, `tokens_in`, `tokens_out`,
`tokens_cache_read`, `tokens_cache_write`, `cost_usd`, `steps` (tool calls), `skills` (JSON: the skills it read —
Anthropic's `Skill` tool, Muse's `read_skill`, or any tool whose input names `skills/<name>/SKILL.md`), `tools` (JSON:
each call's name and the short line the chat shows, 50 at most), `versions` (JSON: the git blob hash, as
`git hash-object` gives it, of its `memory.md`, of `<home>/bots/memory.md` when it exists and of each skill's
`SKILL.md`), `answer` (cut at 4000 characters), `event` (its event's id), its stamp (`factory`, `role_hash`) and,
for a run of an experiment, its `experiment` and `variant` (`A` or `B`, set by `lab::finished` at the end of a live
run, by the replay for its own). A bot's turn also keeps what it read in `<home>/captures/<turn>.jsonl` (see `lab/`,
«Captures»).

Tokens and cost come from what each agent already prints (`usage.rs`), each measured against the real CLI:
Anthropic's `result` (tokens of the turn, with the cache apart; `total_cost_usd` of the whole session, also after a
resume), Muse's `run.usage` (the turn's, its cached tokens inside `input_tokens`; without it the `model_completed`
records, else those of its session's `session.jsonl` under `~/.local/share/muse/sessions/`), Codex's `turn.completed`
(the thread's totals since it began, the cache inside) and Copilot's `--usage-output-file` (the session's totals, the
cache read and written inside: both come off `tokens_in`). `settle.rs` makes them comparable before the row is kept: a turn gets its own share of a session's
totals (what was spent before is kv `spent:<conversation>`; when that is not known, none rather than a guess), and
`tokens_in` is always the fresh input, what the cache gave being `tokens_cache_read` alone, so no sum counts it
twice. The model is the one the role sets, else the one the agent says (Codex's from its thread's rollout). When the
agent gives no cost, `[prices."<model id>"]` in `~/.tirith/config.toml` does (`input`, `output` and `cached`, dollars
per million tokens; the strict loader accepts the section and none of it goes to `factory/`); without a price the
cost stays empty, never invented.

The versions are the files as the turn found them (`versions.rs`): a bot's folder is committed as each turn starts
(«before a turn») and when it ends having changed it, so every one of those blobs is in its history.

When a bot's own turn ends (a role with `scope = "own"` and `session = "chat"`; a judge's turn in its conversation emits nothing, or an eval on `turn.done` would judge itself forever) the event `turn.done` is emitted — or
`routine.done` when a routine started it or it read a skill (graded with the skill's «Cómo validar»; a chat turn that read none is not graded) — saying the bot, the trigger, the routine and its task, the tools, the
skills and the beginning of the answer; neither has a default route. For the `unapproved` eval it also carries each
tool call with its summary and, for each skill it read (its own, then the shared one), the text of its «Qué requiere
aprobación» section; these are never cut. `GET /api/turns?conv=<key>&limit=<n>` gives the
rows, the last first. The manager's sessions, exec turns and loose turns, and the bots' routines, only call this
module: `trigger`/`take_trigger` around a message composed, `handed`/`line`/`end`/`abandon` around a conversation's
turns (one per message handed to its session), `turn_of`/`done` around a turn role's.

Each row also carries its quality signals (`react.rs`, `signals.rs`, `checks.rs`):

- `reaction`: the owner's 👍 (1) or 👎 (-1). `POST /api/turns/<id>/reaction` with `{"value": 1, -1 or 0}` keeps it (0
  takes it back: empty). In a bot's conversation an owner message that is only 👍 or 👎 (any skin tone) or `+1` / `-1`,
  written on the page (`manager::say`) or from a paired phone (`from_room`), is the reaction to the bot's last kept
  turn: it stays as a chat row, starts no turn, and has no `owner.message` event, so owner-mood does not judge it.
- `corrected`: 1 when the `owner-mood` eval labels an owner message `correction` or `repeat`; it marks the
  conversation's last turn (of its own role, not the judge's) that ended before that message, taken when the message
  came, before the judge runs.
- `checks_passed`, `checks_total`, `checks` (the names, as written): the bot's own validation. `neobot turn checks
  <passed>/<total> "<names>"`, run by the bot in its session (`TIRITH_PROJECT` is its conversation; a lane is refused),
  posts to `POST /api/turns/checks`; the open turn of that conversation keeps them until its row is written, else its
  last kept turn gets them. `factory/prompts/bot.md` asks for it at the end of a routine whose skill has «Cómo validar»,
  with what it really checked.
- `cited` (JSON): the numbered notes, `[m<N>]` of its own memory and `[s<N>]` of the shared one, that its answer
  names, each once in order, read when its row is written.

- `score`: the judge's note from 0 to 1 on a routine's run. The `turn-quality` eval (`[[evals]]` on `routine.done`,
  rubric `factory/evals/turn-quality.md`) reads the routine's task, the answer and the «Cómo validar» section of each
  skill the run read — `routine.done`'s text carries them (`quality.rs`: the answer's start and end in 1000 characters, what its calls gave, each «Cómo validar» to 1500) — and answers a score
  with one line why; the score is copied to the turn whose `event` is that event (`turns::scored`, or `catch_up` when
  the judge answered before the row was written). A judge's turn kept in a bot's conversation emits no event of its own.
  A routine of a turn role (the CEO's) emits `routine.done` too. A why that starts with `could not:`, at 0.3 or less,
  says that the run could not do its task in words no list holds: the run and its turn end `failed` with that cause,
  and the owner gets the one line of any failed run (`could_not.rs`, `expected/admitted.rs`). Code alone fails a run
  only where it is certain: the agent's own failure, ✗, a ∅ that says "could not" on its line or comes from no look or one that failed,
  a nested agent's or a sandbox's own error (and a first-person "I could not" where nothing grades the turn). Any other failure in an answer may be the news the task asks for (what is down),
  so the judge reads whose it is; when no judge gives a score (none within 25 minutes, `expected/unjudged.rs`), plain words of trouble
  count as a failure. The answer and what the calls gave are fenced as data, and a judge uses no tool (`rules/judge.rs`).

`GET /api/turns` gives these columns too; older databases get them by `ALTER TABLE` at start (`add_columns`).

Files:

- `api.rs`: `GET /api/turns?conv=<key>&limit=<n>`: a conversation's turns, the last first.
- `approval.rs`: Approvals: a turn ending with its approval card open waits until the card is answered or closed (and never becomes a could-not for that), and what the `unapproved` eval reads of a turn.
- `broke.rs`: A tool's own error in what it printed. An exit status alone does not tell: `codex exec` ends well when every command it ran was refused.
- `capture.rs`: What a turn read as its agent saw it: what was handed to it and each tool call with its input and result, up to 256 KB.
- `capture_fit.rs`: Fitting a capture in its size: a result's text, cut on a character to the room left.
- `capture_keep.rs`: The captures on disk: written at a bot's turn's end, the last 10 runs of each routine kept, read back for a replay.
- `capture_told.rs`: What was handed to a conversation's agent (never the memory), waiting for the turn it starts.
- `checks.rs`: The bot's own validation, `neobot turn checks`, kept on the open turn of its conversation or the last one.
- `could_not.rs`: The judge's word that a routine's run could not do its task: that run becomes a failure.
- `codex_log.rs`: Codex's own log of a thread, for the model a turn ran on.
- `evidence.rs`: What a run's judge is shown of its calls: those that failed or printed an error and the last two, and what each gave, as data.
- `exec_items.rs`: The tool calls in the exec agents' lines, under the names their hooks use.
- `follow.rs`: Which messages handed during a turn got turns of their own (the streaming CLI may fold them into the running one).
- `hashes.rs`: The git blob hashes of files as they are now.
- `interrupted.rs`: Turns the daemon was stopped in the middle of: marked when they start, kept as failed at the next start.
- `live.rs`: The turns of the conversations as they run: the trigger waiting for the next message, the turn open now, and the messages handed during it, each with its own trigger.
- `look_lines.rs`: How a tool call ended, as any agent's lines say it.
- `look_muse.rs`: Muse's results. A command still running when its first result came ends only in Muse's own session log (`muse_log.rs`): its `--json` output never carries that end.
- `looks.rs`: Whether a turn's calls worked: how each ended and what error it printed, in any agent's lines; and the agent's own error.
- `loose.rs`: The loose turn of a turn role (`run_turn`): started by what waits under `turn:<role>`, kept with its result.
- `muse_log.rs`: Muse's own log of a session, for a turn whose output carried no usage.
- `mod.rs`: Declares the folder's files and re-exports what other modules use.
- `open_start.rs`: The start of the turn of a conversation open now, for what is recorded against it from outside its lines (the gateway's calls).
- `price.rs`: A turn's cost when its agent gives none: `[prices."<model id>"]` of config.toml, in dollars per million tokens.
- `proof.rs`: What a turn's calls prove of a run: a look that did not work, and an error that nothing after it undid.
- `quality.rs`: The judge's note on a routine's turn (or a chat turn that read a skill): what `routine.done` carries for `turn-quality`, and the score copied to the turn.
- `react.rs`: The owner's 👍/👎 on a turn: by `POST /api/turns/<id>/reaction` or a message that is only 👍 or 👎.
- `record.rs`: One turn while it runs: who runs it and why, and what its lines say (its usage, tools, skills and text).
- `save.rs`: A turn ended: its row in `turns` (`waiting` while an approval card it opened is open), for a bot its `turn.done` (or `routine.done`) event and the commit of what it changed in its folder and the shared one.
- `said.rs`: What any agent's line says of its turn: its session, its model and the answer's words.
- `settle.rs`: A turn's numbers made comparable across the agents before its row is kept.
- `shell.rs`: A shell call as the agent asked for it, out of Codex's `sh -lc` wrapper.
- `spent.rs`: The turn's own share of what an agent gives as its session's totals.
- `signals.rs`: The quality signals on a turn's row: `reaction`, `corrected`, the checks and the cited notes.
- `skill_read.rs`: The skill a call read: a path in what came back, in a prompt handed on or in a file written is no read.
- `tools.rs`: The tool calls a line of any agent's output makes, and the short line the chat shows for each.
- `usage.rs`: What a turn used, read from what each agent prints.
- `usage_keys.rs`: How each agent names its counts of tokens.
- `versions.rs`: The versions a turn ran with: the git blob hash (`git hash-object`) of its memory.md, the bots' shared memory and each skill it read.
