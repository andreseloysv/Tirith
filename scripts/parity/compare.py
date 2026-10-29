"""Structural comparison of two ``state.json`` payloads (Python collector vs Rust collector).

A section the Rust state does not have yet is reported as "not ported yet" and is not a failure;
everything the Rust state does have must equal the Python value.  Run by ``run.py``; pure functions so
``collector/tests/test_parity_compare.py`` can test them.
"""

from __future__ import annotations

# The one place that lists fields allowed to differ between two runs (keys at any depth).
VOLATILE_KEYS = frozenset({"generated_at"})

def strip_volatile(value):
    if isinstance(value, dict):
        return {k: strip_volatile(v) for k, v in value.items() if k not in VOLATILE_KEYS}
    if isinstance(value, list):
        return [strip_volatile(v) for v in value]
    return value


def _short(value, limit: int = 160) -> str:
    import json

    text = json.dumps(value, ensure_ascii=False, sort_keys=True)
    return text if len(text) <= limit else text[: limit - 3] + "..."


def diff(py, rs, path: str = "") -> list[str]:
    """Readable differences between ``py`` and ``rs`` (``[]`` = equal); ``path`` names where we are."""
    if isinstance(py, dict) and isinstance(rs, dict):
        out: list[str] = []
        for key in sorted(set(py) | set(rs)):
            where = f"{path}.{key}" if path else key
            if key not in rs:
                out.append(f"{where}: missing in rust (python: {_short(py[key])})")
            elif key not in py:
                out.append(f"{where}: only in rust (rust: {_short(rs[key])})")
            else:
                out.extend(diff(py[key], rs[key], where))
        return out
    if isinstance(py, list) and isinstance(rs, list):
        out = []
        if len(py) != len(rs):
            out.append(f"{path}: {len(py)} rows in python, {len(rs)} in rust")
        for index, (a, b) in enumerate(zip(py, rs)):
            out.extend(diff(a, b, f"{path}[{index}]"))
        return out
    if py == rs and type(py) is type(rs):
        return []
    if isinstance(py, (int, float)) and isinstance(rs, (int, float)) and not isinstance(py, bool) \
            and not isinstance(rs, bool) and py == rs:
        return []
    return [f"{path}: python {_short(py)} != rust {_short(rs)}"]


def compare_states(py: dict, rs: dict) -> tuple[list[str], list[str], list[str]]:
    """``(ok, not_ported, differences)`` section names / lines for two states.

    Sections are the keys of the state root and of each ``projects[]`` row (a project is matched by id); a
    section key missing from the Rust state is "not ported yet", a missing key anywhere deeper is a difference."""
    py, rs = strip_volatile(py), strip_volatile(rs)
    ok: list[str] = []
    not_ported: list[str] = []
    diffs: list[str] = []
    for key in sorted(py):
        if key == "projects":
            continue
        if key not in rs:
            not_ported.append(key)
        else:
            found = diff(py[key], rs[key], key)
            (diffs.extend(found) if found else ok.append(key))
    for key in sorted(set(rs) - set(py)):
        diffs.append(f"{key}: only in rust")

    py_rows = {str(p.get("id")): p for p in py.get("projects") or []}
    rs_rows = {str(p.get("id")): p for p in rs.get("projects") or []}
    if "projects" not in rs:
        not_ported.append("projects")
    else:
        if list(py_rows) != list(rs_rows):
            diffs.append(f"projects: ids/order python {list(py_rows)} != rust {list(rs_rows)}")
        ported_keys: set[str] = set()
        bad: set[str] = set()
        for pid, prow in py_rows.items():
            rrow = rs_rows.get(pid)
            if rrow is None:
                continue
            ported_keys |= set(rrow)
            for key in sorted(set(prow) & set(rrow)):
                found = diff(prow[key], rrow[key], f"projects[{pid}].{key}")
                if found:
                    bad.add(key)
                diffs.extend(found)
            for key in sorted(set(rrow) - set(prow)):
                bad.add(key)
                diffs.append(f"projects[{pid}].{key}: only in rust")
        every = {k for row in py_rows.values() for k in row}
        not_ported.extend(f"projects[].{k}" for k in sorted(every - ported_keys))
        ok.extend(f"projects[].{k}" for k in sorted(ported_keys & every) if k not in bad)
    return ok, not_ported, diffs
