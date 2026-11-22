//! An agent that may live on an enterprise's host (Copilot: GitHub Enterprise Cloud with data residency): the guide asks for the host, keeps it with the agent's token and never shows it. All that agent's processes get its variable; its login, which does not read that, gets its flag.
use super::{auth, catalog::Known, context::Context, private::PUBLIC};

/// The host is asked (or read from the agent's own variable in this shell) and kept; with nobody to ask, what was kept stays.
pub(super) fn choose(cx: &Context, known: &Known) -> Result<(), String> {
    if known.host_env.is_empty() {
        return Ok(());
    }
    let kept = kept(cx, known);
    let given = std::env::var(&known.host_env)
        .ok()
        .filter(|v| !v.is_empty());
    let said = match given {
        Some(host) => host,
        None if cx.ask.person() => {
            let question = match kept {
                Some(_) => "  GitHub host [Enter keeps yours]",
                None => "  GitHub host, yours if it is an enterprise's (name.ghe.com) [github.com]",
            };
            let said = cx.ask.text(question, "");
            match (said.is_empty(), &kept) {
                (true, Some(_)) => return Ok(()),
                (true, None) => PUBLIC.to_string(),
                _ => said,
            }
        }
        None => return Ok(()),
    };
    let host = cleaned(&said)?;
    if kept.as_deref() == Some(host.as_str()) {
        return Ok(());
    }
    auth::keep(&cx.home, &known.alias, &known.host_env, &host)?;
    if host != PUBLIC {
        println!("  that host is kept in {}/auth, for its login and all its processes (it is never shown)", cx.home.display());
    }
    Ok(())
}

fn kept(cx: &Context, known: &Known) -> Option<String> {
    let vars = auth::env(&cx.home, &known.alias);
    vars.into_iter()
        .find(|(k, _)| *k == known.host_env)
        .map(|v| v.1)
}

/// A host name, from a name or an address as it is pasted.
pub(super) fn cleaned(said: &str) -> Result<String, String> {
    let said = super::secret::cleaned(said).to_lowercase();
    let bare = said
        .split_once("://")
        .map_or(said.as_str(), |(_, rest)| rest);
    let host = bare.trim_end_matches('/');
    let fine = host.contains('.')
        && !host.starts_with(['-', '.'])
        && !host.ends_with(['-', '.'])
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    fine.then(|| host.to_string())
        .ok_or("that is no host name: like github.com or name.ghe.com".into())
}

/// What the agent's login takes besides its own arguments.
pub(super) fn login_args(cx: &Context, known: &Known) -> Vec<String> {
    match kept(cx, known).filter(|host| host != PUBLIC && !known.host_flag.is_empty()) {
        Some(host) => vec![known.host_flag.clone(), format!("https://{host}")],
        None => vec![],
    }
}
