// Signing in from a browser: the gate's redirect, /auth/login, /auth/callback against a fake token endpoint.
use super::gate_tests::{start, Daemon};
use super::oidc_tests::{good, jwt, CLIENT};
use super::pending;
use reqwest::Url;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

const SITE: &str = "https://neobot.team.example";
const HTML: (&str, &str) = ("accept", "text/html,application/xhtml+xml");

/// A token endpoint that keeps each form it is sent and answers with `reply`.
async fn token_endpoint(status: u16, reply: Value) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let uri = format!(
        "http://127.0.0.1:{}/token",
        listener.local_addr().unwrap().port()
    );
    let forms = Arc::new(Mutex::new(Vec::new()));
    let kept = forms.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                return;
            };
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0u8; 8192];
            let n = s.read(&mut buf).await.unwrap_or(0);
            let text = String::from_utf8_lossy(&buf[..n]).to_string();
            kept.lock()
                .unwrap()
                .push(text.split("\r\n\r\n").nth(1).unwrap_or("").into());
            let body = reply.to_string();
            let said = format!("HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
            let _ = s.write_all(said.as_bytes()).await;
        }
    });
    (uri, forms)
}

async fn member_token() -> Value {
    json!({"access_token": jwt(0, "k1", "RS256", &good(&["Team.Member"]))})
}

/// A daemon with a public address and the fake provider endpoints.
async fn on_site(tag: &str, token: &str) -> Daemon {
    let extra = format!("authorization_endpoint = \"https://login.example.test/tenant-1/oauth2/v2.0/authorize\"\ntoken_endpoint = \"{token}\"\n[server]\npublic_url = \"{SITE}/\"\n");
    start(tag, &extra).await
}

struct Seen {
    status: u16,
    location: String,
    cookies: Vec<String>,
    body: String,
}

async fn get(d: &Daemon, path: &str, headers: &[(&str, &str)]) -> Seen {
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none());
    let mut req = http
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{}{path}", d.port))
        .header("host", "127.0.0.1");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let res = req.send().await.unwrap();
    let header = |n: &str| {
        res.headers()
            .get(n)
            .map(|v| v.to_str().unwrap().to_string())
    };
    let cookies = res.headers().get_all("set-cookie").iter();
    let cookies = cookies.map(|v| v.to_str().unwrap().to_string()).collect();
    Seen {
        status: res.status().as_u16(),
        location: header("location").unwrap_or_default(),
        cookies,
        body: res.text().await.unwrap(),
    }
}

/// `/auth/login`: the state, and the cookie that goes with it.
async fn begin(d: &Daemon, next: &str) -> (Seen, String, String) {
    let path = format!(
        "/auth/login?next={}",
        Url::parse_with_params("http://x/", [("n", next)])
            .unwrap()
            .query()
            .unwrap()
            .trim_start_matches("n=")
    );
    let seen = get(d, &path, &[HTML]).await;
    let state = Url::parse(&seen.location)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    let cookie = seen
        .cookies
        .first()
        .map(|c| c.split(';').next().unwrap().to_string())
        .unwrap_or_default();
    (seen, state, cookie)
}

fn call_back(state: &str) -> String {
    format!("/auth/callback?code=the-code&state={state}")
}

