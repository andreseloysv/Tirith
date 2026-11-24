//! One agent set up from the page, in the guide's order: what the machine needs, its program (installed when missing), its hooks at NeoBot, its sign-in, then one real turn as the proof.
use super::job;
use super::steps::{ready, roles, sign_in};
use crate::app::Shared;
use crate::setup::{catalog, context::Context, prove};

/// Set in kv while an agent is in config.toml but has not answered its check yet.
pub fn unproven(alias: &str) -> String {
    format!("setup_unproven:{alias}")
}

pub async fn run(app: Shared, alias: String) {
    let done = steps(&app, &alias).await;
    if done.is_ok() {
        app.set_kv(&unproven(&alias), None);
    }
    job::end(&alias, done.map_err(for_page));
}

/// The words for the page: what a terminal would be told to type is dropped.
pub(super) fn for_page(why: String) -> String {
    const TOLD: [&str; 3] = [
        ": install it, then run tirith",
        ": run tirith",
        " (tirith doctor",
    ];
    let cut = |part: &str| {
        let at = TOLD.iter().filter_map(|t| part.find(t)).min();
        part[..at.unwrap_or(part.len())].to_string()
    };
    why.split("; ").map(cut).collect::<Vec<_>>().join("; ")
}

async fn steps(app: &Shared, alias: &str) -> Result<(), String> {
    let known = catalog::known(alias).ok_or(format!("no agent '{alias}'"))?;
    let (a, k) = (alias.to_string(), known.clone());
    let (path, mut signed) = blocking(move || ready(&a, &k)).await?;
    app.set_kv(&unproven(alias), Some("1"));
    loop {
        job::step(alias, "check");
        let got = prove::prove(app, alias).await?;
        if got["ok"] == true {
            break;
        }
        if got["answered"] == true {
            return Err("it answered, but NeoBot's rules did not hear it use a tool".into());
        }
        let said = got["said"].as_str().unwrap_or("no answer").to_string();
        if signed || known.signin.is_empty() {
            return Err(format!("it did not answer: {said}"));
        }
        signed = true; // its status said signed in, or said nothing: a sign-in may still be what it lacks
        let (a, k, p) = (alias.to_string(), known.clone(), path.clone());
        blocking(move || sign_in(&Context::page(&a)?, &k, &p)).await?;
    }
    let a = alias.to_string();
    blocking(move || roles(&a)).await?;
    app.reload_config(true);
    Ok(())
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}
