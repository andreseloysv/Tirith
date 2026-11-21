//! One side of a replayed run: a worktree of the bot's main (A) or of the branch (B) that is the bot's folder for one turn, its task the capture's start, its memory and skills that worktree's; its row in `turns` says its experiment and variant.
use super::git::{drop_tree, git};
use super::replay::{Replay, REPLAY};
use super::seat::live;
use super::store::{tree, Experiment};
use crate::app::Shared;
use crate::turns::Captured;
use rusqlite::params;

/// Runs side `variant` of capture `cap` in experiment `e`: its turn's id, none when it could not run.
pub(super) async fn side(
    app: &Shared,
    e: &Experiment,
    cap: &Captured,
    variant: &str,
) -> Option<i64> {
    let main = live(app, &e.bot);
    let dir = tree(app, e).join(format!("{}-{}", variant.to_lowercase(), cap.turn));
    let rev = if variant == "B" {
        e.branch.as_str()
    } else {
        "HEAD"
    };
    let _ = std::fs::create_dir_all(dir.parent()?);
    let path = dir.display().to_string();
    git(&main, &["worktree", "add", "-q", "--detach", &path, rev]).ok()?;
    let conv = format!("replay-{}-{}-{variant}", e.id, cap.turn);
    super::answers::open(&dir, &cap.calls);
    let run = async {
        let memory = crate::bots::context(app, &e.bot); // the worktree's memory and skills: it is the bot's folder now
        let input = memory.map_or(String::new(), |m| format!("{m}\n\n")) + &cap.start;
        crate::manager::run_turn(app, &e.bot, &conv, &input).await
    };
    let r = Replay {
        experiment: e.id,
        dir: dir.clone(),
    };
    let _ = REPLAY.scope(r, run).await;
    super::answers::close(&dir);
    drop_tree(&main, &dir);
    let sql = "UPDATE turns SET experiment = ?1, variant = ?2, routine = ?3, \"trigger\" = 'replay' WHERE conv = ?4";
    app.insert(sql, params![e.id, variant, e.routine, conv]);
    let sql = "SELECT MAX(id) FROM turns WHERE conv = ?1";
    let id = app.query(sql, [&conv], |r| r.get::<_, Option<i64>>(0));
    id.into_iter().next().flatten()
}
