//! Linux's confinement: each process in a user, network and mount namespace of its own, where the files that prove who calls read as nothing and only the daemon's port, its proxy's and `[network] loopback` lead out.
mod allow;
mod child;
mod hide;
use super::handover;
mod namespace;
mod ready;
mod relay;
mod run;
mod source;
mod supervise;

pub(super) use allow::allow;
pub(super) use ready::ready;
pub(super) use source::FROM;

/// The running binary itself, also when a newer one took its place on disk.
const SELF: &str = "/proc/self/exe";

/// `program args…` started through the launcher, `ports` leading out and `hidden` unreadable.
#[cfg(not(test))]
pub(super) fn argv(
    ports: &[u16],
    hidden: &[String],
    program: String,
    args: Vec<String>,
) -> (String, Vec<String>) {
    let mut all: Vec<String> = vec!["net".into(), "run".into()];
    all.extend(ports.iter().map(u16::to_string));
    for file in hidden {
        all.extend(["--hide".to_string(), file.clone()]);
    }
    all.extend(["--".to_string(), program]);
    all.extend(args);
    (SELF.into(), all)
}

/// The test binary is no launcher (`ready.rs`).
#[cfg(test)]
pub(super) fn argv(
    _ports: &[u16],
    _hidden: &[String],
    program: String,
    args: Vec<String>,
) -> (String, Vec<String>) {
    let _ = SELF;
    (program, args)
}

/// `net run [port…] [--hide <file>…] -- [program args…]`: the launcher's exit code.
pub(super) fn launcher(words: &[String]) -> i32 {
    let at = words.iter().position(|w| w == "--");
    match (at.and_then(|i| asked(&words[..i])), at) {
        (Some((ports, hidden)), Some(i)) => run::run(&ports, &hidden, &words[i + 1..]),
        _ => {
            eprintln!("usage: neobot net run [port...] [--hide <file>...] -- <program> [args...]");
            2
        }
    }
}

/// The ports and the hidden files of a launcher's words.
fn asked(words: &[String]) -> Option<(Vec<u16>, Vec<String>)> {
    let (mut ports, mut hidden, mut rest) = (vec![], vec![], words.iter());
    while let Some(word) = rest.next() {
        match word.as_str() {
            "--hide" => hidden.push(rest.next()?.clone()),
            port => ports.push(port.parse().ok()?),
        }
    }
    Some((ports, hidden))
}
