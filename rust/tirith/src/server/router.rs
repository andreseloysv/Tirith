//! The whole router: every module's routes, the daemon's own, the page, the gate and the same-origin guard.
use super::{admit, origin, page, routes, web::web_file};
use crate::app::Shared;
use crate::{
    attach, bots, chat, guard, lab, metrics, models, numbers, projects, rooms, sandbox, secrets,
    setup, teach,
};
use axum::{middleware, Router};

pub fn router(app: Shared, port: u16) -> Router {
    origin::set_port(port);
    let modules = [
        rooms::routes(),
        sandbox::routes(),
        secrets::routes(),
        crate::gateway::routes(),
        crate::access::routes(),
        crate::connector_page::routes(),
        crate::oauth::routes(),
        models::routes(),
        guard::routes(),
        metrics::routes(),
        attach::routes(),
        projects::routes(),
        bots::routes(),
        numbers::routes(),
        chat::routes(),
        teach::routes(),
        lab::routes(),
        setup::routes(),
        page::routes(),
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
        .layer(middleware::from_fn_with_state(app.clone(), admit::admit))
        .layer(middleware::from_fn(origin::same_origin))
        .with_state(app)
}
