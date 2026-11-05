"""On-demand roles: the worker becomes a manager when the work needs one."""

from tirith import promote as pr


def test_promote_to_manager_when_task_does_not_fit_a_lane():
    assert "does not fit" in pr.promote_to_manager("on-demand", "x" * 400)
    assert "does not fit" in pr.promote_to_manager("on-demand", "Fix it", diff=(50, 10, 10))


def test_promote_to_manager_with_parallel_items():
    assert "2 items" in pr.promote_to_manager("on-demand", "Fix the bug", parallel_items=2)
    assert pr.promote_to_manager("on-demand", "Fix the bug", parallel_items=1) is None


def test_promote_to_manager_when_asking_for_subagents():
    assert "sub-agents" in pr.promote_to_manager("on-demand", "Fix the bug", subagents=True)
    assert "sub-agents" in pr.promote_to_manager("on-demand", "Use sub-agents to fix it")


def test_promote_to_manager_only_on_demand():
    assert pr.promote_to_manager("always", "x" * 400, parallel_items=3, subagents=True) is None
    assert pr.promote_to_manager("off", "x" * 400, parallel_items=3, subagents=True) is None
    assert pr.promote_to_manager("on-demand", "Fix the bug") is None


def test_promote_to_ceo_with_more_than_two_workers():
    assert "3 workers" in pr.promote_to_ceo("on-demand", workers=3)
    assert pr.promote_to_ceo("on-demand", workers=2) is None


def test_promote_to_ceo_after_an_hour_unattended():
    assert "nobody watching" in pr.promote_to_ceo("on-demand", unattended_hours=1.5)
    assert pr.promote_to_ceo("on-demand", unattended_hours=1) is None


def test_promote_to_ceo_when_selfcheck_cannot_fix():
    assert "self-check" in pr.promote_to_ceo("on-demand", selfcheck_failed=True)
    assert pr.ceo_decision("on-demand", selfcheck_failed=True)["rule"] == "selfcheck"


def test_promote_to_ceo_only_on_demand():
    for mode in ("always", "off"):
        assert pr.promote_to_ceo(mode, workers=9, unattended_hours=9, selfcheck_failed=True) is None
