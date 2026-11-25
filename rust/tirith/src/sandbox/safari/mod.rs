//! `neobot safari open|read|click|type`: the daemon drives the owner's Safari for the chat roles; on Windows the
//! same four commands under `neobot chrome` and `neobot edge` drive their Chrome or Edge (`uia.rs`); and
//! `neobot browser`, the browser they act in, as the owner chose (NeoBot's own or their own one).
mod act;
mod choice;
mod login;
mod missing;
mod route;
mod run;
pub mod script;
mod uia;
mod uia_run;

pub use route::safari_route;

#[cfg(test)]
#[path = "../../../checks/safari_act.rs"]
mod act_checks;
