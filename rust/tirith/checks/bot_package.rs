use crate::server::Who;
use serde_json::{json, Value};

fn home(name: &str) -> std::path::PathBuf {
    let h = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&h);
    std::fs::create_dir_all(&h).unwrap();
    std::fs::write(h.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    h
}

fn owned(app: &crate::app::Shared) -> usize {
    super::all(app)
        .iter()
        .filter(|b| b["system"] == false)
        .count()
}

/// A bot with a routine, a skill of its own, a memory and a webhook stamp, exported.
fn exported(app: &crate::app::Shared, home: &std::path::Path) -> (String, Value) {
    let bot = super::save(
        app,
        json!({"name": "Notas", "shape": "drop", "color": "#22d3ee"}),
    )
    .unwrap();
    let id = bot["id"].as_str().unwrap().to_string();
    super::routine::routine(app, id.clone(), json!({"title": "Avisar", "prompt": "Di hola", "trigger": "schedule", "days": "daily", "at": "07:30"})).unwrap();
    let dir = home.join("manager").join(format!("bot-{id}"));
    std::fs::create_dir_all(dir.join(".agents/skills/propia")).unwrap();
    std::fs::write(
        dir.join(".agents/skills/propia/SKILL.md"),
        "---\nname: propia\ndescription: x\n---\nuse tirith_browser\n",
    )
    .unwrap();
    std::fs::write(dir.join("memory.md"), "- my secret pin is 4711\n").unwrap();
    app.set_kv(
        &format!("webhook:{id}"),
        Some("https://example.invalid/hook/abc123"),
    );
    let p = super::package::export_for_test(
        app,
        &id,
        &json!({"description": "Saluda", "instructions": "Be brief."}),
    )
    .unwrap();
    (id, p)
}

#[tokio::test]
async fn a_package_round_trips_without_what_is_private() {
    let h = home("pkg-round");
    let app = crate::app::App::open_at(h.clone());
    let (id, p) = exported(&app, &h);
    let text = p.to_string();
    for private in ["4711", "abc123", "webhook", "memory"] {
        assert!(!text.contains(private), "the package leaks {private}");
    }
    assert_eq!(
        p["permissions"]["browser"], true,
        "the browser use is declared from the skill's text"
    );
    let shown = super::package::preview_for_test(&p).unwrap();
    assert_eq!(shown["routines"][0]["at"], "07:30");
    assert_eq!(shown["skills"][0]["name"], "propia");
    assert_eq!(
        shown["permissions"][3]["enforced"], false,
        "the network is honestly not enforced"
    );
    let before = owned(&app);
    assert_eq!(before, 1);
    let made = super::package::import_for_test(&app, &p, shown["hash"].as_str().unwrap()).unwrap();
    let key = made["bot"]["key"].as_str().unwrap();
    assert_ne!(made["bot"]["id"], json!(id), "a new bot, not the old one");
    assert_eq!(owned(&app), 2);
    assert!(super::skill_file(&app, key, "propia").exists());
    assert!(super::skill_file(&app, key, "instructions").exists());
    assert!(
        !super::folder(&app, key).join("memory.md").exists(),
        "no memory comes along"
    );
    let _ = std::fs::remove_dir_all(h);
}

#[tokio::test]
async fn a_changed_or_unseen_package_is_refused() {
    let h = home("pkg-bad");
    let app = crate::app::App::open_at(h.clone());
    let (_, p) = exported(&app, &h);
    let hash = super::package::preview_for_test(&p).unwrap()["hash"]
        .as_str()
        .unwrap()
        .to_string();
    let mut edited = p.clone();
    edited["skills"][0]["text"] = json!("do something else");
    assert!(super::package::preview_for_test(&edited)
        .unwrap_err()
        .contains("signature"));
    let mut extra = p.clone();
    extra["webhook"] = json!("https://example.invalid/x");
    assert!(
        super::package::preview_for_test(&extra).is_err(),
        "unknown fields"
    );
    let mut other = p.clone();
    other["bot"]["name"] = json!("Otro");
    assert!(
        super::package::import_for_test(&app, &other, &hash).is_err(),
        "not the previewed one"
    );
    assert!(
        super::package::import_for_test(&app, &p, "0000").is_err(),
        "a hash that was not seen"
    );
    assert_eq!(owned(&app), 1, "nothing was made");
    let _ = std::fs::remove_dir_all(h);
}

#[test]
fn a_package_must_be_well_formed() {
    let bad = |f: &dyn Fn(&mut Value)| {
        let mut p = json!({"format": "neobot-bot", "v": 1, "bot": {"name": "A", "face": {"shape": "pill", "color": "#112233"}}, "routines": [], "skills": []});
        assert!(super::package::checked_for_test(&p).is_ok());
        f(&mut p);
        assert!(super::package::checked_for_test(&p).is_err(), "{p}");
    };
    bad(&|p| p["bot"]["face"]["shape"] = json!("constructor"));
    bad(&|p| p["skills"] = json!([{"name": "../x", "text": "t"}]));
    bad(&|p| p["skills"] = json!([{"name": "a", "text": "t"}, {"name": "a", "text": "u"}]));
    bad(&|p| {
        p["routines"] =
            json!([{"title": "x", "prompt": "y", "trigger": "schedule", "days": [], "at": "07:00"}])
    });
    bad(&|p| p["bot"]["face"]["color"] = json!("red"));
    bad(&|p| p["v"] = json!(2));
}

/// A package exported before the rename says "openbot-bot" and still imports.
#[test]
fn a_package_from_the_old_name_still_imports() {
    let mut p = json!({"format": "openbot-bot", "v": 1, "bot": {"name": "A", "face": {"shape": "pill", "color": "#112233"}}, "routines": [], "skills": []});
    assert!(super::package::checked_for_test(&p).is_ok());
    p["format"] = json!("other-bot");
    assert!(super::package::checked_for_test(&p).is_err());
}

/// The routes, behind a gate that names the caller from a header, as the server's does from the token.
#[tokio::test]
async fn the_import_routes_are_the_owners_alone() {
    let h = home("pkg-owner");
    let app = crate::app::App::open_at(h.clone());
    let who = |mut req: axum::extract::Request, next: axum::middleware::Next| async move {
        if req.headers().contains_key("x-owner") {
            req.extensions_mut().insert(Who::Owner);
        }
        next.run(req).await
    };
    let router = super::package::routes()
        .merge(super::from_template::routes())
        .layer(axum::middleware::from_fn(who))
        .with_state(app);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let post = move |path: &'static str, owner: bool| async move {
        let mut c = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let head = if owner { "x-owner: 1\r\n" } else { "" };
        let msg = format!("POST {path} HTTP/1.1\r\nhost: x\r\n{head}content-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}");
        tokio::io::AsyncWriteExt::write_all(&mut c, msg.as_bytes())
            .await
            .unwrap();
        let mut out = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut c, &mut out)
            .await
            .unwrap();
        out.lines().next().unwrap().to_string()
    };
    for path in [
        "/api/bots/package/preview",
        "/api/bots/package/import",
        "/api/bots/from-template",
        "/api/bots/x/export",
    ] {
        assert!(
            post(path, false).await.contains("403"),
            "{path} refuses a caller that is not the owner"
        );
        assert!(
            !post(path, true).await.contains("403"),
            "{path} answers the owner"
        );
    }
    let _ = std::fs::remove_dir_all(h);
}
