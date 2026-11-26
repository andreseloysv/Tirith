// Team sign-in: a token is accepted only when it is the provider's and says what this NeoBot takes. The keys are a
// local RSA key whose JWKS a local server hands out; nothing reaches the network.
use crate::app::{App, Shared};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

pub const ISSUER: &str = "https://login.example.test/tenant-1/v2.0";
pub const CLIENT: &str = "client-1";
/// Slow to make in a debug build: made once for every check.
pub static KEYS: LazyLock<[RsaPrivateKey; 2]> = LazyLock::new(|| {
    let make = || RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
    [make(), make()]
});

pub fn now() -> u64 {
    crate::app::now_s()
}

/// A JWT of `claims` signed by key `which` under key id `kid`; `alg` is what its header says.
pub fn jwt(which: usize, kid: &str, alg: &str, claims: &Value) -> String {
    let part = |v: &Value| B64.encode(serde_json::to_vec(v).unwrap());
    let signed = format!(
        "{}.{}",
        part(&json!({"alg": alg, "kid": kid, "typ": "JWT"})),
        part(claims)
    );
    let sig = SigningKey::<sha2::Sha256>::new(KEYS[which].clone()).sign(signed.as_bytes());
    format!("{signed}.{}", B64.encode(sig.to_bytes()))
}

/// What a good token of an admin says; a check changes one thing.
pub fn good(roles: &[&str]) -> Value {
    json!({"iss": ISSUER, "aud": CLIENT, "exp": now() + 3600, "nbf": now() - 10, "tid": "tenant-1",
        "oid": "oid-1", "name": "Ada Lovelace", "preferred_username": "ada@example.test",
        "scp": "access_as_user", "roles": roles})
}

/// A server that answers every request with the JWKS of these keys (`(kid, key index)`), and counts them.
pub async fn jwks(keys: &[(&str, usize)]) -> (String, Arc<AtomicUsize>) {
    let list: Vec<Value> = keys.iter().map(|(kid, i)| {
        let k = &KEYS[*i];
        json!({"kty": "RSA", "kid": kid, "n": B64.encode(k.n().to_bytes_be()), "e": B64.encode(k.e().to_bytes_be())})
    }).collect();
    let body = json!({ "keys": list }).to_string();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let uri = format!(
        "http://127.0.0.1:{}/keys",
        listener.local_addr().unwrap().port()
    );
    let hits = Arc::new(AtomicUsize::new(0));
    let seen = hits.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                return;
            };
            seen.fetch_add(1, Ordering::SeqCst);
            let said = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let _ = s.read(&mut [0u8; 2048]).await;
                let _ = s.write_all(said.as_bytes()).await;
            });
        }
    });
    (uri, hits)
}

