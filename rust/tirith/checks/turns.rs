use serde_json::json;

fn home(name: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    home
}

#[tokio::test]
async fn turns_are_recorded() {
    // The usage in the three shapes of line: Anthropic's result, Muse's model_completed records (summed), Codex's turn.completed.
    let mut a = super::Usage::default();
    a.take(&json!({"type": "result", "is_error": false, "total_cost_usd": 0.5, "usage": {"input_tokens": 10, "output_tokens": 5, "cache_read_input_tokens": 3, "cache_creation_input_tokens": 2}}));
    assert_eq!(
        (
            a.input,
            a.output,
            a.cache_read,
            a.cache_write,
            a.cost,
            a.failed
        ),
        (10, 5, 3, 2, Some(0.5), false)
    );
    let mut m = super::Usage::default();
    let done = |i: i64, o: i64| json!({"payload_type": "runtime.session", "payload": {"event": {"kind": "model_completed", "model": "muse-x", "usage": {"input_tokens": i, "output_tokens": o, "cache_read_tokens": 4, "cache_write_tokens": 1}}}});
    m.take(&done(100, 7));
    m.take(&done(50, 3));
    m.take(&json!({"payload_type": "runtime.session", "payload": {"event": {"kind": "model_started"}}}));
    assert_eq!(
        (m.input, m.output, m.cache_read, m.cache_write, m.cost),
        (150, 10, 8, 2, None)
    );
    let mut c = super::Usage::default();
    c.take(&json!({"type": "turn.completed", "usage": {"input_tokens": 40, "cached_input_tokens": 30, "output_tokens": 6}}));
    assert_eq!(
        (c.input, c.output, c.cache_read, c.input_has_cache),
        (40, 6, 30, true)
    );

    // A bot whose agent is a fake stream-json one: a shell script that answers each turn with one tool call (it reads a skill) and a result with usage and total_cost_usd.
    let home = home("turns-recorded");
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, r#"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"fake-1"}'
while read -r line; do
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cat .agents/skills/notas/SKILL.md"}}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"hecho"}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.0123,"usage":{"input_tokens":11,"output_tokens":7,"cache_read_input_tokens":5,"cache_creation_input_tokens":2}}'
done
"#).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[agents.fake]\ncommand = \"{}\"\nchat_args = []\n[network]\nsandbox = false\n",
            agent.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Notas", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    app.set_kv(&format!("models:{key}"), Some(r#"{"agent":"fake"}"#));
    let dir = home.join("manager").join(&key);
    std::fs::create_dir_all(dir.join(".agents/skills/notas")).unwrap();
    std::fs::write(dir.join(".agents/skills/notas/SKILL.md"), "# notas\n").unwrap();
    std::fs::write(dir.join("memory.md"), "recuerda\n").unwrap();

    crate::manager::say(&app, &key, "apunta esto", "").unwrap();
    let mut rows = vec![];
    for _ in 0..300 {
        // The judge's turn on owner.message is kept in this conversation too: the bot's own row is the one under its role.
        rows = super::turns(&app, &key, 10);
        rows.retain(|r| r["role"] == key.as_str());
        if !rows.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    crate::manager::end_session(&key);
    let row = rows.first().cloned().expect("a turn row after say");
    assert_eq!(
        (
            row["trigger"].as_str(),
            row["status"].as_str(),
            row["agent"].as_str(),
            row["steps"].as_i64()
        ),
        (Some("owner"), Some("ok"), Some("fake"), Some(1)),
        "{row}"
    );
    assert_eq!(
        [
            &row["tokens_in"],
            &row["tokens_out"],
            &row["tokens_cache_read"],
            &row["tokens_cache_write"]
        ]
        .map(|v| v.as_i64()),
        [Some(11), Some(7), Some(5), Some(2)],
        "{row}"
    );
    assert_eq!(row["cost_usd"].as_f64(), Some(0.0123), "{row}");
    assert_eq!(
        (
            row["answer"].as_str(),
            row["conv"].as_str(),
            row["role"].as_str()
        ),
        (Some("hecho"), Some(key.as_str()), Some(key.as_str()))
    );
    assert_eq!(row["skills"], json!(["notas"]), "{row}");
    assert_eq!(row["tools"][0]["name"], "Bash", "{row}");
    let hash = |p: &std::path::Path| {
        String::from_utf8(
            std::process::Command::new("git")
                .arg("hash-object")
                .arg(p)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string()
    };
    assert_eq!(
        row["versions"]["memory.md"].as_str(),
        Some(hash(&dir.join("memory.md")).as_str()),
        "{row}"
    );
    assert_eq!(
        row["versions"]["skills/notas"].as_str(),
        Some(hash(&dir.join(".agents/skills/notas/SKILL.md")).as_str()),
        "{row}"
    );
    assert!(
        row["ms"].as_i64().is_some() && !row["factory"].as_str().unwrap_or("").is_empty(),
        "{row}"
    );
    // Its event: routine.done (a chat turn that read a skill is graded with its «Cómo validar» too), in the bot's conversation, saying the bot, the trigger, the tool, the skill and the answer.
    let event = row["event"].as_i64().expect("its event");
    let (kind, project, summary) = app
        .query(
            "SELECT kind, project, summary FROM events WHERE id = ?1",
            [event],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )
        .into_iter()
        .next()
        .expect("the event row");
    assert_eq!(
        (kind.as_str(), project.as_str()),
        ("routine.done", key.as_str())
    );
    assert!(
        summary.contains("Notas")
            && summary.contains("owner")
            && summary.contains("Bash")
            && summary.contains("notas")
            && summary.contains("hecho"),
        "{summary}"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn prices_give_the_cost() {
    let home = home("turn-prices");
    std::fs::write(home.join("config.toml"), "[prices.\"m-1\"]\ninput = 3\noutput = 15.0\ncached = 0.3\n[prices.\"m-2\"]\ninput = 2.0\noutput = 8.0\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    // The strict loader accepts the section, and none of it goes to the factory.
    assert_eq!(app.cfg().error, None);
    assert!(app.cfg().factory.raw.get("prices").is_none());
    let raw = &app.cfg().raw;
    let u = super::Usage {
        input: 1_000_000,
        output: 100_000,
        cache_read: 2_000_000,
        cache_write: 0,
        ..Default::default()
    };
    let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
    assert!(
        close(super::cost(raw, "m-1", &u), 3.0 + 1.5 + 0.6),
        "{:?}",
        super::cost(raw, "m-1", &u)
    );
    // Without `cached`, cached tokens cost as input; Codex's input already holds its cached tokens.
    assert!(close(super::cost(raw, "m-2", &u), 2.0 + 0.8 + 4.0));
    let codex = super::Usage {
        input: 3_000_000,
        cache_read: 2_000_000,
        input_has_cache: true,
        ..Default::default()
    };
    assert!(close(super::cost(raw, "m-1", &codex), 3.0 + 0.6));
    // A model without a price has no cost: never invented.
    assert_eq!(super::cost(raw, "unpriced", &u), None);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

/// One HTTP request to a test daemon on `port`: the status line and the body.
async fn http(port: u16, method: &str, path: &str, body: &str) -> (String, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    s.write_all(format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nX-Tirith-Token: {}\r\nConnection: close\r\n\r\n{body}", body.len(), crate::server::testing::owner(port)).as_bytes()).await.unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).await.unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    (
        head.lines().next().unwrap_or("").to_string(),
        body.to_string(),
    )
}

/// A test daemon serving `app`: its port.
async fn serve(app: &crate::app::Shared) -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    crate::server::testing::serving(port, app);
    let app = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(app, port)).await) },
    );
    port
}

/// The rows of `key`'s own role (the judge's turns on it are kept in the same conversation), the last first.
fn own_rows(app: &crate::app::Shared, key: &str) -> Vec<serde_json::Value> {
    let mut rows = super::turns(app, key, 50);
    rows.retain(|r| r["role"] == key);
    rows
}

async fn until(what: &str, mut f: impl FnMut() -> bool) {
    for _ in 0..300 {
        if f() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("never happened: {what}");
}

#[tokio::test]
async fn turn_reactions_and_corrections() {
    // A bot and its judge on one fake stream-json agent: every turn answers that the work is wrong, citing a note — so the judge labels every owner message a correction.
    let home = home("turn-reactions");
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, r#"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"fake-2"}'
while read -r line; do
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"correction — no es lo pedido [m1]"}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":1,"output_tokens":1}}'
done
"#).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[agents.fake]\ncommand = \"{}\"\nchat_args = []\n[network]\nsandbox = false\n",
            agent.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Votos", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    app.set_kv(&format!("models:{key}"), Some(r#"{"agent":"fake"}"#));
    app.set_kv("models:judge", Some(r#"{"agent":"fake"}"#));
    let scores = |app: &crate::app::Shared| {
        app.query(
            "SELECT COUNT(*) FROM scores WHERE eval = 'owner-mood'",
            [],
            |r| r.get::<_, i64>(0),
        )[0]
    };

    // The first turn; then the owner says it is wrong: owner-mood labels it a correction and the first turn is corrected.
    crate::manager::say(&app, &key, "apunta esto", "").unwrap();
    until("the first turn", || own_rows(&app, &key).len() == 1).await;
    until("the first message judged", || scores(&app) == 1).await;
    let first = own_rows(&app, &key)[0]["id"].as_i64().unwrap();
    assert_eq!(
        own_rows(&app, &key)[0]["corrected"],
        0,
        "nothing came before the first message"
    );
    crate::manager::say(&app, &key, "esto no es lo que pedí", "").unwrap();
    until("the second turn", || own_rows(&app, &key).len() == 2).await;
    until("the correction", || {
        own_rows(&app, &key)[1]["corrected"] == 1
    })
    .await;
    let rows = own_rows(&app, &key);
    let second = rows[0]["id"].as_i64().unwrap();
    assert_eq!(rows[0]["corrected"], 0, "{}", rows[0]);
    assert_eq!(rows[0]["cited"], json!(["m1"]), "{}", rows[0]);
    until("the second message judged", || scores(&app) == 2).await;

    // A 👎 of the owner on the page: the reaction of the bot's last turn, a chat row, no turn and no owner-mood.
    let events = |app: &crate::app::Shared| {
        app.query(
            "SELECT COUNT(*) FROM events WHERE kind = 'owner.message'",
            [],
            |r| r.get::<_, i64>(0),
        )[0]
    };
    let owner_messages = events(&app);
    crate::manager::say(&app, &key, " 👎🏽 ", "").unwrap();
    assert_eq!(own_rows(&app, &key)[0]["reaction"], -1);
    assert_eq!(own_rows(&app, &key)[1]["reaction"], serde_json::Value::Null);
    let chat = app.query(
        "SELECT role FROM chat WHERE text = ' 👎🏽 ' AND project = ?1",
        [&key],
        |r| r.get::<_, String>(0),
    );
    assert_eq!(chat, ["owner"]);
    // And +1 from a paired phone.
    crate::manager::from_room(&app, &key, "room-1", "phone", "+1");
    assert_eq!(own_rows(&app, &key)[0]["reaction"], 1);
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    assert_eq!(own_rows(&app, &key).len(), 2, "a reaction starts no turn");
    assert_eq!(
        (events(&app), scores(&app)),
        (owner_messages, 2),
        "owner-mood never judges a reaction"
    );
    // Outside a bot's conversation 👍 is an ordinary message (no bot, no reaction).
    assert!(!super::reacted(&app, "nope", "👍"));

    // The reaction by API: 1, 0 (taken back), a wrong value and an unknown turn.
    let port = serve(&app).await;
    let (status, _) = http(
        port,
        "POST",
        &format!("/api/turns/{first}/reaction"),
        r#"{"value": 1}"#,
    )
    .await;
    assert!(status.contains(" 200"), "{status}");
    let (status, _) = http(
        port,
        "POST",
        &format!("/api/turns/{second}/reaction"),
        r#"{"value": 0}"#,
    )
    .await;
    assert!(status.contains(" 200"), "{status}");
    let (status, _) = http(
        port,
        "POST",
        &format!("/api/turns/{second}/reaction"),
        r#"{"value": 5}"#,
    )
    .await;
    assert!(status.contains(" 400"), "{status}");
    let (status, _) = http(
        port,
        "POST",
        "/api/turns/999999/reaction",
        r#"{"value": -1}"#,
    )
    .await;
    assert!(status.contains(" 404"), "{status}");
    let (_, body) = http(port, "GET", &format!("/api/turns?conv={key}"), "").await;
    let listed: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    let row = |id: i64| listed.iter().find(|r| r["id"] == id).cloned().unwrap();
    assert_eq!(
        (
            row(first)["reaction"].clone(),
            row(first)["corrected"].clone()
        ),
        (json!(1), json!(1))
    );
    assert_eq!(row(second)["reaction"], serde_json::Value::Null);
    crate::manager::end_session(&key);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn turn_checks_from_the_cli() {
    // What `tirith turn checks` sends.
    assert_eq!(
        super::request("3/4", " portal, filtros ", "bot-x").unwrap(),
        json!({"conv": "bot-x", "passed": 3, "total": 4, "names": "portal, filtros"})
    );
    assert!(super::request("5/4", "", "bot-x").is_err());
    assert!(super::request("tres", "", "bot-x").is_err());
    assert!(include_str!("../assets/usage.txt").contains("tirith turn checks <passed>/<total>"));
    assert!(include_str!("../../../factory/prompts/bot.md").contains("tirith turn checks"));

    let home = home("turn-checks");
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Revisa", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    let port = serve(&app).await;
    let post = |count: &str, names: &str| {
        let body = super::request(count, names, &key).unwrap().to_string();
        async move { http(port, "POST", "/api/turns/checks", &body).await }
    };
    // Nothing to keep them on yet.
    let (status, body) = post("1/1", "x").await;
    assert!(status.contains(" 400"), "{status} {body}");
    // During its turn: the open turn keeps them, and its row has them at its end.
    super::handed(&app, &key, Default::default());
    let (status, body) = post("3/4", "portal, filtros, fechas, enlaces").await;
    assert!(status.contains(" 200"), "{status} {body}");
    super::end(&app, &key, Some(true), Some("Hecho [s2]"));
    let row = own_rows(&app, &key)[0].clone();
    assert_eq!(
        (
            row["checks_passed"].as_i64(),
            row["checks_total"].as_i64(),
            row["checks"].as_str()
        ),
        (Some(3), Some(4), Some("portal, filtros, fechas, enlaces")),
        "{row}"
    );
    assert_eq!(row["cited"], json!(["s2"]));
    // Just after it closed: the last turn gets them.
    let (status, body) = post("4/4", "todo").await;
    assert!(
        status.contains(" 200") && body.contains(&row["id"].to_string()),
        "{status} {body}"
    );
    let row = own_rows(&app, &key)[0].clone();
    assert_eq!(
        (row["checks_passed"].as_i64(), row["checks"].as_str()),
        (Some(4), Some("todo"))
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn cited_notes() {
    assert_eq!(
        super::cited("Uso [m3] y [s1]; otra vez [m3], pero no [x2], [m] ni m4."),
        ["m3", "s1"]
    );
    assert!(super::cited("sin notas").is_empty());
}

#[test]
fn a_turn_the_daemon_was_stopped_in_is_kept_as_failed() {
    let home = home("turn-interrupted");
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Corte", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    // One turn ended and was kept; one routine turn was running when the daemon died; one loose turn of a turn role too.
    super::handed(&app, &key, ("owner".into(), String::new()));
    super::end(&app, &key, Some(true), Some("hecho"));
    super::handed(&app, &key, ("schedule".into(), "r1".into()));
    let _ = super::turn_of(&app, "judge", &key);
    assert_eq!(
        own_rows(&app, &key).len(),
        1,
        "nothing of the running ones is kept yet"
    );
    // The daemon starts again.
    super::start(app.clone());
    let rows = super::rows_of(&app, &key, 10);
    let by = |status: &str| rows.iter().filter(|r| r["status"] == status).count();
    assert_eq!((rows.len(), by("ok"), by("failed")), (3, 1, 2), "{rows:?}");
    let routine = rows.iter().find(|r| r["routine"] == "r1").unwrap();
    assert_eq!(
        (
            routine["trigger"].as_str(),
            routine["role"].as_str(),
            routine["conv"].as_str()
        ),
        (Some("schedule"), Some(key.as_str()), Some(key.as_str()))
    );
    assert!(
        routine["answer"].as_str().unwrap().contains("restarted"),
        "{routine}"
    );
    // Once is enough: another start keeps nothing more.
    super::start(app.clone());
    assert_eq!(super::rows_of(&app, &key, 10).len(), 3);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_cache_write_is_counted_once() {
    let home = home("cache-write");
    let app = crate::app::App::open_at(home.clone());
    let mut t = super::Turn::new(&app, "c", "judge", Default::default());
    t.loose = true;
    // One real Copilot turn (gpt-5.6-luna, 2026-11-22), which its own billing splits as 6 fresh, 14393 read, 14505 written.
    t.line(&json!({"type": "copilot.usage", "data": {"modelMetrics": {"gpt-5.6-luna": {"usage": {"inputTokens": 28904, "outputTokens": 69, "cacheReadTokens": 14393, "cacheWriteTokens": 14505}}}}}));
    super::settle::settle(&app, &mut t);
    let u = &t.usage;
    assert_eq!(
        (u.input, u.cache_read, u.cache_write, u.output),
        (6, 14393, 14505, 69)
    );
    let raw: toml::Table = "[prices.\"m\"]\ninput = 1.0\noutput = 0.0\ncached = 0.0"
        .parse()
        .unwrap();
    let cost = super::cost(&raw, "m", u).unwrap();
    assert!((cost - 0.014511).abs() < 1e-9, "{cost}");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
