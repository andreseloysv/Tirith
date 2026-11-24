//! A sign-in begun: the remote connector's server discovered, NeoBot registered at it, and the address the owner's browser goes to, with PKCE, a state and the resource.
use super::pending::{put, Pending};
use super::token::Grant;
use super::{discover::discover, pkce, register::register};
use crate::app::Shared;
use reqwest::Url;

/// Where bot `bot` signs in to connector `id`, and the ticket to wait on.
pub(super) async fn start(app: &Shared, id: &str, bot: &str) -> Result<(String, String), String> {
    let cfg = app.cfg();
    let c = cfg.factory.connector(id).cloned();
    let c = c.ok_or_else(|| format!("no connector '{id}' in config.toml"))?;
    if !c.command.is_empty() {
        return Err(format!("connector '{id}' is local: it needs no login, it is given the secrets its definition names"));
    }
    crate::bots::key_for(app, bot).ok_or_else(|| format!("no bot '{bot}'"))?;
    let http = super::http()?;
    let server = discover(&http, &c.url).await?;
    let redirect = format!("http://127.0.0.1:{}/oauth/callback", cfg.port);
    let Some(endpoint) = server.register else {
        return Err(format!(
            "{} offers no client registration: NeoBot cannot sign in to it",
            c.url
        ));
    };
    let client = register(&http, &endpoint, &redirect).await?;
    let (verifier, state, ticket) = (pkce::verifier(), pkce::state(), pkce::state());
    let mut url = Url::parse(&server.authorize).map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client.id)
        .append_pair("redirect_uri", &redirect)
        .append_pair("code_challenge", &pkce::challenge(&verifier))
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("resource", &c.url);
    if !server.scopes.is_empty() {
        url.query_pairs_mut()
            .append_pair("scope", &server.scopes.join(" "));
    }
    let grant = Grant {
        token_url: server.token,
        client,
        resource: c.url.clone(),
        expires_at: 0,
    };
    let (bot, id, at) = (bot.to_string(), id.to_string(), std::time::Instant::now());
    let pending = Pending {
        bot,
        id,
        verifier,
        redirect,
        grant,
        ticket: ticket.clone(),
        at,
    };
    put(state, pending);
    Ok((url.into(), ticket))
}
