//! What proves who calls, kept 0600 in OpenBot's home where no confined process reads or writes it (`sandbox/confine.rs`): the owner's token, the secret every caller's token derives from, and the file that opens the page once.
use crate::os::Mode;
use crate::rooms::sealed::hex;
use rusqlite::Connection;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The owner's token.
pub const OWNER: &str = "owner.token";
const SECRET: &str = "net.secret";
/// What `tirith open` hands the browser: a link that works once.
pub const PAGE: &str = "owner.html";

pub struct Keys {
    pub owner: String,
    pub secret: String,
    /// The secret as the database had it until this start: the proxy alone still takes it, for what that daemon had started.
    pub retired: Option<String>,
}

/// The files no confined process may read.
pub fn kept(home: &Path) -> [PathBuf; 3] {
    [OWNER, SECRET, PAGE].map(|name| home.join(name))
}

impl Keys {
    pub(super) fn open(home: &Path, db: &Connection) -> Keys {
        let (owner, _) = key(&home.join(OWNER));
        let (secret, made) = key(&home.join(SECRET));
        // It is there before anything is confined: on Linux only a file that exists can be hidden.
        let _ = private(&home.join(PAGE));
        let sql = "SELECT value FROM kv WHERE key = 'net_secret'";
        let retired = db.query_row(sql, [], |r| r.get(0)).ok().filter(|_| made);
        let _ = db.execute("DELETE FROM kv WHERE key = 'net_secret'", []);
        Keys {
            owner,
            secret,
            retired,
        }
    }
}

/// The key in `file`, made when it has none, and whether it was made. One that cannot be kept lasts this run alone.
fn key(file: &Path) -> (String, bool) {
    let mut text = String::new();
    let kept = private(file).and_then(|mut f| f.read_to_string(&mut text).map(|_| f));
    if kept.is_ok() && text.trim().len() >= 32 {
        return (text.trim().to_string(), false);
    }
    let new = hex(&rand::random::<[u8; 32]>());
    if let Err(e) = write(file, &new) {
        eprintln!("tirith: cannot keep {}: {e}", file.display());
    }
    (new, true)
}

/// `file` opened as this user's alone, never through a link.
fn private(file: &Path) -> std::io::Result<std::fs::File> {
    let mut open = std::fs::OpenOptions::new();
    open.read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .with_mode(0o600);
    let opened = crate::os::nofollow(&mut open).open(file)?;
    crate::os::set_file_mode(&opened, 0o600)?;
    Ok(opened)
}

/// Writes one of them in place: a new file in its stead would not be the one the sandbox hides.
pub fn write(file: &Path, text: &str) -> std::io::Result<()> {
    let mut kept = private(file)?;
    kept.set_len(0)?;
    kept.write_all(text.as_bytes())
}
