//! The folders workspace-write opens for writing.
use super::{asked::Asked, layers::Layers, settings::Where};
use std::path::{Path, PathBuf};

/// Where workspace-write may write.
pub(super) fn roots(a: &Asked, set: &Layers, dir: &Path, at: &Where) -> Vec<String> {
    let mut all: Vec<PathBuf> = vec![dir.to_path_buf()];
    all.extend(a.adds.iter().map(|d| dir.join(d)));
    let more = set.get("sandbox_workspace_write.writable_roots");
    for root in more
        .as_ref()
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        all.extend(root.as_str().map(|r| dir.join(r)));
    }
    if !set.is("sandbox_workspace_write.exclude_slash_tmp") {
        all.push("/tmp".into());
    }
    if !set.is("sandbox_workspace_write.exclude_tmpdir_env_var") {
        all.extend(at.tmp.map(Path::to_path_buf));
    }
    all.iter().map(|p| p.display().to_string()).collect()
}
