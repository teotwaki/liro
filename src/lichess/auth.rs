use crate::config;

pub fn oauth_url<C, S>(code_challenge: C, state: S) -> String
where
    C: AsRef<str>,
    S: AsRef<str>,
{
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
        code_challenge.as_ref(),
        state.as_ref()
    )
}
