/// A repo with `plan` as its committed docs/TODO.md; the new HEAD.
async fn repo_with_plan(repo: &std::path::Path, plan: &str) -> String {
    std::fs::create_dir_all(repo.join("docs")).unwrap();
    std::fs::write(repo.join("docs/TODO.md"), plan).unwrap();
    for a in [
        "init -q",
        "config user.email t@t",
        "config user.name t",
        "add .",
        "commit -qm i",
    ] {
        crate::app::git(repo, &a.split(' ').collect::<Vec<_>>())
            .await
            .unwrap();
    }
    crate::app::git(repo, &["rev-parse", "HEAD"]).await.unwrap()
}
#[tokio::test]
async fn plan_flushes_on_start() {
    // The 2026-11-20 loss: a plan written by a daemon that restarted before its commit never got committed,
    // because plan_dirty lived only in memory. On start every project counts as dirty, so the first flush
    // commits such a plan (and a project whose plan is clean gets no commit).
    let base = std::env::temp_dir().join(format!("tirith-pfs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (repo, clean) = (base.join("repo"), base.join("clean"));
    repo_with_plan(&repo, "## 1. Step\n- [ ] Old\n").await;
    let clean_head = repo_with_plan(&clean, "## 1. Step\n- [ ] Old\n").await;
    std::fs::write(
        repo.join("docs/TODO.md"),
        "## 1. Step\n- [ ] Old\n- [ ] The ui-deck item (check: `true`)\n",
    )
    .unwrap();
    let home = base.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let cfg = format!(
        "[[projects]]\nid = \"p\"\nrepo = \"{}\"\n[[projects]]\nid = \"q\"\nrepo = \"{}\"\n",
        repo.display(),
        clean.display()
    );
    std::fs::write(home.join("config.toml"), cfg).unwrap();
    let app = crate::app::App::open_at(home);
    assert!(
        app.plan_dirty.lock().unwrap().contains("p"),
        "a fresh daemon starts with every project plan-dirty"
    );
    super::flush(&app, None).await;
    assert_eq!(
        crate::app::git(&repo, &["status", "--porcelain", "--", "docs/TODO.md"])
            .await
            .unwrap(),
        ""
    );
    assert!(crate::app::git(&repo, &["show", "HEAD:docs/TODO.md"])
        .await
        .unwrap()
        .contains("The ui-deck item"));
    assert_eq!(
        crate::app::git(&clean, &["rev-parse", "HEAD"])
            .await
            .unwrap(),
        clean_head,
        "a clean plan gets no start commit"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
#[test]
fn plan_add_rejects_invalid_checks() {
    let b = super::check::check_problem;
    assert!(
        [
            "neobot run -s test",
            "tirith check",
            "cd collector && ls",
            "cargo test && neobot run"
        ]
        .iter()
        .all(|c| b(c).is_some())
            && ["cargo test -q x", "neobot plan show p", "cd collectors"]
                .iter()
                .all(|c| b(c).is_none())
    );
}
#[test]
fn plan_add_accepts_cargo_test_filter() {
    let b = super::check::check_problem;
    for good in [
        "cargo test -q -p tirith mi_test",
        "grep -rq x rust/tirith/checks && cargo test -q -p tirith",
        "cargo test -q -p tirith mi_test && sh tools/size.sh",
        "grep -qE 'cargo|tirith comando_inventado' README.md",
        "grep -q \"; tirith comando_inventado\" README.md",
    ] {
        assert_eq!(b(good), None, "{good}");
    }
    for bad in [
        "tirith comando_inventado",
        "  tirith comando_inventado",
        "true;tirith comando_inventado",
        "true && tirith comando_inventado",
        "false || tirith comando_inventado",
        "echo x | tirith comando_inventado",
        "echo 'x' && tirith comando_inventado",
        "(tirith comando_inventado)",
        "! tirith comando_inventado",
        "for i in 1; do tirith comando_inventado; done",
        "if true; then tirith comando_inventado; fi",
        "test -n \"$(tirith comando_inventado)\"",
        "neobot comando_inventado",
        "true && neobot comando_inventado",
    ] {
        assert!(b(bad).is_some(), "{bad}");
    }
}
#[test]
fn a_rewritten_check_keeps_the_lane_on_its_item() {
    let (old, now) = (
        "Make the chat a PWA (check: `npm test`)",
        "Make the chat a PWA (check: `export PATH=/opt/homebrew/bin:$PATH; npm test`)",
    );
    assert!(
        super::same(old, now)
            && super::same(old, "Make the chat a PWA")
            && !super::same(old, "Make the chat a PWA, with icons (check: `npm test`)")
    );
    // The lane started with the old text: merged, it ticks the item as the plan has it now, and no other.
    let (text, n) = super::tick_text(
        &format!("## 1. Step\n- [ ] {now}\n- [ ] Another\n"),
        &[("1".into(), old.into())],
    );
    assert_eq!(
        (
            n,
            text.contains(&format!("- [x] {now}")),
            text.contains("- [ ] Another")
        ),
        (1, true, true)
    );
}
#[tokio::test]
async fn plan_step_creates_a_step() {
    let base = std::env::temp_dir().join(format!("tirith-psc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (repo, home) = (base.join("repo"), base.join("home"));
    // The last step is not the last heading, and the file ends in blank lines.
    let start = "## 1. One\n- [x] Old\n\n## 2. Two\n\n## Notes\nKept\n\n\n";
    repo_with_plan(&repo, start).await;
    std::fs::create_dir_all(&home).unwrap();
    let cfg = format!(
        "[[projects]]\nid = \"p\"\nrepo = \"{}\"\nguard_words = [\"secretword\"]\n",
        repo.display()
    );
    std::fs::write(home.join("config.toml"), cfg).unwrap();
    let app = crate::app::App::open_at(home);
    super::flush(&app, None).await;
    let file = repo.join("docs/TODO.md");
    let plan = || std::fs::read_to_string(&file).unwrap();
    let dirty = || app.plan_dirty.lock().unwrap().contains("p");

    for (num, title, intro) in [
        ("2", "Again", ""),
        ("4", "Skips one", ""),
        ("0", "Before the first", ""),
        ("03", "Not its number", ""),
        ("", "No number", ""),
        ("3", "  ", ""),
        ("3", "Two\nlines", ""),
        ("3", "Title", "Prose\n## 9. Smuggled"),
        ("3", "Title", "Prose\n- [ ] Smuggled"),
        ("3", "A secretword", ""),
        ("3", "Title", "A secretword"),
    ] {
        let refused = super::add_step(&app, "p", num, title, intro).await;
        assert!(refused.is_err(), "{num:?} {title:?} {intro:?} is refused");
        assert_eq!(plan(), start, "a refusal leaves the file as it was");
        assert!(!dirty(), "a refusal queues no commit");
    }

    super::add_step(&app, "p", "3", "Tres", "Su párrafo.")
        .await
        .unwrap();
    let three =
        "## 1. One\n- [x] Old\n\n## 2. Two\n\n## Notes\nKept\n\n## 3. Tres\n\nSu párrafo.\n";
    assert_eq!(plan(), three);
    assert!(dirty(), "a new step queues its commit");

    super::add(&app, "p", "3", "Its item").await.unwrap();
    let with_item = format!("{three}- [ ] Its item\n");
    assert_eq!(plan(), with_item);
    for num in ["3", "5"] {
        assert!(super::add_step(&app, "p", num, "No", "").await.is_err());
        assert_eq!(plan(), with_item);
    }

    super::add_step(&app, "p", "4", "Cuatro", "").await.unwrap();
    super::add(&app, "p", "4", "Last").await.unwrap();
    let all = format!("{with_item}\n## 4. Cuatro\n- [ ] Last\n");
    assert_eq!(plan(), all);

    super::flush(&app, None).await;
    let committed = crate::app::git(&repo, &["show", "HEAD:docs/TODO.md"])
        .await
        .unwrap();
    assert_eq!(committed.trim_end(), all.trim_end());

    std::fs::write(&file, "# Plan\n").unwrap();
    super::add_step(&app, "p", "1", "First", "").await.unwrap();
    assert_eq!(
        plan(),
        "# Plan\n\n## 1. First\n",
        "a plan without steps starts at 1"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
/// A daemon on a committed plan for project "p": the app, the plan file and the folder to remove.
async fn delete_fixture(
    tag: &str,
    start: &str,
) -> (crate::app::Shared, std::path::PathBuf, std::path::PathBuf) {
    let base = std::env::temp_dir().join(format!("tirith-psd-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (repo, home) = (base.join("repo"), base.join("home"));
    repo_with_plan(&repo, start).await;
    std::fs::create_dir_all(&home).unwrap();
    let cfg = format!("[[projects]]\nid = \"p\"\nrepo = \"{}\"\n", repo.display());
    std::fs::write(home.join("config.toml"), cfg).unwrap();
    let app = crate::app::App::open_at(home);
    super::flush(&app, None).await;
    (app, repo.join("docs/TODO.md"), base)
}
const FOUR: &str = "# Plan\n\nIntro.\n\n## 1. One\n- [x] Done one\n\n## 2. Two\n\nProse of two.\n- [x] Old\n- [ ] Open\n\n## Notes\nInside two\n\n## 3. Three\n- [ ] Third\n\n## 4. Four\n\nLast prose.\n- [ ] Fourth\n";
#[tokio::test]
async fn plan_step_delete_removes_only_that_step() {
    let (app, file, base) = delete_fixture("rm", FOUR).await;
    let plan = || std::fs::read_to_string(&file).unwrap();
    let merged = crate::app::Lane {
        id: "old-merged".into(),
        project: "p".into(),
        step: "2".into(),
        status: "merged".into(),
        ..Default::default()
    };
    app.lanes.lock().unwrap().insert(merged.id.clone(), merged);
    super::delete_step(&app, "p", "2").await.unwrap();
    let middle = "# Plan\n\nIntro.\n\n## 1. One\n- [x] Done one\n\n## 3. Three\n- [ ] Third\n\n## 4. Four\n\nLast prose.\n- [ ] Fourth\n";
    assert_eq!(
        plan(),
        middle,
        "a middle step goes with its prose and items"
    );
    assert!(
        app.plan_dirty.lock().unwrap().contains("p"),
        "the removal queues its commit"
    );
    super::delete_step(&app, "p", "4").await.unwrap();
    let last = "# Plan\n\nIntro.\n\n## 1. One\n- [x] Done one\n\n## 3. Three\n- [ ] Third\n";
    assert_eq!(plan(), last, "the last step goes and leaves no blank tail");
    super::add(&app, "p", "3", "Another").await.unwrap();
    assert_eq!(plan(), format!("{last}- [ ] Another\n"));
    for gone in ["2", "4"] {
        assert!(super::add(&app, "p", gone, "Nope").await.is_err());
    }
    assert_eq!(plan(), format!("{last}- [ ] Another\n"));
    super::flush(&app, None).await;
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
#[tokio::test]
async fn plan_step_delete_refuses_a_live_step() {
    let (app, file, base) = delete_fixture("live", FOUR).await;
    let plan = || std::fs::read_to_string(&file).unwrap();
    for (id, status) in [("lane-stopped", "stopped"), ("lane-merged", "merged")] {
        let lane = crate::app::Lane {
            id: id.into(),
            project: "p".into(),
            step: "3".into(),
            status: status.into(),
            ..Default::default()
        };
        app.lanes.lock().unwrap().insert(lane.id.clone(), lane);
    }
    for num in ["9", "", "3a", "-1", "03"] {
        assert!(super::delete_step(&app, "p", num).await.is_err(), "{num:?}");
        assert_eq!(plan(), FOUR);
    }
    let why = super::delete_step(&app, "p", "3").await.unwrap_err();
    assert!(
        why.contains("lane-stopped") && !why.contains("lane-merged"),
        "{why}"
    );
    assert_eq!(plan(), FOUR, "a refusal leaves the bytes as they were");
    assert!(
        !app.plan_dirty.lock().unwrap().contains("p"),
        "a refusal queues no commit"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
