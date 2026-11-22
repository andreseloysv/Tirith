// A bot's secrets (src/secrets/): a fake OS store in place of the owner's own, and no sandbox.
use super::backend::Backend;
use super::cli::{parsed, Command};
use super::store::Store;
use crate::app::{App, Shared};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Mutex;

pub(super) struct Fake {
    kept: Mutex<BTreeMap<(String, String), String>>,
    /// Whether the store exists.
    up: bool,
    /// Why every call fails, if it does.
    refuses: Option<&'static str>,
}

pub(super) static FAKE: Fake = Fake::new(true, None);

impl Fake {
    const fn new(up: bool, refuses: Option<&'static str>) -> Fake {
        Fake {
            kept: Mutex::new(BTreeMap::new()),
            up,
            refuses,
        }
    }

    fn works(&self) -> Result<(), String> {
        self.refuses.map_or(Ok(()), |why| Err(why.into()))
    }

    /// The values of one bot, as the store holds them.
    fn of(&self, bot: &str) -> Vec<String> {
        let service = format!("tirith.bot.{bot}");
        let kept = self.kept.lock().unwrap();
        let mine = kept.iter().filter(|((s, _), _)| *s == service);
        mine.map(|(_, value)| value.clone()).collect()
    }
}

impl Backend for Fake {
    fn available(&self) -> bool {
        self.up
    }

    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), String> {
        self.works()?;
        let at = (service.to_string(), account.to_string());
        self.kept.lock().unwrap().insert(at, value.to_string());
        Ok(())
    }

    fn get(&self, service: &str, account: &str) -> Result<Option<String>, String> {
        self.works()?;
        let at = (service.to_string(), account.to_string());
        Ok(self.kept.lock().unwrap().get(&at).cloned())
    }

    fn remove(&self, service: &str, account: &str) -> Result<(), String> {
        self.works()?;
        let at = (service.to_string(), account.to_string());
        self.kept.lock().unwrap().remove(&at);
        Ok(())
    }
}

const A_TOKEN: &str = "value-of-a-token-7f31";
const B_TOKEN: &str = "value-of-b-token-92cc";
const B_ONLY: &str = "value-of-b-only-e410";
const A_SEALED: &str = "value-of-a-sealed-1b6d";
const B_SEALED: &str = "value-of-b-sealed-c058";
const VALUES: [&str; 5] = [A_TOKEN, B_TOKEN, B_ONLY, A_SEALED, B_SEALED];

