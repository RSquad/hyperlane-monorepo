use crate::client::provider::TonProvider;
use crate::traits::ton_api_center::TonApiCenter;
use crate::types::run_get_method::GetMethodResponse;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, MultisigIsm, H256,
};
use log::warn;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use tonlib::address::TonAddress;

#[derive(Clone, Debug)]
pub struct TonMultisigIsm {
    provider: TonProvider,
    multisig_address: TonAddress,
}

impl HyperlaneChain for TonMultisigIsm {
    fn domain(&self) -> &HyperlaneDomain {
        &self.provider.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

impl HyperlaneContract for TonMultisigIsm {
    fn address(&self) -> H256 {
        let hex = self.multisig_address.to_hex();
        let bytes = hex.as_bytes();
        H256::from_slice(bytes)
    }
}

impl MultisigIsm for TonMultisigIsm {
    async fn validators_and_threshold(
        &self,
        message: &HyperlaneMessage,
    ) -> ChainResult<(Vec<H256>, u8)> {
        let function_name = "get_validators_and_threshold".to_string();
        let response = self
            .provider
            .run_get_method(self.multisig_address.to_hex(), function_name, None)
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError("Failed to run get methdod".to_string())
            });

        match response {
            GetMethodResponse::Success(result) => {
                let threshold = if let Some(threshold_item) = result.stack.get(0) {
                    threshold_item.value.parse::<u8>().map_err(|_| {
                        ChainCommunicationError::CustomError(
                            "Failed to parse threshold value".to_string(),
                        )
                    })?
                } else {
                    return Err(ChainCommunicationError::CustomError(
                        "No threshold data found".to_string(),
                    ));
                };

                let validators: ChainResult<Vec<H256>> = result
                    .stack
                    .iter()
                    .skip(1)
                    .map(|validator_item| {
                        H256::from_str(&validator_item.value).map_err(|_| {
                            ChainCommunicationError::CustomError(
                                "Failed to parse validator address".to_string(),
                            )
                        })
                    })
                    .collect();

                Ok((validators?, threshold))
            }
            GetMethodResponse::Error(error) => {
                warn!("Error encountered: {:?}", error);
                Err(ChainCommunicationError::CustomError(format!(
                    "Error response: {}",
                    error.error
                )))
            }
        }
    }
}
