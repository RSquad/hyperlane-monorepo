use crate::client::provider::TonProvider;
use crate::traits::ton_api_center::TonApiCenter;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, Mailbox, TxCostEstimate, TxOutcome, H256, U256,
};
use std::fmt::{Debug, Formatter};
use std::num::NonZeroU64;
use tonlib::address::TonAddress;
use tracing::{debug, info, instrument, warn};

pub struct TonMailbox {
    pub(crate) mailbox_address: TonAddress,
    pub(crate) provider: TonProvider,
}

impl HyperlaneContract for TonMailbox {
    fn address(&self) -> H256 {
        let hex = self.mailbox_address.to_hex();
        let bytes = hex.as_bytes();
        H256::from_slice(bytes)
    }
}

impl HyperlaneChain for TonMailbox {
    fn domain(&self) -> &HyperlaneDomain {
        &self.provider.domain()
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

impl Debug for TonMailbox {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Mailbox for TonMailbox {
    async fn count(&self, lag: Option<NonZeroU64>) -> ChainResult<u32> {
        let response = self
            .provider
            .run_get_method(
                self.mailbox_address.to_string(),
                "get_nonce".to_string(),
                Some(vec![]),
            )
            .await;

        if let Ok(response) = response {
            if let Some(stack_item) = response.stack.get(0) {
                if let Ok(count) = u32::from_str_radix(&stack_item.value[2..], 16) {
                    Ok(count)
                } else {
                    Err(ChainCommunicationError::CustomError(
                        "Failed to parse count".to_string(),
                    ))
                }
            } else {
                Err(ChainCommunicationError::CustomError(
                    "Empty stack in response".to_string(),
                ))
            }
        } else {
            Err(ChainCommunicationError::CustomError(
                "Failed to get response".to_string(),
            ))
        }
    }

    #[instrument(skip(self))]
    async fn delivered(&self, id: H256) -> ChainResult<bool> {
        let response = self
            .provider
            .run_get_method(
                self.mailbox_address.to_hex(),
                "get_deliveries".to_string(),
                None,
            )
            .await
            .expect("Some error");

        let is_delivered = response.stack.iter().any(|item| {
            let stored_id = item.value.as_str().unwrap_or("");
            if stored_id == format!("{:x}", id) {
                return Ok(true);
            }
        });
        Ok(is_delivered)
    }

    async fn default_ism(&self) -> ChainResult<H256> {
        let response = self
            .provider
            .run_get_method(
                self.mailbox_address.to_hex(),
                "get_default_ism".to_string(),
                None,
            )
            .await
            .expect("Some error");

        if let Some(stack) = response.stack.first() {
            if stack.r#type == "cell" {
                let decoded_value = base64::decode(&stack.value).map_err(|e| {
                    ChainCommunicationError::CustomError(format!(
                        "Failed to decode base64: {:?}",
                        e
                    ))
                })?;

                if decoded_value.len() >= 32 {
                    let ism_hash = H256::from_slice(&decoded_value[0..32]);
                    return Ok(ism_hash);
                } else {
                    return Err(ChainCommunicationError::CustomError(
                        "Decoded value is too short for H256".to_string(),
                    ));
                }
            } else {
                return Err(ChainCommunicationError::CustomError(
                    "Unexpected data type in stack, expected cell".to_string(),
                ));
            }
        }

        Err(ChainCommunicationError::CustomError(
            "No data in stack".to_string(),
        ))
    }

    async fn recipient_ism(&self, recipient: H256) -> ChainResult<H256> {
        todo!()
    }

    async fn process(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
        tx_gas_limit: Option<U256>,
    ) -> ChainResult<TxOutcome> {
        todo!()
    }

    async fn process_estimate_costs(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
    ) -> ChainResult<TxCostEstimate> {
        todo!()
    }

    fn process_calldata(&self, message: &HyperlaneMessage, metadata: &[u8]) -> Vec<u8> {
        todo!()
    }
}
