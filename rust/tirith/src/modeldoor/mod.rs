//! The model door: an agent never holds the provider key. Where an agent's auth file holds one (`COPILOT_PROVIDER_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) and the door is on (`[door] enabled`, on by default in server mode), its processes get a token valid for that caller alone in place of the key and the daemon's `/door/<alias>` as the base URL. The route forwards to that alias's own base URL, model paths only, with the real key in the provider's header, streams the answer back unchanged and records each call: who, the model, the status, the tokens and the dollars, against a daily budget of the machine and of the role.
mod api;
mod back;
mod build;
mod doctor;
mod env;
mod forward;
mod on;
mod paths;
mod record;
mod route;
mod spec;
mod tally;

pub use api::{cli, routes};
pub use doctor::lines;
pub use env::given;

#[cfg(test)]
#[path = "../../checks/model_door.rs"]
mod tests;
