from tirith import manifest, mergeq, review


def test_include_adds_pipelines_from_other_file(tmp_path):
    (tmp_path / "tirith.toml").write_text(
        'include = ["extra.toml"]\n\n[[pipelines]]\nid = "ci"\n\n[[pipelines.stages]]\nname = "test"\nrun = "true"\n')
    (tmp_path / "extra.toml").write_text('[[pipelines]]\nid = "side"\n\n[[pipelines.stages]]\nname = "go"\nrun = "true"\n')
    loaded = manifest.load(tmp_path)
    assert [p.id for p in loaded.pipelines] == ["ci", "side"]
    assert not loaded.problems


def test_missing_include_is_a_problem(tmp_path):
    (tmp_path / "tirith.toml").write_text('include = ["gone.toml"]\n')
    assert [p.rule for p in manifest.load(tmp_path).problems] == ["toml-syntax"]


def test_rust_port_does_not_gate(tmp_path):
    (tmp_path / "tirith.toml").write_text(
        'include = ["extra.toml"]\n\n[[pipelines]]\nid = "ci"\n\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n')
    (tmp_path / "extra.toml").write_text(
        '[[pipelines]]\nid = "side"\n\n[[pipelines.stages]]\nname = "port-build"\nrun = "true"\n\n'
        '[[pipelines.stages]]\nname = "port-test"\nkind = "test"\nrun = "true"\n')
    assert [(pipeline, stage.name) for pipeline, stage in mergeq.stages_of(tmp_path)] == [("ci", "build")]
    assert review.test_stage(tmp_path) is None
    assert "side" in [p.id for p in manifest.load(tmp_path).pipelines]


def test_coverage_counts_only_gating_pipelines(tmp_path):
    """Build and test only in an include file: the merge queue finds no stage, so the check must say so."""
    (tmp_path / "tirith.toml").write_text(
        'include = ["extra.toml"]\ndeploy = "none"\n\n[[pipelines]]\nid = "ci"\n\n[[pipelines.stages]]\n'
        'name = "notes"\nrun = "true"\n')
    (tmp_path / "extra.toml").write_text(
        '[[pipelines]]\nid = "side"\n\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n\n'
        '[[pipelines.stages]]\nname = "test"\nrun = "true"\n')
    problems = manifest.check(tmp_path, [], ci=True)
    assert [p.rule for p in problems] == ["pipeline-coverage"]
    assert "build, test" in problems[0].message
    (tmp_path / "tirith.toml").write_text(
        'include = ["extra.toml"]\ndeploy = "none"\n\n[[pipelines]]\nid = "ci"\n\n[[pipelines.stages]]\n'
        'name = "build"\nrun = "true"\n\n[[pipelines.stages]]\nname = "test"\nrun = "true"\n')
    assert manifest.check(tmp_path, [], ci=True) == []
