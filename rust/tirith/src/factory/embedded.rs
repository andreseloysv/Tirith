//! The repo's factory, built into the binary: what runs when no factory folder is found, and what every folder's file is laid over.
use std::collections::BTreeMap;

pub(crate) const TEXT: &str = include_str!("../../../../factory/factory.toml");

/// The embedded prompts and eval rubrics, by file stem.
pub(crate) fn embedded_prompts() -> BTreeMap<String, String> {
    [
        (
            "manager",
            include_str!("../../../../factory/prompts/manager.md"),
        ),
        (
            "one-project",
            include_str!("../../../../factory/prompts/one-project.md"),
        ),
        (
            "no-project",
            include_str!("../../../../factory/prompts/no-project.md"),
        ),
        (
            "brief",
            include_str!("../../../../factory/prompts/brief.md"),
        ),
        (
            "clippy",
            include_str!("../../../../factory/prompts/clippy.md"),
        ),
        ("ceo", include_str!("../../../../factory/prompts/ceo.md")),
        (
            "coach",
            include_str!("../../../../factory/prompts/coach.md"),
        ),
        ("bot", include_str!("../../../../factory/prompts/bot.md")),
        ("skill", include_str!("../../../../factory/skill.md")),
        (
            "owner-mood",
            include_str!("../../../../factory/evals/owner-mood.md"),
        ),
        (
            "card-needed",
            include_str!("../../../../factory/evals/card-needed.md"),
        ),
        (
            "turn-quality",
            include_str!("../../../../factory/evals/turn-quality.md"),
        ),
        (
            "unapproved",
            include_str!("../../../../factory/evals/unapproved.md"),
        ),
        (
            "ab-compare",
            include_str!("../../../../factory/evals/ab-compare.md"),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}
