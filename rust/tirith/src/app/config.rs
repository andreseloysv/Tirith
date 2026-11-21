//! The config and the projects' manifests, re-read only when they changed on disk.
use super::App;
use crate::config::{Config, Manifest};
use std::path::Path;
use std::sync::Arc;

impl App {
    pub fn cfg(&self) -> Arc<Config> {
        self.cfg.read().unwrap().clone()
    }

    /// Re-reads `config.toml` and the factory when either changed on disk or main moved (a few stats, see `factory::stamp`) or when `force`d; true when re-read. A bad factory version keeps the last good one (see `factory::load`).
    pub fn reload_config(&self, force: bool) -> bool {
        let now = (
            std::fs::metadata(self.home.join("config.toml"))
                .and_then(|m| m.modified())
                .ok(),
            crate::factory::stamp(&self.cfg().raw),
        );
        let mut last = self.seen.cfg_mtime.lock().unwrap();
        let Some(()) = (*last != now || force).then_some(()) else {
            return false;
        };
        *last = now;
        drop(last);
        *self.cfg.write().unwrap() = Arc::new(Config::load_at(&self.home));
        true
    }

    /// A project's `tirith.toml`, re-read only when its mtime changed.
    pub fn manifest(&self, repo: &Path) -> Manifest {
        let stamp = std::fs::metadata(repo.join("tirith.toml"))
            .and_then(|m| m.modified())
            .ok();
        let mut cache = self.seen.manifests.lock().unwrap();
        cache
            .get(repo)
            .filter(|(m, _)| *m == stamp)
            .map(|(_, man)| man.clone())
            .unwrap_or_else(|| {
                let man = Manifest::load(repo);
                cache.insert(repo.to_path_buf(), (stamp, man.clone()));
                man
            })
    }
}
