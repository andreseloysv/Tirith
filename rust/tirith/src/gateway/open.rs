//! Opening a bot's gateway: the caller proven by its token alone, then every connector its role is granted, started or reached. A blocked connector is never opened: nothing of it could be listed or called.
use super::{link::Link, session::Session};
use crate::app::Shared;
use crate::server::Who;
use axum::http::StatusCode;

type Refused = (StatusCode, String);

/// The bot's key when `who` (with `token`'s identity `raw`) is a run of bot `bot`, else why not.
pub fn proven(
    app: &Shared,
    who: &Who,
    raw: Option<String>,
    bot: &str,
) -> Result<(String, String), Refused> {
    let no = |why: String| (StatusCode::FORBIDDEN, why);
    let agent = who
        .agent()
        .filter(|a| a.bot.starts_with("bot-") && a.lane.is_empty()); // a system bot (a manager, the CEO) holds no grant
    let Some((agent, raw)) = agent.zip(raw) else {
        return Err(no(
            "tirith mcp is a bot run's alone: no lane, manager or owner opens it".into(),
        ));
    };
    let key = crate::bots::key_for(app, bot);
    match key.filter(|k| *k == agent.bot) {
        Some(key) => Ok((key, raw)),
        None => Err(no(format!(
            "this run is bot '{}', not '{bot}': tirith mcp --bot names its own",
            agent.bot.strip_prefix("bot-").unwrap_or(&agent.bot)
        ))),
    }
}

/// The session of bot `bot` (key `key`) run as `raw`: each granted connector that opens, and what did not with why.
pub async fn open(
    app: &Shared,
    (bot, key, raw): (&str, &str, &str),
    pid: u32,
) -> (Session, Vec<String>) {
    let granted: Vec<_> = app
        .cfg()
        .factory
        .granted(key)
        .into_iter()
        .map(|(c, _)| c.clone())
        .collect();
    let (mut links, mut failed) = (vec![], vec![]);
    for c in granted.iter().filter(|c| !c.blocked) {
        match Link::open(app, raw, bot, c).await {
            Ok(link) => links.push(link),
            Err(why) => failed.push(format!("{}: {why}", c.id)),
        }
    }
    let _ = (!failed.is_empty()).then(|| {
        let said = format!("connectors that did not open: {}", failed.join("; "));
        app.event("", key, "error", "gateway", &said, "")
    });
    let (bot, key, who) = (bot.to_string(), key.to_string(), raw.to_string());
    (
        Session {
            bot,
            key,
            who,
            pid,
            links,
        },
        failed,
    )
}
