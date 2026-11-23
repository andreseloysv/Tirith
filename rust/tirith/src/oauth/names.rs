//! The reserved names a connector's sign-in is kept under in a bot's store: `TIRITH_OAUTH_<ID>_ACCESS`, `_REFRESH` and `_GRANT` (the token address, the client and the expiry). The owner cannot set one and no connector's `secrets` may name one, so a token reaches its own connector alone.

const PREFIX: &str = "TIRITH_OAUTH_";

/// Whether `name` is one of the reserved names.
pub fn reserved(name: &str) -> bool {
    name.starts_with(PREFIX)
}

fn of(id: &str, what: &str) -> String {
    format!(
        "{PREFIX}{}_{what}",
        id.to_ascii_uppercase().replace('-', "_")
    )
}

pub(super) fn access(id: &str) -> String {
    of(id, "ACCESS")
}

pub(super) fn refresh(id: &str) -> String {
    of(id, "REFRESH")
}

pub(super) fn grant(id: &str) -> String {
    of(id, "GRANT")
}

/// The three names connector `id`'s sign-in is kept under.
pub fn all(id: &str) -> [String; 3] {
    [access(id), refresh(id), grant(id)]
}

/// Whether `names` (a bot's) hold a sign-in to connector `id`.
pub fn signed_in(names: &[String], id: &str) -> bool {
    names.contains(&access(id))
}
