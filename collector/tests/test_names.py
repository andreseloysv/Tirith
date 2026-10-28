"""Hero names, epithets, renown arithmetic and rank thresholds."""

from tirith.names import apply_renown, hero_epithet, hero_name, rank_for


def test_names_are_stable_and_distinct() -> None:
    assert hero_name("w1-data") == hero_name("w1-data")
    assert hero_name("w1-data") != hero_name("w6-perf")
    assert len(hero_name("w1-data")) >= 3
    assert hero_name("w1-data")[:1].isupper()


def test_epithets_are_stable() -> None:
    first = hero_epithet("w1-data", "stonewright", "Room database")
    assert first == hero_epithet("w1-data", "stonewright", "Room database")
    assert first.strip()


def test_rank_thresholds() -> None:
    assert rank_for(0) == "recruit"
    assert rank_for(99) == "recruit"
    assert rank_for(100) == "sentinel"
    assert rank_for(299) == "sentinel"
    assert rank_for(300) == "sergeant"
    assert rank_for(699) == "sergeant"
    assert rank_for(700) == "captain"
    assert rank_for(1499) == "captain"
    assert rank_for(1500) == "marshal"
    assert rank_for(9000) == "marshal"


def test_renown_never_goes_negative() -> None:
    assert apply_renown(10, -30) == 0
    assert apply_renown(50, 100) == 150
