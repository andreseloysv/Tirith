//! The bot's values of a connector's secrets, read from its store for that connector's process or headers alone.
use crate::app::Shared;

/// The bot's values of the connector's secrets, read where the store blocks; one that is not set is left out.
pub async fn secrets(
    app: &Shared,
    bot: &str,
    names: &[String],
) -> Result<Vec<(String, String)>, String> {
    let (app, bot, names) = (app.clone(), bot.to_string(), names.to_vec());
    let read = move || {
        let mut found = vec![];
        for name in names {
            let value = crate::secrets::value(&app, &bot, &name)?;
            found.extend(value.map(|v| (name, v)));
        }
        Ok(found)
    };
    tokio::task::spawn_blocking(read)
        .await
        .map_err(|_| "the secret store stopped".to_string())?
}
