//! `tirith`: the daemon (`neobot serve`) and its thin command-line client. Every other command but the guide's (`setup`, `connect`, `doctor`, `service`) is one HTTP call to the daemon on 127.0.0.1, so it starts in milliseconds.

mod access;
mod app;
mod attach;
mod bots;
mod bundle;
mod chat;
mod cli;
mod client;
mod codex;
mod config;
mod connector_page;
mod connectors;
mod expected;
mod factory;
mod gateway;
mod guard;
mod lab;
mod lanes;
mod manager;
mod merge;
mod metrics;
mod models;
mod numbers;
mod oauth;
mod os;
mod plan;
mod power;
mod projects;
mod rooms;
mod rules;
mod sandbox;
mod secrets;
mod server;
mod service;
mod setup;
mod status;
mod teach;
mod turns;

#[cfg(all(test, unix))]
#[path = "../checks/tidy.rs"]
mod tidy;

fn main() {
    cli::main()
}
