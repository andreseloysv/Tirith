"""<home>/company.jsonl: every promotion or retirement with the rule, the measured value and the threshold."""

import pytest

from tirith import promote as pr


def test_company_log_records_a_promotion(tmp_path):
    decision = pr.manager_decision("on-demand", "Fix the bug", parallel_items=3)
    pr.log_decision(tmp_path, decision)
    (row,) = pr.read_log(tmp_path)
    assert (tmp_path / "company.jsonl").exists()
    assert row["event"] == "promote" and row["role"] == "manager"
    assert row["rule"] == "parallel_items" and row["measured"] == 3 and row["threshold"] == pr.PARALLEL_ITEMS
    assert row["ts"].endswith("Z")


def test_company_log_records_the_size_rule_with_its_numbers(tmp_path):
    decision = pr.manager_decision("on-demand", "x" * 400)
    pr.log_decision(tmp_path, decision)
    row = pr.read_log(tmp_path)[0]
    assert row["rule"] == "task_size" and row["measured"] == 400 and row["threshold"] < 400


def test_company_log_records_a_retirement_and_appends(tmp_path):
    pr.log_change(tmp_path, "promote", "ceo", "workers", 3, 2, "3 workers")
    pr.log_change(tmp_path, "retire", "ceo", "idle_seconds", 3700, 3600, "idle")
    rows = pr.read_log(tmp_path)
    assert [r["event"] for r in rows] == ["promote", "retire"]
    assert rows[1]["measured"] == 3700 and rows[1]["threshold"] == 3600


def test_company_log_no_decision_for_a_small_task():
    assert pr.manager_decision("on-demand", "Fix the typo") is None
    assert pr.manager_decision("always", "x" * 400) is None


def test_company_log_rejects_an_unknown_event(tmp_path):
    with pytest.raises(ValueError):
        pr.log_change(tmp_path, "hire", "ceo", "r", 1, 1)
