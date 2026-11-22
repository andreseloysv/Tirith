use serde_json::{json, Value};

fn home(name: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    home
}

/// One HTTP request to a test daemon on `port`: the status line and the body.
async fn http(port: u16, method: &str, path: &str, body: &str) -> (String, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    s.write_all(format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
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
    let app = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(app, port)).await) },
    );
    port
}

/// A bot with one routine made through the API: (its page id, its key, the routine's id, the port).
async fn bot_with_routine(
    app: &crate::app::Shared,
    name: &str,
    task: &str,
) -> (String, String, String, u16) {
    let bot = crate::bots::save(app, json!({"name": name, "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let id = bot["id"].as_str().unwrap().to_string();
    let port = serve(app).await;
    let body =
        json!({"title": "Ofertas", "prompt": task, "days": [1, 2, 3, 4, 5, 6, 7], "at": "08:00"});
    let (status, body) = http(
        port,
        "POST",
        &format!("/api/bots/{id}/routines"),
        &body.to_string(),
    )
    .await;
    assert!(status.contains(" 200"), "{status} {body}");
    let (_, body) = http(port, "GET", &format!("/api/bots/{id}/routines"), "").await;
    let routines: Vec<Value> = serde_json::from_str(&body).unwrap();
    let rid = routines[0]["id"].as_str().unwrap().to_string();
    (id.clone(), format!("bot-{id}"), rid, port)
}

/// One seeded row of `turns`.
#[allow(clippy::too_many_arguments)]
fn seed(
    app: &crate::app::Shared,
    conv: &str,
    role: &str,
    routine: &str,
    ok: bool,
    (ms, cost, tokens): (i64, f64, i64),
    skills: &[&str],
    checks: Option<(i64, i64)>,
    (reaction, corrected, score): (Option<i64>, i64, Option<f64>),
    cited: &[&str],
) {
    let status = if ok { "ok" } else { "failed" };
    let (skills, cited) = (json!(skills).to_string(), json!(cited).to_string());
    app.insert("INSERT INTO turns (conv, role, routine, status, ms, cost_usd, tokens_in, tokens_out, skills, checks_passed, checks_total, reaction, corrected, score, cited) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)", rusqlite::params![conv, role, routine, status, ms, cost, tokens - 100, 100, skills, checks.map(|c| c.0), checks.map(|c| c.1), reaction, corrected, score, cited]);
}

fn close(a: &Value, b: f64) -> bool {
    a.as_f64().is_some_and(|a| (a - b).abs() < 1e-6)
}

#[tokio::test]
async fn bot_numbers() {
    let home = home("bot-numbers");
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let (id, key, rid, port) = bot_with_routine(&app, "Cifras", "busca ofertas").await;
    // 20 runs of its routine: the first 10 cost 0.2, take 4 s and 2000 tokens, and every other one fails; the last 10, with the skill «ofertas», cost 0.1, take 2 s and 1000 tokens, and all end ok.
    for i in 1..=20 {
        let late = i > 10;
        let (ms, cost, tokens) = if late {
            (2000, 0.1, 1000)
        } else {
            (4000, 0.2, 2000)
        };
        let checks = late
            .then_some(if i == 19 { (2, 3) } else { (3, 3) })
            .filter(|_| i != 20);
        let reaction = [3, 12].contains(&i).then_some(-1);
        let score = if late {
            Some(0.8)
        } else {
            (i == 1).then_some(0.4)
        };
        let cited: &[&str] = if i == 15 { &["s1"] } else { &[] };
        seed(
            &app,
            &key,
            &key,
            &rid,
            late || i % 2 == 1,
            (ms, cost, tokens),
            if late { &["ofertas"] } else { &[] },
            checks,
            (reaction, i64::from(i == 5), score),
            cited,
        );
    }
    // A chat turn citing [m1], and a judge's turn in its conversation that counts for nothing.
    seed(
        &app,
        &key,
        &key,
        "",
        true,
        (1000, 0.05, 500),
        &[],
        None,
        (Some(1), 0, None),
        &["m1"],
    );
    seed(
        &app,
        &key,
        "judge",
        "",
        false,
        (1, 9.0, 1),
        &["ofertas"],
        None,
        (Some(-1), 1, Some(0.0)),
        &["m2"],
    );
    // Its memory, the shared one, and one owner message owner-mood judged a repeat.
    let own = "# Notas\n- [m1] usa el portal\n- [m2] nunca los lunes\n";
    let shared = "- [s1] el dueño se llama Ana\n";
    std::fs::create_dir_all(home.join("manager").join(&key)).unwrap();
    std::fs::write(home.join("manager").join(&key).join("memory.md"), own).unwrap();
    std::fs::create_dir_all(home.join("bots")).unwrap();
    std::fs::write(home.join("bots/memory.md"), shared).unwrap();
    let event = app.event(
        "",
        &key,
        "owner.message",
        "",
        "otra vez: te dije sin los lunes",
        "",
    );
    app.insert(
        "INSERT INTO scores (at, event, eval, label) VALUES ('now', ?1, 'owner-mood', 'repeat')",
        rusqlite::params![event],
    );

    let (status, body) = http(port, "GET", &format!("/api/bots/{id}/numbers"), "").await;
    assert!(status.contains(" 200"), "{status} {body}");
    let n: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(n["runs"], 21, "{n}");
    let r = &n["routines"][0];
    assert_eq!(
        (r["id"].as_str(), r["title"].as_str(), r["runs"].as_i64()),
        (Some(rid.as_str()), Some("Ofertas"), Some(20)),
        "{r}"
    );
    assert!(close(&r["success"], 0.75) && close(&r["ms"], 3000.0), "{r}");
    // Cost per good run: what all 20 cost (3.0) over the 15 that ended ok.
    assert!(close(&r["cost_per_good"], 0.2), "{r}");
    assert!(
        close(&r["down_per_10"], 1.0) && close(&r["corrections_per_10"], 0.5),
        "{r}"
    );
    assert!(close(&r["score"], 8.4 / 11.0), "{r}");
    // The trend: the last 10 against the 10 before, in cost, time and success.
    let (last, before) = (&r["trend"]["last"], &r["trend"]["before"]);
    assert!(
        close(&last["cost_per_good"], 0.1)
            && close(&last["ms"], 2000.0)
            && close(&last["success"], 1.0),
        "{r}"
    );
    assert!(
        close(&before["cost_per_good"], 0.4)
            && close(&before["ms"], 4000.0)
            && close(&before["success"], 0.5),
        "{r}"
    );
    assert_eq!(
        (
            n["routines"][1]["title"].as_str(),
            n["routines"][1]["runs"].as_i64()
        ),
        (Some("Chat"), Some(1))
    );
    // The skill: 10 uses, 8 validated, and its routine's tokens halved after its first use.
    let s = &n["skills"][0];
    assert_eq!(n["skills"].as_array().unwrap().len(), 1, "{n}");
    assert_eq!(
        (
            s["name"].as_str(),
            s["used"].as_i64(),
            s["validated"].as_i64()
        ),
        (Some("ofertas"), Some(10), Some(8)),
        "{s}"
    );
    assert_eq!(
        (
            s["tokens_before"].as_i64(),
            s["tokens_after"].as_i64(),
            s["saved_percent"].as_f64()
        ),
        (Some(2000), Some(1000), Some(50.0)),
        "{s}"
    );
    // The memory: its loading cost, the notes cited, the one none cited (never deleted) and the miss.
    let m = &n["memory"];
    assert_eq!(
        m["tokens"],
        json!({"own": own.chars().count() / 4, "shared": shared.chars().count() / 4}),
        "{m}"
    );
    let cites = |id: &str| {
        m["notes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .map(|x| x["cited"].clone())
    };
    assert_eq!(
        (cites("s1"), cites("m1"), cites("m2")),
        (Some(json!(1)), Some(json!(1)), Some(json!(0))),
        "{m}"
    );
    // No note of this memory is 30 runs old (it has no history yet), so none is a candidate to delete.
    assert_eq!(m["uncited"], json!([]), "{m}");
    assert!(
        std::fs::read_to_string(home.join("manager").join(&key).join("memory.md"))
            .unwrap()
            .contains("[m2]")
    );
    assert_eq!(
        (m["misses"].as_i64(), m["missed"][0].as_str()),
        (Some(1), Some("otra vez: te dije sin los lunes")),
        "{m}"
    );
    let (status, _) = http(port, "GET", "/api/bots/nobody/numbers", "").await;
    assert!(status.contains(" 400"), "{status}");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn turn_quality_score_lands_on_its_turn() {
    let f = crate::factory::Factory::default();
    assert!(f.evals_on("routine.done").contains(&(
        "turn-quality".to_string(),
        "judge".to_string(),
        "turn-quality".to_string(),
        1.0
    )));
    assert!(f.prompt("turn-quality").contains("Cómo validar"));
    // A bot and its judge on one fake agent that reads the skill «ofertas» and answers with a score.
    let home = home("turn-quality");
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, r#"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"fake-q"}'
while read -r line; do
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cat .agents/skills/ofertas/SKILL.md"}}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"0.8 — tres ofertas con enlace"}]}}'
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
    let (id, key, rid, port) =
        bot_with_routine(&app, "Juzgado", "busca ofertas de trabajo nuevas").await;
    app.set_kv(&format!("models:{key}"), Some(r#"{"agent":"fake"}"#));
    app.set_kv("models:judge", Some(r#"{"agent":"fake"}"#));
    let skill = home
        .join("manager")
        .join(&key)
        .join(".agents/skills/ofertas");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "---\nname: ofertas\n---\n## Pasos\n1. mira\n## Cómo validar\n- cada oferta tiene enlace\n## Qué devuelve\nTres líneas.\n").unwrap();

    let (status, body) = http(port, "POST", &format!("/api/bots/{id}/run/{rid}"), "").await;
    assert!(status.contains(" 200"), "{status} {body}");
    let own = |app: &crate::app::Shared| crate::turns::rows_of(app, &key, 100);
    let mut row = Value::Null;
    for _ in 0..300 {
        row = own(&app)
            .into_iter()
            .find(|r| r["role"] == key.as_str() && !r["score"].is_null())
            .unwrap_or(Value::Null);
        if !row.is_null() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    crate::manager::end_session(&key);
    assert!(
        close(&row["score"], 0.8),
        "the score never landed on the routine's turn: {:?}",
        own(&app)
    );
    assert_eq!(row["routine"].as_str(), Some(rid.as_str()));
    // The score is the judge's on that turn's routine.done, which carried the task, the answer and the skill's «Cómo validar».
    let event = row["event"].as_i64().unwrap();
    let (kind, summary) = app.query(
        "SELECT kind, summary FROM events WHERE id = ?1",
        [event],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )[0]
    .clone();
    assert_eq!(kind, "routine.done");
    assert!(
        summary.contains("busca ofertas de trabajo nuevas")
            && summary.contains("answer: 0.8 — tres ofertas")
            && summary.contains("how to validate «ofertas»: - cada oferta tiene enlace")
            && !summary.contains("Tres líneas"),
        "{summary}"
    );
    assert!(summary.chars().count() <= 2000);
    let scores = app.query(
        "SELECT score FROM scores WHERE eval = 'turn-quality' AND event = ?1",
        [event],
        |r| r.get::<_, f64>(0),
    );
    assert_eq!(scores, [0.8]);
    // The judge's own turn in that conversation keeps no score and emits no event of its own.
    let judge: Vec<Value> = own(&app)
        .into_iter()
        .filter(|r| r["role"] == "judge")
        .collect();
    assert!(
        !judge.is_empty()
            && judge
                .iter()
                .all(|r| r["score"].is_null() && r["event"].is_null()),
        "{judge:?}"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn waiting_and_b_runs_are_not_in_the_routines_numbers() {
    let home = home("numbers-waiting");
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let (id, key, rid, _) = bot_with_routine(&app, "Espera", "busca ofertas").await;
    let seed_as = |status: &str, variant: Option<&str>| {
        app.insert("INSERT INTO turns (conv, role, routine, status, ms, cost_usd, tokens_in, tokens_out, variant) VALUES (?1, ?1, ?2, ?3, 1000, 0.1, 900, 100, ?4)", rusqlite::params![key, rid, status, variant]);
    };
    // 3 ok, 1 failed, 2 waiting for the owner's yes, and 5 failed B runs of an experiment.
    (0..3).for_each(|_| seed_as("ok", None));
    seed_as("failed", None);
    (0..2).for_each(|_| seed_as("waiting", None));
    (0..5).for_each(|_| seed_as("failed", Some("B")));
    let n = super::numbers(&app, &id).unwrap();
    let r = &n["routines"][0];
    assert_eq!(
        (r["runs"].as_i64(), r["waiting"].as_i64()),
        (Some(6), Some(2)),
        "the B runs are no runs of the routine: {r}"
    );
    assert!(
        close(&r["success"], 0.75),
        "3 of the 4 that ended; the waiting ones are neither: {r}"
    );
    // Not a good run either: the cost per good run divides all the cost by the 3 ok ones.
    assert!(close(&r["cost_per_good"], 0.6 / 3.0), "{r}");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn memory_candidates_wait_for_thirty_runs_after_the_note() {
    let home = home("numbers-age");
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let (id, key, rid, _) = bot_with_routine(&app, "Notas", "busca ofertas").await;
    let at = |started: &str, cited: &str| {
        app.insert("INSERT INTO turns (conv, role, routine, status, started_at, cited) VALUES (?1, ?1, ?2, 'ok', ?3, ?4)", rusqlite::params![key, rid, started, cited]);
    };
    let dir = home.join("manager").join(&key);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(home.join("bots")).unwrap();
    std::fs::write(
        home.join("bots/memory.md"),
        "- [s1] el dueño se llama Ana\n",
    )
    .unwrap();
    // Two notes written now, then 31 runs after them (one cites m1).
    std::fs::write(
        dir.join("memory.md"),
        "- [m1] usa el portal\n- [m2] nunca los lunes\n",
    )
    .unwrap();
    crate::bots::versioned(&app, &key, "library");
    (0..30).for_each(|_| at("2080-01-01T00:00:00Z", "[]"));
    at("2080-01-01T00:00:00Z", "[\"m1\"]");
    let candidates =
        |app: &crate::app::Shared| super::numbers(app, &id).unwrap()["memory"]["uncited"].clone();
    // m2 is 31 runs old and nobody cites it; the shared note nobody cites is no candidate of this bot's runs.
    assert_eq!(candidates(&app), json!(["m2"]));
    // A note written later (a commit after those runs) is not a candidate until 30 runs came after it, cited or not.
    std::fs::write(
        dir.join("memory.md"),
        "- [m1] usa el portal\n- [m2] nunca los lunes\n- [m3] antes de las 9\n",
    )
    .unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args([
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .env("GIT_COMMITTER_DATE", "2090-01-01T00:00:00Z")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "turn 99"]);
    (0..5).for_each(|_| at("2091-01-01T00:00:00Z", "[]"));
    assert_eq!(candidates(&app), json!(["m2"]), "m3 is 5 runs old");
    (0..25).for_each(|_| at("2091-01-01T00:00:00Z", "[]"));
    // (m1's one citation is out of the last 30 runs by now, and it is older than that: a candidate too.)
    assert_eq!(
        candidates(&app),
        json!(["m1", "m2", "m3"]),
        "m3 is 30 runs old now"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_run_that_a_restart_cut_adds_no_time_and_no_tokens_to_compare() {
    let home = home("numbers-cut");
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let key = "bot-corta";
    seed(
        &app,
        key,
        key,
        "r1",
        true,
        (4000, 0.1, 2000),
        &["regar"],
        None,
        (None, 0, None),
        &[],
    );
    // As turns/interrupted.rs keeps it: the agent that ran is known, what it took and spent is not.
    app.insert(
        "INSERT INTO turns (conv, role, routine, status, agent) VALUES (?1, ?1, 'r1', 'failed', 'fake')",
        rusqlite::params![key],
    );
    let rows = super::rows::rows(&app, key);
    let r = &super::routines::by_routine(&rows, &[("r1".to_string(), "Regar".to_string())])[0];
    assert_eq!(r["runs"], 2, "{r}");
    assert!(close(&r["success"], 0.5), "it is a failure: {r}");
    assert!(close(&r["ms"], 4000.0), "and no run of zero seconds: {r}");
    let skill = &super::skills::by_skill(&rows)[0];
    assert_eq!(skill["tokens_after"], 2000, "nor of zero tokens: {skill}");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
