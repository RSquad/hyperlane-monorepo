use url::Url;

#[derive(Clone, Debug)]
pub struct TonConnectionConf {
    pub url: Url,
    pub api_key: String,
}

#[derive(thiserror::Error, Debug)]
pub enum TonConnectionConfError {
    /// Missing `url` for connection configuration
    #[error("Missing `url` for connection configuration")]
    MissingConnectionUrl,
    /// Missing `api_key` for connection configuration
    #[error("Missing `api_key` for connection configuration")]
    MissingApiKey,
    /// Invalid `url` for connection configuration
    #[error("Invalid `url` for connection configuration: `{0}` ({1})")]
    InvalidConnectionUrl(String, url::ParseError),
}

impl TonConnectionConf {
    pub fn new(url: Url, api_key: String) -> Self {
        Self { url, api_key }
    }
}
