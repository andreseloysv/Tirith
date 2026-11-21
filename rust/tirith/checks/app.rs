use super::*;
#[test]
fn lane_log_redacts_key_prefixes() {
    let log = "GOOGLE_API_KEY=AIzaSyABCDEFGHIJKLMNOP\n{\"GITHUB_TOKEN\":\"ghp_abcdefghijklmnop12\"}\nMY_KEY: sk-abcdef123456789\nused AIzaSyZZZZZZZZZZZZZZ here\nplain line";
    let out = redact_secrets(log);
    ["AIza", "ghp_", "sk-", "abcdefghijklmnop12"]
        .into_iter()
        .for_each(|leak| assert!(!out.contains(leak), "leaked {leak}: {out}"));
    assert!(out.contains("GOOGLE_API_KEY=[redacted]") && out.contains("plain line"));
}
#[test]
fn child_path_includes_cargo_bin() {
    let home = std::env::temp_dir().join(format!("tirith-cp-{}", std::process::id()));
    let cargo = home.join(".cargo/bin");
    std::fs::create_dir_all(&cargo).unwrap();
    let p = path_with_cargo(Some(Path::new("/x")), "/usr/bin", &home, &[]);
    assert_eq!(p, format!("/x:/usr/bin:{}", cargo.display()));
    assert_eq!(
        path_with_cargo(None, &format!("/a:{}", cargo.display()), &home, &[]),
        format!("/a:{}", cargo.display())
    );
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(path_with_cargo(None, "/usr/bin", &home, &[]), "/usr/bin");
}
#[test]
fn child_path_includes_homebrew_bin() {
    let base = std::env::temp_dir().join(format!("tirith-hb-{}", std::process::id()));
    let brew = base.join("brew/bin");
    let dirs = [brew.to_str().unwrap(), "/nonexistent/bin"];
    std::fs::create_dir_all(&brew).unwrap();
    let p = path_with_cargo(None, "/usr/bin", &base, &dirs);
    assert_eq!(
        (p.clone(), path_with_cargo(None, &p, &base, &dirs)),
        (format!("/usr/bin:{}", brew.display()), p)
    );
}
/// A repository with one commit holding `files` (each a folder-relative path and its text).
async fn committed(repo: &Path, files: &[(&str, &str)]) {
    for (file, text) in files {
        let path = repo.join(file);
        std::fs::create_dir_all(path.parent().unwrap())
            .and_then(|_| std::fs::write(path, text))
            .unwrap();
    }
    for a in [
        "init -q",
        "config user.email t@t",
        "config user.name t",
        "add .",
        "commit -qm i",
    ] {
        git(repo, &a.split(' ').collect::<Vec<_>>()).await.unwrap();
    }
}
#[tokio::test]
async fn worktree_local_properties() {
    let base = std::env::temp_dir().join(format!("tirith-lp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let repo = base.join("repo");
    committed(
        &repo,
        &[(".gitignore", "local.properties\n"), ("README", "x")],
    )
    .await;
    std::fs::create_dir_all(repo.join("android")).unwrap();
    std::fs::write(repo.join("android/local.properties"), "sdk.dir=/sdk\n").unwrap();
    let wt = base.join("wt");
    add_worktree(&repo, &wt, None, &[]).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(wt.join("android/local.properties")).unwrap(),
        "sdk.dir=/sdk\n"
    );
    let _ = std::fs::remove_dir_all(&base);
}
/// Removes its folder when dropped, so a failing (panicking) test leaves nothing behind either.
struct RemoveOnDrop(std::path::PathBuf);
impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[tokio::test]
async fn sparse_zoo_lane() {
    let base = std::env::temp_dir().join(format!("tirith-sp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let _clean = RemoveOnDrop(base.clone());
    let (repo, wt) = (base.join("repo"), base.join("wt"));
    let ex: Vec<String> = ["output/research", "tools/content", "ZooBundles"]
        .map(String::from)
        .into();
    let files: Vec<(String, &str)> = ex
        .iter()
        .map(String::as_str)
        .chain(["cms"])
        .map(|d| (format!("{d}/f"), "x"))
        .collect();
    committed(
        &repo,
        &files
            .iter()
            .map(|(f, t)| (f.as_str(), *t))
            .collect::<Vec<_>>(),
    )
    .await;
    add_worktree(&repo, &wt, Some("lane/z"), &ex).await.unwrap();
    assert!(wt.join("cms/f").exists() && ex.iter().all(|p| !wt.join(p).exists()));
}
#[test]
fn rows_carry_the_factory_stamp() {
    let home = std::env::temp_dir().join(format!("tirith-stamp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = App::open_at(home);
    let version = app.cfg().factory.version.clone();
    assert!(
        version.contains('+'),
        "the running factory version: {version}"
    );
    let stamp = |table: &str, where_: &str| {
        app.query(
            &format!("SELECT factory, role_hash FROM {table} WHERE {where_}"),
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .pop()
        .expect("a stamped row")
    };
    // A lane: the stamp it was created with, in its JSON and in its columns (one GROUP BY factory, role_hash compares versions).
    let (factory, role_hash) = app.stamp_for(&crate::models::lane_role(&app.cfg()));
    assert_eq!(factory, version);
    assert_eq!(role_hash.len(), 8);
    app.save_lane(&Lane {
        id: "stamp-lane".into(),
        factory: factory.clone(),
        role_hash: role_hash.clone(),
        ..Default::default()
    });
    assert_eq!(
        stamp("lanes", "id = 'stamp-lane'"),
        (factory, role_hash.clone())
    );
    assert_eq!(
        app.lane("stamp-lane").map(|l| (l.factory, l.role_hash)),
        Some((version.clone(), role_hash.clone()))
    );
    // Its events carry its stamp; an event of no lane keeps the version with an empty role hash.
    app.event("stamp-lane", "", "test.kind", "", "", "");
    app.event("", "", "test.plain", "", "", "");
    assert_eq!(
        stamp("events", "kind = 'test.kind'"),
        (version.clone(), role_hash)
    );
    assert_eq!(
        stamp("events", "kind = 'test.plain'"),
        (version.clone(), String::new())
    );
    // A turn's rows carry its conversation role's hash; the owner's lines keep only the version.
    app.chat("manager", "stamp turn text", "");
    app.chat("owner", "stamp owner text", "");
    assert_eq!(
        stamp("chat", "text = 'stamp turn text'"),
        (version.clone(), app.stamp_for(&app.chat_role("")).1)
    );
    assert_eq!(
        stamp("chat", "text = 'stamp owner text'"),
        (version.clone(), String::new())
    );
    // A card carries its conversation role's stamp (the manager asked the owner).
    crate::manager::ask_owner(&app, "", "decision", "stamped card?", "").unwrap();
    assert_eq!(
        stamp("owner_items", "question = 'stamped card?'"),
        (version.clone(), app.stamp_for(&app.chat_role("")).1)
    );
    // A score row is born stamped (the evals fill the table; its stamp columns are these).
    let (f2, h2) = app.stamp_for("ceo");
    app.insert("INSERT INTO scores (at, event, eval, label, factory, role_hash) VALUES (?1, 1, 'owner-mood', 'approval', ?2, ?3)", rusqlite::params![now_iso(), f2, h2]);
    assert_eq!(stamp("scores", "eval = 'owner-mood'"), (version, h2));
}
#[test]
fn renames_move_the_model_and_the_conversation_of_no_project() {
    let home = std::env::temp_dir().join(format!("tirith-renames-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = App::open_at(home);
    // The page's choices under yesterday's role names, one already under its new name (it wins).
    app.set_kv(
        "models:lead",
        Some("{\"agent\":\"anthropic\",\"model\":\"m-lead\"}"),
    );
    app.set_kv(
        "models:reviewer",
        Some("{\"agent\":\"anthropic\",\"model\":\"m-rev\"}"),
    );
    app.set_kv(
        "models:workers",
        Some("{\"agent\":\"anthropic\",\"model\":\"m-old\"}"),
    );
    app.set_kv(
        "models:worker",
        Some("{\"agent\":\"anthropic\",\"model\":\"m-new\"}"),
    );
    app.insert("INSERT INTO chat (at, role, text, project) VALUES ('2026-11-01T00:00:00Z', 'owner', 'old line', '')", &[]);
    app.set_kv("manager_session:anthropic:", Some("sess-1"));
    migrate(&app.db.lock().unwrap());
    assert!(
        app.kv("models:manager-general").unwrap().contains("m-lead"),
        "lead's model is manager-general's now"
    );
    assert!(
        app.kv("models:ceo").unwrap().contains("m-rev"),
        "the reviewer's model is the CEO's now"
    );
    assert!(
        app.kv("models:worker").unwrap().contains("m-new"),
        "a choice under the new name is never overwritten"
    );
    assert_eq!(
        app.kv("manager_session:anthropic:manager-general")
            .as_deref(),
        Some("sess-1"),
        "its session resumes under its new key"
    );
    assert_eq!(app.kv("manager_session:anthropic:"), None);
    assert_eq!(
        app.query(
            "SELECT project FROM chat WHERE text = 'old line'",
            [],
            |r| r.get::<_, String>(0)
        ),
        ["manager-general"],
        "its chat rows keep their history under the new conversation"
    );
    assert_eq!(
        (app.conv(""), app.chat_role("manager-general").as_str()),
        ("manager-general", "manager-general")
    );
    // The system bots are bots: the CEO and the manager of no project shown, the judge not.
    let bots = crate::bots::all(&app);
    let shown = |k: &str| {
        bots.iter()
            .find(|b| b["key"] == k)
            .map(|b| (b["name"].clone(), b["show"].clone(), b["system"].clone()))
    };
    assert_eq!(
        shown("ceo"),
        Some((
            serde_json::json!("CEO"),
            serde_json::json!(true),
            serde_json::json!(true)
        ))
    );
    assert_eq!(
        shown("judge"),
        Some((
            serde_json::json!("Judge"),
            serde_json::json!(false),
            serde_json::json!(true)
        ))
    );
    assert_eq!(
        shown("manager-general"),
        Some((
            serde_json::json!("Manager"),
            serde_json::json!(true),
            serde_json::json!(true)
        ))
    );
    assert_eq!(
        crate::models::role(&app, "judge").agent,
        "muse",
        "the judge keeps its cheap factory agent, not the managers'"
    );
}
