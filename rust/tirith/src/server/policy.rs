//! What each call needs: a level of a signed-in person, or the machine owner. Anything not listed here that
//! changes something is the owner's, so a new route is closed to people until it is listed.
use crate::app::Shared;
use crate::oidc::Level;
use axum::http::Method;

pub(super) enum Need {
    Level(Level),
    Owner,
}

impl Need {
    pub(super) fn allows(&self, level: Level) -> bool {
        matches!(self, Need::Level(l) if *l <= level)
    }

    pub(super) fn name(&self) -> &'static str {
        match self {
            Need::Level(l) => l.name(),
            Need::Owner => "the machine owner",
        }
    }
}

use Level::{Admin, Member, Viewer};

/// What `method path` needs.
pub(super) fn need(app: &Shared, method: &Method, path: &str) -> Need {
    let parts: Vec<&str> = path.split('/').skip(1).collect();
    if let ["api", "owner", id] = parts.as_slice() {
        return card(app, id);
    }
    if method == Method::GET {
        return reads(&parts);
    }
    changes(&parts)
}

/// The owner's cards: credentials, payment, accounts and devices are the owner's, an approval of an outward action an admin's, a decision or a lane's question a member's.
fn card(app: &Shared, id: &str) -> Need {
    let sql = "SELECT kind FROM owner_items WHERE id = ?1 AND status = 'open'";
    let kind: Option<String> = app
        .db
        .lock()
        .unwrap()
        .query_row(sql, [id], |r| r.get(0))
        .ok();

    match kind.as_deref() {
        Some("decision") | None => Need::Level(Member),
        Some("approval") => Need::Level(Admin),
        Some(_) => Need::Owner,
    }
}

fn reads(parts: &[&str]) -> Need {
    let owner = matches!(
        parts,
        [
            "api",
            "secrets" | "mcp" | "setup" | "teach" | "door" | "oauth",
            ..
        ] | ["api", "rooms", .., "qr" | "qr.svg"]
            | ["api", "projects", "folders"]
            | ["api", "connectors", "logins" | "calls" | "blocks", ..]
    );
    match parts {
        _ if owner => Need::Owner,
        ["api", "bots", _, "webhook"] => Need::Level(Admin),
        _ => Need::Level(Viewer),
    }
}

fn changes(parts: &[&str]) -> Need {
    let level = match parts {
        ["api", "bots", "package" | "from-template", ..] => return Need::Owner,
        ["api", "auth", "logout"] => Viewer,
        ["api", "chat", "clear" | "keep"]
        | ["api", "lanes", _, "delete"]
        | ["api", "owner", _, "close"] => Admin,
        ["api", "chat" | "owner" | "attachments" | "plan", ..] | ["api", "lanes"] => Member,
        ["api", "manager", "interrupt"] | ["api", "turns", _, "reaction"] => Member,
        ["api", "lanes", _, "stop" | "answer" | "ask"] | ["api", "bots", _, "run", _] => Member,
        ["api", "pipelines", _, "retry"] => Member,
        ["api", "bots", ..] | ["api", "net", "allow"] | ["api", "projects"] => Admin,
        ["api", "autopilot" | "stage-limit" | "models" | "guard" | "lab" | "review" | "lang", ..] => {
            Admin
        }
        _ => return Need::Owner,
    };
    Need::Level(level)
}