/// A home whose config.toml says `[auth.oidc]` with this JWKS address, plus `extra`.
pub fn home(tag: &str, uri: &str, extra: &str) -> Shared {
    let home = std::env::temp_dir().join(format!("tirith-oidc-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let text = format!("[network]\nsandbox = false\n[auth.oidc]\nissuer = \"{ISSUER}\"\nclient_id = \"{CLIENT}\"\njwks_uri = \"{uri}\"\n{extra}");
    std::fs::write(home.join("config.toml"), text).unwrap();
    App::open_at(home)
}

async fn verdict(app: &Shared, token: &str) -> Result<String, String> {
    let s = super::Settings::of(app).unwrap();
    super::verify::check(&s, token)
        .await
        .map(|c| format!("{}:{}", c.oid, c.level.name()))
}

#[tokio::test]
async fn a_good_token_is_accepted_with_the_highest_role() {
    let (uri, _) = jwks(&[("k1", 0)]).await;
    let app = home("good", &uri, "");
    let token = jwt(0, "k1", "RS256", &good(&["Team.Viewer", "Team.Admin"]));
    assert_eq!(verdict(&app, &token).await, Ok("oid-1:admin".into()));
    let mut by_api = good(&["Team.Member"]);
    by_api["aud"] = json!(["other", format!("api://{CLIENT}")]); // the api:// form, in a list
    by_api["exp"] = json!(now() - 30); // inside the leeway
    assert_eq!(
        verdict(&app, &jwt(0, "k1", "RS256", &by_api)).await,
        Ok("oid-1:member".into())
    );
}

#[tokio::test]
async fn a_token_that_says_the_wrong_thing_is_refused() {
    let (uri, _) = jwks(&[("k1", 0)]).await;
    let app = home(
        "wrong",
        &uri,
        "[auth.oidc.roles]\n\"Team.Admin\" = \"admin\"\n",
    );
    let bad = |key: &str, value: Value, why: &str| {
        let mut c = good(&["Team.Admin"]);
        c[key] = value;
        (jwt(0, "k1", "RS256", &c), why.to_string())
    };
    let cases = [
        bad(
            "iss",
            json!("https://evil.example/tenant-1/v2.0"),
            "wrong issuer",
        ),
        bad("aud", json!("someone-else"), "another audience"),
        bad("exp", json!(now() - 120), "expired"),
        bad("nbf", json!(now() + 600), "not valid yet"),
        bad("tid", json!("tenant-2"), "another tenant"),
        bad("scp", json!("something_else"), "lacks the scope"),
        bad("roles", json!([]), "no NeoBot role"),
        bad("roles", json!(["Team.Member"]), "no NeoBot role"), // not in this config's roles
        bad("oid", json!(""), "names no user"),
    ];
    for (token, why) in cases {
        let got = verdict(&app, &token).await;
        assert!(
            got.as_ref().is_err_and(|e| e.contains(&why)),
            "{why}: {got:?}"
        );
    }
}

#[tokio::test]
async fn a_token_that_is_not_the_providers_is_refused() {
    let (uri, hits) = jwks(&[("k1", 0)]).await;
    let app = home("forged", &uri, "");
    let c = good(&["Team.Admin"]);
    assert!(verdict(&app, &jwt(0, "k1", "RS256", &c)).await.is_ok());
    let wrong_key = verdict(&app, &jwt(1, "k1", "RS256", &c)).await;
    assert!(wrong_key.is_err_and(|e| e.contains("signature")));
    let hs = verdict(&app, &jwt(0, "k1", "HS256", &c)).await;
    assert!(hs.is_err_and(|e| e.contains("RS256")));
    let none = format!(
        "{}.{}.",
        B64.encode(br#"{"alg":"none"}"#),
        B64.encode(c.to_string())
    );
    assert!(verdict(&app, &none).await.is_err());
    let good_token = jwt(0, "k1", "RS256", &c);
    let mut parts: Vec<String> = good_token.split('.').map(String::from).collect();
    parts[1] = B64.encode(good(&["Team.Admin", "Team.Member"]).to_string()); // another body, the old signature
    assert!(verdict(&app, &parts.join("."))
        .await
        .is_err_and(|e| e.contains("signature")));
    assert!(verdict(&app, "not.a.jwt").await.is_err());
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the keys are kept, not fetched per token"
    );
}

#[tokio::test]
async fn the_keys_are_fetched_again_when_the_key_id_changes() {
    let (uri, hits) = jwks(&[("k1", 0), ("k2", 1)]).await;
    let app = home("roll", &uri, "");
    let c = good(&["Team.Viewer"]);
    assert!(verdict(&app, &jwt(0, "k1", "RS256", &c)).await.is_ok());
    // a key id nobody has: refused, and the provider is not asked again so soon
    assert!(verdict(&app, &jwt(0, "k9", "RS256", &c)).await.is_err());
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(verdict(&app, &jwt(0, "k9", "RS256", &c)).await.is_err());
    assert_eq!(
        hits.load(Ordering::SeqCst),
        2,
        "a new key id, later: fetched again"
    );
}
