//! The proxy's address with the credentials of `who`: what the daemon itself goes out through when it reaches somewhere on a caller's behalf, so the proxy counts it as that caller's, never the daemon's.
use super::{profile::proxy_port, token};
use crate::app::Shared;

/// `http://<who>:<token>@127.0.0.1:<proxy port>`, as `wrap` gives a process of `who` in HTTPS_PROXY.
pub fn proxy_url(app: &Shared, who: &str) -> String {
    let (user, token) = (token::user(who), token::issued(app, who));
    format!("http://{user}:{token}@127.0.0.1:{}", proxy_port(app))
}
