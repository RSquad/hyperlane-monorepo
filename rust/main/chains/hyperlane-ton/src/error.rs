use hyperlane_core::ChainCommunicationError;
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

/// Errors specific to the Hyperlane-TON implementation.
#[derive(Debug, Error)]
pub enum HyperlaneTonError {
    #[error("No account found for the provided address: {0}")]
    AccountNotFound(String),
    /// Toncenter API connection error
    #[error("Failed to connect to Toncenter API: {0}")]
    ApiConnectionError(String),
    /// Invalid response from Toncenter API
    #[error("Invalid response from Toncenter API: {0}")]
    ApiInvalidResponse(String),
    /// Timeout while waiting for API response
    #[error("API response timeout")]
    ApiTimeout,
    /// Error related to API rate limits
    #[error("Rate limit exceeded for Toncenter API")]
    ApiRateLimitExceeded,
    #[error("API request failed")]
    ApiRequestFailed(String),
    /// Error while making a call to a smart contract
    #[error("Contract call failed: {0}")]
    ContractCallError(String),
    /// Insufficient gas
    #[error("Insufficient gas for transaction")]
    InsufficientGas,
    /// Insufficient funds
    #[error("Insufficient funds. Required: {required:?}, available: {available:?}")]
    InsufficientFunds {
        required: u64,
        available: u64,
    },
    /// Data parsing error
    #[error("Data parsing error: {0}")]
    ParsingError(String),
    #[error("Failed to construct URL: {0}")]
    UrlConstructionError(String),
    #[error("No transaction found for the provided hash")]
    TransactionNotFound,
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("No blocks found in the response")]
    NoBlocksFound,
    /// Unknown error
    #[error("Unknown error: {0}")]
    UnknownError(String),
    Timeout,
}

impl From<HyperlaneTonError> for ChainCommunicationError {
    fn from(value: HyperlaneTonError) -> Self {
        ChainCommunicationError::from_other(value)
    }
}
