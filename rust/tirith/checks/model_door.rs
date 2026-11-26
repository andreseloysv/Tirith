// The model door: the agent's environment holds a token and the door's address, never the key; the door forwards model paths with the real key, streams the answer back, counts it and keeps a daily budget.
use super::{given, paths::known, spec::find, tally::Tally};
use crate::app::{App, Shared};
use axum::{http::HeaderMap, response::IntoResponse, routing::post, Router};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

const KEY: &str = "sk-real-provider-key-0123456789";

fn vars(url: &str) -> Vec<(String, String)> {
    let v = |k: &str, x: &str| (k.to_string(), x.to_string());
    vec![
        v("COPILOT_OFFLINE", "true"),
        v("COPILOT_PROVIDER_TYPE", "anthropic"),
        v("COPILOT_PROVIDER_BASE_URL", url),
        v("COPILOT_PROVIDER_API_KEY", KEY),
    ]
}

fn app(name: &str, config: &str) -> Shared {
    let home = std::env::temp_dir().join(format!("tirith-door-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(home.join("auth")).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!("[network]\nsandbox = false\n{config}"),
    )
    .unwrap();
    App::open_at(home)
}

#[test]
fn the_agent_gets_a_token_and_the_door_never_the_key() {
    let off = app("env-off", "");
    assert_eq!(
        given(&off, "lane-1", "copilot", vars("https://api.anthropic.com")),
        vars("https://api.anthropic.com"),
        "off: as it was"
    );
    let on = app("env-on", "[door]\nenabled = true\n");
    let got = given(&on, "lane-1", "copilot", vars("https://api.anthropic.com"));
    let get = |k: &str| {
        got.iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .unwrap()
    };
    assert!(
        !got.iter().any(|(_, v)| v.contains(KEY)),
        "the key is not in it"
    );
    let door = format!("http://127.0.0.1:{}/door/copilot", on.cfg().port);
    assert_eq!(get("COPILOT_PROVIDER_BASE_URL"), door);
    assert_eq!(get("COPILOT_OFFLINE"), "true");
    let who = crate::sandbox::who_of(&on, &get("COPILOT_PROVIDER_API_KEY"));
    assert_eq!(
        who.as_deref(),
        Some("door:copilot:lane-1"),
        "valid for that caller alone"
    );
    assert_ne!(
        get("COPILOT_PROVIDER_API_KEY"),
        given(&on, "lane-2", "copilot", vars("https://api.anthropic.com"))[3].1
    );
    let server = app("env-server", "[server]\nmode = \"server\"\n");
    assert!(
        given(&server, "l", "copilot", vars("https://x.example/v1"))
            .iter()
            .all(|(_, v)| !v.contains(KEY)),
        "on by default in server mode"
    );
    assert_eq!(find(&vars("https://x.example/v1"))[0].path, "/v1");
}

#[test]
fn only_model_paths_are_known() {
    for (m, p) in [
        ("POST", "/v1/messages"),
        ("POST", "/v1/messages/count_tokens"),
        ("GET", "/v1/models"),
        ("GET", "/v1/models/m"),
        ("POST", "/v1/chat/completions"),
        ("POST", "/chat/completions"),
        ("POST", "/v1/responses"),
        ("POST", "/openai/deployments/d/chat/completions"),
    ] {
        assert!(known(m, p), "{m} {p}");
    }
    for (m, p) in [
        ("GET", "/v1/messages"),
        ("POST", "/v1/files"),
        ("DELETE", "/v1/models/m"),
        ("GET", "/v1/organizations/x"),
        ("POST", "/v1/messages/../files"),
        ("POST", "/v1/batches"),
        ("GET", "/"),
    ] {
        assert!(!known(m, p), "{m} {p}");
    }
}

#[test]
fn the_tally_reads_each_providers_usage() {
    let mut t = Tally::default();
    let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":100,\"cache_read_input_tokens\":900,\"cache_creation_input_tokens\":50,\"output_tokens\":1}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":200}}\n\n";
    let (a, b) = sse.split_at(70);
    t.feed(a.as_bytes(), true);
    t.feed(b.as_bytes(), true);
    t.end();
    assert_eq!(
        (t.input, t.output, t.cache_read, t.cache_write),
        (100, 200, 900, 50)
    );
    let mut o = Tally::default();
    o.feed(br#"{"usage":{"prompt_tokens":1000,"completion_tokens":30,"prompt_tokens_details":{"cached_tokens":400}}}"#, false);
    o.end();
    assert_eq!((o.input, o.output, o.cache_read), (600, 30, 400));
    let mut r = Tally::default();
    r.feed(b"data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":50,\"output_tokens\":5,\"input_tokens_details\":{\"cached_tokens\":10}}}}\n", true);
    assert_eq!((r.input, r.output, r.cache_read), (40, 5, 10));
}

