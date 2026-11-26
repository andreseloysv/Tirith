//! Signing in to a NeoBot team server from the desktop app: the server says which provider (Microsoft Entra ID)
//! and which client id; the person signs in at the provider in the system browser (Authorization Code with PKCE, a
//! public client, a loopback redirect: RFC 8252); the app trades the code for an access token and hands that token
//! to the server, which checks it and opens a session. The token is used once and dropped: no refresh token is
//! asked for (no `offline_access`), so nothing of the provider's is kept on disk or in the keychain; when the
//! session ends the person signs in again.
mod flow;
mod loopback;
pub mod pkce;
mod server;

pub use flow::{sign_in, Session};
