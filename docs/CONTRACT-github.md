# Contract addition: remote CI and cloud agents (GitHub, v1)

Work that happens outside the owner's machine shows up in the same Pipelines and Agents views:
GitHub Actions runs become pipeline runs, open pull requests by cloud-agent accounts become agents.
Both carry `source: "github"` and an `html_url`.

## Configuration (`~/.tirith/config.toml`)

```toml
[github]
token_env = "MY_GITHUB_TOKEN"            # optional: one more env var to read a token from
agent_logins = ["cloudbot[bot]"]         # accounts whose open pull requests are agents
```

The token comes from `GITHUB_TOKEN`, then `GH_TOKEN`, then the variable named by `token_env`.
**No token → nothing is read, nothing is reported, no error.** Only projects whose `origin` remote
is on github.com are read (`https://`, `git@` and `ssh://` forms). Without `agent_logins` only
Actions are read.

## Reading (`collector/tirith/github.py`)

Plain `urllib`, bearer token, `If-None-Match` with the stored ETag (a 304 reuses the cached body and
does not count against the rate limit). The whole result is cached and refreshed at most every 60 s.
A failing request is skipped silently (kept in `GithubReader.last_error`); what was read before it stays.

| Endpoint | Used for |
|---|---|
| `GET /repos/{o}/{r}/actions/runs?per_page=5` | the latest runs |
| `GET /repos/{o}/{r}/actions/runs/{id}/jobs?per_page=30` | jobs and steps of each run |
| `GET /repos/{o}/{r}/pulls?state=open&per_page=30` | candidate pull requests (filtered by author login) |
| `GET /repos/{o}/{r}/commits/{sha}/check-runs?per_page=50` | checks of the PR head |
| `GET /repos/{o}/{r}/pulls/{n}/files?per_page=100` | files of the PR |

## Pipelines

One entry in `state.pipelines` per workflow run:

| Field | Value |
|---|---|
| `run` | `github:<owner>/<repo>:<workflow>#<run_number>` |
| `pipeline` | workflow name |
| `status` | `failed` if any job failed or the run failed/cancelled/timed out; `passed`; `running`; `pending` (queued) |
| `started_at` / `ended_at` | run start; last update when finished |
| `stages[]` | one per job: `name`, `status` (adds `skipped`), `started_at`, `ended_at`, `duration_s`, `html_url`, `log_tail` = the job's steps (`✓ name`, `✗ name`, `- name`, `… name`) |
| `source`, `html_url` | `"github"`, the run page |

Finished remote runs do not create `report` flows (the first poll would flood the feed with history).
The list is not capped by `MAX_PIPELINE_RUNS`; each project contributes at most 5 runs.

## Agents

One entry in `state.agents` per open pull request by a configured login (at most 10 per project):

| Field | Value |
|---|---|
| `id` | `gh-<owner>-<repo>-pr<n>` |
| `name`, `epithet` | login without `[bot]`; `PR #<n>` |
| `feature` | PR title |
| `status` | `failed` if a check failed; `working` if draft, no checks, or checks pending; else `done` |
| `files`, `files_changed`, `insertions`, `deletions` | from the PR files API |
| `source`, `html_url` | `"github"`, the PR page |

## Web

Runs and agents with `source: "github"` show a small GitHub mark; the pipeline canvas toolbar, the job
drawer and the agent detail show an "Open on GitHub" link (only `https://` URLs are linked).

## CI gate (`.github/workflows/tirith.yml`)

On push and pull request: collector tests, `tirith check --ci ..`, `node --check web/js/*.js` and
`node web/js/md.test.mjs`. Make the `collector` and `web` jobs required checks in branch protection
(private repositories need GitHub Pro for that).

`tirith check --ci` skips the `commit-guard` rule: it is "not applicable" outside a local clone with
hooks installed. Everything else is checked, and with `--ci` the folder defaults to the cwd.
