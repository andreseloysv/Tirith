//! `tirith safari open|read`: the daemon drives the owner's Safari for the chat roles.
mod route;
mod run;
pub mod script;

pub use route::safari_route;
