//! A plan's steps and their items, with the line each one is on.
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Item {
    pub text: String,
    pub done: bool,
    /// The command that proves the item is done, from `(check: `cmd`)` in its text.
    pub check: Option<String>,
    #[serde(skip)]
    pub(super) line: usize,
}

#[derive(Serialize, Clone)]
pub struct Step {
    pub num: String,
    pub title: String,
    pub items: Vec<Item>,
    #[serde(skip)]
    pub(super) line: usize,
}
