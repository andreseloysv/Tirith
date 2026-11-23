//! Signing a bot in to a remote connector: `tirith connector login <id> --bot <bot>`. The daemon finds the server's OAuth metadata (`.well-known`), registers itself when it must, sends the owner's browser to the sign-in with PKCE and a state, takes the code on its own 127.0.0.1 callback alone, and keeps the tokens in that bot's secret store under reserved names. The gateway reads and renews them itself, for that connector alone (docs/DAEMON.md).
mod api;
mod callback;
mod cli;
mod discover;
mod fresh;
mod keep;
mod names;
mod pending;
mod pkce;
mod register;
mod start;
mod token;

pub use api::{route, routes};
pub use callback::own_route;
pub use cli::cli;
pub use fresh::bearer;
pub use names::{all as sign_in_names, reserved, signed_in};

/// The daemon's own client for OAuth: straight to the server (no agent's proxy), never following a redirect.
fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())
}
