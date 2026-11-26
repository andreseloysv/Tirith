// Team sign-in at the gate: a person's session, what each role may do, the cards, the refusals as events, and the
// page behind a reverse proxy. The daemon is the real router on a free port with a local JWKS.
use super::oidc_tests::{good, home, jwks, jwt};
use crate::app::Shared;
use serde_json::{json, Value};

struct Daemon {
    app: Shared,
    port: u16,
    http: reqwest::Client,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.app.home);
    }
}

async fn start(tag: &str, extra: &str) -> Daemon {
    let (uri, _) = jwks(&[("k1", 0)]).await;
    let app = home(tag, &uri, extra);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = crate::server::router(app.clone(), port);
    let door = crate::server::door(listener, app.clone());
    tokio::spawn(async move {
        drop(
            axum::serve(
                door,
                router.into_make_service_with_connect_info::<crate::server::Caller>(),
            )
            .await,
        )
    });
    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    Daemon { app, port, http }
}

impl Daemon {
    /// (status, body, set-cookie) of one call with these headers.
    async fn call(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Value,
    ) -> (u16, String, String) {
        let url = format!("http://127.0.0.1:{}{path}", self.port);
        let mut req = self
            .http
            .request(method.parse().unwrap(), url)
            .header("host", "127.0.0.1");
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        let res = req.json(&body).send().await.unwrap();
        let cookie = res
            .headers()
            .get("set-cookie")
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        (res.status().as_u16(), res.text().await.unwrap(), cookie)
    }

    /// Signs `roles` in with a token; the session cookie `name=value`.
    async fn sign_in(&self, roles: &[&str]) -> String {
        let mut claims = good(roles);
        claims["oid"] = json!(roles.join("+")); // a person per role set
        let token = jwt(0, "k1", "RS256", &claims);
        let (code, body, cookie) = self
            .call(
                "POST",
                "/api/auth/oidc",
                &[],
                json!({ "access_token": token }),
            )
            .await;
        assert_eq!(code, 200, "{body}");
        cookie.split(';').next().unwrap().to_string()
    }

    async fn status(&self, who: &str, method: &str, path: &str, body: Value) -> u16 {
        self.call(method, path, &[("cookie", who)], body).await.0
    }

    fn events(&self, tool: &str, like: &str) -> i64 {
        let sql =
            "SELECT COUNT(*) FROM events WHERE kind = 'auth' AND tool = ?1 AND summary LIKE ?2";
        self.app
            .db
            .lock()
            .unwrap()
            .query_row(sql, [tool, like], |r| r.get(0))
            .unwrap()
    }
}

#[tokio::test]
async fn a_person_signs_in_and_is_known_by_the_session() {
    let d = start("session", "").await;
    assert_eq!(
        d.call("GET", "/api/state", &[], Value::Null).await.0,
        403,
        "nobody without a session"
    );
    let (code, body, cookie) = d
        .call(
            "POST",
            "/api/auth/oidc",
            &[],
            json!({"access_token": jwt(0, "k1", "RS256", &good(&["Team.Member"]))}),
        )
        .await;
    assert_eq!(code, 200);
    let said: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (said["name"].as_str(), said["role"].as_str()),
        (Some("Ada Lovelace"), Some("member"))
    );
    assert!(
        cookie.contains("HttpOnly") && !cookie.contains("Secure"),
        "{cookie}"
    );
    let who = cookie.split(';').next().unwrap();
    let (_, me, _) = d
        .call("GET", "/api/auth/me", &[("cookie", who)], Value::Null)
        .await;
    assert!(
        me.contains("\"member\"") && me.contains("Ada Lovelace"),
        "{me}"
    );
    assert_eq!(
        d.status("neobot-person-0=abc", "GET", "/api/state", Value::Null)
            .await,
        403
    );
    // the link hands the same session to a window that cannot read the answer, once
    let link = said["link"].as_str().unwrap();
    let first = d.call("GET", link, &[], Value::Null).await;
    assert!(first.0 == 200 && first.2.contains(who), "{first:?}");
    assert_eq!(d.call("GET", link, &[], Value::Null).await.0, 403);
    assert_eq!(d.events("login", "%Ada Lovelace (member)%"), 1);
    d.call("POST", "/api/auth/logout", &[("cookie", who)], json!({}))
        .await;
    assert_eq!(
        d.status(who, "GET", "/api/state", Value::Null).await,
        403,
        "signed out"
    );
}

