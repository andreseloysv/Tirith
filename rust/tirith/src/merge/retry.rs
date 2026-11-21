//! Retry from the page: a merge pipeline's failed lanes go back to the queue; a main pipeline runs its checks again on the current main; a deploy pipeline deploys the current main.
use super::{deploy::deploy, locks::BUSY, main_check::main_check, pipeline::pipeline};
use super::{slot::merge_slot, slot::requeue};
use crate::app::{git, Lane, Shared};
use crate::config::Project;
use serde_json::Value;
pub fn retry(app: &Shared, id: &str) -> Result<(), String> {
    let p = pipeline(app, id);
    (!(p.is_null()))
        .then_some(())
        .ok_or_else(|| format!("no pipeline '{id}'"))?;
    if p["kind"] == "merge" {
        return retry_lanes(app, &p);
    }
    let project = app
        .cfg()
        .project(p["project"].as_str().unwrap_or(""))
        .cloned()
        .ok_or("unknown project")?;
    let names: Vec<String> = p["stages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["stage"].as_str().map(String::from))
        .collect();
    let (app, deploying) = (app.clone(), p["kind"] == "deploy");
    tokio::spawn(async move {
        let _busy = BUSY.lock().await;
        let Some(()) = (!deploying).then_some(()) else {
            return deploy(&app, &project, &[]).await;
        };
        recheck_main(&app, &project, &names).await;
    });
    Ok(())
}
/// The failed lanes of a merge pipeline go back to the queue with their attempts reset.
fn retry_lanes(app: &Shared, p: &Value) -> Result<(), String> {
    let failed: Vec<Lane> = p["lanes"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .filter_map(|l| app.lane(l))
        .filter(|l| l.status == "failed")
        .collect();
    (!failed.is_empty())
        .then_some(())
        .ok_or("no lane of this pipeline is failed".to_string())?;
    failed
        .iter()
        .for_each(|l| drop(app.update_lane(&l.id, |x| x.attempts = 0)));
    requeue(app, &failed, false, "retried");
    app.merge_wake.notify_one();
    Ok(())
}
/// The named stages run again, fresh, on main as it is now.
async fn recheck_main(app: &Shared, project: &Project, names: &[String]) {
    let manifest = app.manifest(&project.repo());
    let (Ok(slot), Ok(sha)) = (
        merge_slot(project, &manifest).await,
        git(&project.repo(), &["rev-parse", "HEAD"]).await,
    ) else {
        return;
    };
    for st in manifest.stages.iter().filter(|s| names.contains(&s.name)) {
        main_check(app, project, &slot, &sha, st, true).await;
    }
}
