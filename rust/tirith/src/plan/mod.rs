//! The plan: the project's markdown checklist, written by the daemon only; an item may carry its own `(check: `cmd`)` that the merge queue runs.

mod add;
mod change;
mod check;
mod flush;
mod item;
mod lookup;
mod parse;
mod step;

pub use add::add;
pub use change::drop_item;
pub use flush::{flush, flush_locked};
pub use item::Item;
pub use lookup::{find, read, same, tick_text};
pub use parse::{clean, parse};
pub use step::add_step;

#[cfg(test)]
#[path = "../../checks/plan.rs"]
mod tests;
