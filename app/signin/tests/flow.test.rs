// The sign-in against a fake provider and a fake team server on local ports: the authorization URL carries PKCE and
// a loopback redirect, the token endpoint checks the verifier, and the server gets the access token, nothing else.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tirith_signin::{pkce, sign_in};

#[derive(Default)]
struct Seen {
    authorize: Vec<(String, String)>,
    challenge: String,
    token_form: String,
    bearer: String,
}

fn serve(seen: Arc<Mutex<Seen>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let own = base.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let (seen, base) = (seen.clone(), own.clone());
            std::thread::spawn(move || handle(conn, &seen, &base));
        }
    });
    base
}

fn handle(mut conn: TcpStream, seen: &Mutex<Seen>, base: &str) {
    let mut buf = vec![0u8; 8192];
    let n = conn.read(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    let target = text.split_whitespace().nth(1).unwrap().to_string();
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    let url = reqwest::Url::parse(&format!("http://x{target}")).unwrap();
    let mut seen = seen.lock().unwrap();
    let (status, head, out) = match url.path() {
        "/api/auth/oidc" if text.starts_with("GET") => {
            let t = format!("{base}/tenant/oauth2/v2.0");
            let cfg = format!(
                r#"{{"issuer":"{base}/tenant/v2.0","client_id":"cid","scope":"api://cid/access_as_user","endpoints":{{"authorization_endpoint":"{t}/authorize","token_endpoint":"{t}/token"}}}}"#
            );
            ("200 OK", String::new(), cfg)
        }
        "/api/auth/oidc" => {
            let auth = text
                .lines()
                .find(|l| l.to_lowercase().starts_with("authorization:"))
                .unwrap_or("");
            seen.bearer = auth.split(": ").nth(1).unwrap_or("").to_string();
            (
                "200 OK",
                String::new(),
                r#"{"name":"Ada","role":"admin","link":"/auth/session/abc"}"#.into(),
            )
        }
        "/tenant/oauth2/v2.0/authorize" => {
            seen.authorize = url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let pairs = seen.authorize.clone();
            let q = |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            };
            seen.challenge = q("code_challenge");
            let back = format!(
                "Location: {}/?code=the-code&state={}\r\n",
                q("redirect_uri"),
                q("state")
            );
            ("302 Found", back, String::new())
        }
        _ => {
            seen.token_form = body.clone();
            let verifier = body
                .split('&')
                .find_map(|p| p.strip_prefix("code_verifier="))
                .unwrap_or("");
            let ok = pkce::challenge(verifier) == seen.challenge && body.contains("code=the-code");
            (
                "200 OK",
                String::new(),
                if ok {
                    r#"{"access_token":"tok-1"}"#
                } else {
                    r#"{"error":"bad verifier"}"#
                }
                .into(),
            )
        }
    };
    let said = format!("HTTP/1.1 {status}\r\n{head}Content-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{out}", out.len());
    let _ = conn.write_all(said.as_bytes());
}

fn browser(url: &str) -> Result<(), String> {
    let url = url.to_string();
    std::thread::spawn(move || {
        let none = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let back = none.get(&url).send().unwrap();
        let to = back.headers()["location"].to_str().unwrap().to_string();
        reqwest::blocking::get(to).unwrap(); // the browser follows the redirect to the app's loopback port
    });
    Ok(())
}

#[test]
fn the_app_signs_in_with_pkce_and_hands_over_the_access_token() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let base = serve(seen.clone());
    let session = sign_in(&base, &browser, Duration::from_secs(10)).unwrap();
    assert_eq!(
        (
            session.name.as_str(),
            session.role.as_str(),
            session.link.as_str()
        ),
        ("Ada", "admin", "/auth/session/abc")
    );
    let seen = seen.lock().unwrap();
    let q = |k: &str| {
        seen.authorize
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    assert_eq!(
        (
            q("client_id"),
            q("response_type"),
            q("code_challenge_method")
        ),
        ("cid", "code", "S256")
    );
    assert_eq!(q("scope"), "api://cid/access_as_user");
    assert!(
        !q("scope").contains("offline_access"),
        "no refresh token is asked for"
    );
    assert!(
        q("redirect_uri").starts_with("http://localhost:")
            && q("state").len() >= 24
            && q("code_challenge").len() == 43
    );
    assert!(
        seen.token_form.contains("grant_type=authorization_code")
            && !seen.token_form.contains("client_secret")
    );
    assert_eq!(
        seen.bearer, "Bearer tok-1",
        "the server gets the access token, only"
    );
}

#[test]
fn an_endpoint_that_is_not_the_issuers_or_not_https_is_refused() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for mut c in l.incoming().flatten() {
            let cfg = r#"{"issuer":"https://login.example/t/v2.0","client_id":"c","scope":"s","endpoints":{"authorization_endpoint":"http://evil.example/a","token_endpoint":"https://login.example/t"}}"#;
            let _ = c.read(&mut [0u8; 2048]);
            let _ = c.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cfg}",
                    cfg.len()
                )
                .as_bytes(),
            );
        }
    });
    let opened = |_: &str| -> Result<(), String> { panic!("the browser must not open") };
    assert!(sign_in(&base, &opened, Duration::from_secs(1)).is_err());
}
