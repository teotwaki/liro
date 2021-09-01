use crate::{bot, db, web};

pub async fn run() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let pool = db::connect().await.expect("Couldn't connect to pool");

    tokio::select! {
        _ = web::run(pool) => {
            info!("Web server exited.");
        }

        _ = bot::run() => {
            info!("Bot client exited.");
        }
    }
}
