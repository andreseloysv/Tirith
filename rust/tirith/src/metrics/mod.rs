//! Health metrics computed every minute from the daemon's own data, and the passes of the turn roles: the one a metric red for long enough starts (the factory's delayed `metric.red` route names the role: the CEO), and any turn role's on its `schedule` route (the CEO's daily one at 07:00, the Coach's weekly one) or on an event routed to it (the Coach's `routine.worse`, which the measures emit too), each with the brief its `brief` key names; while everything is green and no pass is due, no model is called.
mod api;
mod book;
mod bots_brief;
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
mod roles;
mod tick;
mod trouble;
mod unapproved;
mod work;
mod worse;
pub use api::{routes, start};
pub use roles::heard;
#[cfg(test)]
#[path = "../../checks/coach.rs"]
mod coach_tests;
#[cfg(test)]
#[path = "../../checks/metrics.rs"]
mod tests;
#[cfg(test)]
#[path = "../../checks/unapproved.rs"]
mod unapproved_tests;
