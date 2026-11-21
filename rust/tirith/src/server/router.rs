//! The whole router: every module's routes, the daemon's own, the page and the same-origin guard.
use super::{origin, routes, web::web_file};
use crate::app::Shared;
use crate::{attach, bots, chat, guard, metrics, models, projects, rooms, sandbox};
use axum::{middleware, Router};

pub fn router(app: Shared, port: u16) -> Router {
    origin::set_port(port);
    let modules = [
        rooms::routes(),
        sandbox::routes(),
        models::routes(),
        guard::routes(),
        metrics::routes(),
        attach::routes(),
        projects::routes(),
        bots::routes(),
        chat::routes(),
    ];
    let own = [
        routes::state(),
        routes::lanes(),
        routes::owner(),
        routes::runs(),
        routes::talk(),
    ];
    modules
        .into_iter()
        .chain(own)
        .fold(Router::new(), Router::merge)
        .fallback(web_file)
        .layer(middleware::from_fn(origin::same_origin))
        .with_state(app)
}
