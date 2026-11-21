//! The managers: one long-lived agent session per project (and one for what belongs to no project); its text, tool calls and turn ends go straight to that project's chat.
mod answer;
mod ask;
mod blocks;
mod cards;
mod deliver;
mod evals;
mod events;
mod exec;
mod exec_stream;
mod interrupt;
mod judged;
mod open;
mod progress;
mod prompt;
mod run;
mod say;
mod session;
mod session_end;
mod spawn;
mod stale;
mod stream;
mod talk;
mod turn;
mod typing;
pub use answer::owner_answer;
pub use ask::ask_owner;
pub use cards::close_card;
pub use deliver::deliver;
pub use events::{emit_event, emitted};
pub use interrupt::interrupt;
pub use judged::card_id_of;
pub use open::owner_open;
pub use say::{from_room, model_command, say};
pub use spawn::spawn_chat;
pub use stale::close_stale_questions;
#[cfg(test)]
pub use stale::stale_reason; // only the lanes' checks call it from outside
pub use talk::{busy, end_session, restart};
pub use turn::run_turn;
pub use typing::typing_heartbeats;
#[cfg(test)]
use {
    judged::judged, open::waiting_cards, progress::progress_line, spawn::turn_permission,
    talk::talk, typing::typing_rooms,
};
#[cfg(test)]
#[path = "../../checks/manager.rs"]
mod tests;
