//! Who signs in on a server whose Tirith is another user's (install.sh's `tirith`): their SSH tunnel may reach the page, so `[server] users` names them.
use super::edit::{edit, table};
use std::path::Path;
use toml_edit::{value, Array};

/// Keeps who ran the installer or sudo among `[server] users`.
pub(super) fn keep(home: &Path) -> Result<(), String> {
    match std::env::var(super::as_user::VIA) {
        Ok(user) if !user.is_empty() && user != "root" => add(home, &user),
        _ => Ok(()),
    }
}

pub(super) fn add(home: &Path, user: &str) -> Result<(), String> {
    edit(home, |doc| {
        let server = table(doc, &["server"])?;
        if !server.contains_key("users") {
            server.insert("users", value(Array::new()));
        }
        let users = server.get_mut("users").and_then(|v| v.as_array_mut());
        let users = users.ok_or("config.toml: [server] users is not a list")?;
        if !users.iter().any(|v| v.as_str() == Some(user)) {
            users.push(user);
        }
        Ok(())
    })
}
