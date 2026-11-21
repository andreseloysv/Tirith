//! The guard at the daemon's start.
use super::{install, intact};
use crate::app::Shared;

/// Installs the guard where it is missing (never over a foreign hook); an event for each project that needs attention.
pub fn start(app: Shared) {
    tokio::task::spawn_blocking(move || {
        for p in app.cfg().projects.iter() {
            let installed = if intact(&app, p).is_err_and(|e| e.contains("is missing")) {
                install(&app, p)
            } else {
                Ok(())
            };
            let _ = installed
                .and_then(|_| intact(&app, p))
                .map_err(|why| app.event("", &p.id, "guard", "", &why, ""));
        }
    });
}
