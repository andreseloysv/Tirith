//! Team sign-in with an OpenID Connect provider (Microsoft Entra ID): the desktop app signs a person in at the
//! provider and hands their access token to `POST /api/auth/oidc`, which checks it (`verify.rs`: signature
//! against the provider's keys in `jwks.rs`, issuer, audience, times, tenant, app role), keeps the person
//! (`people.rs`) and opens a session for them. A browser signs in itself: `/auth/login` sends it to the provider and
//! `/auth/callback` (`browser.rs`) trades the code for that same token. The gate (`server/person.rs`) then knows every call as theirs.
mod browser;
mod der;
mod flow;
mod gate;
mod jwks;
mod level;
mod login;
mod pending;
mod people;
mod redeem;
mod reply;
mod routes;
mod session;
mod settings;
mod verify;

pub use gate::sign_in_redirect;
pub use level::Level;
pub use people::{by_session, cookie_name, Person};
pub use routes::{own_route, routes};
pub use settings::Settings;

#[cfg(test)]
#[path = "../../checks/oidc_browser.rs"]
mod browser_tests;
#[cfg(test)]
#[path = "../../checks/oidc_gate.rs"]
mod gate_tests;
#[cfg(test)]
#[path = "../../checks/oidc.rs"]
mod oidc_tests;
