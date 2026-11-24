//! The gateway's bearer for a remote connector: the bot's signed-in access, renewed with its refresh token when it ends within a minute or the server refused it. One renewal at a time, as a server may rotate the refresh token.
use super::keep::{alive, held, keep};
use crate::app::Shared;

static ONE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Bot `bot`'s access for connector `id`, none when it never signed in; `renew` asks a new one even if it seems alive.
pub async fn bearer(
    app: &Shared,
    bot: &str,
    id: &str,
    renew: bool,
) -> Result<Option<String>, String> {
    let _one = ONE.lock().await;
    let Some(h) = held(app, bot, id).await? else {
        return Ok(None);
    };
    if !renew && alive(h.grant.expires_at) {
        return Ok(Some(h.access));
    }
    let again = format!("run neobot connector login {id} --bot {bot} again");
    let Some(refresh) = h.refresh else {
        return Err(format!("its sign-in ended: {again}"));
    };
    let tokens = super::token::refresh(&h.grant, &refresh).await;
    let tokens =
        tokens.map_err(|why| format!("its sign-in could not be renewed ({why}): {again}"))?;
    keep(app, (bot, id), tokens, h.grant).await.map(Some)
}
