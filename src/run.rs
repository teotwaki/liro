use crate::{bot, db, web};

pub async fn run() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    openssl_probe::init_ssl_cert_env_vars();

    let pool = db::connect().await.expect("Couldn't connect to pool");

    tokio::select! {
        _ = web::run(&pool) => {
            info!("Web server exited.");
        }

        _ = bot::run(&pool) => {
            info!("Bot client exited.");
        }
    }
}
