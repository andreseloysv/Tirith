//! A bot package: everything a bot is made of (its routines, skills, instructions, the connectors it needs and what it declares), signed by its creator's key, never its memory, chats, secrets, logins or webhook addresses. Exported from the app, previewed before anything is made, imported only by the owner.
mod api;
mod check;
mod export;
mod import;
mod preview;
mod sign;

pub(super) use api::routes;

/// The package format's name and version.
const FORMAT: &str = "neobot-bot";
const VERSION: u64 = 1;
/// The fields a package carries, and no others: a package is data, so what it does not name is refused.
const FIELDS: [&str; 10] = [
    "format",
    "v",
    "bot",
    "routines",
    "skills",
    "integrations",
    "permissions",
    "creator",
    "created",
    "sig",
];

#[cfg(test)]
pub(super) use {
    check::checked as checked_for_test, export::export as export_for_test,
    import::import as import_for_test, preview::preview as preview_for_test,
};
