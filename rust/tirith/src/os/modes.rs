//! File modes: Unix keeps them; on Windows a user's files are theirs by their folder's ACL and nothing is set.
use std::path::Path;

/// `mode` of `OpenOptions` and `DirBuilder` on Unix; nothing on Windows.
pub trait Mode {
    fn with_mode(&mut self, mode: u32) -> &mut Self;
}

impl Mode for std::fs::OpenOptions {
    #[cfg(unix)]
    fn with_mode(&mut self, mode: u32) -> &mut Self {
        std::os::unix::fs::OpenOptionsExt::mode(self, mode)
    }

    #[cfg(not(unix))]
    fn with_mode(&mut self, _mode: u32) -> &mut Self {
        self
    }
}

impl Mode for std::fs::DirBuilder {
    #[cfg(unix)]
    fn with_mode(&mut self, mode: u32) -> &mut Self {
        std::os::unix::fs::DirBuilderExt::mode(self, mode)
    }

    #[cfg(not(unix))]
    fn with_mode(&mut self, _mode: u32) -> &mut Self {
        self
    }
}

/// Opening refuses a link (`O_NOFOLLOW`); Windows has no such flag and opens what is there.
#[cfg(unix)]
pub fn nofollow(open: &mut std::fs::OpenOptions) -> &mut std::fs::OpenOptions {
    std::os::unix::fs::OpenOptionsExt::custom_flags(open, libc::O_NOFOLLOW)
}

#[cfg(not(unix))]
pub fn nofollow(open: &mut std::fs::OpenOptions) -> &mut std::fs::OpenOptions {
    open
}

/// Sets `path`'s mode (Unix).
#[cfg(unix)]
pub fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode))
}

#[cfg(not(unix))]
pub fn set_mode(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

/// Sets an open file's mode (Unix).
#[cfg(unix)]
pub fn set_file_mode(file: &std::fs::File, mode: u32) -> std::io::Result<()> {
    file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(mode))
}

#[cfg(not(unix))]
pub fn set_file_mode(_file: &std::fs::File, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

/// Whether only its owner may read or write it: no bit for the group or others on Unix; on Windows always (its folder's ACL decides).
#[cfg(unix)]
pub fn private_mode(meta: &std::fs::Metadata) -> bool {
    std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o077 == 0
}

#[cfg(not(unix))]
pub fn private_mode(_meta: &std::fs::Metadata) -> bool {
    true
}
