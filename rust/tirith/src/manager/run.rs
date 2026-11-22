//! A conversation's sessions, one after another while messages wait, started again more slowly when they fail young.
use super::deliver::report;
use super::jobs::release_run;
use super::session::session;
use super::talk::{set_busy, talk};
use super::typing::typing_stop;
use crate::app::Shared;
use std::time::Duration;
pub(super) async fn run(app: Shared, project: String) {
    let mut backoff = 2;
    loop {
        let started = std::time::Instant::now();
        let result = session(&app, &project).await;
        result
            .as_ref()
            .err()
            .filter(|_| backoff == 2)
            .into_iter()
            .for_each(|e| (app.event("", &project, "error", "manager", e, ""), report(&app, &project, &format!("The manager could not start: {e}. Tirith keeps trying; what you wrote waits for it."))).1);
        let _ = result.is_err().then(|| release_run(&app, &project)); // a session that never started held what its run took
        typing_stop(&project); // a session that ended, failed or never got ready leaves no turn typing on the phones
        let rest = talk(&project, |t| {
            (t.tx, t.pid, t.in_run) = (None, 0, false);
            t.text.clear();
            set_busy(&app, &project, t, false);
            t.alive = !t.pending.is_empty();
            !t.alive
        });
        let Some(()) = (!rest).then_some(()) else {
            return;
        };
        // A session that ended young is started again more and more slowly; one that lived is started again at once.
        backoff = if result.is_ok() || started.elapsed().as_secs() > 60 {
            2
        } else {
            (backoff * 2).min(60)
        };
        tokio::time::sleep(Duration::from_secs(if backoff > 2 { backoff } else { 0 })).await;
    }
}
