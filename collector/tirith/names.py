"""Hero identities: original syllable-table names, epithets, renown, ranks.

Names are generated deterministically from the agent id (a stable hash picks
syllables), so the same agent always gets the same hero name without any
stored seed. Identities persist in ``agents.json``; this module only does
the pure generation and the renown/rank arithmetic.
"""

from __future__ import annotations

import hashlib

# Original syllable tables for Tirith hero names.
_ONSETS = (
    "Al", "Bel", "Cor", "Dal", "El", "Fen", "Gal", "Hal", "Il", "Jor",
    "Kel", "Lor", "Mar", "Nel", "Od", "Per", "Quil", "Ren", "Sel", "Tar",
    "Ul", "Ver", "Wil", "Yar", "Zel", "Bran", "Dun", "Es", "Fal", "Gor",
)
_CORES = (
    "a", "e", "i", "o", "u", "ae", "ia", "io", "ea", "au",
)
_CODAS = (
    "ric", "vard", "ren", "dric", "mar", "win", "gar", "las", "nor", "fin",
    "dor", "mir", "sar", "thil", "wen", "dil", "ron", "phin", "tar", "vik",
)
_ENDINGS = ("", "", "", "a", "e", "is", "on", "us")

_EPITHET_HEADS = (
    "Stone", "Iron", "Swift", "Keen", "Bright", "Quiet", "Steady", "Amber",
    "Silver", "Prism", "Ember", "Harbour", "Lantern", "Ledger", "Quill",
    "Shield", "Star", "Tower", "Gate", "Road",
)
_EPITHET_TAILS = (
    "hand", "step", "eye", "well", "keeper", "watch", "smith", "runner",
    "scribe", "warden", "hand", "step", "brow", "song", "mark",
)
_EPITHET_PHRASES = (
    "of the Seventh Gate", "of the White Road", "of the Tower Stair",
    "of the Third Ring", "of the Lantern Stair", "of the Harbour Bell",
    "of the Star Court", "of the Long Watch",
)

# Rank thresholds from the lore: (minimum renown, rank name), ascending.
RANKS = (
    (0, "recruit"), (100, "sentinel"), (300, "sergeant"),
    (700, "captain"), (1500, "marshal"),
)

RENOWN_LANE_DONE = 100
RENOWN_LANE_FAILED = -30
RENOWN_STAGE_PASSED = 20


def _pick(seed: str, options: tuple[str, ...], salt: str) -> str:
    digest = hashlib.sha256(f"{seed}:{salt}".encode()).digest()
    return options[int.from_bytes(digest[:4], "big") % len(options)]


def hero_name(agent_id: str) -> str:
    """Stable given name for an agent id, from the syllable tables."""
    name = _pick(agent_id, _ONSETS, "onset") + _pick(agent_id, _CORES, "core")
    name += _pick(agent_id, _CODAS, "coda") + _pick(agent_id, _ENDINGS, "ending")
    # Tidy doubled vowels from table joins, keep at least 3 letters.
    while "aa" in name or "ee" in name or "ii" in name:
        name = name.replace("aa", "a").replace("ee", "e").replace("ii", "i")
    return name[:1].upper() + name[1:]


def hero_epithet(agent_id: str, hero_class: str = "", feature: str = "") -> str:
    """Stable fallback epithet; the brain may replace it from real work."""
    seed = f"{agent_id}|{hero_class}|{feature}"
    digest = hashlib.sha256(seed.encode()).digest()
    if digest[0] % 5 == 0:
        return _pick(seed, _EPITHET_PHRASES, "phrase")
    return _pick(seed, _EPITHET_HEADS, "head") + _pick(seed, _EPITHET_TAILS, "tail")


def rank_for(renown: int) -> str:
    """Rank name for a renown score (thresholds 0/100/300/700/1500)."""
    rank = RANKS[0][1]
    for minimum, name in RANKS:
        if renown >= minimum:
            rank = name
    return rank


def apply_renown(current: int, delta: int) -> int:
    """Apply a renown change; renown never drops below zero."""
    return max(0, current + delta)
