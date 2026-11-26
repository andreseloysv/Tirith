//! The bot's values of a connector's secrets, read from its store for that connector's process or headers alone.
use crate::app::Shared;

/// The bot's values of the connector's secrets, read where the store blocks. One that is not set is left out and the connector opens without it, unless its name is in `required`: then it is an error naming it (never a value). A store that cannot be read is the same for an optional one.
pub async fn secrets(
    app: &Shared,
    bot: &str,
    (names, required): (&[String], &[String]),
) -> Result<Vec<(String, String)>, String> {
    let (app, bot) = (app.clone(), bot.to_string());
    let (names, required) = (names.to_vec(), required.to_vec());
    let read = move || {
        let mut found = vec![];
        for name in names {
            let value = match crate::secrets::value(&app, &bot, &name) {
                Ok(v) => v,
                Err(e) if required.contains(&name) => return Err(e),
                Err(_) => None,
            };
            if value.is_none() && required.contains(&name) {
                return Err(format!(
                    "the required secret {name} is not set for this bot"
                ));
            }
            found.extend(value.map(|v| (name, v)));
        }
        Ok(found)
    };
    tokio::task::spawn_blocking(read)
        .await
        .map_err(|_| "the secret store stopped".to_string())?
}
