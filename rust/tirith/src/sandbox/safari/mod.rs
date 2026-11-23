//! `tirith safari open|read|click|type`: the daemon drives the owner's Safari for the chat roles.
mod act;
mod route;
mod run;
pub mod script;

pub use route::safari_route;

#[cfg(test)]
#[path = "../../../checks/safari_act.rs"]
mod act_checks;
