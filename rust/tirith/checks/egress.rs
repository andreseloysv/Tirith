// The allowlists: `net` of a role, `net = "ask"`, `[network] allow`, and how a yes or `neobot net permit` writes them.
use super::{allow, denied, settled};
use crate::app::Shared;

fn app(name: &str, config: &str) -> Shared {
    let home = std::env::temp_dir().join(format!("tirith-egress-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!("[network]\nsandbox = false\n{config}"),
    )
    .unwrap();
    crate::app::App::open_at(home)
}

const ROLES: &str = "[roles.bot-a]\nextends = \"manager\"\nscope = \"own\"\nnet = [\"mail.google.com\", \"*.googleusercontent.com\"]\n\n[roles.bot-b]\nextends = \"manager\"\nscope = \"own\"\nnet = \"ask\"\n";

#[test]
fn bot_net_allow_list() {
    let app = app("bot", "");
    let none = |who: &str, host: &str| denied(&app, who, host).is_none();
    std::fs::write(
        app.home.join("config.toml"),
        format!("[network]\nsandbox = false\n\n{ROLES}"),
    )
    .unwrap();
    app.reload_config(true);
    assert!(none("bot-a@bot-a", "mail.google.com"));
    assert!(none("bot-a@bot-a", "lh3.googleusercontent.com"));
    assert!(
        none("bot-a@bot-a", "localhost"),
        "its own machine is never held back"
    );
    let no = denied(&app, "bot-a@bot-a", "googleusercontent.com").expect("a wildcard is no apex");
    assert!(
        no.body
            .contains("neobot net permit googleusercontent.com --role bot-a"),
        "{}",
        no.body
    );
    assert!(denied(&app, "bot-a@bot-a", "api.stripe.com").is_some());
    assert!(
        none("bot-c@bot-c", "api.stripe.com"),
        "a role with no net is as before"
    );
    assert!(none("manager@demo", "api.stripe.com"));
    // ask: the first new destination opens one card, and a yes puts it on the list
    let first = denied(&app, "bot-b@bot-b", "api.stripe.com").unwrap();
    assert!(
        first.body.contains("The owner was asked (card"),
        "{}",
        first.body
    );
    denied(&app, "bot-b@bot-b", "api.stripe.com").unwrap();
    let open = |a: &Shared| {
        a.query(
            "SELECT COUNT(*) FROM owner_items WHERE status = 'open'",
            [],
            |r| r.get::<_, i64>(0),
        )
    };
    assert_eq!(open(&app), [1], "asking twice opens one card");
    let card = app.query("SELECT card FROM connector_permits", [], |r| {
        r.get::<_, String>(0)
    })[0]
        .clone();
    app.insert(
        "UPDATE connector_permits SET status = 'granted' WHERE card = ?1",
        &[&card],
    );
    settled(&app, &card);
    assert!(none("bot-b@bot-b", "api.stripe.com"));
    assert!(
        denied(&app, "bot-b@bot-b", "api.other.com").is_some(),
        "ask stays on for the others"
    );
    let text = std::fs::read_to_string(app.home.join("config.toml")).unwrap();
    assert!(
        text.contains("\"ask\"") && text.contains("api.stripe.com"),
        "{text}"
    );
}

#[test]
fn the_machines_allow_list_is_the_ceiling() {
    let app = app("home", "allow = [\"*.example.com\", \"10.1.0.0/16\"]\n");
    let ok = |host: &str| denied(&app, "demo-1-lane", host).is_none();
    assert!(ok("api.example.com") && ok("10.1.2.3") && ok("127.0.0.1"));
    let no = denied(&app, "merge:demo-1-lane", "example.com").unwrap();
    assert!(
        no.body.contains("neobot net permit example.com"),
        "{}",
        no.body
    );
    assert!(denied(&app, "git", "github.com").is_some() && !ok("10.2.0.1"));
    allow(&app, None, "github.com").unwrap();
    assert!(denied(&app, "git", "github.com").is_none());
}

/// One CONNECT through the proxy as `who`: the first line of its answer.
async fn connect(app: &Shared, who: &str, to: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let url = crate::sandbox::proxy_url(app, who);
    let (creds, addr) = url
        .strip_prefix("http://")
        .unwrap()
        .rsplit_once('@')
        .unwrap();
    use base64::Engine;
    let auth = base64::engine::general_purpose::STANDARD.encode(creds);
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    let head =
        format!("CONNECT {to} HTTP/1.1\r\nHost: {to}\r\nProxy-Authorization: Basic {auth}\r\n\r\n");
    s.write_all(head.as_bytes()).await.unwrap();
    let mut buf = vec![0u8; 512];
    let n = s.read(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

#[tokio::test]
async fn the_proxy_refuses_what_is_off_the_list_and_says_how_to_allow_it() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let app = app(
        "proxy",
        &format!("proxy_port = {port}\nallow = [\"only.example.com\"]\n"),
    );
    tokio::spawn(crate::sandbox::serve_proxy(app.clone()));
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let answer = connect(&app, "demo-1-lane", "other.example.org:443").await;
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert!(
        answer.contains("X-Tirith-Blocked: not on [network] allow")
            && answer.contains("neobot net permit other.example.org"),
        "{answer}"
    );
    let seen = app.query(
        "SELECT kind, summary FROM events WHERE kind = 'net-blocked'",
        [],
        |r| r.get::<_, String>(1),
    );
    assert_eq!(seen, ["other.example.org:443"], "the refusal is an event");
    let local = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let to = format!("127.0.0.1:{}", local.local_addr().unwrap().port());
    assert!(
        connect(&app, "demo-1-lane", &to)
            .await
            .starts_with("HTTP/1.1 200"),
        "the machine's own loopback is open"
    );
}
