use std::env;

fn db_host() -> Option<String> {
    match env::var("DB_HOST") {
        Ok(v) => Some(v),
        Err(e) => {
            error!("Could not read DB_HOST environment variable: {}", e);
            None
        }
    }
}

pub fn redis_uri() -> String {
    match db_host() {
        Some(v) => format!("redis://{}/", v),
        None => {
            warn!("Using default value 127.0.0.1 as db_host returned None");
            "redis://127.0.0.1/".to_string()
        }
    }
}

pub fn hostname() -> String {
    match env::var("HOSTNAME") {
        Ok(v) => v,
        Err(e) => {
            error!("Could not read HOSTNAME environment variable: {}", e);
            warn!("Using default value localhost:8000 instead");
            "http://localhost:8000".to_string()
        }
    }
}

pub fn client_id() -> String {
    match env::var("CLIENT_ID") {
        Ok(v) => v,
        Err(e) => {
            error!("Could not read CLIENT_ID environment variable: {}", e);
            warn!("Using default value liro-test-bot instead");
            "liro-test-bot".to_string()
        }
    }
}
