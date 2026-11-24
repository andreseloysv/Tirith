//! A caller's connection, and whether it comes from inside NeoBot's sandbox as the kernel tells it (`sandbox/caller.rs`): nothing from in there is the owner, whatever it holds.
use super::door::Door;
use axum::extract::{connect_info::Connected, ConnectInfo, Request};
use axum::serve::IncomingStream;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Both ends of a caller's connection, and what the kernel said of it: asked once, as it costs some milliseconds.
#[derive(Clone)]
pub struct Caller {
    from: SocketAddr,
    to: SocketAddr,
    confined: Arc<OnceCell<bool>>,
}

impl Connected<IncomingStream<'_, Door>> for Caller {
    fn connect_info(stream: IncomingStream<'_, Door>) -> Self {
        let from = *stream.remote_addr();
        Caller {
            from,
            to: stream.io().local_addr().unwrap_or(from),
            confined: Arc::default(),
        }
    }
}

/// The request's connection; none outside `neobot serve`.
pub(super) fn of(req: &Request) -> Option<Caller> {
    req.extensions()
        .get::<ConnectInfo<Caller>>()
        .map(|info| info.0.clone())
}

impl Caller {
    pub(super) async fn confined(&self) -> bool {
        let (from, to) = (self.from, self.to);
        let look = || async move {
            tokio::task::spawn_blocking(move || crate::sandbox::confined(from, to))
                .await
                .unwrap_or(true)
        };
        *self.confined.get_or_init(look).await
    }
}

/// Whether a request's caller may be the owner at all.
pub(super) async fn outside(caller: &Option<Caller>) -> bool {
    match caller {
        Some(caller) => !caller.confined().await,
        None => true,
    }
}
