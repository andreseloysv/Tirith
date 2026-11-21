//! Lanes: one agent, one plan item, one worktree slot of its own on its own branch (removed the moment the lane ends for good); the autopilot starts open plan items, and a lane that ends goes to the merge queue, where the checks decide.
mod ask;
mod autopilot;
mod blocked;
mod brief;
mod brief_parts;
mod builds;
mod cleanup;
mod delete;
mod disk;
mod ended;
mod exclude;
mod failed;
mod heal;
mod idle;
mod ignored;
mod keep;
mod launch;
mod live;
mod logscan;
mod long;
mod pick;
mod scheduler;
mod send_back;
mod session;
mod skip;
mod spawn;
mod stamp;
mod start;
mod stop;
mod sweep;
mod watch;
mod worktree;

pub use ask::{answer, ask};
pub use autopilot::autopilot_on;
pub use blocked::paused_fix;
pub use builds::shared_target;
pub use cleanup::{cleanup_lane, cleanup_later};
pub use delete::delete;
pub use disk::free_gb;
pub use live::{drain, tell};
pub use scheduler::scheduler;
pub use send_back::send_back;
pub use skip::is_autopilot_skipped;
pub use start::{slug, start};
pub use stop::stop;

#[cfg(test)]
#[path = "../../checks/lanes.rs"]
mod tests;
