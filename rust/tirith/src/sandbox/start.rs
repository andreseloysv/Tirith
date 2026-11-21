//! Starting the sandbox's part of the daemon.
use super::{blocks, proxy, token::load_secret, wrappers::write_wrappers};
use crate::app::Shared;

/// Starts the proxy inside the daemon, with the block list, the token secret and the tool wrappers ready.
pub fn start(app: Shared) {
    blocks::load(&app);
    load_secret(&app);
    write_wrappers(&app);
    proxy::seed(&app);
    tokio::spawn(proxy::serve(app));
}