#[tokio::test]
async fn a_browser_without_a_session_is_sent_to_sign_in_only_with_a_public_address() {
    let d = on_site("nav", "http://127.0.0.1:9/token").await;
    let page = get(&d, "/", &[HTML]).await;
    assert_eq!(
        (page.status, page.location.as_str()),
        (302, "/auth/login?next=%2F")
    );
    let deep = get(&d, "/rooms?id=1", &[HTML]).await;
    assert_eq!(deep.location, "/auth/login?next=%2Frooms%3Fid%3D1");
    for (path, accept) in [
        ("/api/bots", "application/json"),
        ("/api/bots", "text/html"),
        ("/api/events", "text/event-stream"),
        ("/", "application/json"),
    ] {
        assert_eq!(
            get(&d, path, &[("accept", accept)]).await.status,
            403,
            "{path} {accept}"
        );
    }
    let post = d
        .http
        .post(format!("http://127.0.0.1:{}/", d.port))
        .header("accept", "text/html")
        .header("host", "127.0.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(post.status(), 403);
    let plain = start("nav-plain", "").await;
    assert_eq!(
        get(&plain, "/", &[HTML]).await.status,
        403,
        "no public_url: as before"
    );
}

#[tokio::test]
async fn login_sends_the_browser_to_the_provider_with_pkce_and_binds_it() {
    let d = on_site("begin", "http://127.0.0.1:9/token").await;
    let (seen, state, cookie) = begin(&d, "/rooms").await;
    assert_eq!(seen.status, 302);
    let url = Url::parse(&seen.location).unwrap();
    assert_eq!(url.host_str(), Some("login.example.test"));
    let q = |k: &str| {
        url.query_pairs()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.to_string())
    };
    assert_eq!(q("redirect_uri").unwrap(), format!("{SITE}/auth/callback"));
    assert_eq!(q("code_challenge_method").as_deref(), Some("S256"));
    assert_eq!(
        (q("client_id").as_deref(), q("response_type").as_deref()),
        (Some(CLIENT), Some("code"))
    );
    assert_eq!(q("response_mode").as_deref(), Some("query"));
    assert!(state.len() >= 22 && q("code_challenge").unwrap().len() >= 43);
    let (_, app, _) = d.call("GET", "/api/auth/oidc", &[], Value::Null).await;
    let app: Value = serde_json::from_str(&app).unwrap();
    assert_eq!(
        q("scope").unwrap(),
        app["scope"].as_str().unwrap(),
        "the app's scope, nothing more"
    );
    let set = &seen.cookies[0];
    assert!(
        set.contains(&state)
            && set.contains("HttpOnly")
            && set.contains("SameSite=Lax")
            && set.contains("Secure"),
        "{set}"
    );
    assert!(!cookie.is_empty());
    let (_, other, _) = begin(&d, "/").await;
    assert_ne!(state, other);
}

