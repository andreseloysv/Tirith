

def test_linked_worktree_belongs_to_its_main_repository(tmp_path):
    """An agent lane working in a git worktree is part of the repository's project, not a new one."""
    import subprocess
    from tirith import projects
    repo = tmp_path / "repo"
    repo.mkdir()
    run = lambda *a: subprocess.run(["git", "-C", str(repo), *a], check=True, capture_output=True)
    run("init", "-q")
    (repo / "a.txt").write_text("a", encoding="utf-8")
    run("add", "a.txt")
    run("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "init")
    lane = tmp_path / "lanes" / "x"
    run("worktree", "add", "-q", "-b", "lane-x", str(lane))
    items = [projects.Entry("p", "P", projects.real(repo), "#888888", False)]
    found = projects.match(lane / "a.txt", items)
    assert found is not None and found.id == "p"
