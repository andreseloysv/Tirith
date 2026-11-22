//! A sign-in's tokens in the bot's store under the reserved names, and read back: by the daemon alone, off the async threads.
use super::names;
use super::token::{Grant, Tokens};
use crate::app::Shared;

/// What the bot's store holds of connector `id`'s sign-in.
pub(super) struct Held {
    pub access: String,
    pub refresh: Option<String>,
    pub grant: Grant,
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| "the secret store stopped".to_string())?
}

fn now() -> u64 {
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since.map_or(0, |d| d.as_secs())
}

/// `tokens` kept as bot `bot`'s sign-in to connector `id`; a refresh token the server did not renew stays.
pub(super) async fn keep(
    app: &Shared,
    (bot, id): (&str, &str),
    tokens: Tokens,
    mut grant: Grant,
) -> Result<String, String> {
    grant.expires_at = tokens.lives.map_or(0, |s| now() + s);
    let grant = serde_json::to_string(&grant).map_err(|e| e.to_string())?;
    let (app, bot, id) = (app.clone(), bot.to_string(), id.to_string());
    blocking(move || {
        let set = |name: String, value: &str| crate::secrets::keep(&app, &bot, &name, value);
        set(names::grant(&id), &grant)?;
        if let Some(refresh) = &tokens.refresh {
            set(names::refresh(&id), refresh)?;
        }
        set(names::access(&id), &tokens.access)?;
        Ok(tokens.access)
    })
    .await
}

/// Bot `bot`'s sign-in to connector `id`, if it has one.
pub(super) async fn held(app: &Shared, bot: &str, id: &str) -> Result<Option<Held>, String> {
    let (app, bot, id) = (app.clone(), bot.to_string(), id.to_string());
    blocking(move || {
        let get = |name: String| crate::secrets::value(&app, &bot, &name);
        let (Some(access), Some(grant)) = (get(names::access(&id))?, get(names::grant(&id))?)
        else {
            return Ok(None);
        };
        let grant = serde_json::from_str(&grant).map_err(|_| "a damaged sign-in")?;
        let refresh = get(names::refresh(&id))?;
        Ok(Some(Held {
            access,
            refresh,
            grant,
        }))
    })
    .await
}

/// Whether an access ending at `at` (0: unknown) still has a minute.
pub(super) fn alive(at: u64) -> bool {
    at == 0 || at > now() + 60
}
