#[test]
fn brief_warns_against_blind_waits() {
    let text = crate::factory::current().prompt("brief");
    assert!(text.contains("Never wait blindly") && text.contains("two minutes") && text.contains("find out why"));
}

#[test]
fn codex_run_completed_ends_lane() {
    // The shape of the real incident (forest-art-second-pass-docs-art-9728): its record clocks are not wall time, so only the order in the log counts.
    let ev = |kind: &str, at: i64| format!("{{\"schema_version\":1,\"recorded_at\":{at},\"payload_type\":\"{kind}\",\"payload\":{{\"kind\":\"run_terminal\",\"terminal\":\"completed\",\"text\":\"done\"}}}}");
    let (done, started, launch) = (ev("run.terminal.completed", 1780531400005239), ev("run.lifecycle.started", 1780531400000001), "--- tirith: lane run started 2026-11-14T12:00:00Z");
    assert!(!super::run_completed("run.terminal.completed\n")); // a plain mention is not the event
    let nested = "{\"payload_type\":\"tool.result\",\"payload\":{\"text\":\"\\\"payload_type\\\":\\\"run.terminal.completed\\\"\"}}";
    assert!(!super::run_completed(&format!("{launch}\n{started}\n{nested}\n"))); // nor one inside output the agent read
    assert!(super::run_completed(&done)); // the start of a long run fell out of the tail window
    assert!(super::run_completed(&format!("{launch}\n{started}\n{done}\n")));
    assert!(!super::run_completed(&format!("{done}\n{launch}\n{started}\n"))); // an earlier attempt's terminal, left in the appended log
    assert!(!super::run_completed(&format!("{done}\n{started}\n"))); // a later run is under way (a launch line may predate this code)
}

#[test]
fn idle_lane_after_api_retry_is_stopped() {
    let mut seen = std::collections::HashMap::new();
    let (real, retry) = ("{\"type\":\"assistant\"}\n", "{\"subtype\":\"api_retry\"}\n");
    assert_eq!(super::quiet_s(&mut seen, "a", real, 0, 1000), 0);
    assert_eq!(super::quiet_s(&mut seen, "a", &format!("{real}{retry}{retry}"), 0, 2200), 1200);
    assert_eq!(super::quiet_s(&mut seen, "a", &format!("{real}{retry}x\n"), 0, 2201), 0);
}

