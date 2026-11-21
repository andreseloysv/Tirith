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
        super::git(repo, &a.split(' ').collect::<Vec<_>>())
            .await
            .unwrap();
    }
    super::git(repo, &["rev-parse", "HEAD"]).await.unwrap()
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
        super::git(&repo, &["status", "--porcelain", "--", "docs/TODO.md"])
            .await
            .unwrap(),
        ""
    );
    assert!(super::git(&repo, &["show", "HEAD:docs/TODO.md"])
        .await
        .unwrap()
        .contains("The ui-deck item"));
    assert_eq!(
        super::git(&clean, &["rev-parse", "HEAD"]).await.unwrap(),
        clean_head,
        "a clean plan gets no start commit"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
#[test]
fn plan_add_rejects_invalid_checks() {
    let b = super::check_problem;
    assert!(
        [
            "tirith run -s test",
            "tirith check",
            "cd collector && ls",
            "cargo test && tirith run"
        ]
        .iter()
        .all(|c| b(c).is_some())
            && ["cargo test -q x", "tirith plan show p", "cd collectors"]
                .iter()
                .all(|c| b(c).is_none())
    );
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
