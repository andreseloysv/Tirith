// Who calls is known by a token alone (server/admit.rs). An agent here is plain HTTP with the token wrap gave it: nothing is confined or started.
use super::testing::{token, HEADER};
use crate::app::{App, Shared};
use serde_json::{json, Value};

const PROJECT: &str = "apitoken";
const CARD: &str = "o901";

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

impl Daemon {
    async fn start(tag: &str) -> Daemon {
        let home = std::env::temp_dir().join(format!("tirith-api-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("repo")).unwrap();
        let repo = home.join("repo");
        let config = format!(
            "[network]\nsandbox = false\n[[projects]]\nid = \"{PROJECT}\"\nrepo = \"{}\"\n",
            repo.display()
        );
        std::fs::write(home.join("config.toml"), config).unwrap();
        let app = App::open_at(home);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = super::router(app.clone(), port);
        tokio::spawn(async move { drop(axum::serve(listener, router).await) });
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        Daemon { app, port, http }
    }

    /// One call with exactly these headers: the status, the body and the cookie it set.
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
            .header("host", "127.0.0.1"); // the guard keeps the first test's port
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        if method != "GET" {
            req = req.json(&body);
        }
        let res = req.send().await.unwrap();
        let cookie = res
            .headers()
            .get("set-cookie")
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        let code = res.status().as_u16();
        (code, res.text().await.unwrap(), cookie)
    }

    async fn code(&self, method: &str, path: &str, headers: &[(&str, &str)], body: Value) -> u16 {
        self.call(method, path, headers, body).await.0
    }

    fn lane(&self, id: &str) -> String {
        self.app.save_lane(&crate::app::Lane {
            id: id.into(),
            project: PROJECT.into(),
            status: "running".into(),
            workspace: self.app.home.join("ws").display().to_string(),
            ..Default::default()
        });
        // The token as the lane's process gets it.
        crate::sandbox::wrap(&self.app, id, vec!["true".into()])
            .env
            .into_iter()
            .find(|(name, _)| name == "TIRITH_TOKEN")
            .expect("wrap gives TIRITH_TOKEN")
            .1
    }

    fn card(&self) {
        self.app.insert(
            "INSERT INTO owner_items (id, at, kind, question, status, project) VALUES (?1, '2026-01-02T00:00:00Z', 'approval', 'Send it?', 'open', ?2)",
            &[&CARD, &PROJECT],
        );
    }

    fn count(&self, sql: &str) -> i64 {
        self.app.query(sql, [], |r| r.get::<_, i64>(0))[0]
    }
}

fn identity(app: &Shared, token: &str) -> super::who::Agent {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(HEADER, token.parse().unwrap());
    super::identify::agent(app, &headers).expect("a valid token")
}

#[tokio::test]
async fn api_rejects_a_spoofed_role() {
    let d = Daemon::start("spoof").await;
    let mine = d.lane("lane-mine");
    d.lane("lane-other");
    d.card();
    let manager = token(&d.app, &format!("manager@{PROJECT}"));

    // Everything a lane could say of itself to pass for the manager or the owner.
    for claimed in ["manager", "owner", ""] {
        let spoofed = [
            (HEADER, mine.as_str()),
            ("x-tirith-role", claimed),
            ("x-tirith-lane", ""),
            ("x-tirith-confined", ""),
        ];
        for (method, path) in [
            ("POST", "/api/page".to_string()),
            ("POST", "/api/rooms/r1/qr".to_string()),
            ("GET", "/api/rooms/r1/qr.svg".to_string()),
            ("DELETE", "/api/net/blocks".to_string()),
            ("POST", "/api/turns/1/reaction".to_string()),
            ("POST", "/api/teach/b1/events".to_string()),
            ("POST", "/api/owner".to_string()),
            ("POST", "/api/chat/clear".to_string()),
            ("POST", "/api/lanes/lane-other/answer".to_string()),
            ("POST", "/api/lanes/lane-other/ask".to_string()),
        ] {
            let code = d.code(method, &path, &spoofed, json!({})).await;
            assert_eq!(code, 403, "a lane as {claimed:?}: {method} {path}");
        }
        // No script is reachable with this command, also for a role that may drive Safari.
        let safari = json!({"cmd": "none"});
        assert_eq!(d.code("POST", "/api/safari", &spoofed, safari).await, 403);
        let yes = json!({"answer": "yes", "by": "owner"});
        let path = format!("/api/owner/{CARD}");
        assert_eq!(d.code("POST", &path, &spoofed, yes).await, 400, "{claimed}");
        let say = json!({"text": format!("said as {claimed:?}"), "project": PROJECT});
        assert_eq!(d.code("POST", "/api/chat", &spoofed, say).await, 200);
    }
    let said = d.app.query(
        "SELECT role FROM chat WHERE text LIKE 'said as %'",
        [],
        |r| r.get::<_, String>(0),
    );
    assert_eq!(
        said, ["lane-mine"; 3],
        "its words go under its token's name"
    );
    assert_eq!(
        d.count("SELECT COUNT(*) FROM events WHERE kind = 'owner.message'"),
        0
    );
    let open = format!("SELECT COUNT(*) FROM owner_items WHERE id = '{CARD}' AND status = 'open'");
    assert_eq!(d.count(&open), 1, "the owner's yes was not given");

    // The hook takes the role and the lane from the token, whatever the headers say.
    let wait = json!({"cwd": d.app.home.join("repo"), "session_id": "s1", "tool_name": "Bash", "tool_input": {"command": "sleep 30"}});
    let as_manager = [(HEADER, mine.as_str()), ("x-tirith-role", "manager")];
    let (_, lanes, _) = d
        .call("POST", "/hook/PreToolUse", &as_manager, wait.clone())
        .await;
    assert!(lanes.contains("\"allow\""), "a lane stays a lane: {lanes}");
    let as_lane = [(HEADER, manager.as_str()), ("x-tirith-lane", "lane-mine")];
    let (_, managers, _) = d.call("POST", "/hook/PreToolUse", &as_lane, wait).await;
    assert!(
        managers.contains("\"deny\""),
        "a manager is one: {managers}"
    );

    let lane = identity(&d.app, &mine);
    assert_eq!(
        (lane.lane.as_str(), lane.project.as_str()),
        ("lane-mine", PROJECT)
    );
    let chat = identity(&d.app, &manager);
    assert_eq!(
        (chat.role.as_str(), chat.project.as_str()),
        ("manager", PROJECT)
    );
    assert!(chat.lane.is_empty() && chat.bot.is_empty());
    let saved = crate::bots::save(&d.app, json!({"name": "Correo", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let bot = format!("bot-{}", saved["id"].as_str().unwrap());
    let bots = token(&d.app, &format!("manager@{bot}"));
    assert_eq!(identity(&d.app, &bots).bot, bot);
    let approve = format!("/api/teach/{}/approve", saved["id"].as_str().unwrap());
    let posed = [(HEADER, bots.as_str()), ("x-tirith-role", "owner")];
    assert_eq!(
        d.code("POST", &approve, &posed, json!({"skill": "x"}))
            .await,
        403
    );
    let rehearse = format!("/api/teach/{}/rehearse", saved["id"].as_str().unwrap());
    let other = [(HEADER, manager.as_str()), ("x-tirith-role", bot.as_str())];
    assert_eq!(
        d.code("POST", &rehearse, &other, json!({})).await,
        403,
        "not that bot"
    );

    // The database is writable inside the sandbox: a row there that calls a lane the manager proves nothing.
    let row = format!("caller:{}", crate::rooms::sealed::sha_hex(mine.as_bytes()));
    d.app.set_kv(&row, Some("manager@"));
    let forged = [(HEADER, mine.as_str())];
    assert_eq!(d.code("GET", "/api/state", &forged, Value::Null).await, 403);

    // The owner's path still works, and no header turns them into an agent.
    let owner = [
        (HEADER, d.app.keys.owner.as_str()),
        ("x-tirith-role", "manager"),
    ];
    let yes = json!({"answer": "yes"});
    let path = format!("/api/owner/{CARD}");
    assert_eq!(d.code("POST", &path, &owner, yes).await, 200);
    let answer = format!("SELECT COUNT(*) FROM owner_items WHERE id = '{CARD}' AND answer = 'yes'");
    assert_eq!(d.count(&answer), 1);
    // The answer also stands in the chat as the owner's own message.
    assert_eq!(
        d.count("SELECT COUNT(*) FROM chat WHERE role = 'owner' AND text = 'yes'"),
        1
    );
}

#[tokio::test]
async fn api_rejects_calls_without_token() {
    let d = Daemon::start("tokenless").await;
    d.lane("lane-mine");
    let teach = crate::bots::save(&d.app, json!({"name": "Aprende", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let bot = teach["id"].as_str().unwrap();

    let nobody: [&[(&str, &str)]; 6] = [
        &[],
        &[("x-tirith-role", "manager")],
        &[("x-tirith-role", "owner"), ("x-tirith-lane", "lane-mine")],
        &[(HEADER, "$TIRITH_TOKEN")],
        &[(HEADER, "0123456789abcdef0123456789abcdef")],
        &[("cookie", "tirith-00000000=owner")],
    ];
    let routes = [
        ("GET", "/".to_string()),
        ("GET", "/js/app.js".to_string()),
        ("GET", "/teach-extension.zip".to_string()),
        ("GET", "/api/state".to_string()),
        ("GET", "/api/stream".to_string()),
        ("GET", "/api/rooms".to_string()),
        ("GET", "/api/setup".to_string()),
        ("GET", format!("/api/teach/{bot}")),
        ("POST", format!("/api/teach/{bot}/start")),
        ("POST", "/api/chat".to_string()),
        ("POST", "/api/page".to_string()),
        ("POST", "/api/lanes/lane-mine/ask".to_string()),
        ("POST", "/api/safari".to_string()),
        ("POST", "/api/%6Fwner/o1".to_string()),
        ("POST", "/hook/PreToolUse/more".to_string()),
        ("GET", "/hook/PreToolUse".to_string()),
    ];
    for headers in nobody {
        for (method, path) in &routes {
            let code = d.code(method, path, headers, json!({"text": "hi"})).await;
            assert_eq!(code, 403, "{method} {path} with {headers:?}");
        }
    }

    // A tokenless hook can only be refused: it is no lane, though it names one and stands in its folder.
    let hook = |event: &str, command: &str| {
        let body = json!({"cwd": d.app.home.join("ws"), "session_id": "forged", "tool_name": "Bash", "tool_input": {"command": command}, "last_assistant_message": "forged?"});
        (format!("/hook/{event}"), body)
    };
    let named = [("x-tirith-lane", "lane-mine")];
    for event in ["PreToolUse", "PostToolUse", "UserPromptSubmit", "Stop"] {
        let (path, body) = hook(event, "ls");
        let (code, said, _) = d.call("POST", &path, &named, body).await;
        assert_eq!((code, said.as_str()), (200, "{}"), "{event}");
    }
    let (path, body) = hook("PreToolUse", "git commit --no-verify -m x");
    let (_, refused, _) = d.call("POST", &path, &[], body).await;
    assert!(refused.contains("\"deny\""), "refusals stay: {refused}");
    let lane = d.app.lane("lane-mine").unwrap();
    assert!(
        lane.session.is_empty() && lane.note.is_empty(),
        "nothing of the lane was written"
    );
    let written = "SELECT COUNT(*) FROM events WHERE kind IN ('PreToolUse', 'Stop', 'access')";
    assert_eq!(d.count(written), 0);

    // The owner's token is a private file, and the secret behind the agents' has left the database.
    use std::os::unix::fs::PermissionsExt;
    let file = d.app.home.join(crate::app::OWNER);
    assert_eq!(
        std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), d.app.keys.owner);
    assert!(d.app.kv("net_secret").is_none());
    let owner = [(HEADER, d.app.keys.owner.as_str())];
    assert_eq!(d.code("GET", "/api/state", &owner, Value::Null).await, 200);

    // The page opens by a link that works once, and is then the owner by its cookie.
    let (code, made, _) = d.call("POST", "/api/page", &owner, json!({})).await;
    assert_eq!(code, 200, "{made}");
    let url = serde_json::from_str::<Value>(&made).unwrap()["url"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        !url.contains(&d.app.keys.owner),
        "the link is not the token"
    );
    let link = &url[url.find("/auth/").unwrap()..];
    let (code, _, set) = d.call("GET", link, &[], Value::Null).await;
    assert!(code == 200 && set.contains("HttpOnly"), "{code} {set}");
    assert_eq!(d.code("GET", link, &[], Value::Null).await, 403, "once");
    let cookie = set.split(';').next().unwrap();
    let page = [("cookie", cookie)];
    for path in ["/", "/js/app.js", "/api/state"] {
        assert_eq!(d.code("GET", path, &page, Value::Null).await, 200, "{path}");
    }
    for (name, value) in [
        ("sec-fetch-site", "same-site"),
        ("sec-fetch-dest", "document"),
    ] {
        let visited = [("cookie", cookie), (name, value)];
        assert_eq!(
            d.code("GET", "/api/state", &visited, Value::Null).await,
            403,
            "{name}"
        );
    }

    // The extension's own token opens its routes alone, with or without an origin.
    let path = format!("/api/teach/{bot}");
    let (_, shown, _) = d.call("GET", &path, &page, Value::Null).await;
    let bearer = format!(
        "Bearer {}",
        serde_json::from_str::<Value>(&shown).unwrap()["token"]
            .as_str()
            .unwrap()
    );
    let extension = [("authorization", bearer.as_str())];
    let (code, seen, _) = d.call("GET", &path, &extension, Value::Null).await;
    assert!(code == 200 && !seen.contains("\"token\""), "{code} {seen}");
    assert_eq!(
        d.code("GET", "/api/state", &extension, Value::Null).await,
        403
    );
}
