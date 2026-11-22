//! Whether an experiment is ready to be decided, by the rule it started with and by what is still on its way.
use super::rule::Tally;
use super::store::Experiment;
use crate::app::Shared;
use crate::factory::Lab;

/// What an experiment's numbers allow now.
pub(super) enum Due {
    /// Not yet: comparisons, a replay's pairs or days are still to come.
    No,
    /// Decide it by the rule.
    Yes,
    /// Its days ran out with too few comparisons (fewer than min(5, `inputs`)): it proves nothing and is discarded.
    Expired(i64),
}

/// What is still on its way: a run of its bot, a shadow B beside one, or the word of a run's judge, who may call it a failure.
pub(super) fn on_way(app: &Shared, e: &Experiment) -> bool {
    super::seat::seated(&super::seat::live(app, &e.bot))
        || super::shadow_open::running(&super::store::tree(app, e))
        || crate::expected::awaited(app, e.id)
}

/// Whether experiment `e` has every comparison it asked for: it only waits to be decided, so no more of its routine's runs are its own (else runs back to back keep it from ever being decided).
pub(super) fn enough(app: &Shared, e: &Experiment) -> bool {
    let t = super::numbers::tally(app, e.id);
    t.wins + t.ties + t.losses >= i64::from(super::fixed::of(app, e).inputs)
}

/// The comparisons below which an experiment whose days ran out proves nothing.
pub(super) fn minimum(lab: &Lab) -> i64 {
    i64::from(lab.inputs.min(5))
}

/// Whether experiment `e` is due: `inputs` comparisons judged, a replay's pairs all run and judged, or `max_days` since it started (expired when it has too few comparisons then).
pub(super) fn due(app: &Shared, e: &Experiment, t: &Tally, lab: &Lab) -> Due {
    let started = chrono::DateTime::parse_from_rfc3339(&e.created_at).ok();
    let days = started.map_or(0, |s| (chrono::Utc::now() - s.to_utc()).num_days());
    let n = t.wins + t.ties + t.losses;
    let replayed = e.mode == "replay"
        && app.kv(&super::replay::done_key(e.id)).is_some()
        && super::numbers::unjudged(app, e.id) == 0;
    if n >= i64::from(lab.inputs) || replayed {
        Due::Yes
    } else if days >= lab.max_days as i64 {
        if n < minimum(lab) {
            Due::Expired(n)
        } else {
            Due::Yes
        }
    } else {
        Due::No
    }
}
