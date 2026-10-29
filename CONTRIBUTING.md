# Contributing to Tirith

Thanks for helping. Tirith is built by agents that Tirith itself manages, so a few rules are enforced by code.

## Run it

```bash
git clone https://github.com/andreseloysv/Tirith && cd Tirith
./run --demo     # the page on demo data, no collector
./run            # collector in the background + the page (data from ~/.tirith)
```

Requirements: [uv](https://docs.astral.sh/uv/) and a browser. The desktop app is in `app/` ([`app/README.md`](app/README.md)).

## Test

Inside a project with a `tirith.toml`, run checks through Tirith so they show up in Pipelines:

```bash
tirith run --project tirith --stage test    # collector tests
tirith run --project tirith --stage build
```

Without Tirith installed: `cd collector && uv run pytest -q`. Web tests use `playwright-core` with Microsoft Edge.
A change that alters behaviour needs a test that fails without it.

## Code rules

- Rules live in code, not in prose: see [`docs/CONTRACT-gates.md`](docs/CONTRACT-gates.md). Do not delete or weaken a rule.
- Do not skip or change the checks that run before a commit; Tirith blocks the ways to do it.
- Keep the data contracts (`docs/CONTRACT-*.md`) in step with the code that implements them.
- No servers and no ports: the page and the app read a local state file.
- Never write to a real `~/.tirith` from tests; use `TIRITH_HOME` with a temporary folder.
- Match the style of the surrounding code; keep changes small and focused.

## Pull requests

Branch from `main`, keep one topic per PR, fill in the template, and make sure the tests above pass.
Please read the [Code of Conduct](CODE_OF_CONDUCT.md).
