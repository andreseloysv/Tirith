//! `tirith service status`: installed, loaded and alive.
use super::manager::{Manager, State};
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub(super) enum Life {
    Alive(u32),
    /// Another daemon has the port, and the service's own retries until it is free.
    Other,
    Silent,
}

/// Alive is the service's own process answering: while it retries on a taken port it has a pid too, so the pid that answers must be that one.
pub(super) fn life(state: &State, answer: Option<&serde_json::Value>) -> Life {
    match (answer, state.pid) {
        (None, _) => Life::Silent,
        (Some(v), Some(pid)) if v["pid"] == pid => Life::Alive(pid),
        (Some(_), _) => Life::Other,
    }
}

fn now(m: &Manager) -> Result<(State, Life), String> {
    let state = (m.state)()?;
    let answer = crate::client::call_json("GET", "/api/state", None).ok();
    let life = life(&state, answer.as_ref());
    Ok((state, life))
}

pub(super) fn status(m: &Manager) -> Result<(), String> {
    let (state, life) = now(m)?;
    show(m, &state, &life);
    Ok(())
}

/// The status once the daemon answers, or after 15 s without an answer.
pub(super) fn after_start(m: &Manager) -> Result<(), String> {
    for _ in 0..75 {
        let (state, life) = now(m)?;
        if life != Life::Silent {
            show(m, &state, &life);
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    status(m)?;
    Err("the service was loaded but tirith does not answer yet: its log says why".into())
}

fn show(m: &Manager, state: &State, life: &Life) {
    let file = m.path();
    let yes = |b: bool| if b { "yes" } else { "no" };
    println!("installed: {} ({})", yes(file.exists()), file.display());
    println!("loaded:    {}", yes(state.loaded));
    let alive = match life {
        Life::Alive(pid) => format!("yes (pid {pid})"),
        Life::Other if state.loaded => {
            "no (a tirith the service did not start has the port: the service takes over when that one stops)".into()
        }
        Life::Other => "no (a tirith runs, but not as the service)".into(),
        Life::Silent => "no".into(),
    };
    println!("alive:     {alive}");
}
