//! Starting the sandbox's part of the daemon.
use super::{blocks, proxy, wrappers::write_wrappers};
use crate::app::Shared;

/// Starts the proxy inside the daemon, with the block list and the tool wrappers ready.
pub fn start(app: Shared) {
    blocks::load(&app);
    write_wrappers(&app);
    #[cfg(target_os = "macos")]
    super::nested::start(&app);
    proxy::seed(&app);
    tokio::spawn(proxy::serve(app));
}
