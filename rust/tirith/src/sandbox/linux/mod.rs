//! Linux's confinement: each process in a user and network namespace of its own, where only the daemon's port, its proxy's and `[network] loopback` lead out.
mod allow;
mod child;
mod handover;
mod namespace;
mod ready;
mod relay;
mod run;
mod supervise;

pub(super) use allow::allow;
pub(super) use ready::ready;

/// The running binary itself, also when a newer one took its place on disk.
const SELF: &str = "/proc/self/exe";

/// `program args…` started through the launcher, `ports` leading out.
#[cfg(not(test))]
pub(super) fn argv(ports: &[u16], program: String, args: Vec<String>) -> (String, Vec<String>) {
    let mut all: Vec<String> = vec!["net".into(), "run".into()];
    all.extend(ports.iter().map(u16::to_string));
    all.extend(["--".to_string(), program]);
    all.extend(args);
    (SELF.into(), all)
}

/// The test binary is no launcher (`ready.rs`).
#[cfg(test)]
pub(super) fn argv(_ports: &[u16], program: String, args: Vec<String>) -> (String, Vec<String>) {
    let _ = SELF;
    (program, args)
}

/// `net run [port…] -- [program args…]`: the launcher's exit code.
pub(super) fn launcher(words: &[String]) -> i32 {
    let at = words.iter().position(|w| w == "--");
    let ports = at.map(|i| {
        words[..i]
            .iter()
            .map(|p| p.parse::<u16>())
            .collect::<Result<Vec<_>, _>>()
    });
    match (ports, at) {
        (Some(Ok(ports)), Some(i)) => run::run(&ports, &words[i + 1..]),
        _ => {
            eprintln!("usage: tirith net run [port...] -- <program> [args...]");
            2
        }
    }
}
