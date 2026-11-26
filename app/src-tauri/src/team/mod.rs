//! A team server instead of this Mac: the window shows that server's page, signed in as the person with the company's
//! account (`tirith-signin`). No local daemon is started or needed while one is shown; "This Mac" goes back to it.
mod browser;
pub mod commands;
mod store;
mod view;

pub use view::{menu, start_url, title};