type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

/// A provider that answers /v1/messages with a stream and records the key header it was sent.
async fn provider(seen: Seen) -> u16 {
    let route = post(move |h: HeaderMap, body: String| {
        let seen = seen.clone();
        async move {
            let header = |n: &str| {
                h.get(n)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string()
            };
            seen.lock()
                .unwrap()
                .push((header("x-api-key"), header("authorization"), body));
            let sse = "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1000000,\"output_tokens\":1}}}\n\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":100000}}\n\n";
            ([("content-type", "text/event-stream")], sse).into_response()
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        drop(axum::serve(listener, Router::new().route("/v1/messages", route)).await)
    });
    port
}

#[tokio::test]
async fn the_door_forwards_with_the_real_key_and_counts_the_call() {
    let seen: Seen = Default::default();
    let up = provider(seen.clone()).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = format!("[server]\nport = {port}\n[door]\nenabled = true\nbudget_usd = 0.2\n[prices.model-a]\ninput = 0.10\ncached = 0.01\noutput = 0.50\n");
    let app = app("route", &config);
    let env = format!("COPILOT_PROVIDER_TYPE=anthropic\nCOPILOT_PROVIDER_BASE_URL=http://127.0.0.1:{up}\nCOPILOT_PROVIDER_API_KEY={KEY}\n");
    std::fs::write(app.home.join("auth/copilot.env"), env).unwrap();
    crate::server::testing::serving(port, &app);
    let served = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(served, port)).await) },
    );
    let got = given(
        &app,
        "lane-1",
        "copilot",
        std::fs::read_to_string(app.home.join("auth/copilot.env"))
            .unwrap()
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
    );
    let token = got
        .iter()
        .find(|(k, _)| k == "COPILOT_PROVIDER_API_KEY")
        .unwrap()
        .1
        .clone();
    let base = got
        .iter()
        .find(|(k, _)| k == "COPILOT_PROVIDER_BASE_URL")
        .unwrap()
        .1
        .clone();
    let call = |path: &str, key: &str, method: reqwest::Method| {
        let req = reqwest::Client::new().request(method, format!("{base}{path}"));
        req.header("host", "127.0.0.1") // the guard knows one port per process: the first test daemon's
            .header("x-api-key", key.to_string())
            .body(json!({"model": "model-a"}).to_string())
            .send()
    };
    let ok = call("/v1/messages", &token, reqwest::Method::POST)
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    assert!(
        ok.text().await.unwrap().contains("message_delta"),
        "the stream comes back as it is"
    );
    assert_eq!(
        seen.lock().unwrap()[0].0,
        KEY,
        "the provider got the real key"
    );
    assert!(!seen.lock().unwrap()[0].2.contains(&token));
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let rows: Vec<Value> = super::record::recent(&app, 10);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        (
            rows[0]["who"].as_str(),
            rows[0]["model"].as_str(),
            rows[0]["status"].as_i64()
        ),
        (Some("lane-1"), Some("model-a"), Some(200))
    );
    assert_eq!(
        (
            rows[0]["tokens_in"].as_i64(),
            rows[0]["tokens_out"].as_i64()
        ),
        (Some(1_000_000), Some(100_000))
    );
    assert!(
        (rows[0]["cost_usd"].as_f64().unwrap() - 0.15).abs() < 1e-9,
        "1M in at 0.10 and 100k out at 0.50"
    );
    assert_eq!(
        call("/v1/files", &token, reqwest::Method::POST)
            .await
            .unwrap()
            .status(),
        403,
        "other paths are refused"
    );
    assert_eq!(
        call("/v1/messages", KEY, reqwest::Method::POST)
            .await
            .unwrap()
            .status(),
        401,
        "the key is no token"
    );
    assert_eq!(
        call("/v1/messages", "nonsense", reqwest::Method::POST)
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        call("/v1/messages", &token, reqwest::Method::POST)
            .await
            .unwrap()
            .status(),
        200,
        "0.15 of 0.20 is spent"
    );
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let over = call("/v1/messages", &token, reqwest::Method::POST)
        .await
        .unwrap();
    assert_eq!(over.status(), 429, "the daily budget is spent");
    assert!(over.text().await.unwrap().contains("[door] budget_usd"));
    assert_eq!(
        seen.lock().unwrap().len(),
        2,
        "a refused call never reaches the provider"
    );
}
