use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};
use thiserror::Error;

#[derive(Debug)]
pub struct CustomHyperlaneError(pub String);

impl Display for CustomHyperlaneError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for CustomHyperlaneError {}

#[derive(Debug, Error)]
pub enum TonProviderError {
    #[error("Failed to fetch latest block: {0}")]
    FetchError(String),
    #[error("No blocks found in the response")]
    NoBlocksFound,
}