#[tokio::test]
async fn a_token_without_a_role_or_a_good_signature_signs_nobody_in() {
    let d = start("refuse", "").await;
    for roles in [vec![], vec!["Other.Role"]] {
        let token = jwt(0, "k1", "RS256", &good(&roles));
        let (code, body, cookie) = d
            .call(
                "POST",
                "/api/auth/oidc",
                &[],
                json!({"access_token": token}),
            )
            .await;
        assert!(
            code == 403 && cookie.is_empty() && body.contains("no NeoBot role"),
            "{code} {body}"
        );
    }
    let forged = jwt(1, "k1", "RS256", &good(&["Team.Admin"]));
    assert_eq!(
        d.call(
            "POST",
            "/api/auth/oidc",
            &[("authorization", &format!("Bearer {forged}"))],
            json!({})
        )
        .await
        .0,
        403
    );
    assert_eq!(d.events("refused", "sign-in refused%"), 3);
    let people: i64 = d
        .app
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM people", [], |r| r.get(0))
        .unwrap();
    assert_eq!(people, 0);
}

#[tokio::test]
async fn each_role_reaches_what_it_may_and_a_refusal_is_an_event() {
    let d = start(
        "roles",
        "[[projects]]\nid = \"p\"\nrepo = \"/nonexistent\"\n",
    )
    .await;
    let viewer = d.sign_in(&["Team.Viewer"]).await;
    assert_eq!(
        d.status(&viewer, "GET", "/api/state", Value::Null).await,
        200
    );
    assert_eq!(
        d.status(&viewer, "GET", "/api/costs", Value::Null).await,
        200
    );
    let ask = json!({"project": "p", "kind": "decision", "question": "ship it?"});
    assert_eq!(
        d.status(&viewer, "POST", "/api/owner", ask.clone()).await,
        403
    );
    assert_eq!(
        d.events(
            "refused",
            "Ada Lovelace (viewer) may not POST /api/owner%needs member%"
        ),
        1
    );
    let (_, body, _) = d
        .call("POST", "/api/owner", &[("cookie", &viewer)], ask.clone())
        .await;
    assert!(
        body.contains("your role (viewer)") && body.contains("needs member"),
        "{body}"
    );

    let member = d.sign_in(&["Team.Member"]).await;
    assert_ne!(d.status(&member, "POST", "/api/owner", ask).await, 403);
    assert_eq!(
        d.status(&member, "POST", "/api/bots", json!({})).await,
        403,
        "configuring is an admin's"
    );
    assert_eq!(
        d.status(&member, "POST", "/api/net/allow", json!({"host": "x.test"}))
            .await,
        403
    );
    assert_eq!(
        d.status(&member, "GET", "/api/secrets/b", Value::Null)
            .await,
        403
    );

    let admin = d.sign_in(&["Team.Admin"]).await;
    assert_ne!(d.status(&admin, "POST", "/api/bots", json!({})).await, 403);
    assert_ne!(
        d.status(&admin, "POST", "/api/net/allow", json!({"host": "x.test"}))
            .await,
        403
    );
    for owners in [
        ("GET", "/api/secrets/b"),
        ("POST", "/api/page"),
        ("POST", "/api/setup/agents/x"),
        ("POST", "/api/mcp/open"),
        ("POST", "/api/rooms/r/qr"),
    ] {
        let (code, body, _) = d
            .call(owners.0, owners.1, &[("cookie", &admin)], json!({}))
            .await;
        assert!(
            code == 403 && body.contains("the machine owner"),
            "{owners:?}: {code} {body}"
        );
    }
    assert!(d
        .events("action", "Ada Lovelace (admin): POST /api/bots")
        .ge(&1));
}