#[tokio::test]
async fn a_callback_that_is_not_this_browsers_live_sign_in_is_refused() {
    let (uri, forms) = token_endpoint(200, member_token().await).await;
    let d = on_site("refuse-cb", &uri).await;
    let (_, state, _) = begin(&d, "/").await;
    let without = get(&d, &call_back(&state), &[]).await;
    assert_eq!(without.status, 403, "no cookie");
    assert!(without.body.contains("/auth/login") && !without.body.contains("the-code"));
    let (_, state, cookie) = begin(&d, "/").await;
    let name = cookie.split('=').next().unwrap();
    let wrong = get(
        &d,
        &call_back(&state),
        &[("cookie", &format!("{name}=nope"))],
    )
    .await;
    assert_eq!(wrong.status, 403, "another cookie");
    let right = get(
        &d,
        "/auth/callback?error=x&state=zzz",
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(right.status, 403, "a cookie of another state");
    let (_, state, cookie2) = begin(&d, "/").await;
    assert_eq!(
        get(&d, &call_back("not-a-state"), &[("cookie", &cookie2)])
            .await
            .status,
        403
    );
    assert_eq!(
        get(&d, "/auth/callback?code=x&state=y", &[]).await.status,
        403
    );
    pending::age(&state);
    assert_eq!(
        get(&d, &call_back(&state), &[("cookie", &cookie2)])
            .await
            .status,
        403,
        "expired"
    );
    let (_, state, cookie3) = begin(&d, "/").await;
    let said = format!(
        "/auth/callback?error=access_denied&error_description=%3Cb%3Eno%3C/b%3E&state={state}"
    );
    let denied = get(&d, &said, &[("cookie", &cookie3)]).await;
    assert!(
        denied.status == 403
            && denied.body.contains("access_denied")
            && denied.body.contains("&lt;b&gt;no"),
        "{}",
        denied.body
    );
    assert_eq!(
        get(&d, &call_back(&state), &[("cookie", &cookie3)])
            .await
            .status,
        403,
        "used up by the error"
    );
    assert!(
        forms.lock().unwrap().is_empty(),
        "the provider was never asked"
    );
}

#[tokio::test]
async fn a_good_callback_trades_the_code_without_a_secret_and_opens_the_session() {
    let (uri, forms) = token_endpoint(200, member_token().await).await;
    let d = on_site("success", &uri).await;
    let (seen, state, cookie) = begin(&d, "/rooms?id=1").await;
    let challenge = Url::parse(&seen.location)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "code_challenge")
        .unwrap()
        .1
        .to_string();
    let done = get(&d, &call_back(&state), &[("cookie", &cookie)]).await;
    assert_eq!(
        (done.status, done.location.as_str()),
        (302, "/rooms?id=1"),
        "{}",
        done.body
    );
    let session = done
        .cookies
        .iter()
        .find(|c| c.starts_with("neobot-person-") && !c.contains("-login="))
        .expect("a session cookie");
    assert!(session.contains("HttpOnly") && session.contains("Secure"));
    assert!(
        done.cookies
            .iter()
            .any(|c| c.contains("-login=;") && c.contains("Max-Age=0")),
        "the flow's cookie ends"
    );
    let form = forms.lock().unwrap()[0].clone();
    let sent = Url::parse(&format!("http://x/?{form}")).unwrap();
    let f = |k: &str| {
        sent.query_pairs()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.to_string())
    };
    assert_eq!(
        (
            f("grant_type").as_deref(),
            f("code").as_deref(),
            f("client_id").as_deref()
        ),
        (Some("authorization_code"), Some("the-code"), Some(CLIENT))
    );
    assert_eq!(f("redirect_uri").unwrap(), format!("{SITE}/auth/callback"));
    assert!(f("scope").is_some() && f("client_secret").is_none());
    let verifier = f("code_verifier").unwrap();
    let want = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        Sha256::digest(verifier.as_bytes()),
    );
    assert_eq!(want, challenge);
    let me = d
        .call(
            "GET",
            "/api/auth/me",
            &[("cookie", session.split(';').next().unwrap())],
            Value::Null,
        )
        .await;
    assert!(
        me.1.contains("\"member\"") && me.1.contains("Ada Lovelace"),
        "{}",
        me.1
    );
    assert_eq!(
        get(&d, &call_back(&state), &[("cookie", &cookie)])
            .await
            .status,
        403,
        "once"
    );
    assert_eq!(forms.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn only_a_path_of_this_site_is_gone_back_to() {
    let (uri, _) = token_endpoint(200, member_token().await).await;
    let d = on_site("next", &uri).await;
    for next in [
        "//evil.example",
        "https://evil.example/x",
        "/\\evil.example",
        "evil",
        "",
        "/ok",
    ] {
        let (_, state, cookie) = begin(&d, next).await;
        let done = get(&d, &call_back(&state), &[("cookie", &cookie)]).await;
        let want = if next == "/ok" { "/ok" } else { "/" };
        assert_eq!((done.status, done.location.as_str()), (302, want), "{next}");
    }
}

#[tokio::test]
async fn a_provider_that_refuses_the_code_is_told_without_the_code() {
    let (uri, _) = token_endpoint(
        400,
        json!({"error": "invalid_grant", "error_description": "AADSTS70008: expired"}),
    )
    .await;
    let d = on_site("provider-no", &uri).await;
    let (_, state, cookie) = begin(&d, "/").await;
    let seen = get(&d, &call_back(&state), &[("cookie", &cookie)]).await;
    assert!(
        seen.status == 403 && seen.body.contains("AADSTS70008") && !seen.body.contains("the-code"),
        "{}",
        seen.body
    );
    assert!(
        seen.body.contains("Try again")
            && seen
                .cookies
                .iter()
                .all(|c| !c.starts_with("neobot-person-") || c.contains("Max-Age=0"))
    );
}
