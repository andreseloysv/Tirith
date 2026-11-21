//! Listening for the proxy's callers.
use super::handle::handle;
use crate::{app::Shared, sandbox::profile::proxy_port};
use std::{sync::Mutex, time::Duration};
use tokio::net::TcpListener;

static STATE: Mutex<String> = Mutex::new(String::new());

pub fn state() -> String {
    STATE.lock().unwrap().clone()
}

/// Listens on 127.0.0.1:<proxy_port> for the daemon's lifetime (retrying while the port is taken).
pub async fn serve(app: Shared) {
    let port = proxy_port(&app);
    let mut told = false;
    loop {
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(listener) => accept(&app, listener).await,
            Err(e) => {
                *STATE.lock().unwrap() = format!("cannot listen on 127.0.0.1:{port}: {e}");
                let _ = (!std::mem::replace(&mut told, true))
                    .then(|| app.event("", "", "error", "proxy", &state(), ""));
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

/// Each caller in its own task, forever.
async fn accept(app: &Shared, listener: TcpListener) {
    *STATE.lock().unwrap() = "listening".into();
    loop {
        let Ok((conn, _)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(handle(app.clone(), conn));
    }
}
