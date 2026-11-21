//! Health metrics computed every minute from the daemon's own data, and the turn-role pass that a metric red for long enough starts (the factory's delayed `metric.red` route names the role: the CEO; the factory's `schedule` route to it runs its daily pass, at 07:00, with no red measure); while everything is green and the day's pass ran, no model is called.
mod api;
mod book;
mod brief;
mod cards;
mod compute;
mod episode;
mod facts;
mod flow;
mod idle;
mod interruptions;
mod metric;
mod mood;
mod pass;
mod plans;
mod review;
mod tick;
mod trouble;
mod unapproved;
mod work;
pub use api::{routes, start};
#[cfg(test)]
#[path = "../../checks/metrics.rs"]
mod tests;
#[cfg(test)]
#[path = "../../checks/unapproved.rs"]
mod unapproved_tests;
