"""``tirith do "<goal>"``: a big goal becomes small plan items, each with its check, and the lanes start.

The split is mechanical (no model): lines, bullets, numbers, sentences and ``;`` each become one item of about
30 minutes. Every item gets a done check, ``pytest -k <slug>``, so the lane writes the test that proves it.
"""

from __future__ import annotations

import re
import unicodedata

from tirith import plangate as plangate_mod

MAX_ITEMS = 8
_BULLET = re.compile(r"^\s*(?:[-*•]|\d+[.)])\s+")
_SPLIT = re.compile(r"(?<=[.!?])\s+(?=[A-ZÁÉÍÓÚÑ¿¡])|;\s+|\s+(?:and then|and also|y luego|y también)\s+", re.IGNORECASE)


def split_goal(goal: str) -> list[str]:
    """The small deliverables of ``goal``, in order, without duplicates; a goal that cannot be split is one."""
    parts: list[str] = []
    for line in goal.splitlines():
        line = _BULLET.sub("", line).strip()
        parts += [p.strip(" .;") for p in _SPLIT.split(line) if p and p.strip(" .;")]
    seen, out = set(), []
    for part in parts:
        text = part[:1].upper() + part[1:]
        text = text.rstrip(".") + "."
        if plangate_mod.size_problem(text):  # still too long: cut it at its last comma before the limit
            cut = text[:plangate_mod.MAX_ITEM_CHARS - 1].rsplit(",", 1)[0].rstrip(" ,")
            text = cut.rstrip(".") + "."
        if text.lower() not in seen:
            seen.add(text.lower())
            out.append(text)
    return out[:MAX_ITEMS]


def slug(text: str, taken: set[str]) -> str:
    """A short pytest-safe name from the first words of ``text`` (unique among ``taken``)."""
    plain = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode().lower()
    words = re.findall(r"[a-z0-9]+", plain)[:4] or ["item"]
    base = "do_" + "_".join(words)
    name, n = base, 2
    while name in taken:
        name, n = f"{base}_{n}", n + 1
    taken.add(name)
    return name


def check_for(name: str) -> str:
    return f"cd collector && uv run -q pytest -q tests -k {name}"


def plan_items(goal: str) -> list[tuple[str, str]]:
    """``(item text, done check)`` pairs for ``goal``."""
    taken: set[str] = set()
    items = []
    for text in split_goal(goal):
        name = slug(text, taken)
        items.append((f"{text[:-1]} (test: {name}).", check_for(name)))
    return items


def next_step(steps: list[dict]) -> str:
    ids = [int(s["id"]) for s in steps if str(s["id"]).isdigit()]
    return str(max(ids, default=0) + 1)
