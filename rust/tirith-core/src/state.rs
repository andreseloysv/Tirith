//! `state.json` schema types (`collector/state.schema.json`). Only the sections that are ported are typed;
//! each lane adds its own section here as it ports a module (see `docs/RUST-PORT.md`).

use serde::{Deserialize, Serialize};

pub const STATE_VERSION: u32 = 1;

/// One item of `state.projects[].roadmap.steps[].items`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoadmapItem {
    pub text: String,
    pub done: bool,
    /// `agent`, `owner`, `date` or `out`.
    pub who: String,
    /// The day a `date` item unblocks (`YYYY-MM-DD`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// The id of the owner item raised for an open owner-only item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_item: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoadmapStep {
    pub id: String,
    pub title: String,
    /// `empty|done|active|ready|blocked-owner|blocked-date|todo`.
    pub status: String,
    pub done: usize,
    pub total: usize,
    pub items: Vec<RoadmapItem>,
    /// Ids of the agents working on this step.
    pub agents: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
}

/// `state.projects[].roadmap`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roadmap {
    pub file: String,
    pub deadline: Option<String>,
    pub done: usize,
    pub total: usize,
    pub steps: Vec<RoadmapStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanItem {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub status: String,
    pub done: usize,
    pub total: usize,
    pub items: Vec<PlanItem>,
    pub agents: Vec<String>,
}

/// One entry of `state.projects[].plans`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub current: bool,
    pub file: String,
    pub deadline: Option<String>,
    pub done: usize,
    pub total: usize,
    pub finished: bool,
    pub steps: Vec<PlanStep>,
}

/// One row of `state.projects` (ported keys only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub color: String,
    pub folder: Option<String>,
    pub auto: bool,
    pub roadmap: Option<Roadmap>,
    pub plans: Vec<Plan>,
}

/// The `state.json` payload (ported sections only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub version: u32,
    pub generated_at: String,
    pub projects: Vec<ProjectRow>,
}
