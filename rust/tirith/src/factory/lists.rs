//! The closed lists a factory is checked against: its sections, its measures and the events the code emits.
pub(super) const SECTIONS: [&str; 8] = [
    "agents", "roles", "limits", "plan", "cards", "metrics", "routes", "evals",
];

pub(super) const MEASURES: [&str; 11] = [
    "merged",
    "median",
    "first_try",
    "idle",
    "stuck",
    "main_red",
    "failed",
    "owner",
    "interruptions",
    "needless_cards",
    "satisfaction",
];

/// The closed list of events the code emits, `;`-joined like the moves in config.rs; a route on anything else never fires, so it is invalid. The page offers a bot's routes from this list (but `schedule`: a bot's routines are its schedule routes).
pub(crate) const EVENTS: &str = "owner.message;room.message;lane.asked;lane.failed;lane.not_started;lane.long;merge.blocked;merge.landed;main.red;main.green;plan.empty;deploy.done;deploy.failed;metric.red;review.done;schedule;card.opened;card.answered;card.closed;factory.invalid;turn.done;routine.done";

/// True when `on` is one of the events the code emits.
pub(super) fn is_event(on: &str) -> bool {
    EVENTS.split(';').any(|e| e == on)
}
