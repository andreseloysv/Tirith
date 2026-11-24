//! Replay mode: for each of the routine's last `inputs` captured runs, A (the bot's main) and B (the branch) do its task again, each in a throwaway worktree, with the network closed but to its agent's model (the sandbox's proxy, `reach.rs`) and every call that reads outside answered from the capture (`answers.rs`). Each pair is compared like a live one and decided by the same `[lab]` rule. Nothing of a replay reaches the owner: its turns run in a conversation of their own that no chat, phone, room or card knows (`replay-<experiment>-<turn>-<A|B>`), and the hook refuses `neobot say`, `ask` and `ask-owner` in it.
use super::store::Experiment;
use crate::app::Shared;
use std::path::PathBuf;

/// The replay a turn runs in: its experiment, and the worktree that is the bot's folder for it.
#[derive(Clone)]
pub(super) struct Replay {
    pub experiment: i64,
    pub routine: String,
    pub dir: PathBuf,
}

tokio::task_local! {
    pub(super) static REPLAY: Replay;
}

/// Inside a replay's turn, the worktree every folder of the bot resolves to.
pub(super) fn dir() -> Option<PathBuf> {
    REPLAY.try_with(|r| r.dir.clone()).ok()
}

/// Inside a replay's turn, its worktree and the routine it runs.
pub(super) fn run() -> Option<(PathBuf, String)> {
    REPLAY.try_with(|r| (r.dir.clone(), r.routine.clone())).ok()
}

/// Inside a replay's turn, its experiment.
pub(super) fn replaying() -> Option<i64> {
    REPLAY.try_with(|r| r.experiment).ok()
}

/// The proxy's caller for a process started inside a replay's turn, `replay:<experiment>:<who>`; else `who`.
pub fn proxy_caller(who: &str) -> String {
    replaying().map_or(who.to_string(), |e| format!("replay:{e}:{who}"))
}

/// True for the proxy's caller of a replay: the world is closed to it (`reach.rs` says what is not).
pub fn closed(who: &str) -> bool {
    who.starts_with("replay:")
}

/// The kv key that says the pairs of replay `id` were all run.
pub(super) fn done_key(id: i64) -> String {
    format!("lab:replayed:{id}")
}

/// Runs the pairs of experiment `e` on the captured runs `turns`, one after another, then decides it when it is due.
pub(super) fn launch(app: &Shared, e: Experiment, turns: Vec<i64>) {
    let app = app.clone();
    tokio::spawn(async move {
        if let Some(first) = turns.first() {
            super::replay_warm::warm(&app, &e, *first).await;
        }
        for (n, turn) in turns.into_iter().enumerate() {
            pair(&app, &e, turn, n % 2 == 1).await; // neither side always goes first
        }
        app.set_kv(&done_key(e.id), Some("done"));
        super::decide::decide(&app, e.id);
    });
}

/// One captured run done again by A and by B, the first of them B when `b_first`, and B compared with that A.
async fn pair(app: &Shared, e: &Experiment, turn: i64, b_first: bool) {
    let Some(cap) = crate::turns::capture_of(app, turn) else {
        return;
    };
    let order = if b_first { ["B", "A"] } else { ["A", "B"] };
    let Some(first) = super::replay_side::side(app, e, &cap, order[0]).await else {
        return;
    };
    let Some(second) = super::replay_side::side(app, e, &cap, order[1]).await else {
        return;
    };
    let ((a, a_void), (b, b_void)) = if b_first {
        (second, first)
    } else {
        (first, second)
    };
    // A side refused what the capture lacks, or unheard by the hook: the pair proves nothing, and is neither a tie nor a win.
    match a_void.or(b_void) {
        None => super::compare::compare_pair(app, e, a, b),
        Some(why) => super::void::void(app, e, (a, b), &why),
    }
}
