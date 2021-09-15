use std::env;

fn db_host() -> Option<String> {
    trace!("db_host() called");
    match env::var("DB_HOST") {
        Ok(v) => Some(v),
        Err(e) => {
            error!("Could not read DB_HOST environment variable: {}", e);
            None
        }
    }
}

pub fn redis_uri() -> String {
    trace!("redis_uri() called");
    match db_host() {
        Some(v) => format!("redis://{}/", v),
        None => {
            warn!("Using default value 127.0.0.1 as db_host returned None");
            "redis://127.0.0.1/".to_string()
        }
    }
}

pub fn hostname() -> String {
    trace!("hostname() called");
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
    trace!("client_id() called");
    match env::var("CLIENT_ID") {
        Ok(v) => v,
        Err(e) => {
            error!("Could not read CLIENT_ID environment variable: {}", e);
            warn!("Using default value liro-test-bot instead");
            "liro-test-bot".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[serial]
    #[test]
    fn db_host_reads_env_var() {
        env::set_var("DB_HOST", "foo");
        assert_eq!(db_host(), Some("foo".into()));
    }

    #[serial]
    #[test]
    fn db_host_returns_none_when_env_var_doesnt_exist() {
        env::remove_var("DB_HOST");
        assert_eq!(db_host(), None);
    }

    #[serial]
    #[test]
    fn redis_uri_reads_env_var() {
        env::set_var("DB_HOST", "foo");
        assert_eq!(redis_uri(), "redis://foo/");
    }

    #[serial]
    #[test]
    fn redis_uri_uses_default_value() {
        env::remove_var("DB_HOST");
        assert_eq!(redis_uri(), "redis://127.0.0.1/");
    }

    #[serial]
    #[test]
    fn hostname_reads_env_var() {
        env::set_var("HOSTNAME", "foo");
        assert_eq!(hostname(), "foo");
    }

    #[serial]
    #[test]
    fn hostname_uses_default_value() {
        env::remove_var("HOSTNAME");
        assert_eq!(hostname(), "http://localhost:8000");
    }

    #[serial]
    #[test]
    fn client_id_reads_env_var() {
        env::set_var("CLIENT_ID", "foo");
        assert_eq!(client_id(), "foo");
    }

    #[serial]
    #[test]
    fn client_id_uses_default_value() {
        env::remove_var("CLIENT_ID");
        assert_eq!(client_id(), "liro-test-bot");
    }
}
