//! The end of the guide: what is connected, what is left, and how to open Tirith.
use super::context::Context;

pub(super) fn show(cx: &Context, connected: &[String], pending: &[String]) {
    println!("\n----------------------------------------");
    if connected.is_empty() {
        println!("Tirith runs, but no agent is connected yet: it can do no work.");
    } else {
        println!("Tirith is ready. Connected: {}.", connected.join(", "));
    }
    if !pending.is_empty() {
        println!("\nStill to do:");
        pending.iter().for_each(|p| println!("  - {p}"));
    }
    open(cx.cfg().port);
    println!("\nThen add a project:  tirith project add <folder>");
    let on_path = super::find::program("tirith")
        .is_some_and(|p| std::fs::canonicalize(p).ok() == std::fs::canonicalize(&cx.exe).ok());
    if let Some(dir) = cx.exe.parent().filter(|_| !on_path) {
        println!(
            "(`tirith` is not on your PATH yet: export PATH=\"{}:$PATH\")",
            dir.display()
        );
    }
}

/// The page is on this machine's loopback only: from another one, an SSH tunnel with the same port at both ends.
fn open(port: u16) {
    let page = format!("http://127.0.0.1:{port}/");
    let ssh = std::env::var("SSH_CONNECTION").ok();
    let remote =
        ssh.is_some() || (cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_none());
    if !remote {
        println!("\nOpen {page}");
        return;
    }
    let at = ssh
        .as_deref()
        .and_then(|s| s.split(' ').nth(2))
        .map(String::from)
        .or_else(|| super::run::output("hostname", &[]))
        .unwrap_or_else(|| "this-server".into());
    let user = super::run::output("id", &["-un"]).unwrap_or_else(|| "you".into());
    println!("\nOn your own computer (the page never listens on the network):");
    println!("  ssh -N -L {port}:127.0.0.1:{port} {user}@{at}");
    println!("  then open {page}");
}
