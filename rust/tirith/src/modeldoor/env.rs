//! What an agent's process is given in place of its provider key: a token that opens the door for it alone, and the door's address as the provider's base URL. The real key never enters its environment.
use super::spec::find;
use crate::app::Shared;

/// The caller token of `who` at `alias`'s door (`door:<alias>:<who>`), the same across restarts.
pub fn token(app: &Shared, alias: &str, who: &str) -> String {
    crate::sandbox::caller_token(app, &format!("door:{alias}:{who}"))
}

/// `vars` (the agent's auth file) with each provider key replaced by its token and each base URL by the door's; as they are when the door is off or there is no key.
pub fn given(
    app: &Shared,
    who: &str,
    alias: &str,
    vars: Vec<(String, String)>,
) -> Vec<(String, String)> {
    let providers = find(&vars);
    if alias.is_empty() || providers.is_empty() || !super::on::enabled(app) {
        return vars;
    }
    let door = format!("http://127.0.0.1:{}/door/{alias}", app.cfg().port);
    let token = token(app, alias, who);
    let mut out: Vec<(String, String)> = vars
        .into_iter()
        .map(
            |(k, v)| match providers.iter().find(|p| p.key_var == k || p.url_var == k) {
                Some(p) if p.key_var == k => (k, token.clone()),
                Some(p) => (k, format!("{door}{}", p.path)),
                None => (k, v),
            },
        )
        .collect();
    let missing: Vec<_> = providers
        .iter()
        .filter(|p| !out.iter().any(|(k, _)| k == p.url_var))
        .collect();
    out.extend(
        missing
            .iter()
            .map(|p| (p.url_var.to_string(), format!("{door}{}", p.path))),
    );
    out
}
