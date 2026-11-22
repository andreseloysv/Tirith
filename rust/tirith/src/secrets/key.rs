//! The fallback's private files and the daemon's key material in them: a folder of 0700, each file 0600 and written whole or not at all.
use rand::RngCore;
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

const FILE: &str = "key";

/// The key, if there is one. One that others could read is refused.
pub(super) fn read(dir: &Path) -> Result<Option<[u8; 32]>, String> {
    let path = dir.join(FILE);
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(failed(e)),
    };
    (meta.is_file() && meta.permissions().mode() & 0o077 == 0)
        .then_some(())
        .ok_or_else(|| format!("{} must be a file of mode 0600", path.display()))?;
    std::fs::read(&path)
        .map_err(failed)?
        .try_into()
        .map(Some)
        .map_err(|_| format!("{} is damaged", path.display()))
}

/// The key, made on the first value that needs one.
pub(super) fn made(dir: &Path) -> Result<[u8; 32], String> {
    if let Some(key) = read(dir)? {
        return Ok(key);
    }
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    write(dir, FILE, &key)?;
    Ok(key)
}

pub(super) fn write(dir: &Path, file: &str, bytes: &[u8]) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("{}: {e}", dir.join(file).display());
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(failed)?;
    std::fs::set_permissions(dir, PermissionsExt::from_mode(0o700)).map_err(failed)?;
    let new = dir.join(format!(".{file}.{:016x}", rand::random::<u64>()));
    let done = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&new)
        .and_then(|mut out| out.write_all(bytes).and_then(|_| out.sync_all()))
        .and_then(|_| std::fs::rename(&new, dir.join(file)));
    done.map_err(|e| {
        let _ = std::fs::remove_file(&new);
        failed(e)
    })
}
