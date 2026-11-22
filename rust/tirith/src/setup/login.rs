//! An agent's own login on the owner's terminal, and how to do it later.
use super::{auth, catalog::Known, context::Context, host, run};
use std::path::Path;

pub(super) fn login(cx: &Context, known: &Known, path: &Path) -> bool {
    println!("  Sign in: open the link it shows on any computer or phone, and follow it.");
    let more = host::login_args(cx, known);
    let args: Vec<&str> = known
        .signin
        .iter()
        .chain(&more)
        .map(String::as_str)
        .collect();
    run::shown(&cx.ask, path, &args, &auth::env(&cx.home, &known.alias))
}

/// How the owner signs it in later.
pub(super) fn how(known: &Known) -> String {
    let alias = &known.alias;
    let mut text = format!("run tirith connect {alias} in a terminal");
    if !known.token_env.is_empty() {
        text += &format!(
            ", or give it {} on stdin: tirith connect {alias} --token",
            known.token_hint
        );
    }
    text
}
