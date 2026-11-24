//! One agent set up from the page, in the guide's order: what the machine needs, its program (installed when missing), its hooks at OpenBot, its sign-in, then one real turn as the proof.
use super::job;
use crate::app::Shared;
use crate::setup::{
    catalog, catalog::Known, configure, connect, context::Context, find, login, needs, program,
    prove, signin,
};
use std::path::PathBuf;

/// Set in kv while an agent is in config.toml but has not answered its check yet.
pub fn unproven(alias: &str) -> String {
    format!("setup_unproven:{alias}")
}

pub async fn run(app: Shared, alias: String) {
    let done = steps(&app, &alias).await;
    if done.is_ok() {
        app.set_kv(&unproven(&alias), None);
    }
    job::end(&alias, done);
}

async fn steps(app: &Shared, alias: &str) -> Result<(), String> {
    let known = catalog::known(alias).ok_or(format!("no agent '{alias}'"))?;
    let (a, k) = (alias.to_string(), known.clone());
    let path = blocking(move || ready(&a, &k)).await?;
    app.set_kv(&unproven(alias), Some("1"));
    let mut signed = false;
    loop {
        job::step(alias, "check");
        let got = prove::prove(app, alias).await?;
        if got["ok"] == true {
            break;
        }
        if got["answered"] == true {
            return Err("it answered, but OpenBot's rules did not hear it use a tool".into());
        }
        let said = got["said"].as_str().unwrap_or("no answer").to_string();
        if signed || known.signin.is_empty() {
            return Err(format!("it did not answer: {said}"));
        }
        signed = true; // its status said signed in, or says nothing: a sign-in may still be what it lacks
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

/// The machine, the program and its hooks; signed in when its status says it is not.
fn ready(alias: &str, known: &Known) -> Result<PathBuf, String> {
    let cx = Context::page(alias)?;
    job::step(alias, "machine");
    let pending = needs::machine(&cx);
    if !pending.is_empty() {
        return Err(pending.join("; "));
    }
    job::step(alias, "install");
    let path = program::ready(&cx, known)?;
    connect::wire(&cx, known, &path)?;
    if signin::signed_in(&cx, known, &path) == Some(false) {
        sign_in(&cx, known, &path)?;
    }
    Ok(path)
}

fn sign_in(cx: &Context, known: &Known, path: &std::path::Path) -> Result<(), String> {
    job::step(&known.alias, "signin");
    login::login(cx, known, path)
        .then_some(())
        .ok_or_else(|| format!("{} was not signed in", known.name))
}

/// The roles whose agent this machine cannot run move to the one just connected.
fn roles(alias: &str) -> Result<(), String> {
    let cx = Context::page(alias)?;
    let cfg = cx.cfg();
    let runs = |a: &String| {
        let set = cfg.factory.command(a);
        let known = catalog::known(a).unwrap_or_default();
        find::agent(&known, set.as_deref()).is_some()
    };
    let usable: Vec<String> = cfg
        .factory
        .commands()
        .into_iter()
        .map(|(a, _)| a)
        .filter(runs)
        .collect();
    configure::roles(&cx.home, &usable, alias).map(|_| ())
}