#[test]
fn long_running_lane_is_reported_once() {
    let start = "2026-11-13T10:00:00Z";
    let t0 = chrono::DateTime::parse_from_rfc3339(start).unwrap().timestamp() as u64;
    assert!(!super::long_running_due(Some(start), t0 + 29 * 60, 30, false));
    assert!(super::long_running_due(Some(start), t0 + 30 * 60, 30, false));
    assert!(!super::long_running_due(Some(start), t0 + 40 * 60, 30, true)); // once
    assert!(!super::long_running_due(Some(start), t0 + 40 * 60, 0, false)); // off
    assert!(!super::long_running_due(None, t0, 30, false));
    let log = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"input\":{\"command\":\"sleep 590\"}}]}}\nnot json\n{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hi\"},{\"type\":\"tool_use\",\"input\":{\"command\":\"ls\"}}]}}\n";
    assert_eq!(super::last_commands(log, 1), ["ls"]);
    assert_eq!(super::last_commands(log, 5), ["sleep 590", "ls"]);
}
#[test]
fn lane_runs_clippy_before_finish() {
    let home = std::env::temp_dir().join(format!("tirith-clippy-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home);
    let dir = std::env::temp_dir().join(format!("tirith-clippy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let lane = crate::app::Lane { workspace: dir.display().to_string(), ..Default::default() };
    assert!(!super::brief(&app, &lane).contains("cargo clippy -- -D warnings"));
    std::fs::write(dir.join("Cargo.toml"), "").unwrap();
    assert!(super::brief(&app, &lane).contains("cargo clippy -- -D warnings"));
}
#[test]
fn low_disk_asks_owner_once() {
    let asks = [(8.0, false), (4.0, false), (3.0, true), (5.5, true), (9.0, true)].map(|(gb, asked)| super::low_disk_ask(gb, asked));
    assert_eq!(asks, [None, Some(true), None, None, Some(false)]);
    let dir = std::env::temp_dir().join(format!("tirith-lowdisk-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("big")).unwrap();
    std::fs::write(dir.join("big/f"), vec![1u8; 300_000]).unwrap();
    let rows = super::biggest_dirs(&dir, 5);
    assert!(rows[0].0.ends_with("big"));
    std::fs::remove_dir_all(&dir).unwrap();
}
#[test]
fn diskdiet_clears_idle_build_cache() {
    let root = std::env::temp_dir().join(format!("tirith-builds-{}", std::process::id()));
    ["tirith/cargo", "zoo/cargo"].iter().for_each(|p| std::fs::create_dir_all(root.join(p)).unwrap());
    assert_eq!(super::clear_idle_builds(&root, &["zoo".into()]), [root.join("tirith")]);
    assert!(!root.join("tirith").exists() && root.join("zoo/cargo").is_dir());
    std::fs::remove_dir_all(&root).unwrap();
}
#[test]
fn lane_waits_for_payment_answer() {
    assert!(super::is_payment("¿Aprobar 0,74 $ de voces TTS de pago?") && !super::is_payment("Which name?"));
}
#[test]
fn autopilot_skips_owner_wait_items() {
    let skipped = ["Elegir dominio (espera: el dueño decide)", "CEO: revisar", "Manager de guardia: atender el chat"];
    assert!(skipped.iter().all(|t| super::is_autopilot_skipped(t)));
    assert!(!super::is_autopilot_skipped("Arreglar el parser") && !super::is_autopilot_skipped("Arreglar el CEO: bug"));
}
#[test]
fn autopilot_waits_for_dependency_items() {
    let open = ["Generar los vídeos de la demo en 3 idiomas", "Subir a YouTube"];
    let up = "Subir los vídeos de la demo a YouTube: wait until they are on main, made by the item «Generar los vídeos de la demo en 3 idiomas»";
    assert!(super::waits_for_open_item(up, &open));
    assert!(!super::waits_for_open_item(up, &["Subir a YouTube"]));
    assert!(!super::waits_for_open_item("Arreglar el parser", &open));
}
#[test]
fn stale_owner_questions_close() {
    use crate::manager::stale_reason;
    let off = "Should I turn off the autopilot of tirith?";
    assert_eq!(stale_reason(off, true, 50.0), None); // still on: the question applies
    assert!(stale_reason(off, false, 50.0).is_some()); // already off: stale
    assert!(stale_reason("¿Apagar el autopilot de superpolitika?", false, 50.0).is_some());
    assert!(stale_reason("Enable the autopilot again?", true, 50.0).is_some());
    assert_eq!(stale_reason("Enable the autopilot again?", false, 50.0), None);
    assert_eq!(stale_reason("Approve $0.74 for the translation?", false, 50.0), None);
}
#[test]
fn low_disk_question_closes_when_freed() {
    use crate::manager::stale_reason;
    let q = "Only 4.6 GB free on this disk: lanes will fail with No space left on device.";
    assert_eq!(stale_reason(q, true, 4.6), None); // still low
    assert_eq!(stale_reason(q, true, 9.0), None); // not above 10 GB yet
    assert!(stale_reason(q, true, 39.0).is_some());
}
/// A temp project `clnp` known to a fresh App at its own home: (app, repo, base). Its committed .gitignore ignores the owner's folders and the build caches.
async fn cleanup_fixture(name: &str) -> (crate::app::Shared, std::path::PathBuf, std::path::PathBuf) {
    let base = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    let (repo, home) = (base.join("repo"), base.join("home"));
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join(".gitignore"), "private/\nevidence/\ntarget/\nnode_modules/\nbuild/\n").unwrap();
    std::fs::write(repo.join("README"), "hello\n").unwrap();
    for a in ["init -q", "config user.email t@t", "config user.name t", "add .", "commit -qm i"] {
        super::git(&repo, &a.split(' ').collect::<Vec<_>>()).await.unwrap();
    }
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), format!("[[projects]]\nid = \"clnp\"\nrepo = \"{}\"\n", repo.display())).unwrap();
    (crate::app::App::open_at(home), repo, base)
}
/// A lane of the fixture project in its own worktree on `lane/<id>`, saved in the app.
async fn cleanup_lane_fixture(app: &crate::app::Shared, repo: &std::path::Path, id: &str) -> crate::app::Lane {
    let ws = repo.parent().unwrap().join(format!("ws-{id}"));
    super::add_worktree(repo, &ws, Some(&format!("lane/{id}")), &[]).await.unwrap();
    let lane = crate::app::Lane { id: id.into(), project: "clnp".into(), workspace: ws.display().to_string(), branch: format!("lane/{id}"), status: "failed".into(), ..Default::default() };
    app.save_lane(&lane);
    lane
}
#[tokio::test]
async fn lane_done_leaves_no_worktree() {
    let (app, repo, base) = cleanup_fixture("done").await;
    // A merged lane: worktree, branch, log, brief and its run logs go at once; its ignored files still reach the project.
    let lane = cleanup_lane_fixture(&app, &repo, "done-1").await;
    let ws = std::path::PathBuf::from(&lane.workspace);
    std::fs::create_dir_all(ws.join("private")).unwrap();
    std::fs::write(ws.join("private/keep.txt"), "owner doc").unwrap();
    let (log, brief, run_log, other_log) = (app.home.join("logs/lanes/done-1.log"), app.home.join("briefs/done-1.md"), app.home.join("logs/runs/77.log"), app.home.join("logs/runs/78.log"));
    [&log, &brief, &run_log, &other_log].iter().for_each(|p| std::fs::write(p, "x").unwrap());
    app.insert("INSERT INTO runs (lane, project, stage, status, log) VALUES ('done-1', 'clnp', 'check done-1', 'passed', ?1)", rusqlite::params![run_log.display().to_string()]);
    app.insert("INSERT INTO runs (lane, project, stage, status, log) VALUES ('done-1b', 'clnp', 'test', 'passed', ?1)", rusqlite::params![other_log.display().to_string()]);
    super::cleanup_lane(&app, &lane, true).await;
    assert!(!ws.exists(), "a merged lane's worktree goes at once");
    assert!(super::git(&repo, &["rev-parse", "--verify", "-q", "lane/done-1"]).await.is_err(), "a merged lane's branch goes");
    assert!(!log.exists() && !brief.exists() && !run_log.exists(), "a merged lane's log, brief and check logs go at once");
    assert!(other_log.exists(), "another lane's run log (an id containing this one's) stays");
    assert_eq!(app.lane("done-1").map(|l| l.workspace), Some(String::new()));
    assert_eq!(std::fs::read_to_string(repo.join("private/keep.txt")).unwrap(), "owner doc");
    assert!(!super::git(&repo, &["worktree", "list", "--porcelain"]).await.unwrap().contains(&ws.display().to_string()), "pruned: git holds no registration of it");
    // A stopped lane: its uncommitted work is saved on its (kept) branch, the worktree goes, and its log goes only after 24 h.
    let mut lane2 = cleanup_lane_fixture(&app, &repo, "done-2").await;
    let ws2 = std::path::PathBuf::from(&lane2.workspace);
    std::fs::write(ws2.join("README"), "changed\n").unwrap();
    std::fs::write(ws2.join("loose.txt"), "new\n").unwrap();
    let log2 = app.home.join("logs/lanes/done-2.log");
    std::fs::write(&log2, "x").unwrap();
    super::cleanup_lane(&app, &lane2, false).await;
    assert!(!ws2.exists(), "a stopped lane's worktree goes at once");
    assert_eq!(super::git(&repo, &["show", "lane/done-2:README"]).await.unwrap(), "changed", "its uncommitted work was saved on its branch");
    assert_eq!(super::git(&repo, &["show", "lane/done-2:loose.txt"]).await.unwrap(), "new");
    assert!(log2.exists(), "a stopped lane keeps its log for now");
    lane2 = app.lane("done-2").unwrap();
    lane2.ended_at = Some((chrono::Utc::now() - chrono::Duration::hours(25)).format("%Y-%m-%dT%H:%M:%SZ").to_string());
    app.save_lane(&lane2);
    super::sweep_cleanups(&app, &[lane2]).await;
    assert!(!log2.exists(), "a stopped lane's log goes after 24 h");
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
#[tokio::test]
async fn private_files_move_to_project() {
    let (app, repo, base) = cleanup_fixture("priv").await;
    std::fs::create_dir_all(repo.join("private")).unwrap();
    std::fs::write(repo.join("private/existing.txt"), "project version").unwrap();
    std::fs::create_dir_all(repo.join("evidence")).unwrap();
    std::fs::write(repo.join("evidence/same.txt"), "same").unwrap();
    let lane = cleanup_lane_fixture(&app, &repo, "priv-1").await;
    let ws = std::path::PathBuf::from(&lane.workspace);
    for (file, text) in [
        ("private/doc.txt", "lane doc"),
        ("private/existing.txt", "lane version"),
        ("evidence/same.txt", "same"),
        ("evidence/nested/deep.txt", "deep"),
        ("target/junk", "x"),
        ("node_modules/pkg.js", "x"),
        ("sub/build/out", "x"),
        ("secret.txt", "s3cret"),
    ] {
        let p = ws.join(file);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    // Ignored only by the lane's own .gitignore (not on main): it must land in the project kept out of git there.
    std::fs::write(ws.join(".gitignore"), "private/\nevidence/\ntarget/\nnode_modules/\nbuild/\nsecret.txt\n").unwrap();
    super::cleanup_lane(&app, &lane, false).await;
    assert!(!ws.exists());
    assert_eq!(std::fs::read_to_string(repo.join("private/doc.txt")).unwrap(), "lane doc");
    assert_eq!(std::fs::read_to_string(repo.join("private/existing.txt")).unwrap(), "project version", "nothing in the project is overwritten");
    assert_eq!(std::fs::read_to_string(repo.join("private/existing-priv-1.txt")).unwrap(), "lane version", "a different file lands next to it with the lane's id in its name");
    assert!(!repo.join("evidence/same-priv-1.txt").exists(), "an identical file needs no side copy");
    assert_eq!(std::fs::read_to_string(repo.join("evidence/nested/deep.txt")).unwrap(), "deep");
    assert!(!repo.join("target").exists() && !repo.join("node_modules").exists() && !repo.join("sub").exists(), "the ignored build caches go with the worktree, never to the project");
    assert_eq!(std::fs::read_to_string(repo.join("secret.txt")).unwrap(), "s3cret");
    assert!(std::fs::read_to_string(repo.join(".git/info/exclude")).unwrap().contains("/secret.txt"), "a path the project does not ignore joins .git/info/exclude");
    assert!(super::git(&repo, &["check-ignore", "-q", "--", "secret.txt"]).await.is_ok());
    assert_eq!(super::git(&repo, &["status", "--porcelain"]).await.unwrap(), "", "everything moved in stays out of git in the project");
    assert!(super::git(&repo, &["show", "lane/priv-1:.gitignore"]).await.unwrap().contains("secret.txt"), "its uncommitted work was saved on its branch");
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
#[tokio::test]
async fn moved_folder_stays_excluded() {
    let (app, repo, base) = cleanup_fixture("moved").await;
    // As in the incident, the `.private` folder sits beside tracked work, so git reports it itself as the ignored folder.
    std::fs::create_dir_all(repo.join("research/konsens")).unwrap();
    std::fs::write(repo.join("research/konsens/README"), "tracked\n").unwrap();
    super::git(&repo, &["add", "research"]).await.unwrap();
    super::git(&repo, &["commit", "-qm", "research"]).await.unwrap();
    let lane = cleanup_lane_fixture(&app, &repo, "moved-1").await;
    let ws = std::path::PathBuf::from(&lane.workspace);
    // The incident's shape: a Windows-zip name with `\`, spaces and `[` inside a folder only the lane ignores. A per-file exclude line would let `\` escape the next character and git would still see it as new.
    let rel = "research/konsens/.private/documents/Unterlagen\\Vergabe [1].pdf";
    let p = ws.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "pdf").unwrap();
    std::fs::write(p.parent().unwrap().join("Otro\\Doc [2].pdf"), "pdf2").unwrap();
    std::fs::write(ws.join(".gitignore"), "private/\nevidence/\ntarget/\nnode_modules/\nbuild/\n.private/\n").unwrap();
    super::cleanup_lane(&app, &lane, false).await;
    assert_eq!(std::fs::read_to_string(repo.join(rel)).unwrap(), "pdf");
    let exclude = std::fs::read_to_string(repo.join(".git/info/exclude")).unwrap();
    assert_eq!(exclude.lines().filter(|l| l.contains(".private")).collect::<Vec<_>>(), ["/research/konsens/.private/"], "one line for the whole folder moved, not one per file: {exclude}");
    assert!(super::git(&repo, &["check-ignore", "-q", "--", rel]).await.is_ok(), "a name with \\, spaces and [ stays ignored");
    assert_eq!(super::git(&repo, &["status", "--porcelain"]).await.unwrap(), "", "git add -A in the project would take nothing moved in");
    // Already moved before the fix, with its broken per-file line: the startup sweep heals it by excluding its folder.
    let old = "old/.private/documents/Bad\\Name [2].pdf";
    let p = repo.join(old);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "old").unwrap();
    std::fs::write(repo.join(".git/info/exclude"), format!("{exclude}/{old}\n")).unwrap();
    assert!(super::git(&repo, &["check-ignore", "-q", "--", old]).await.is_err(), "the broken per-file line leaves it visible");
    super::sweep_cleanups(&app, &[]).await;
    assert!(super::git(&repo, &["check-ignore", "-q", "--", old]).await.is_ok(), "the startup sweep excludes its folder");
    assert!(std::fs::read_to_string(repo.join(".git/info/exclude")).unwrap().contains("/old/.private/documents/"));
    assert_eq!(super::git(&repo, &["status", "--porcelain"]).await.unwrap(), "");
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
#[tokio::test]
async fn idle_build_cache_goes() {
    let (app, repo, base) = cleanup_fixture("idle").await;
    assert_eq!(crate::factory::current().limits.build_idle_minutes, 30, "the factory's idle limit");
    let build = base.join("build/clnp");
    std::fs::create_dir_all(build.join("cargo/debug")).unwrap();
    std::fs::write(build.join("cargo/debug/x"), "x").unwrap();
    let merge_dir = base.join("merge-wt");
    super::add_worktree(&repo, &merge_dir, None, &[]).await.unwrap();
    let now = super::now_s();
    assert!(!super::drop_idle_cache(&repo, &build, &merge_dir, true, now - 7200, now, 30).await, "a project with work under way keeps its cache and merge folder");
    assert!(!super::drop_idle_cache(&repo, &build, &merge_dir, false, now - 60, now, 30).await, "idle for less than the limit keeps them");
    assert!(build.exists() && merge_dir.exists());
    assert!(super::drop_idle_cache(&repo, &build, &merge_dir, false, now - 7200, now, 30).await, "idle for longer than the limit: both go");
    assert!(!build.exists() && !merge_dir.exists());
    assert!(!super::git(&repo, &["worktree", "list", "--porcelain"]).await.unwrap().contains(&merge_dir.display().to_string()), "the merge folder is pruned, not just deleted");
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
#[tokio::test]
async fn manager_sessions_build_in_the_shared_cache() {
    use tokio::io::AsyncReadExt;
    let (app, _repo, base) = cleanup_fixture("cargoenv").await;
    for (role, project, want) in [("manager", "clnp", app.home.join("build/clnp/cargo")), ("ceo", "", app.home.join("build/ceo/cargo"))] {
        let argv = ["sh", "-c", "printf %s \"$CARGO_TARGET_DIR\""].map(String::from).into();
        let (mut child, _stdin, mut stdout) = crate::manager::spawn_chat(&app, role, project, argv).unwrap();
        let mut out = String::new();
        stdout.read_to_string(&mut out).await.unwrap();
        let _ = child.wait().await;
        assert_eq!(out, want.display().to_string(), "a {role} session builds into the shared cache, so its cargo leaves no target/ in a project folder");
    }
    drop(app);
    let _ = std::fs::remove_dir_all(&base);
}
