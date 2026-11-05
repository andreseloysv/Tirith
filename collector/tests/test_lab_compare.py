"""tirith lab compare: fake lab run reports, no models."""

import json

from tirith import labcompare
from tirith.cli import main


def _rep(name, cost, secs, items=1, ok=True, dup=0, ints=0, promos=()):
    return {"scenario": name, "cost_usd": cost, "seconds": secs, "items_to_main": items,
            "owner_interruptions": ints, "blocked_asks": 0, "duplicate_items": dup, "checked": None,
            "ok": ok, "promotions": [{"role": r, "reason": "x"} for r in promos]}


def _dump(folder, reps):
    folder.mkdir()
    for r in reps:
        (folder / f"{r['scenario']}.json").write_text(json.dumps(r))
    return folder


def _fixtures(tmp_path):
    on = _dump(tmp_path / "on", [_rep("a", 0.5, 100, 3, promos=["manager"]), _rep("b", 0.1, 20),
                                 _rep("only-on", 1, 1)])
    fx = _dump(tmp_path / "fx", [_rep("a", 1.0, 150, 3, dup=1), _rep("b", 0.2, 30, ints=1)])
    return on, fx


def test_lab_compare_metrics_and_recommendation(tmp_path):
    on, fx = _fixtures(tmp_path)
    c = labcompare.compare(labcompare.load_reports(on), labcompare.load_reports(fx))
    assert c["scenarios"] == ["a", "b"] and c["skipped"] == ["only-on"]
    assert c["on_demand"]["cost_usd"] == 0.6 and c["fixed"]["cost_usd"] == 1.2
    assert c["delta"]["seconds"] == -60
    assert c["on_demand"]["first_try_success"] == 2 and c["fixed"]["first_try_success"] == 0
    assert c["recommendation"] == {"promote_min_items": 3, "max_cost_ratio": 0.5,
                                   "promotions_that_paid_off": 1, "promotions_total": 1}


def test_lab_compare_no_payoff(tmp_path):
    on = _dump(tmp_path / "on", [_rep("a", 1, 1, promos=["manager"], ok=False)])
    fx = _dump(tmp_path / "fx", [_rep("a", 1, 1)])
    c = labcompare.compare(labcompare.load_reports(on), labcompare.load_reports(fx))
    assert c["recommendation"]["promote_min_items"] is None
    assert "n/a" in labcompare.markdown(c)


def test_lab_compare_cli(tmp_path):
    on, fx = _fixtures(tmp_path)
    out = tmp_path / "out"
    assert main(["lab", "compare", "--on-demand", str(on), "--fixed", str(fx), "--out", str(out)]) == 0
    assert json.loads((out / "compare.json").read_text())["scenarios"] == ["a", "b"]
    assert "| metric | on-demand | fixed | delta |" in (out / "compare.md").read_text()
    empty = tmp_path / "empty"
    empty.mkdir()
    assert main(["lab", "compare", "--on-demand", str(on), "--fixed", str(empty), "--out", str(out)]) == 2
