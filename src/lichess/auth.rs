use crate::config;

pub fn oauth_url(code_challenge: &str, state: &str) -> String {
    format!(
        "https://lichess.org/oauth\
             ?response_type=code\
             &redirect_uri={}/oauth/callback\
             &client_id={}\
             &code_challenge_method=S256\
             &code_challenge={}\
             &state={}",
        config::hostname(),
        config::client_id(),
        code_challenge,
        state
    )
}
