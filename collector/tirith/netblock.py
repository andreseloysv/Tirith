"""Blocked destinations: domains and addresses no agent may reach (``docs/CONTRACT-access-audit.md``).

The owner blocks a destination from the «Accesos» page (a ``setup`` request with action ``net-block``, applied by
code at once) or with ``tirith net block <domain|ip|cidr>``. A rule ``example.com`` matches it and every
subdomain; ``*.example.com`` the same; an address or a CIDR range matches addresses. Two guards use the rules:
the tool gate refuses a call whose destination is blocked before it runs (rule ``net-blocked``), and the network
watch stops a lane's process that opened a connection to one anyway (a script calling an API inside). Without
root nothing can cut a single connection of the agent program itself: that is recorded, not stopped.
"""

from __future__ import annotations

import ipaddress
from pathlib import Path

from tirith.home import now_iso
from tirith.store import read_json, write_json_atomic

FILE = "netblock.json"


def rules(home: Path) -> list[dict]:
    data = read_json(Path(home) / FILE, {}) or {}
    return [r for r in data.get("rules") or [] if isinstance(r, dict) and r.get("match")]


def clean(match: str) -> str:
    """``https://api.example.com:443/x`` -> ``api.example.com``; addresses and ranges stay; ``*.`` is dropped."""
    text = str(match or "").strip().lower()
    if "://" in text:
        text = text.split("://", 1)[1]
    text = text.split("/", 1)[0] if not _network(text) else text
    if text.count(":") == 1:  # host:port (an IPv6 address has more colons)
        text = text.split(":", 1)[0]
    return text.removeprefix("*.").strip(".[]")


def _network(text: str):
    try:
        return ipaddress.ip_network(text, strict=False)
    except ValueError:
        return None


def add(home: Path, match: str, by: str = "owner") -> str:
    target = clean(match)
    if not target or (" " in target):
        raise ValueError(f"not a domain, address or range: {match!r}")
    kept = [r for r in rules(home) if r["match"] != target]
    write_json_atomic(Path(home) / FILE, {"rules": [*kept, {"match": target, "at": now_iso(), "by": by}]})
    return target


def remove(home: Path, match: str) -> bool:
    target, before = clean(match), rules(home)
    kept = [r for r in before if r["match"] != target]
    write_json_atomic(Path(home) / FILE, {"rules": kept})
    return len(kept) != len(before)


def blocked(rule_list: list[dict], *names) -> str | None:
    """The rule that blocks any of ``names`` (domains or addresses), else None."""
    for name in names:
        host = clean(name) if name else ""
        if not host:
            continue
        address = _network(host)
        for rule in rule_list:
            match = rule["match"]
            net = _network(match)
            if net is not None and address is not None and address.num_addresses == 1:
                if address.network_address in net:
                    return match
            elif net is None and address is None and (host == match or host.endswith("." + match)):
                return match
    return None
