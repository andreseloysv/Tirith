//! What checks a train: the files it changes, the stages they ask for and the merged lanes' item checks.
use super::train::{item_check, Riders, Train};
use super::{paths::deploys, paths::wanted};
use crate::app::{git, Shared};
use crate::config::Manifest;
/// The files the train changes against main.
pub(super) async fn changed_files(t: &Train) -> Vec<String> {
    git(&t.slot, &["diff", "--name-only", &t.base, "HEAD"])
        .await
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect()
}
/// The project's non-deploy stages the change asks for (none when only ticks ride), then each merged lane's item check.
pub(super) fn stages_to_run(
    app: &Shared,
    t: &Train,
    manifest: &Manifest,
    riders: &Riders,
    changed: &[String],
) -> Vec<(String, String)> {
    let mut stages: Vec<(String, String)> = if riders.merged.is_empty() {
        vec![]
    } else {
        manifest
            .stages
            .iter()
            .filter(|s| !deploys(s) && wanted(s, changed))
            .map(|s| (s.name.clone(), s.run.clone()))
            .collect()
    };
    stages.extend(
        riders.merged.iter().filter_map(|l| {
            item_check(app, &t.project.id, l).map(|c| (format!("check {}", l.id), c))
        }),
    );
    stages
}
