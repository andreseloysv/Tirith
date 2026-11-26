//! The people who signed in (table `people`) and their sessions (`person_sessions`: the hash of a random token, so
//! the database never holds one that works).
use super::{verify::Claims, Level};
use crate::app::{now_iso, now_s, App};
use crate::rooms::sealed::{hex, sha_hex};
use rusqlite::params;

#[derive(Clone, Debug)]
pub struct Person {
    pub oid: String,
    pub name: String,
    pub username: String,
    pub level: Level,
}

/// The person is new or known: their name and role are the token's of now.
pub fn upsert(app: &App, c: &Claims) {
    let at = now_iso();
    let sql = "INSERT INTO people (oid, name, username, role, first_seen, last_seen) VALUES (?1,?2,?3,?4,?5,?5) \
               ON CONFLICT(oid) DO UPDATE SET name=?2, username=?3, role=?4, last_seen=?5";
    app.insert(sql, params![c.oid, c.name, c.username, c.level.name(), at]);
}

/// A new session of `oid` that ends in `hours`; the token that proves it.
pub fn open_session(app: &App, oid: &str, hours: u64) -> String {
    let token = hex(&rand::random::<[u8; 32]>());
    let end = (now_s() + hours * 3600) as i64;
    app.insert(
        "DELETE FROM person_sessions WHERE expires < ?1",
        params![now_s() as i64],
    );
    app.insert(
        "INSERT INTO person_sessions (hash, oid, expires) VALUES (?1,?2,?3)",
        params![sha_hex(token.as_bytes()), oid, end],
    );
    token
}

pub fn end_session(app: &App, token: &str) {
    app.insert(
        "DELETE FROM person_sessions WHERE hash = ?1",
        params![sha_hex(token.as_bytes())],
    );
}

/// The person a live session belongs to, as the people table has them now.
pub fn by_session(app: &App, token: &str) -> Option<Person> {
    let sql = "SELECT p.oid, p.name, p.username, p.role FROM person_sessions s JOIN people p ON p.oid = s.oid \
               WHERE s.hash = ?1 AND s.expires > ?2";
    let row = app.db.lock().unwrap().query_row(
        sql,
        params![sha_hex(token.as_bytes()), now_s() as i64],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        },
    );
    let (oid, name, username, role) = row.ok()?;
    Some(Person {
        oid,
        name,
        username,
        level: Level::parse(&role)?,
    })
}

/// The session cookie's name. Every port of 127.0.0.1 shares cookies, so the name is this home's.
pub fn cookie_name(app: &App) -> String {
    format!(
        "neobot-person-{}",
        &sha_hex(format!("person\n{}", app.keys.owner).as_bytes())[..8]
    )
}

/// The `Set-Cookie` of a session of `hours` (0 ends it); `Secure` when the page is served over https.
pub fn cookie(app: &App, token: &str, hours: u64) -> String {
    let secure = if crate::server::public_https(app) {
        "; Secure"
    } else {
        ""
    };
    format!(
        "{}={token}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{secure}",
        cookie_name(app),
        hours * 3600
    )
}
