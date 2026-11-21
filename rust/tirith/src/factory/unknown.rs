//! The checks that name the first key or section a factory should not hold.
use super::lists::SECTIONS;

/// One table section of the factory, when it has it.
pub(super) fn sub<'a>(t: &'a toml::Table, sec: &str) -> Option<&'a toml::Table> {
    t.get(sec).and_then(|v| v.as_table())
}

/// The first top-level section the factory does not know.
pub(super) fn unknown_section(t: &toml::Table) -> Option<String> {
    t.keys()
        .find(|k| !SECTIONS.contains(&k.as_str()))
        .map(|k| format!("unknown section '{k}'"))
}

/// The first unknown key inside any named entry of `sec` (`[agents.x]`, `[roles.x]`, ...).
pub(super) fn unknown_in(t: &toml::Table, sec: &str, allowed: &[&str]) -> Option<String> {
    sub(t, sec).and_then(|m| {
        m.iter().find_map(|(n, s)| {
            s.as_table().and_then(|s| {
                s.keys()
                    .find(|k| !allowed.contains(&k.as_str()))
                    .map(|k| format!("unknown key {sec}.{n}.{k}"))
            })
        })
    })
}

/// The first unknown key right under `sec` (`[plan]`, `[cards]`).
pub(super) fn unknown_flat(t: &toml::Table, sec: &str, allowed: &[&str]) -> Option<String> {
    sub(t, sec).and_then(|m| {
        m.keys()
            .find(|k| !allowed.contains(&k.as_str()))
            .map(|k| format!("unknown key {sec}.{k}"))
    })
}

/// The first unknown key in any row of an array section (`[[routes]]`, `[[evals]]`).
pub(super) fn unknown_keys(rows: &[toml::Value], allowed: &[&str], sec: &str) -> Option<String> {
    rows.iter().find_map(|r| {
        r.as_table().and_then(|r| {
            r.keys()
                .find(|k| !allowed.contains(&k.as_str()))
                .map(|k| format!("unknown key {sec}.{k}"))
        })
    })
}
