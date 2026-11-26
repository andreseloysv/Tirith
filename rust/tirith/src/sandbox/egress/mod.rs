//! Where a confined process may go: the owner's block list refuses (`blocks.rs`), and an allowlist, when there is one, refuses all else: `[network] allow = [...]` for the whole machine (a server install's ceiling) and `net = [...]` in `[roles.<role>]` for one role (a bot's, the lane role's); `net = "ask"` asks the owner on a card for each new destination. Everything that leaves goes through the proxy (git over ssh and `neobot net tunnel` too), so this one check is all of them.
mod ask;
mod lists;
mod matcher;
mod permit;

pub use ask::settled;
pub use lists::role_of;
pub use permit::allow;

use crate::app::Shared;
use lists::Net;

/// What the caller is told, and the rule the event names.
pub struct Denial {
    pub rule: String,
    pub body: String,
}

fn denial(host: &str, rule: &str, how: &str) -> Option<Denial> {
    let body = format!("Not allowed by NeoBot: {host} ({rule}). {how}\n");
    Some(Denial {
        rule: rule.into(),
        body,
    })
}

/// Why `who` may not reach `host`, when it may not.
pub fn denied(app: &Shared, who: &str, host: &str) -> Option<Denial> {
    let host = matcher::plain(host);
    let home = lists::home(app);
    if matcher::loopback(&host) || (home.is_none() && !has_net(app, who)) {
        return None;
    }
    if home.is_some_and(|rules| !matcher::listed(&rules, &host)) {
        let how = format!("The owner allows only listed destinations on this machine. Allow it with: neobot net permit {host}");
        return denial(&host, "not on [network] allow", &how);
    }
    let role = lists::role_of(app, who);
    let Net::Listed(rules, ask) = lists::net(app, &role) else {
        return None;
    };
    if matcher::listed(&rules, &host) {
        return None;
    }
    let card = ask.then(|| ask::open(app, who, &role, &host)).flatten();
    let how = match card {
        Some(card) => format!("The owner was asked (card {card}); their yes allows it."),
        None => format!("Allow it for {role} with: neobot net permit {host} --role {role}"),
    };
    denial(&host, &format!("not on the net list of {role}"), &how)
}

fn has_net(app: &Shared, who: &str) -> bool {
    matches!(lists::net(app, &lists::role_of(app, who)), Net::Listed(..))
}

#[cfg(test)]
#[path = "../../../checks/egress.rs"]
mod tests;
