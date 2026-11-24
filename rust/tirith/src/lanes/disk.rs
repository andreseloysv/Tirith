//! Free disk space: how much is left, and one card to the owner when it runs low.
use crate::app::Shared;
use std::path::Path;

pub fn free_gb(dir: &Path) -> f64 {
    crate::os::free_gb(dir).unwrap_or(f64::MAX)
}
/// Ask once per low-disk spell: `asked` is whether a card is already out; it re-arms after the disk recovers (2 GB above the factory's limit).
pub(super) fn low_disk_ask(free: f64, asked: bool) -> Option<bool> {
    let low = crate::factory::current().limits.low_disk_gb;
    (!asked && free < low)
        .then_some(true)
        .or((asked && free >= low + 2.0).then_some(false))
}
pub(super) fn biggest_dirs(dir: &Path, n: usize) -> Vec<(String, u64)> {
    let text = std::process::Command::new("du")
        .arg("-sk")
        .args(
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path()),
        )
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let mut rows: Vec<(String, u64)> = text
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter_map(|(k, p)| Some((p.to_string(), k.trim().parse().ok()?)))
        .collect();
    rows.sort_by_key(|a| std::cmp::Reverse(a.1));
    rows.into_iter().take(n).collect()
}
/// Below the factory's low-disk limit on the disk of the lanes' worktrees: one ask-owner card (kind device, of no project, since every project shares that disk) naming the biggest of OpenBot's own folders.
pub(super) async fn warn_low_disk(app: &Shared) {
    let (home, key, free) = (
        app.home.clone(),
        "low_disk_asked",
        free_gb(&app.home.join("lanes")),
    );
    let Some(ask) = low_disk_ask(free, app.kv(key).is_some()) else {
        return;
    };
    let Some(()) = ask.then_some(()) else {
        return app.set_kv(key, None);
    };
    app.set_kv(key, Some("1"));
    let rows = tokio::task::spawn_blocking(move || biggest_dirs(&home, 5))
        .await
        .unwrap_or_default();
    let list = rows
        .iter()
        .map(|(p, k)| format!("{p} ({:.1} GB)", *k as f64 / 1e6))
        .collect::<Vec<_>>()
        .join(", ");
    let q = format!("Only {free:.1} GB free on this disk: lanes will fail with No space left on device. The biggest of OpenBot's own folders: {list}. Free some space (delete what you no longer need).");
    let _ = crate::manager::ask_owner(app, "", "device", &q, "");
}
