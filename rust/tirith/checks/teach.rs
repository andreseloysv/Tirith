use serde_json::{json, Value};

pub(super) fn home(name: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    home
}

/// The daemon on a free port, with one bot: its page id, its conversation and the port.
pub(super) async fn daemon(app: &crate::app::Shared) -> (String, String, u16) {
    let bot = crate::bots::save(app, json!({"name": "Aprende", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let id = bot["id"].as_str().unwrap().to_string();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let app2 = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(app2, port)).await) },
    );
    (id.clone(), format!("bot-{id}"), port)
}

/// A call to the teach API: `origin` and `token` as the extension sends them (empty: not sent).
pub(super) async fn call(
    port: u16,
    method: &str,
    path: &str,
    origin: &str,
    token: &str,
    body: Value,
) -> (u16, Value, String) {
    let url = format!("http://127.0.0.1:{port}/api/teach/{path}");
    // The daemon's port is one per process (other checks serve too): Host without a port passes the guard in any of them.
    let mut req = reqwest::Client::new()
        .request(method.parse().unwrap(), url)
        .header("host", "127.0.0.1");
    req = if origin.is_empty() {
        req
    } else {
        req.header("origin", origin)
    };
    req = if token.is_empty() {
        req
    } else {
        req.bearer_auth(token)
    };
    let req = if method == "POST" {
        req.json(&body)
    } else {
        req
    };
    let res = req.send().await.unwrap();
    let cors = res
        .headers()
        .get("access-control-allow-origin")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let code = res.status().as_u16();
    (code, res.json().await.unwrap_or(Value::Null), cors)
}

pub(super) const EXT: &str = super::ORIGIN;

