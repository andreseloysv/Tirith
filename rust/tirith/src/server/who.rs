//! Who calls, as the gate found out (`admit.rs`) and as each route is told. Nothing a caller says of itself is read for it.
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, HeaderMap};
use std::convert::Infallible;

/// The chat's other speakers: an agent named as one of them is just an agent.
const SPEAKERS: [&str; 6] = ["owner", "room", "tirith", "card", "tool", "routine"];

/// A caller with a token of `sandbox::wrap`'s, and what that token alone says of it.
#[derive(Clone, Debug, Default)]
pub struct Agent {
    /// Its lane or role, else what it runs as (`merge:<lane>`).
    pub name: String,
    pub lane: String,
    pub role: String,
    /// The lane's project, or the role's conversation.
    pub project: String,
    /// The bot's key, when it is one.
    pub bot: String,
}

#[derive(Clone, Debug, Default)]
pub enum Who {
    /// `owner.token` or the page opened with it, from outside the sandbox.
    Owner,
    /// A teach route's own token.
    Extension,
    Agent(Agent),
    #[default]
    Nobody,
}

impl Who {
    /// Whether it is kept from what is the owner's: whoever is not known to be them, the extension apart.
    pub fn is_agent(&self) -> bool {
        matches!(self, Who::Agent(_) | Who::Nobody)
    }

    pub fn agent(&self) -> Option<&Agent> {
        match self {
            Who::Agent(a) => Some(a),
            _ => None,
        }
    }

    /// The name its words go under; empty for the owner.
    pub fn name(&self) -> &str {
        match self {
            Who::Agent(a) if !SPEAKERS.contains(&a.name.as_str()) => &a.name,
            Who::Agent(_) | Who::Nobody => "agent",
            Who::Owner | Who::Extension => "",
        }
    }
}

/// A route outside the gate knows nobody.
impl<S: Send + Sync> FromRequestParts<S> for Who {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.extensions.get::<Who>().cloned().unwrap_or_default())
    }
}

/// `X-Tirith-Token`; an unset `$VAR` that a hook passed on as it is counts as none.
pub(super) fn sent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-tirith-token")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && !v.starts_with('$'))
}