struct Home(std::path::PathBuf);

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn bot_secrets_are_scoped() {
    let home = std::env::temp_dir().join(format!("tirith-secrets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let home = Home(home);
    std::fs::write(home.0.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    let app = App::open_at(home.0.clone());
    let made = |name: &str| {
        let card = json!({"name": name, "shape": "drop", "color": "#22d3ee"});
        let bot = crate::bots::save(&app, card).unwrap();
        bot["id"].as_str().unwrap().to_string()
    };
    let (a, b) = (made("Correo"), made("Banco"));

    argv_holds_no_value(&a);
    the_owner_sets_and_sees_names_only(&app, &a, &b).await;
    each_bot_reaches_its_own_alone(&app, &a, &b);
    no_agent_reads_lists_or_changes(&app, &a, &b).await;
    without_an_os_store_it_is_sealed(&app, &a, &b);
    no_value_lies_readable(&home.0);
    deletion_takes_that_bots_alone(&app, &a, &b).await;
}

fn argv_holds_no_value(a: &str) {
    assert_eq!(parsed(&["set", a, "TOKEN"]), Ok(Command::Set(a, "TOKEN")));
    assert_eq!(parsed(&["list", a]), Ok(Command::List(a)));
    let tries: [&[&str]; 8] = [
        &["set", a, "TOKEN", A_TOKEN],
        &["set", a, "TOKEN", "--value", A_TOKEN],
        &["set", "--value", A_TOKEN, a, "TOKEN"],
        &["set", a, A_TOKEN],
        &["set", A_TOKEN],
        &["list", a, A_TOKEN],
        &["remove", a, "TOKEN", A_TOKEN],
        &["get", a, "TOKEN"],
    ];
    for words in tries {
        let why = parsed(words).unwrap_err();
        assert!(!why.contains(A_TOKEN), "argv's word is repeated: {why}");
    }
    assert!(parsed(&["list", "x/../y"]).is_err() && parsed(&["list", "x\r\ny"]).is_err());

    let piped = |text: &str| super::input::piped(text.as_bytes());
    assert_eq!(piped("one word\n").unwrap(), "one word");
    assert_eq!(piped("one word\r\n").unwrap(), "one word");
    assert_eq!(piped("-----\nabc\n-----\n").unwrap(), "-----\nabc\n-----");
    assert!(piped("\n").is_err() && piped("").is_err() && piped("a\0b").is_err());
    assert!(
        piped(&"x".repeat(64 * 1024 + 1)).is_err(),
        "never cut short"
    );
}

async fn served(router: axum::Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let at = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { drop(axum::serve(listener, router).await) });
    at
}

async fn sent(
    method: &str,
    url: String,
    who: Option<(&str, &str)>,
    body: Option<Value>,
) -> (u16, String) {
    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut call = http
        .request(method.parse().unwrap(), url)
        .header("host", "127.0.0.1"); // the guard keeps the first test's port
    if let Some((header, name)) = who {
        call = call.header(header, name);
    }
    if let Some(body) = body {
        call = call.json(&body);
    }
    let got = call.send().await.unwrap();
    (got.status().as_u16(), got.text().await.unwrap())
}

/// The module's routes as the owner reaches them, behind whatever tells the owner apart.
async fn the_owner_sets_and_sees_names_only(app: &Shared, a: &str, b: &str) {
    let at = served(super::routes().with_state(app.clone())).await;
    let url = |bot: &str| format!("{at}/api/secrets/{bot}");
    for (bot, name, value) in [
        (a, "TOKEN", A_TOKEN),
        (b, "TOKEN", B_TOKEN),
        (b, "ONLY_B", B_ONLY),
        (b, "OLD", "to be removed"),
    ] {
        let body = json!({"name": name, "value": value});
        let (code, said) = sent("POST", url(bot), None, Some(body)).await;
        assert_eq!((code, said.contains(value)), (200, false), "{said}");
    }
    let names = |said: &str| serde_json::from_str::<Vec<String>>(said).unwrap();
    assert_eq!(names(&sent("GET", url(a), None, None).await.1), ["TOKEN"]);
    let (_, listed) = sent("GET", url(b), None, None).await;
    assert_eq!(names(&listed), ["OLD", "ONLY_B", "TOKEN"]);

    let one = |name: &str| format!("{}/{name}", url(b));
    assert_eq!(
        sent("GET", one("TOKEN"), None, None).await.0,
        405,
        "no value"
    );
    assert_eq!(sent("DELETE", one("OLD"), None, None).await.0, 200);
    assert_eq!(sent("DELETE", one("OLD"), None, None).await.0, 400);
    assert_eq!(Store::of(app).list(b).unwrap(), ["ONLY_B", "TOKEN"]);

    let wrong = json!({"name": "lower case", "value": A_TOKEN});
    let (code, said) = sent("POST", url(a), None, Some(wrong)).await;
    assert_eq!((code, said.contains(A_TOKEN)), (400, false), "{said}");
    let unknown = json!({"name": "TOKEN", "value": A_TOKEN});
    let (code, said) = sent("POST", url("nobody"), None, Some(unknown)).await;
    assert_eq!((code, said.contains(A_TOKEN)), (400, false), "{said}");
}

fn each_bot_reaches_its_own_alone(app: &Shared, a: &str, b: &str) {
    assert_eq!(FAKE.of(a), [A_TOKEN], "service tirith.bot.<id>");
    let mut of_b = FAKE.of(b);
    of_b.sort();
    assert_eq!(of_b, [B_ONLY, B_TOKEN]);

    let store = Store::of(app);
    let value = |bot: &str, name: &str| store.value(bot, name);
    assert_eq!(value(a, "TOKEN").unwrap().as_deref(), Some(A_TOKEN));
    assert_eq!(value(b, "TOKEN").unwrap().as_deref(), Some(B_TOKEN));
    assert_eq!(value(a, "ONLY_B").unwrap(), None, "A cannot reach B's");
    for (bot, name) in [
        (a.to_string(), format!("../{b}/ONLY_B")),
        (format!("{a}/../{b}"), "ONLY_B".into()),
        (format!("bot-{b}"), "ONLY_B".into()),
        (String::new(), "ONLY_B".into()),
    ] {
        assert!(value(&bot, &name).is_err(), "{bot} {name}");
    }
}

async fn no_agent_reads_lists_or_changes(app: &Shared, a: &str, b: &str) {
    let at = served(crate::server::router(app.clone(), 0)).await;
    let (own, other) = (format!("bot-{a}"), format!("bot-{b}"));
    let callers = [
        ("x-tirith-role", own.as_str()),
        ("x-tirith-role", other.as_str()),
        ("x-tirith-role", "manager"),
        ("x-tirith-role", "owner"),
        ("x-tirith-lane", "a-lane-1234"),
    ];
    for who in callers {
        for bot in [a, b] {
            let all = format!("{at}/api/secrets/{bot}");
            let one = format!("{all}/TOKEN");
            let new = json!({"name": "TOKEN", "value": "an agent's"});
            for (method, url, body) in [
                ("GET", all.clone(), None),
                ("HEAD", all.clone(), None),
                ("POST", all.clone(), Some(new)),
                ("GET", one.clone(), None),
                ("DELETE", one.clone(), None),
            ] {
                let (code, said) = sent(method, url, Some(who), body).await;
                assert_eq!(code, 403, "{who:?} {method} {bot}");
                assert!(!said.contains("TOKEN") && VALUES.iter().all(|v| !said.contains(v)));
            }
            let spelled = format!("{at}/api/%73ecrets/{bot}");
            let (code, said) = sent("GET", spelled, Some(who), None).await;
            assert!(matches!(code, 403..=405), "no second spelling: {code}");
            assert!(!said.contains("TOKEN"));
        }
    }
    the_routes_refuse_by_themselves(app, a, callers).await;
    let store = Store::of(app);
    assert_eq!(store.value(a, "TOKEN").unwrap().as_deref(), Some(A_TOKEN));
    assert_eq!(store.value(b, "TOKEN").unwrap().as_deref(), Some(B_TOKEN));
}

/// With no list of the server's in front: a change to that list does not open them.
async fn the_routes_refuse_by_themselves(app: &Shared, a: &str, callers: [(&str, &str); 5]) {
    let at = served(super::routes().with_state(app.clone())).await;
    let all = format!("{at}/api/secrets/{a}");
    for who in callers {
        let new = json!({"name": "TOKEN", "value": "an agent's"});
        for (method, url, body) in [
            ("GET", all.clone(), None),
            ("POST", all.clone(), Some(new)),
            ("DELETE", format!("{all}/TOKEN"), None),
        ] {
            let (code, _) = sent(method, url, Some(who), body).await;
            assert_eq!(code, 403, "the route alone: {who:?} {method}");
        }
    }
}

fn without_an_os_store_it_is_sealed(app: &Shared, a: &str, b: &str) {
    let none = Fake::new(false, Some("there is no OS store"));
    let store = Store::on(app, &none);
    store.set(a, "SEALED", A_SEALED).unwrap();
    store.set(b, "SEALED", B_SEALED).unwrap();
    assert_eq!(store.value(a, "SEALED").unwrap().as_deref(), Some(A_SEALED));
    assert_eq!(store.value(b, "SEALED").unwrap().as_deref(), Some(B_SEALED));

    let file = |bot: &str| store.file.file(&super::names::service(bot));
    let dir = app.home.join("bot-secrets");
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700);
    for path in [file(a), file(b), dir.join("key")] {
        assert_eq!(mode(&path), 0o600, "{}", path.display());
    }

    let mine = std::fs::read(file(a)).unwrap();
    std::fs::copy(file(b), file(a)).unwrap();
    assert!(
        store.value(a, "SEALED").is_err(),
        "B's file opens as B's alone"
    );
    std::fs::write(file(a), mine).unwrap();

    let key = dir.join("key");
    std::fs::set_permissions(&key, PermissionsExt::from_mode(0o644)).unwrap();
    assert!(store.value(a, "SEALED").is_err(), "a key others can read");
    std::fs::set_permissions(&key, PermissionsExt::from_mode(0o600)).unwrap();

    assert!(
        store.set(a, "TOKEN", "new").is_err(),
        "never down to the file"
    );
    let locked = Fake::new(true, Some("locked"));
    assert!(Store::on(app, &locked).set(a, "FRESH", "x").is_err());
    assert_eq!(
        store.list(a).unwrap(),
        ["SEALED", "TOKEN"],
        "none left behind"
    );

    let up = Store::of(app);
    up.set(b, "MOVES", "up").unwrap();
    up.remove(b, "MOVES").unwrap();
    store.set(b, "MOVES", "down").unwrap();
    up.set(b, "MOVES", "up").unwrap();
    assert!(
        FAKE.of(b).contains(&"up".to_string()),
        "up when a store exists"
    );
    up.remove(b, "MOVES").unwrap();
    assert!(!FAKE.of(b).contains(&"up".to_string()));
    assert_eq!(up.value(b, "SEALED").unwrap().as_deref(), Some(B_SEALED));
}

/// The database, its journal, config.toml, the sealed files: none holds a value as it is.
fn no_value_lies_readable(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            no_value_lies_readable(&entry.path());
        } else if kind.is_file() {
            let bytes = std::fs::read(entry.path()).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            for value in VALUES {
                assert!(!text.contains(value), "{}", entry.path().display());
            }
        }
    }
}

