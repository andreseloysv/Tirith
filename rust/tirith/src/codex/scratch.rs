//! The small files of one exec turn, in its folder: named for that turn alone (a role's turns can run at the same time) and gone with it.
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct Scratch {
    files: Vec<PathBuf>,
}

impl Scratch {
    /// A new path in `dir` for a file of this turn, ending in `name`.
    pub fn path(&mut self, dir: &Path, name: &str) -> PathBuf {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join(format!(".turn-{:016x}.{name}", rand::random::<u64>()));
        self.files.push(path.clone());
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        self.files
            .iter()
            .for_each(|f| drop(std::fs::remove_file(f)));
    }
}
