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
    // Its event: turn.done, in the bot's conversation, saying the bot, the trigger, the tool, the skill and the answer.
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
        ("turn.done", key.as_str())
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
