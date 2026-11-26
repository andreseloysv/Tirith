//! The lists: `[network] allow` for the whole machine, `net` in `[roles.<role>]` for one role.
use crate::app::Shared;
use crate::config::get;

/// What a role's `net` says.
pub(super) enum Net {
    /// No `net`: nothing of its own.
    Free,
    /// Its destinations, and whether the owner is asked about any other.
    Listed(Vec<String>, bool),
}

fn words(v: &toml::Value) -> Option<Vec<String>> {
    match v {
        toml::Value::String(s) => Some(vec![s.clone()]),
        toml::Value::Array(a) => Some(
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
        ),
        _ => None,
    }
}

/// `[network] allow`: set (even empty) means everything else is refused.
pub(super) fn home(app: &Shared) -> Option<Vec<String>> {
    get::<toml::Value>(&app.cfg().raw, &["network", "allow"]).and_then(|v| words(&v))
}

/// The role `who` is: a lane's is the lane role's, a conversation's (`<role>@<conversation>`) its part, `merge:<lane>` its lane's; any other name is itself (`git`, the daemon's own).
pub(super) fn role_of(app: &Shared, who: &str) -> String {
    let who = who.strip_prefix("merge:").unwrap_or(who);
    if let Some((role, _)) = who.split_once('@') {
        return role.to_string();
    }
    match app.lane(who) {
        Some(_) => crate::models::lane_role(&app.cfg()),
        None => who.to_string(),
    }
}

/// The `net` of `role`, or of the role it extends.
pub(super) fn net(app: &Shared, role: &str) -> Net {
    let cfg = app.cfg();
    let mut name = role.to_string();
    for _ in 0..5 {
        if let Some(rules) = cfg
            .factory
            .role::<toml::Value>(&name, "net")
            .and_then(|v| words(&v))
        {
            let ask = rules.iter().any(|r| r == "ask");
            return Net::Listed(rules, ask);
        }
        match cfg.factory.role::<String>(&name, "extends") {
            Some(base) => name = base,
            None => break,
        }
    }
    Net::Free
}
