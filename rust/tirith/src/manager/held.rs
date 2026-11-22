//! What a routine's run says waits for its end: on the phones only when the run was good and has news, once its judge read it (`expected/news.rs`). A failed one is the one line of `expected/`.
use super::talk::talk;
use crate::app::Shared;
use crate::rooms;

/// The turn of `project` said `text`: to the phones now when the turn is marked for them, held when it is a routine's run.
pub(super) fn say(app: &Shared, project: &str, text: &str) {
    let now = talk(project, |t| {
        if t.phones && t.in_run {
            t.said.push(text.to_string());
        }
        t.phones && !t.in_run
    });
    if now {
        rooms::to_phones(app, project, text);
    }
}

/// The run of `project` ended, `good` or not: what it held that is news, none when it ends saying that nothing is new.
pub fn run_over(project: &str, good: bool) -> Option<String> {
    let said = talk(project, |t| std::mem::take(&mut t.said));
    let news = said
        .last()
        .is_some_and(|last| !crate::expected::quiet(last));
    (good && news).then(|| said.join("\n\n"))
}
