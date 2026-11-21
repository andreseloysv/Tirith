//! Yesterday's sections of config.toml read under today's names, in memory only.

/// Yesterday's key -> today's: `[models.*]` and `[ceo]` become `[roles.*]` and `[metrics.*]`, the `[lanes]`/`[merge]`/`[plan]` limits become `[limits]`.
const MOVES: &str = concat!("models.manager>roles.manager;models.workers>roles.worker;roles.workers>roles.worker;roles.lead>roles.manager-general;models.lead>roles.manager-general;roles.impr", "over>roles.ceo;models.impr", "over>roles.ceo;roles.reviewer>roles.ceo;models.ceo>roles.ceo;ceo.agent>roles.ceo.agent;ceo.model>roles.ceo.model;ceo.effort>roles.ceo.effort;ceo.fallback>roles.ceo.fallback;ceo.max_per_day>roles.ceo.max_per_day;ceo.review_after_minutes>roles.ceo.review_after_minutes;ceo.targets.merged_per_day>metrics.merged.target;ceo.targets.median_merge_minutes>metrics.median.target;ceo.targets.first_try_pct>metrics.first_try.target;ceo.targets.idle_minutes>metrics.idle.target;ceo.targets.stuck>metrics.stuck.target;ceo.targets.main_red>metrics.main_red.target;lanes.max_load>limits.max_load;lanes.inactivity_minutes>limits.inactivity_minutes;lanes.long_running_minutes>limits.long_running_minutes;lanes.max_parallel>limits.max_parallel;autopilot.max_parallel>limits.max_parallel;merge.stage_timeout_minutes>limits.stage_timeout_minutes;plan.commit_seconds>limits.plan_commit_seconds");

/// Takes the value at `path` out, and its section when that leaves it empty.
fn take(t: &mut toml::Table, path: &[&str]) -> Option<toml::Value> {
    let (first, rest) = path.split_first()?;
    let Some(()) = (!rest.is_empty()).then_some(()) else {
        return t.remove(*first);
    };
    let child = t.get_mut(*first)?.as_table_mut()?;
    let v = take(child, rest)?;
    let _ = child.is_empty().then(|| t.remove(*first));
    Some(v)
}

/// Puts `v` (yesterday's value) at `path`, under anything already written there under its new name.
fn put(t: &mut toml::Table, path: &[&str], v: toml::Value) {
    if path.len() == 1 {
        let e = t.entry(path[0]).or_insert_with(|| v.clone());
        let old = std::mem::replace(e, v);
        crate::factory::merge(e, &old);
    } else if let Some(child) = t
        .entry(path[0])
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
    {
        put(child, &path[1..], v);
    }
}

/// Moves yesterday's sections under their new names (a key already written under its new name wins); true when anything moved.
pub fn migrate(t: &mut toml::Table) -> bool {
    MOVES
        .split(';')
        .filter_map(|m| m.split_once('>'))
        .filter_map(|(from, to)| {
            take(t, &from.split('.').collect::<Vec<_>>())
                .map(|v| put(t, &to.split('.').collect::<Vec<_>>(), v))
        })
        .count()
        > 0
}
