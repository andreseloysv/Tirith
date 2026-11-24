//! `tirith serve`: the daemon.
use crate::app::{App, Shared};
use crate::{
    bots, chat, config, expected, guard, lab, lanes, manager, merge, metrics, models, plan, power,
    rooms, sandbox, server, teach, turns,
};
use std::time::Duration;

pub(super) fn serve() -> Result<(), String> {
    super::login_path::use_login_path();
    // The daemon is nobody's agent, wherever it was started from: what it starts gets a token of its own.
    std::env::remove_var(crate::client::VAR);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let app = App::open_at(config::tirith_home());
        let port = app.cfg().port;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|e| format!("port {port} is taken (is tirith already running?): {e}"))?;
        sandbox::prepare(&app);
        super::hooks_token::at_start(&app);
        metrics::born(&app);
        start_workers(&app);
        println!(
            "tirith: http://127.0.0.1:{port}/ (home {})",
            app.home.display()
        );
        let door = server::door(listener, app.clone());
        let page =
            server::router(app, port).into_make_service_with_connect_info::<server::Caller>();
        axum::serve(door, page).await.map_err(|e| e.to_string())
    })
}

/// Every module's background work, the lane scheduler, the merge worker and the plan flusher.
fn start_workers(app: &Shared) {
    [
        turns::start,
        lab::interrupted,
        sandbox::start,
        rooms::start,
        expected::start, // after the rooms: the phones hear what was cut
        guard::start,
        manager::start, // what a conversation was still to be told
        metrics::start,
        bots::start,
        teach::start,
        chat::start,
        models::start,
        power::start,
    ]
    .into_iter()
    .for_each(|start| start(app.clone()));
    tokio::spawn(lanes::scheduler(app.clone()));
    tokio::spawn(merge::worker(app.clone()));
    tokio::spawn(flush_plans(app.clone()));
}

/// Commits the plans that changed and keeps the event table bounded, every `plan_commit_seconds`.
async fn flush_plans(app: Shared) {
    loop {
        plan::flush(&app, None).await;
        app.trim();
        tokio::time::sleep(Duration::from_secs(
            app.cfg().factory.limits.plan_commit_seconds,
        ))
        .await;
    }
}
