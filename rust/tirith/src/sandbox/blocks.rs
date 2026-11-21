//! The owner's block list: in memory (mirroring the `net_blocks` table), checked for every destination.
use super::rules::{cidr, clean, in_range};
use crate::app::{now_iso, Shared};
use serde_json::json;
use std::{net::IpAddr, sync::RwLock};

static BLOCKS: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// Reads the block list from the database.
pub fn load(app: &Shared) {
    *BLOCKS.write().unwrap() =
        app.query("SELECT rule FROM net_blocks ORDER BY at", [], |r| r.get(0));
}

/// The rules, oldest first.
pub fn list() -> Vec<String> {
    BLOCKS.read().unwrap().clone()
}

pub fn none() -> bool {
    BLOCKS.read().unwrap().is_empty()
}

/// The rule that blocks `host` (a domain and its subdomains, or an address inside a rule's range), else `None`.
pub fn blocked(host: &str) -> Option<String> {
    let host = host.trim().trim_matches(['.', '[', ']']).to_lowercase();
    let ip: Option<IpAddr> = host.parse().ok();
    BLOCKS
        .read()
        .unwrap()
        .iter()
        .find(|rule| {
            ip.zip(cidr(rule))
                .is_some_and(|(ip, range)| in_range(ip, range))
                || (ip.is_none()
                    && cidr(rule).is_none()
                    && (host == **rule || host.ends_with(&format!(".{rule}"))))
        })
        .cloned()
}

/// Blocks (`on`) or unblocks a rule, in the database and in memory, and tells the page.
pub fn block(app: &Shared, rule: &str, on: bool) -> Result<String, String> {
    let rule = valid(rule)?;
    let db = app.db.lock().unwrap();
    let mut rules = BLOCKS.write().unwrap();
    rules.retain(|r| *r != rule);
    let _ = if on {
        db.execute(
            "INSERT OR REPLACE INTO net_blocks (rule, at) VALUES (?1, ?2)",
            [&rule, &now_iso()],
        )
        .map(|_| rules.push(rule.clone()))
    } else {
        db.execute("DELETE FROM net_blocks WHERE rule = ?1", [&rule])
            .map(|_| ())
    };
    drop((db, rules));
    app.emit("net", json!({"blocks": list()}));
    Ok(rule)
}

/// The cleaned rule, if it is a domain, an address or a range.
fn valid(rule: &str) -> Result<String, String> {
    let rule = clean(rule);
    let domain = rule
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    (!(rule.is_empty() || !(domain || cidr(&rule).is_some())))
        .then_some(())
        .ok_or_else(|| format!("not a domain, address or range: {rule:?}"))?;
    Ok(rule)
}
