"""Slim console entry: a hook call imports only what it needs, every other command loads ``tirith.cli``.

An agent CLI starts ``tirith hook --agent <name> --event <e>`` on every tool call, so its start-up time is paid
many times per request. ``tirith.cli`` imports the whole collector; the plain hook path (no ``install`` /
``uninstall`` / ``verify``) needs only ``tirith.hooks``, ``tirith.home`` and ``tirith.gates``."""

from __future__ import annotations

import sys

def _hook_args(argv: list[str]):
    """``(agent, event, home)`` from ``hook --agent A [--event E] [--home H]``; ``None`` if anything else is there."""
    agent = event = home = None
    rest = argv[1:]
    i = 0
    while i < len(rest):
        a = rest[i]
        name, eq, val = a.partition("=")
        if name in ("--agent", "--event", "--home"):
            if not eq:
                i += 1
                if i >= len(rest):
                    return None
                val = rest[i]
            if name == "--agent":
                agent = val
            elif name == "--event":
                event = val
            else:
                home = val
        else:
            return None
        i += 1
    return (agent, event, home) if agent else None


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    parsed = _hook_args(argv) if argv[:1] == ["hook"] else None
    if parsed is not None:
        agent, event, home = parsed
        try:  # a hook call must never break the agent
            from tirith import hooks as hooks_mod
            from tirith.home import tirith_home
            return hooks_mod.run_hook(tirith_home(home), agent, event, sys.stdin)
        except BaseException:
            return 0
    from tirith.cli import main as cli_main
    return cli_main(argv)


if __name__ == "__main__":
    sys.exit(main())
