# Contributing

```bash
./run                       # build, start the daemon, open the page
cargo build -p tirith       # the daemon and CLI (rust/tirith)
cargo clippy -p tirith      # must be clean
cargo check -p tirith-app   # the desktop app (app/)
```

Try changes against a throwaway home so your real one is untouched:
`TIRITH_HOME=/tmp/th TIRITH_PORT=7799 target/debug/tirith serve`, with a `config.toml` in `/tmp/th`
that lists a scratch git repo as a project.

Rules of the codebase:

- `rust/tirith/src` has a size limit (tirith.toml and CI): simplify before adding.
- No unit tests: check a change by running it (the daemon, a lane through the merge queue, the page).
- Commit messages, file names and added lines never contain the words the commit guard forbids.

The existing unit checks live in `rust/tirith/checks/`, outside the daemon source size budget. Run them with
`cargo test -p tirith`. For the Codex integration, build the daemon and run `scripts/checks/codex.py` in a Python
environment with PyNaCl installed. It uses a temporary home, a fake Codex executable and a local signed,
encrypted phone service; it checks model changes, all four roles, queued turns, interruption, persistence and
a lane through the merge queue without calling a live model. `--keep` leaves the preview running until a
`stop` file is created in the printed temporary home.

`node scripts/checks/models-ui.mjs` checks the visual model buttons and effort slider in a real browser, including
encrypted phone save/reset and desktop-to-phone refresh. Set `AM_ROOT` to the AgentsMessenger checkout (the sibling
folder by default) and `PLAYWRIGHT_MODULE` to an installed Playwright module with Chromium. It starts both servers
with temporary state and does not call a model. Screenshots are left in its printed temporary directory.
