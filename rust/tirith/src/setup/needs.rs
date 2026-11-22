//! What this machine must have before the agents can work: the programs, a git identity, the sandbox.
use super::{context::Context, find, packages, run};

pub(super) const PROGRAMS: [&str; 3] = ["git", "curl", "bash"];

pub(super) fn missing() -> Vec<&'static str> {
    PROGRAMS
        .into_iter()
        .filter(|p| find::program(p).is_none())
        .collect()
}

pub(super) fn git_identity() -> bool {
    ["user.name", "user.email"]
        .into_iter()
        .all(|k| run::output("git", &["config", "--get", k]).is_some_and(|v| !v.is_empty()))
}

/// Programs and the identity Tirith commits with; what is still missing.
pub(super) fn machine(cx: &Context) -> Vec<String> {
    let gone = missing();
    if !gone.is_empty() {
        if let Err(why) = packages::provide(&cx.ask, &gone) {
            return vec![why];
        }
    }
    if git_identity() {
        return vec![];
    }
    println!("Git has no name and email on this machine, and the agents' commits need both.");
    let name = cx.ask.text("  Name for the commits", "Tirith");
    let email = cx.ask.text("  Email for the commits", "tirith@localhost");
    let set = |k: &str, v: &str| run::output("git", &["config", "--global", k, v]).is_some();
    if set("user.name", &name) && set("user.email", &email) {
        println!("  commits are made as {name} <{email}>");
        vec![]
    } else {
        vec!["git has no identity: git config --global user.name and user.email".into()]
    }
}

/// The network sandbox, as the daemon sees it: why not, when it is off.
pub(super) fn sandbox_off() -> Option<String> {
    let said = crate::client::call_json("GET", "/api/net/status", None).ok()?;
    (said["sandbox"] != true).then(|| said["why"].as_str().unwrap_or("off").to_string())
}

/// The sandbox on, allowed with sudo where Ubuntu asks for that; else what to do.
pub(super) fn sandbox(cx: &Context) -> Vec<String> {
    let Some(mut why) = sandbox_off() else {
        println!("Network sandbox: on.");
        return vec![];
    };
    println!("Network sandbox: off ({why}).");
    let sudo = find::program("sudo").filter(|_| why.contains("net allow") && cx.ask.person());
    if let Some(sudo) = sudo {
        let exe = cx.exe.display().to_string();
        let yes = cx
            .ask
            .yes_no("  Allow it now (sudo asks for your password)?", true);
        if yes && run::shown(&cx.ask, &sudo, &[&exe, "net", "allow"], &[]) {
            std::thread::sleep(std::time::Duration::from_secs(16)); // the daemon asks again after 15 s
            match sandbox_off() {
                None => return vec![],
                Some(now) => why = now,
            }
        }
    }
    vec![format!("the network sandbox is off: {why}")]
}
