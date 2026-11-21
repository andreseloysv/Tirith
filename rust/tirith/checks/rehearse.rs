use super::tests::{call, daemon, home, EXT};
use serde_json::{json, Value};

/// A 1×1 PNG, as the extension's captureVisibleTab gives it.
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

#[tokio::test]
async fn teach_rehearsal_relays_commands() {
    let app = crate::app::App::open_at(home("teach-rehearse"));
    let (id, key, port) = daemon(&app).await;
    let token = super::state::token(&app, &key);
    let step = json!({"kind": "click", "target": "Filtrar"});
    // No rehearsal yet: a step is refused.
    let (code, out, _) = call(port, "POST", &format!("{id}/do"), "", "", step.clone()).await;
    assert_eq!(code, 409, "{out}");
    let (code, out, _) = call(
        port,
        "POST",
        &format!("{id}/rehearse"),
        "",
        "",
        json!({"skill": "ofertas"}),
    )
    .await;
    assert_eq!(code, 200, "{out}");
    // The extension asks for its next step only with the token.
    let (code, _, cors) = call(port, "GET", &format!("{id}/next"), EXT, "", Value::Null).await;
    assert_eq!((code, cors.as_str()), (401, EXT));
    // The extension may not send steps to itself.
    let (code, _, _) = call(port, "POST", &format!("{id}/do"), EXT, &token, step.clone()).await;
    assert_eq!(code, 403);
    // The fake extension: takes the step with its token, does it and answers with a result and a screenshot.
    let (id2, token2) = (id.clone(), token.clone());
    let ext = tokio::spawn(async move {
        let (code, got, _) = call(
            port,
            "GET",
            &format!("{id2}/next"),
            EXT,
            &token2,
            Value::Null,
        )
        .await;
        assert_eq!(code, 200, "{got}");
        let cmd = got["cmd"].clone();
        assert_eq!(
            (cmd["kind"].as_str(), cmd["target"].as_str()),
            (Some("click"), Some("Filtrar")),
            "{got}"
        );
        assert!(got["until"].as_u64().unwrap() > cmd["until"].as_u64().unwrap() - 1);
        let answer = json!({"id": cmd["id"], "ok": true, "result": "clicked «Filtrar» (button#filtro)", "shot": format!("data:image/png;base64,{PNG}")});
        let (code, done, _) =
            call(port, "POST", &format!("{id2}/done"), EXT, &token2, answer).await;
        assert_eq!((code, done["ok"].as_bool()), (200, Some(true)), "{done}");
    });
    let (code, out, _) = call(port, "POST", &format!("{id}/do"), "", "", step.clone()).await;
    ext.await.unwrap();
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["result"], "clicked «Filtrar» (button#filtro)", "{out}");
    assert_eq!(out["n"], 1, "{out}");
    let shot = std::path::PathBuf::from(out["shot"].as_str().unwrap());
    assert!(
        shot.starts_with(crate::bots::folder(&app, &key).join("rehearsals")),
        "{out}"
    );
    assert_eq!(&std::fs::read(&shot).unwrap()[1..4], b"PNG");
    // The page sees the rehearsal with its screenshot, and may fetch it.
    let (_, shown, _) = call(port, "GET", &id, "", "", Value::Null).await;
    let r = &shown["rehearsals"][0];
    assert_eq!(
        (r["skill"].as_str(), r["steps"][0]["ok"].as_bool()),
        (Some("ofertas"), Some(true)),
        "{shown}"
    );
    let file = r["steps"][0]["shot"].as_str().unwrap();
    assert_eq!(
        call(
            port,
            "GET",
            &format!("{id}/shot/{file}"),
            "",
            "",
            Value::Null
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(
            port,
            "GET",
            &format!("{id}/shot/{file}"),
            EXT,
            &token,
            Value::Null
        )
        .await
        .0,
        403
    );
    // A step nobody answers times out without hanging: covered by the relay; here the 10 minutes run out.
    let mut list = super::rehearsal::all(&app, &key);
    list[0]["since_s"] = json!(crate::app::now_s() - 601);
    app.set_kv(
        &format!("rehearse:{key}"),
        Some(&Value::from(list).to_string()),
    );
    let (code, out, _) = call(port, "POST", &format!("{id}/do"), "", "", step).await;
    assert_eq!(code, 410, "{out}");
    let (_, got, _) = call(port, "GET", &format!("{id}/next"), EXT, &token, Value::Null).await;
    assert_eq!(got["on"], false, "{got}");
    // The commands the bot runs.
    let req = super::cli::request;
    assert_eq!(
        req(&["do", "type", "Lugar", "remoto"]).unwrap().1,
        json!({"kind": "type", "target": "Lugar", "value": "remoto"})
    );
    assert_eq!(req(&["do", "open", "https://x.example"]).unwrap().0, "do");
    assert_eq!(req(&["rehearse", "ofertas"]).unwrap().0, "rehearse");
    assert!(req(&["do", "submit", "x"]).is_err());
}

#[tokio::test]
async fn teach_approve_activates_the_skill() {
    let app = crate::app::App::open_at(home("teach-approve"));
    let (id, key, port) = daemon(&app).await;
    let file = crate::bots::skill_file(&app, &key, "ofertas");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let text = "---\nname: ofertas\ndescription: Mira las ofertas nuevas\ndraft: true\n---\n\n## Pasos\n\ndraft: true se queda aquí, fuera de la cabecera\n";
    std::fs::write(&file, text).unwrap();
    let path = format!("{id}/approve");
    let approve =
        |origin: &'static str| call(port, "POST", &path, origin, "", json!({"skill": "ofertas"}));
    // Not from the extension, even with its origin.
    assert_eq!(approve(EXT).await.0, 401);
    let (code, out, _) = approve("").await;
    assert_eq!(code, 200, "{out}");
    let now = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        now,
        "---\nname: ofertas\ndescription: Mira las ofertas nuevas\n---\n\n## Pasos\n\ndraft: true se queda aquí, fuera de la cabecera\n"
    );
    // Once active, it is no draft to approve again; an unknown skill neither.
    let (code, out, _) = approve("").await;
    assert_eq!(code, 400, "{out}");
    let (code, _, _) = call(
        port,
        "POST",
        &format!("{id}/approve"),
        "",
        "",
        json!({"skill": "nada"}),
    )
    .await;
    assert_eq!(code, 400);
    assert!(app
        .query("SELECT text FROM chat WHERE project = ?1", [&key], |r| {
            r.get::<_, String>(0)
        })
        .iter()
        .any(|t| t.contains("approved «ofertas»")));
}
