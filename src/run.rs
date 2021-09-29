use crate::{bot, db, lichess, web};

pub async fn run() {
    trace!("run() called");
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let pool = db::connect().await.expect("Couldn't connect to pool");
    let lichess = lichess::Client::new();

    tokio::select! {
        _ = web::run(&pool, &lichess) => {
            info!("Web server exited.");
        }

        _ = bot::run(&pool, &lichess) => {
            info!("Bot client exited.");
        }
    }
}