async fn deletion_takes_that_bots_alone(app: &Shared, a: &str, b: &str) {
    let locked = Fake::new(true, Some("locked"));
    let kept = Store::on(app, &locked).forgotten(a, || -> Result<(), String> {
        panic!("the role went before its values")
    });
    assert!(kept.is_err(), "a value that cannot go keeps the bot");
    assert_eq!(Store::of(app).list(a).unwrap(), ["SEALED", "TOKEN"]);

    let sealed =
        |bot: &str| super::fallback::Fallback::at(&app.home).file(&super::names::service(bot));
    let at = served(crate::bots::routes().with_state(app.clone())).await;
    let (code, said) = sent("POST", format!("{at}/api/bots/{a}/delete"), None, None).await;
    assert_eq!(code, 200, "{said}");

    assert!(crate::bots::key_for(app, a).is_none(), "its role");
    assert!(FAKE.of(a).is_empty() && !sealed(a).exists(), "its values");
    assert_eq!(app.kv(&format!("secrets:{a}")), None, "its names");
    let store = Store::of(app);
    assert!(store.value(a, "TOKEN").is_err() && store.list(a).is_err());
    assert_eq!(store.list(b).unwrap(), ["ONLY_B", "SEALED", "TOKEN"]);
    assert_eq!(store.value(b, "TOKEN").unwrap().as_deref(), Some(B_TOKEN));
    assert_eq!(store.value(b, "ONLY_B").unwrap().as_deref(), Some(B_ONLY));
    assert_eq!(store.value(b, "SEALED").unwrap().as_deref(), Some(B_SEALED));
}
