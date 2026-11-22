//! The token of each `who`: its proxy password and its proof at the daemon's door (`TIRITH_TOKEN`). It derives from a secret no confined process reads (`app/keys.rs`). The database only says whose a token may be: a row that lies proves nothing, as the token is derived again.
use crate::{app::App, rooms::sealed::sha_hex};

fn derived(secret: &str, who: &str) -> String {
    sha_hex(format!("{secret}\n{who}").as_bytes())[..32].to_string()
}

/// The database's key for a token: never the token itself, which confined processes could read there.
fn row(token: &str) -> String {
    format!("caller:{}", sha_hex(token.as_bytes()))
}

/// The token of `who`, the same across restarts, findable from itself alone.
pub fn issued(app: &App, who: &str) -> String {
    let token = derived(&app.keys.secret, who);
    let key = row(&token);
    if app.kv(&key).as_deref() != Some(who) {
        app.set_kv(&key, Some(who));
    }
    token
}

/// Whose `token` is, when it is one.
pub fn who_of(app: &App, token: &str) -> Option<String> {
    let who = app.kv(&row(token))?;
    (derived(&app.keys.secret, &who) == token).then_some(who)
}

/// A proxy password from before the secret left the database: good for the proxy until the next start, and for nothing else.
pub(super) fn retired(app: &App, user: &str, password: &str) -> Option<String> {
    let secret = app.keys.retired.as_deref()?;
    let who = user.replace("%3A", ":").replace('~', ":");
    (derived(secret, &who) == password).then_some(who)
}

/// `who` as a user name in the proxy's URL. It is only for people to read: the password says who calls.
pub(super) fn user(who: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "-._".contains(c);
    who.chars()
        .map(|c| match c {
            ':' => '~',
            c if plain(c) => c,
            _ => '-',
        })
        .collect()
}
