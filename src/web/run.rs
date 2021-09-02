use super::{error, handlers::*};
use crate::db::Pool;
use std::convert::Infallible;
use warp::Filter;

fn with_db(pool: Pool) -> impl Filter<Extract = (Pool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

pub async fn run(pool: Pool) {
    let connect_lichess_route = warp::path!("connect" / "lichess" / u64)
        .and(with_db(pool.clone()))
        .and_then(connect_lichess_handler);

    let oauth_callback_route = warp::path!("oauth" / "callback")
        .and(warp::query::<CallbackParams>())
        .and(with_db(pool.clone()))
        .and_then(oauth_callback_handler);

    let assets_route = warp::path("assets").and(warp::fs::dir("assets"));

    let routes = warp::get()
        .and(
            connect_lichess_route
                .or(oauth_callback_route)
                .or(assets_route),
        )
        .with(warp::log("web"))
        .recover(error::handle_rejection);

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
