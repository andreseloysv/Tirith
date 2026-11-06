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

- `rust/tirith/src` stays under 2000 lines. Simplify before adding.
- No unit tests: check a change by running it (the daemon, a lane through the merge queue, the page).
- Commit messages, file names and added lines never contain the words the commit guard forbids.
