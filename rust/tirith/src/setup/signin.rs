//! Signing an agent in (its own login, which works over SSH, or a token on stdin) and proving it with one real turn.
use super::{auth, catalog::Known, context::Context, daemon, run, secret};
use std::path::Path;
use std::time::Duration;

pub(super) fn token(cx: &Context, known: &Known) -> Result<(), String> {
    let none = || {
        format!(
            "{} takes no token here: sign it in with its own login",
            known.name
        )
    };
    (!known.token_env.is_empty())
        .then_some(())
        .ok_or_else(none)?;
    let value = secret::read(&format!("  Paste {}", known.token_hint))?;
    auth::keep(&cx.home, &known.alias, &known.token_env, &value)?;
    println!(
        "  token kept in {}/auth, for this agent alone",
        cx.home.display()
    );
    Ok(())
}

/// None when only a real turn can tell.
pub(super) fn signed_in(cx: &Context, known: &Known, path: &Path) -> Option<bool> {
    (!known.status.is_empty()).then(|| {
        let args: Vec<&str> = known.status.iter().map(String::as_str).collect();
        let env = auth::env(&cx.home, &known.alias);
        run::quiet(path, &args, &env, Duration::from_secs(30))
    })
}

fn login(cx: &Context, known: &Known, path: &Path) -> bool {
    println!("  Sign in: open the link it shows on any computer or phone, and follow it.");
    let args: Vec<&str> = known.signin.iter().map(String::as_str).collect();
    run::shown(&cx.ask, path, &args, &auth::env(&cx.home, &known.alias))
}

fn how(known: &Known) -> String {
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

/// The model that answered the real turn.
pub(super) fn signed_then_checked(
    cx: &Context,
    known: &Known,
    path: &Path,
) -> Result<String, String> {
    let (name, person) = (&known.name, cx.ask.person() && !known.signin.is_empty());
    let mut asked = false;
    if signed_in(cx, known, path) == Some(false) {
        person
            .then_some(())
            .ok_or_else(|| format!("{name} is not signed in: {}", how(known)))?;
        asked = true;
        login(cx, known, path)
            .then_some(())
            .ok_or_else(|| format!("the sign-in did not end well: {}", how(known)))?;
    }
    loop {
        println!("  checking with one short real turn (the cheap setting)...");
        let got = daemon::check(&known.alias)?;
        if got["ok"] == true {
            return Ok(got["model"].as_str().unwrap_or("").to_string());
        }
        if got["answered"] == true {
            return Err(format!(
                "{name} answered, but Tirith's rules did not hear its tool call: without them it is not connected (tirith doctor says what is missing)"
            ));
        }
        let said = got["said"]
            .as_str()
            .unwrap_or("no answer")
            .replace('\n', " ");
        if asked
            || !person
            || !cx
                .ask
                .yes_no(&format!("  It did not answer ({said}). Sign in now?"), true)
        {
            return Err(format!("{name} did not answer ({said}): {}", how(known)));
        }
        asked = true;
        let _ = login(cx, known, path);
    }
}
