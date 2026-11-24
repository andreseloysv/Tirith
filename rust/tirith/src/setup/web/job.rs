//! One agent's setup as the page sees it, while it runs and after: its step, the last words of the program it runs, the sign-in link and code, its end.
use super::seen;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// What a program printed that is kept: enough for a link and a code, never a whole installer's log.
const KEPT: usize = 16 * 1024;

#[derive(Serialize, Clone, Default)]
pub struct Job {
    /// machine, install, signin or check.
    pub step: String,
    pub running: bool,
    pub ok: bool,
    pub error: String,
    /// The last lines the running program printed, plain.
    pub said: String,
    pub link: String,
    pub code: String,
    /// It waits for a code pasted from the browser.
    pub paste: bool,
    #[serde(skip)]
    text: String,
}

static JOBS: LazyLock<Mutex<HashMap<String, Job>>> = LazyLock::new(Default::default);

pub(super) fn with<T>(alias: &str, f: impl FnOnce(&mut Job) -> T) -> T {
    let mut all = JOBS.lock().unwrap_or_else(|e| e.into_inner());
    f(all.entry(alias.to_string()).or_default())
}

/// Starts one; refused while that agent's runs.
pub fn start(alias: &str) -> Result<(), String> {
    with(alias, |job| {
        (!job.running).then_some(()).ok_or("its setup runs")?;
        *job = Job {
            running: true,
            ..Job::default()
        };
        Ok(())
    })
}

/// The next step: what the last program printed is gone.
pub fn step(alias: &str, step: &str) {
    with(alias, |job| {
        let running = job.running;
        *job = Job {
            step: step.into(),
            running,
            ..Job::default()
        };
    });
}

/// More of what the running program printed.
pub fn heard(alias: &str, more: &str) {
    with(alias, |job| {
        job.text.push_str(&seen::plain(more));
        if job.text.len() > KEPT {
            let cut = job.text.len() - KEPT;
            let cut = (cut..job.text.len())
                .find(|i| job.text.is_char_boundary(*i))
                .unwrap_or(0);
            job.text.drain(..cut);
        }
        let lines: Vec<&str> = job.text.lines().filter(|l| !l.trim().is_empty()).collect();
        job.said = lines[lines.len().saturating_sub(4)..].join("\n");
        job.link = seen::link(&job.text).unwrap_or_default();
        job.code = seen::code(&job.text).unwrap_or_default();
        job.paste = seen::wants_paste(&job.text);
    });
}

pub fn end(alias: &str, done: Result<(), String>) {
    super::input::release(alias);
    with(alias, |job| {
        job.running = false;
        job.ok = done.is_ok();
        job.error = done.err().unwrap_or_default();
        job.paste = false;
    });
}

pub fn get(alias: &str) -> Option<Job> {
    let all = JOBS.lock().unwrap_or_else(|e| e.into_inner());
    all.get(alias)
        .filter(|j| j.running || !j.step.is_empty())
        .cloned()
}
