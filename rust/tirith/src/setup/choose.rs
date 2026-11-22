//! Which agents the owner connects, and which one the work runs on.
use super::{catalog, catalog::Known, context::Context, find};

pub(super) fn agents(cx: &Context, asked: Option<&[String]>) -> Result<Vec<Known>, String> {
    let all = catalog::all();
    if let Some(asked) = asked {
        let names = all.iter().map(|k| k.alias.as_str()).collect::<Vec<_>>();
        return asked
            .iter()
            .map(|a| {
                all.iter()
                    .find(|k| k.alias == *a)
                    .cloned()
                    .ok_or(format!("no agent '{a}': {}", names.join(", ")))
            })
            .collect();
    }
    let cfg = cx.cfg();
    println!("\nWhich agents do you connect?");
    let mut picked = vec![];
    for known in all {
        let here = find::agent(&known, cfg.factory.command(&known.alias).as_deref()).is_some();
        let state = if here {
            "installed"
        } else {
            "not installed yet"
        };
        if cx.ask.yes_no(&format!("  {} ({state})", known.name), here) {
            picked.push(known);
        }
    }
    Ok(picked)
}

/// The agent the managers, bots and lanes run on: the one asked for, else the owner's choice, else the first.
pub(super) fn primary(cx: &Context, connected: &[String], asked: Option<&str>) -> Option<String> {
    let first = connected.first()?;
    if let Some(asked) = asked.filter(|a| connected.iter().any(|c| c == a)) {
        return Some(asked.to_string());
    }
    if connected.len() < 2 {
        return Some(first.clone());
    }
    let question = format!(
        "\nWhich one runs the managers, bots and lanes ({})",
        connected.join(", ")
    );
    let said = cx.ask.text(&question, first);
    Some(if connected.contains(&said) {
        said
    } else {
        first.clone()
    })
}
