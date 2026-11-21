//! `tirith`: the daemon (`tirith serve`) and its thin command-line client. Every other command is one HTTP call to the daemon on 127.0.0.1, so it starts in milliseconds.

mod app;
mod attach;
mod bots;
mod chat;
mod cli;
mod client;
mod codex;
mod config;
mod factory;
mod guard;
mod lanes;
mod manager;
mod merge;
mod metrics;
mod models;
mod plan;
mod projects;
mod rooms;
mod rules;
mod sandbox;
mod server;
mod turns;

fn main() {
    cli::main()
}
