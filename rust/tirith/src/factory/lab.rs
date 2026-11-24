//! The factory's `[lab]`: the rule an experiment is decided by, fixed by the factory before any test (docs/BOTS.md, part 3).
use serde::Deserialize;

#[derive(Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Lab {
    /// Comparisons an experiment waits for before NeoBot decides.
    pub inputs: u32,
    /// The share of comparisons B must win or tie to be no worse.
    pub not_worse_share: f64,
    /// The share of the mean cost or time B must save to be adopted.
    pub min_saving: f64,
    /// Days after which an experiment is decided with the comparisons it has.
    pub max_days: u64,
}

impl Default for Lab {
    fn default() -> Self {
        Lab {
            inputs: 8,
            not_worse_share: 0.5,
            min_saving: 0.15,
            max_days: 7,
        }
    }
}

/// The `[lab]` section of a merged factory table, strictly read: an unknown key or a share outside 0 to 1 makes the version invalid.
pub(super) fn read(raw: &toml::Table) -> Result<Lab, String> {
    let lab: Lab = raw
        .get("lab")
        .cloned()
        .map(|v| v.try_into())
        .transpose()
        .map_err(|e| format!("factory: lab: {e}"))?
        .unwrap_or_default();
    let share = |v: f64| (0.0..=1.0).contains(&v);
    let ok = share(lab.not_worse_share) && share(lab.min_saving) && lab.inputs > 0;
    ok.then_some(lab)
        .ok_or("factory: lab: inputs is at least 1, the shares are from 0 to 1".into())
}
