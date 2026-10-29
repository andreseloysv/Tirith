# Contract addition: file references in the chat (v1)

Every file a chat message names (manager answers, says, waits, the owner's requests) is a link. A click
opens the file with the `[app] editor` or the OS default app, at the line when the reference has one.

```
message text -> collector: paths that exist inside a registered project folder -> state.file_refs
             -> md.js links only those -> click (files.js)
             -> desktop app: invoke open_file -> tirith open --project <id> --path <p> [--line N] [--col N]
             -> browser:     tirith://open?project=<id>&path=<p>&line=<n>&token=<t> -> tirith url open
             -> filerefs.resolve again (refuses outside the project folders) -> editor / OS opener
```

## What counts as a file reference

The same rules in `collector/tirith/filerefs.py` and `web/js/md.js`:

- A word shaped like a path (`[\w.@+-]` segments split by `/`, optional leading `~/`, `./`, `../`
  or `/`, optional trailing `/`) that has a `/` in it, a leading `~/` `./` `../` `/`, or a file name
  with an extension (`README.md`, `.gitignore`).
- An optional `:line` or `:line:col` (`collector/tirith/cli.py:42:7`). Trailing `.,:;!?` stays text.
- Found in inline code (one word only: `` `uv run pytest x.py` `` stays code), plain text, and the target
  of a Markdown link (`[the cli](collector/tirith/cli.py:42)`). Fenced code blocks are never linked.
- Never: URLs (`://`), flags (`-x`), bare words (`docs`, `tirith`), times (`12:30`), versions
  (`v1.2`), `$VAR/x`.

## `state.file_refs`

`{project id: [path, ...]}`, `""` for messages with no project. Paths are normalized (no leading `./`,
no trailing `/`), sorted, and listed only when they exist. Relative paths resolve against the
message's project folder; a message with no project resolves against the tirith repo. A path whose
real location (symlinks followed) is outside every registered project folder is not listed. At most
2000 distinct paths are checked per collector cycle.

## Opening

`tirith open --project <id> --path <p> [--line N] [--col N]` (the desktop app's `open_file` command)
and `tirith://open?...` (the link handler, token checked as for every `tirith://` link) both:

1. Resolve the path again as above. Exit 3 with the reason on stderr when it is not a path, does not
   exist, has no project folder to resolve in, or is outside the registered project folders.
2. Open it:
   - `[app] editor` in `config.toml`: a template with `{file}` `{line}` `{col}`, or a command name.
     `code`, `code-insiders`, `codium`, `cursor`, `windsurf` get `-g file:line:col`; `subl`, `zed`,
     `idea`, `pycharm`, `webstorm` get `file:line:col`; anything else gets the file.
   - Else macOS: images, PDFs and media with `open`; folders in Finder (bundles revealed, never
     started); everything else with `open -t` (the default text editor), so a click never runs a
     script. Linux: `xdg-open`, refused for executables. Windows needs `[app] editor`.
3. Print `opened <real path>`.

The page never navigates for a click. Without the desktop app and without the link handler, a click
copies the path and says how to install the handler (`tirith url install`).
