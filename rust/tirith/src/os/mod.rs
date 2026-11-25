//! What differs between Unix and Windows, in one place: file modes and links, how a child is started and ended, the terminal's echo, the shell and the machine's own facts. On Unix each call does what the code did before; on Windows the nearest thing, or nothing where Windows has none. Signalling processes stays in `app/process.rs`.
mod echo;
mod links;
#[cfg(unix)]
pub mod login_env;
#[cfg(unix)]
pub mod login_run;
mod modes;
mod open;
mod person;
mod shell;
mod spawn;
mod system;

pub use echo::NoEcho;
#[cfg(windows)]
pub use links::exts;
pub use links::{link_any, link_dir, runs, unlink};
pub use modes::{nofollow, private_mode, set_file_mode, set_mode, Mode};
pub use open::open;
pub use person::full_name;
pub use shell::sh;
pub use spawn::{detach, own_group, parent_id, signal_of, terminate};
pub use system::{free_gb, hostname, is_root, load, uid};