#[tokio::test]
async fn a_card_is_answered_by_the_role_its_kind_needs() {
    let d = start(
        "cards",
        "[[projects]]\nid = \"p\"\nrepo = \"/nonexistent\"\n",
    )
    .await;
    let card = |kind: &str| {
        crate::manager::ask_owner(&d.app, "p", kind, "q?", "").unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let (decision, approval, payment) = (card("decision"), card("approval"), card("payment"));
    let (member, admin) = (
        d.sign_in(&["Team.Member"]).await,
        d.sign_in(&["Team.Admin"]).await,
    );
    let answer = json!({"answer": "yes"});
    assert_eq!(
        d.status(
            &member,
            "POST",
            &format!("/api/owner/{approval}"),
            answer.clone()
        )
        .await,
        403,
        "an outward action is an admin's"
    );
    assert_eq!(
        d.status(
            &member,
            "POST",
            &format!("/api/owner/{payment}"),
            answer.clone()
        )
        .await,
        403
    );
    assert_eq!(
        d.status(
            &admin,
            "POST",
            &format!("/api/owner/{payment}"),
            answer.clone()
        )
        .await,
        403,
        "payment is the owner's alone"
    );
    assert_eq!(
        d.status(
            &member,
            "POST",
            &format!("/api/owner/{decision}"),
            answer.clone()
        )
        .await,
        200
    );
    assert_eq!(
        d.status(&admin, "POST", &format!("/api/owner/{approval}"), answer)
            .await,
        200
    );
    let open: i64 = d
        .app
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM owner_items WHERE status = 'open'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(open, 1, "only the payment card is still open");
}

/// What `GET /api/auth/oidc` answers to a request with exactly these headers.
async fn answer(d: &Daemon, headers: &[(&str, &str)]) -> String {
    let url = format!("http://127.0.0.1:{}/api/auth/oidc", d.port);
    let mut req = d.http.get(url);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    req.send().await.unwrap().text().await.unwrap()
}

#[tokio::test]
async fn behind_a_proxy_the_public_host_is_let_in_and_nothing_else() {
    let foreign = "foreign origin";
    let plain = start("proxy", "").await;
    assert_eq!(
        answer(&plain, &[("host", "neobot.team.example")]).await,
        foreign
    );
    let forwarded = [
        ("host", "other.example"),
        ("x-forwarded-host", "neobot.team.example"),
    ];
    assert_eq!(
        answer(&plain, &forwarded).await,
        foreign,
        "no public_url: X-Forwarded-* is not read"
    );

    let d = start(
        "proxy2",
        "[server]\npublic_url = \"https://neobot.team.example/\"\n",
    )
    .await;
    let site = "https://neobot.team.example";
    assert_ne!(
        answer(&d, &[("host", "neobot.team.example"), ("origin", site)]).await,
        foreign
    );
    assert_ne!(
        answer(
            &d,
            &[
                ("host", "127.0.0.1"),
                ("origin", site),
                ("x-forwarded-host", "neobot.team.example")
            ]
        )
        .await,
        foreign
    );
    assert_ne!(
        answer(&d, &forwarded).await,
        foreign,
        "the proxy names the public host"
    );
    assert_eq!(answer(&d, &[("host", "evil.example")]).await, foreign);
    assert_eq!(
        answer(
            &d,
            &[
                ("host", "neobot.team.example"),
                ("origin", "https://evil.example")
            ]
        )
        .await,
        foreign
    );
    assert_eq!(
        answer(
            &d,
            &[
                ("host", "neobot.team.example"),
                ("origin", "http://neobot.team.example")
            ]
        )
        .await,
        foreign
    );
    assert_eq!(
        answer(
            &d,
            &[
                ("host", "127.0.0.1"),
                ("origin", "https://evil.example"),
                ("x-forwarded-host", "evil.example")
            ]
        )
        .await,
        foreign
    );
    let token = jwt(0, "k1", "RS256", &good(&["Team.Viewer"]));
    let (_, _, cookie) = d
        .call(
            "POST",
            "/api/auth/oidc",
            &[],
            json!({"access_token": token}),
        )
        .await;
    assert!(
        cookie.contains("; Secure"),
        "over https the cookie is Secure: {cookie}"
    );
}
