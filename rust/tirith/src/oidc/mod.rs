//! Team sign-in with an OpenID Connect provider (Microsoft Entra ID): the desktop app signs a person in at the
//! provider and hands their access token to `POST /api/auth/oidc`, which checks it (`verify.rs`: signature
//! against the provider's keys in `jwks.rs`, issuer, audience, times, tenant, app role), keeps the person
//! (`people.rs`) and opens a session for them. The gate (`server/person.rs`) then knows every call as theirs.
mod der;
mod jwks;
mod level;
mod login;
mod people;
mod routes;
mod settings;
mod verify;

pub use level::Level;
pub use people::{by_session, cookie_name, Person};
pub use routes::{own_route, routes};
pub use settings::Settings;

#[cfg(test)]
#[path = "../../checks/oidc_gate.rs"]
mod gate_tests;
#[cfg(test)]
#[path = "../../checks/oidc.rs"]
mod oidc_tests;
