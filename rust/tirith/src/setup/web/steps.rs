//! The steps of a page setup that wait on programs (run off the async threads): the machine, the program and its hooks, a sign-in, and which agent the roles run on once it answers.
use super::job;
use crate::app::Shared;
use crate::setup::{
    catalog, catalog::Known, configure, connect, context::Context, find, login, needs, program,
    signin,
};
use std::path::PathBuf;

/// The machine, the program and its hooks; signed in when its status says it is not (and whether it was, now).
pub(super) fn ready(alias: &str, known: &Known) -> Result<(PathBuf, bool), String> {
    let cx = Context::page(alias)?;
    job::step(alias, "machine");
    let pending = needs::machine(&cx);
    if !pending.is_empty() {
        return Err(pending.join("; "));
    }
    job::step(alias, "install");
    let path = program::ready(&cx, known)?;
    connect::wire(&cx, known, &path)?;
    let signed = signin::signed_in(&cx, known, &path) == Some(false);
    if signed {
        sign_in(&cx, known, &path)?;
    }
    Ok((path, signed))
}

pub(super) fn sign_in(cx: &Context, known: &Known, path: &std::path::Path) -> Result<(), String> {
    job::step(&known.alias, "signin");
    login::login(cx, known, path)
        .then_some(())
        .ok_or_else(|| format!("{} was not signed in", known.name))
}

/// The roles whose agent has not worked here (not installed, a stand-in, or never answered) move to the one just connected.
pub(super) fn roles(app: &Shared, alias: &str) -> Result<(), String> {
    let cx = Context::page(alias)?;
    let cfg = cx.cfg();
    let works = |a: &String| {
        let set = cfg.factory.command(a);
        let known = catalog::known(a).unwrap_or_default();
        find::agent(&known, set.as_deref()).is_some() && super::list::works(app, a)
    };
    let usable: Vec<String> = cfg
        .factory
        .commands()
        .into_iter()
        .map(|(a, _)| a)
        .filter(works)
        .collect();
    configure::roles(&cx.home, &usable, alias).map(|_| ())
}
