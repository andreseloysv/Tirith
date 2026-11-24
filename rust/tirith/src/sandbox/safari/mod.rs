//! `tirith safari open|read|click|type`: the daemon drives the owner's Safari for the chat roles; and `tirith browser`,
//! the browser they act in, as the owner chose (OpenBot's own or their Safari).
mod act;
mod choice;
mod login;
mod route;
mod run;
pub mod script;

pub use route::safari_route;

#[cfg(test)]
#[path = "../../../checks/safari_act.rs"]
mod act_checks;
