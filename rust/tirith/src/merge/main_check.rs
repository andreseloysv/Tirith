//! One stage on a main commit in the merge slot, remembered per commit.
use super::pipeline::{close_pipeline, open_pipeline};
use super::{locks::MAIN_OK, red::clear_red, slot::reset, stage::run_stage};
use crate::app::Shared;
use crate::config::{Project, Stage};
use std::path::Path;
/// Runs one stage on a main commit in the merge slot (cached per commit unless `fresh`); green clears main_red.
pub(super) async fn main_check(
    app: &Shared,
    project: &Project,
    slot: &Path,
    sha: &str,
    stage: &Stage,
    fresh: bool,
) -> bool {
    let key = (project.id.clone(), sha.to_string(), stage.name.clone());
    if let Some(ok) = MAIN_OK.lock().unwrap().get(&key).filter(|_| !fresh) {
        return *ok;
    }
    let pipe = open_pipeline(app, &project.id, "main", "", sha);
    let ok = reset(slot, sha).await.is_ok()
        && run_stage(app, &pipe, slot, &stage.name, &stage.run)
            .await
            .is_ok();
    close_pipeline(app, &pipe, if ok { "passed" } else { "failed" });
    MAIN_OK.lock().unwrap().insert(key, ok);
    let _ = ok.then(|| clear_red(app, &project.id));
    ok
}
