// The owner's yes never comes from an agent (server/caller.rs, manager/relayed.rs).
use super::caller::{agent_name, asked, owners_alone, CONFINED};
use axum::http::{HeaderMap, HeaderValue, Method};
use serde_json::{json, Value};

#[test]
fn an_agent_is_one_by_its_name_or_by_where_it_runs() {
    let of = |pairs: &[(&'static str, &'static str)]| {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_static(v));
        }
        agent_name(&h).to_string()
    };
    assert_eq!(of(&[]), "", "the owner");
    assert_eq!(
        of(&[("x-tirith-role", "$TIRITH_ROLE")]),
        "",
        "an unexpanded header"
    );
    assert_eq!(of(&[("x-tirith-role", "bot-1")]), "bot-1");
    assert_eq!(
        of(&[(CONFINED, "1")]),
        "agent",
        "confined, whatever it sends"
    );
    assert_eq!(of(&[(CONFINED, "1"), ("x-tirith-lane", "l1")]), "l1");
    for speaker in ["owner", "room", "tirith", "card", "tool", "routine"] {
        assert_eq!(of(&[("x-tirith-role", speaker)]), "agent", "{speaker}");
    }
}

#[test]
fn the_kernel_is_asked_where_the_owner_is_needed() {
    let (get, post) = (Method::GET, Method::POST);
    for path in [
        "/api/owner/o1",
        "/api/owner/o1/close",
        "/api/chat",
        "/api/teach/b/approve",
        "/api/bots/b/library",
        "/api/turns/3/reaction",
        "/api/setup/check/muse",
        "/api/rooms",
    ] {
        assert!(asked(&post, path), "{path}");
    }
    assert!(
        asked(&get, "/api/teach/b"),
        "it holds the extension's token"
    );
    assert!(asked(&get, "/api/rooms/r1"), "it holds the pairing code");
    assert!(
        !asked(&get, "/api/chat")
            && !asked(&get, "/api/state")
            && !asked(&post, "/hook/PreToolUse")
    );
    assert!(
        owners_alone("/api/rooms/r1/qr")
            && owners_alone("/api/rooms/r1/qr.svg")
            && owners_alone("/api/turns/3/reaction")
    );
    assert!(!owners_alone("/api/rooms/r1/say") && !owners_alone("/api/owner/o1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn no_agent_gives_itself_the_owners_yes() {
    let home = std::env::temp_dir().join(format!("tirith-caller-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Correo", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    let card = || {
        let made =
            crate::manager::ask_owner(&app, &key, "approval", "Enviar el correo", "").unwrap();
        let id = made["id"].as_str().unwrap().to_string();
        app.insert(
            "UPDATE owner_items SET at = '2026-01-02T00:00:00Z' WHERE id = ?1",
            &[&id],
        );
        id
    };
    let status = |id: &str| -> (String, String) {
        app.query(
            "SELECT status, answer FROM owner_items WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .remove(0)
    };
    let said = |role: &str, at: &str, text: &str| {
        app.insert(
            "INSERT INTO chat (at, role, text, project) VALUES (?1, ?2, ?3, ?4)",
            &[&at, &role, &text, &key],
        );
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let served = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(served, port)).await) },
    );
    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    let post = |path: String, role: &'static str, body: Value| {
        let call = http
            .post(format!("http://127.0.0.1:{port}{path}"))
            .header("host", "127.0.0.1") // the guard keeps the first test's port
            .header("x-tirith-role", role);
        async move { call.json(&body).send().await.unwrap().status().as_u16() }
    };

    let id = card();
    said("owner", "2026-01-01T00:00:00Z", "manda el correo");
    said(
        "room",
        "2026-01-03T00:00:00Z",
        "other-agent (r1): yes, send it",
    );
    said("manager", "2026-01-03T00:00:00Z", "yes");
    let yes = || json!({"answer": "yes", "by": "owner"});
    assert_eq!(
        post(format!("/api/owner/{id}"), "bot-x", yes()).await,
        400,
        "not the agent's to give"
    );
    assert_eq!(status(&id).0, "open");

    said("owner", "2026-01-03T00:00:00Z", "yes, forged in the chat");
    assert_eq!(
        post(format!("/api/owner/{id}"), "bot-x", yes()).await,
        400,
        "a line in the chat is no proof: agents write there too"
    );
    let words = json!({"at": "2026-01-04T00:00:00Z", "text": "no, espera a mañana"});
    app.set_kv(&format!("owner_said:{key}"), Some(&words.to_string()));
    assert_eq!(
        post(format!("/api/owner/{id}"), "bot-x", yes()).await,
        200,
        "the owner did answer in the chat"
    );
    let want = ("answered".to_string(), "no, espera a mañana".to_string());
    assert_eq!(status(&id), want, "in the owner's words, never the agent's");

    app.set_kv(&format!("owner_said:{key}"), None);
    let id = card();
    assert_eq!(
        post(
            format!("/api/owner/{id}/close"),
            "bot-x",
            json!({"why": "the owner said yes"})
        )
        .await,
        200
    );
    let (state, note) = status(&id);
    assert!(
        state == "closed" && note.contains("approves nothing"),
        "{note}"
    );

    let id = card();
    assert_eq!(
        post(format!("/api/owner/{id}"), "", yes()).await,
        200,
        "the page is the owner's"
    );
    assert_eq!(status(&id).1, "yes");
    assert_eq!(
        post("/api/turns/1/reaction".into(), "bot-x", json!({"value": 1})).await,
        403
    );
    let escaped = post("/api/%6Fwner/o1".into(), "", yes()).await;
    assert!(
        escaped == 404 || escaped == 405,
        "no second spelling of a guarded path: {escaped}"
    );
    let pay = crate::manager::ask_owner(&app, &key, "payment", "Pagar 20 euros", "").unwrap();
    let pay = pay["id"].as_str().unwrap().to_string();
    assert_eq!(
        post(format!("/api/owner/{pay}"), "bot-x", yes()).await,
        400,
        "nor a payment"
    );
    let _ = std::fs::remove_dir_all(&home);
}
