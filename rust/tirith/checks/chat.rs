use super::*;

fn at(ts: u64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .unwrap()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// An app at a fresh home with one project `p1` (config.toml written before it opens, as the owner's would be).
fn home(tag: &str) -> Shared {
    let dir = std::env::temp_dir().join(format!("tirith-chat-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("config.toml"),
        "# the owner's\n[[projects]]\nid = \"p1\"\nname = \"P One\"\nrepo = \"/tmp\"\n",
    )
    .unwrap();
    let app = crate::app::App::open_at(dir);
    assert!(
        app.cfg().project("p1").is_some(),
        "config: {:?}",
        app.cfg().error
    );
    app
}

fn row(app: &Shared, ts: u64, project: &str, text: &str) {
    app.insert(
        "INSERT INTO chat (at, role, text, project) VALUES (?1, 'owner', ?2, ?3)",
        rusqlite::params![at(ts), text, project],
    );
}

fn rows(app: &Shared, project: &str) -> Vec<String> {
    app.query(
        "SELECT text FROM chat WHERE project = ?1 ORDER BY id",
        [project],
        |r| r.get::<_, String>(0),
    )
}

#[test]
fn chat_clear_forgets_session() {
    let app = home("clear");
    row(&app, 1_000, "p1", "hola");
    row(&app, 2_000, "p1", "qué tal");
    row(&app, 2_000, "manager-general", "de otra conversación");
    app.set_kv("manager_session:anthropic:p1", Some("sesion-1"));
    app.set_kv("manager_session:codex:p1", Some("sesion-2"));
    app.set_kv("manager_session:codex:manager-general", Some("sesion-3"));
    app.set_kv("manager_session:codex:xp1", Some("sesion-4"));
    clear(&app, "p1").unwrap();
    assert!(rows(&app, "p1").is_empty(), "its rows go");
    assert_eq!(
        app.kv("manager_session:anthropic:p1"),
        None,
        "its saved session is forgotten"
    );
    assert_eq!(
        app.kv("manager_session:codex:p1"),
        None,
        "every agent's saved session is forgotten"
    );
    assert_eq!(
        app.kv("manager_session:codex:manager-general").as_deref(),
        Some("sesion-3"),
        "another conversation's session stays"
    );
    assert_eq!(
        app.kv("manager_session:codex:xp1").as_deref(),
        Some("sesion-4"),
        "a longer id ending alike stays"
    );
    assert_eq!(
        rows(&app, "manager-general"),
        ["de otra conversación"],
        "another conversation's rows stay"
    );
    assert!(
        clear(&app, "no-tal").is_err(),
        "an unknown conversation is refused"
    );
}

#[test]
fn chat_keep_drops_old_rows() {
    let app = home("keep");
    let now: u64 = 1_781_000_000; // some fixed noon
    set_keep(&app, "p1", "1h").unwrap();
    assert_eq!(keep_words(&app, "p1"), "1h", "the owner's words are kept");
    assert!(
        std::fs::read_to_string(app.home.join("config.toml"))
            .unwrap()
            .contains("chat_keep = \"1h\""),
        "written into config.toml"
    );
    row(&app, now - 7200, "p1", "viejo");
    row(&app, now - 1800, "p1", "nuevo");
    row(&app, now - 7200, "manager-general", "sin límite");
    app.set_kv("manager_session:anthropic:p1", Some("sesion-1"));
    sweep(&app, now);
    assert_eq!(
        rows(&app, "p1"),
        ["nuevo"],
        "only the rows inside the hour stay"
    );
    assert_eq!(
        rows(&app, "manager-general"),
        ["sin límite"],
        "a conversation with no limit keeps everything"
    );
    assert_eq!(
        app.kv("manager_session:anthropic:p1"),
        None,
        "a sweep that dropped rows forgets the saved session"
    );
    app.set_kv("manager_session:anthropic:p1", Some("sesion-2"));
    sweep(&app, now);
    assert_eq!(
        app.kv("manager_session:anthropic:p1").as_deref(),
        Some("sesion-2"),
        "a sweep that dropped nothing keeps the session"
    );
    set_keep(&app, "p1", "siempre").unwrap();
    assert_eq!(keep_words(&app, "p1"), "", "«siempre» takes the limit away");
    assert!(
        !std::fs::read_to_string(app.home.join("config.toml"))
            .unwrap()
            .contains("chat_keep"),
        "and out of config.toml"
    );
    sweep(&app, now + 86400 * 365);
    assert_eq!(
        rows(&app, "p1"),
        ["nuevo"],
        "with no limit nothing is dropped any more"
    );
}

#[test]
fn chat_keep_parses_owner_words() {
    assert_eq!(parse_keep("30m").unwrap(), Some(1800));
    assert_eq!(parse_keep("1h").unwrap(), Some(3600));
    assert_eq!(parse_keep("24 h").unwrap(), Some(86400));
    assert_eq!(parse_keep("24h").unwrap(), Some(86400));
    assert_eq!(parse_keep("2 días").unwrap(), Some(172800));
    assert_eq!(parse_keep("una semana").unwrap(), Some(604800));
    assert_eq!(parse_keep("una hora").unwrap(), Some(3600));
    assert_eq!(parse_keep("un día").unwrap(), Some(86400));
    assert_eq!(parse_keep(" 45 minutos ").unwrap(), Some(2700));
    assert_eq!(parse_keep("90 segundos").unwrap(), Some(90));
    assert_eq!(parse_keep("siempre").unwrap(), None);
    assert_eq!(parse_keep("Siempre").unwrap(), None);
    assert_eq!(parse_keep("").unwrap(), None);
    assert!(
        parse_keep("mañana").is_err(),
        "a word that is no time is refused"
    );
    assert!(parse_keep("24").is_err(), "a bare number is refused");
    assert!(parse_keep("0h").is_err(), "zero is refused");
    assert!(
        parse_keep("3 meses").is_err(),
        "an unlisted unit is refused"
    );
}
