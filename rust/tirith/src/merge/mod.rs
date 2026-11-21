//! The merge queue: up to five lanes of one project ride one train in the project's merge slot; only a green train moves main (fast-forward), then the deploy stages run on that main.
mod batch;
mod deploy;
mod env;
mod land;
mod landed;
mod limits;
mod locks;
mod main_check;
mod news;
mod paths;
mod pipeline;
mod prepare;
mod red;
mod red_train;
mod retry;
mod riders;
mod slot;
mod stage;
mod train;
mod train_checks;
mod worker;
pub use pipeline::list;
pub use retry::retry;
pub use worker::worker;
#[cfg(test)]
#[path = "../../checks/merge.rs"]
mod tests;