#[tokio::test]
async fn teach_records_and_masks() {
    let app = crate::app::App::open_at(home("teach-masks"));
    let (id, _, port) = daemon(&app).await;
    let (code, shown, _) = call(port, "GET", &id, "", "", Value::Null).await;
    let token = shown["token"].as_str().unwrap().to_string();
    assert!(code == 200 && token.len() == 32, "{shown}");
    assert_eq!(
        call(port, "POST", &format!("{id}/start"), "", "", Value::Null)
            .await
            .0,
        200
    );
    let ev = format!("{id}/events");
    let step =
        json!({"kind": "type", "label": "Email", "name": "email", "value": "yo@example.com"});
    // No token, or another one: 401, from the page or from the extension.
    assert_eq!(call(port, "POST", &ev, EXT, "", step.clone()).await.0, 401);
    assert_eq!(
        call(
            port,
            "POST",
            &ev,
            EXT,
            "0123456789abcdef0123456789abcdef",
            step.clone()
        )
        .await
        .0,
        401
    );
    assert_eq!(call(port, "POST", &ev, "", "", step.clone()).await.0, 401);
    // The extension's start and read need the token too.
    assert_eq!(
        call(port, "POST", &format!("{id}/start"), EXT, "", Value::Null)
            .await
            .0,
        401
    );
    // Another origin is refused before anything, and gets no CORS.
    let (code, _, cors) = call(
        port,
        "POST",
        &ev,
        "https://evil.example",
        &token,
        step.clone(),
    )
    .await;
    assert_eq!((code, cors.as_str()), (403, ""));
    // The preflight and the answers carry CORS for the extension's origin only.
    let (code, _, cors) = call(port, "OPTIONS", &ev, EXT, "", Value::Null).await;
    assert_eq!((code, cors.as_str()), (204, EXT));
    let steps = json!({"steps": [
        step,
        {"kind": "type", "type": "password", "label": "Contraseña", "value": "hunter2"},
        {"kind": "type", "autocomplete": "cc-number", "label": "Número", "value": "4111111111111111"},
        {"kind": "type", "name": "otp", "value": "123456"},
        {"kind": "type", "label": "Código de verificación", "value": "998877"},
        {"kind": "type", "label": "Nota", "value": "hola", "sensitive": true},
        {"kind": "type", "name": "shipping", "label": "Envío", "value": "Calle Mayor 1"},
        {"kind": "click", "text": "Entrar", "role": "button", "selector": "form > button"},
        {"kind": "script", "value": "not a step"},
    ]});
    let (code, out, cors) = call(port, "POST", &ev, EXT, &token, steps).await;
    assert_eq!(
        (code, out["kept"].as_i64(), cors.as_str()),
        (200, Some(8), EXT),
        "{out}"
    );
    let (_, shown, _) = call(port, "GET", &id, EXT, &token, Value::Null).await;
    assert!(
        shown["token"].is_null(),
        "the extension is not shown the token: {shown}"
    );
    let values: Vec<&str> = shown["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["value"].as_str().unwrap())
        .collect();
    assert_eq!(
        values,
        [
            "yo@example.com",
            "•••",
            "•••",
            "•••",
            "•••",
            "•••",
            "Calle Mayor 1",
            ""
        ],
        "{shown}"
    );
    let raw = app.kv(&format!("teach:bot-{id}")).unwrap();
    assert!(
        !raw.contains("hunter2") && !raw.contains("4111") && !raw.contains("998877"),
        "{raw}"
    );
    // Stopped, a step is not kept.
    assert_eq!(
        call(
            port,
            "POST",
            &format!("{id}/stop"),
            EXT,
            &token,
            Value::Null
        )
        .await
        .0,
        200
    );
    let (_, out, _) = call(
        port,
        "POST",
        &ev,
        EXT,
        &token,
        json!({"kind": "page", "url": "https://x.example"}),
    )
    .await;
    assert_eq!(
        (out["kept"].as_i64(), out["on"].as_bool()),
        (Some(0), Some(false)),
        "{out}"
    );
}

#[tokio::test]
async fn teach_hands_the_steps() {
    let home = home("teach-hands");
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, "#!/bin/sh\necho '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"fake-1\"}'\nwhile read -r line; do\n  printf '%s\\n' \"$line\" >> \"$PWD/got.jsonl\"\n  echo '{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"listo\"}]}}'\n  echo '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false}'\ndone\n").unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let config = format!(
        "[agents.fake]\ncommand = \"{}\"\nchat_args = []\n[network]\nsandbox = false\n",
        agent.display()
    );
    std::fs::write(home.join("config.toml"), config).unwrap();
    let app = crate::app::App::open_at(home.clone());
    let (id, key, port) = daemon(&app).await;
    app.set_kv(&format!("models:{key}"), Some(r#"{"agent":"fake"}"#));
    std::fs::create_dir_all(home.join("manager").join(&key)).unwrap();
    let token = super::state::token(&app, &key);
    let ev = format!("{id}/events");
    let step = |v: Value| call(port, "POST", &ev, EXT, &token, v);
    let started = call(
        port,
        "POST",
        &format!("{id}/start"),
        EXT,
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(started.0, 200, "{started:?}");
    let first =
        step(json!({"kind": "page", "url": "https://portal.example/ofertas", "title": "Ofertas"}))
            .await;
    assert_eq!(
        first.1["kept"],
        1,
        "{first:?} {:?}",
        app.kv(&format!("teach:{key}"))
    );
    crate::manager::say(&app, &key, "primero miro las ofertas nuevas", "").unwrap();
    step(json!({"kind": "click", "text": "Filtrar", "role": "button", "selector": "#filtro"}))
        .await;
    crate::manager::say(&app, &key, "solo las de remoto", "").unwrap();
    step(json!({"kind": "type", "label": "Lugar", "value": "remoto"})).await;
    let got = home.join("manager").join(&key).join("got.jsonl");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(
        !got.exists(),
        "the owner's words while recording start no turn"
    );
    call(port, "POST", &format!("{id}/stop"), "", "", Value::Null).await;
    let mut text = String::new();
    for _ in 0..100 {
        text = std::fs::read_to_string(&got).unwrap_or_default();
        if !text.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    crate::manager::end_session(&key);
    assert_eq!(text.lines().count(), 1, "one turn at stop: {text}");
    let line: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let told = line["message"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    let at = |needle: &str| {
        told.find(needle)
            .unwrap_or_else(|| panic!("no «{needle}» in: {told}"))
    };
    let order = [
        at("1. "),
        at("page «Ofertas» https://portal.example/ofertas"),
        at("The owner wrote: «primero miro las ofertas nuevas»"),
        at("2. "),
        at("click «Filtrar» (button, #filtro)"),
        at("The owner wrote: «solo las de remoto»"),
        at("3. "),
        at("type «Lugar» = remoto"),
    ];
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "steps and words in time order: {told}"
    );
    assert!(
        told.contains("draft: true") && told.contains("## Cómo validar") && told.contains("3 to 5"),
        "{told}"
    );
    assert!(!app
        .query(
            "SELECT text FROM chat WHERE project = ?1 AND role = 'owner'",
            [&key],
            |r| r.get::<_, String>(0)
        )
        .is_empty());
}
