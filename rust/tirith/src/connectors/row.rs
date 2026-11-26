//! The strict check of one `[[connectors]]` row.
use super::check::string_list;
use super::read::strings;

const KEYS: [&str; 8] = [
    "id", "command", "url", "blocked", "deny", "approval", "secrets", "required",
];

/// An id is lowercase letters, digits, `-` and `_`.
fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// A secret is named like an environment variable: never a value.
fn secret_name(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Where it runs: an argv, or an http(s) address, never both.
fn wrong_place(t: &toml::Table) -> Option<&'static str> {
    let (cmd, url) = (t.get("command"), t.get("url"));
    if cmd.is_some() == url.is_some() {
        return Some("either command or url, not both");
    }
    let argv = |c: &toml::Value| string_list(c) && c.as_array().is_some_and(|a| !a.is_empty());
    let web = |u: &str| u.starts_with("https://") || u.starts_with("http://");
    if cmd.is_some_and(|c| !argv(c)) {
        Some("command as an argv array of strings")
    } else if url.is_some_and(|u| !u.as_str().is_some_and(web)) {
        Some("url as an http(s) address")
    } else {
        None
    }
}

/// What is wrong with the row at `i`, said by its id (or its place when it has none).
pub(super) fn wrong(i: usize, t: &toml::Table) -> Option<String> {
    let id = t.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let at = match id {
        "" => format!("connectors[{i}]"),
        _ => format!("connector '{id}'"),
    };
    let lists = ["deny", "approval", "secrets", "required"];
    let bad = t
        .keys()
        .find(|k| !KEYS.contains(&k.as_str()))
        .map(|k| format!("unknown key {k}"))
        .or_else(|| (!id_ok(id)).then(|| "an id of lowercase letters, digits, - and _".into()))
        .or_else(|| wrong_place(t).map(String::from))
        .or_else(|| {
            t.get("blocked")
                .filter(|b| !b.is_bool())
                .map(|_| "blocked as true or false".into())
        })
        .or_else(|| {
            lists
                .into_iter()
                .find(|k| t.get(*k).is_some_and(|v| !string_list(v)))
                .map(|k| format!("{k} as a list of strings"))
        })
        .or_else(|| {
            strings(t, "secrets")
                .iter()
                .any(|s| !secret_name(s))
                .then(|| "secrets as variable names (A-Z, 0-9, _), never values".into())
        })
        .or_else(|| {
            let all = strings(t, "secrets");
            let stray = strings(t, "required")
                .into_iter()
                .any(|r| !all.contains(&r));
            stray.then(|| "required names only secrets it declares".into())
        })
        .or_else(|| {
            strings(t, "secrets")
                .iter()
                .any(|s| crate::oauth::reserved(s))
                .then(|| "secrets without a TIRITH_OAUTH_ name, a sign-in's own".into())
        });
    bad.map(|b| format!("{at}: {b}"))
}
