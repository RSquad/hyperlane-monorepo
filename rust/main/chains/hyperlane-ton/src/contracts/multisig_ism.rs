use crate::client::provider::TonProvider;
use crate::run_get_method::StackItem;
use crate::traits::ton_api_center::TonApiCenter;
use async_trait::async_trait;
use derive_new::new;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, MultisigIsm, H256,
};
use num_bigint::BigUint;
use std::str::FromStr;
use tonlib_core::{
    cell::{BagOfCells, CellBuilder},
    TonAddress,
};
use tracing::info;

#[derive(Clone, Debug, new)]
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

#[async_trait]
impl MultisigIsm for TonMultisigIsm {
    async fn validators_and_threshold(
        &self,
        message: &HyperlaneMessage,
    ) -> ChainResult<(Vec<H256>, u8)> {
        info!("validators_and_threshold call");
        let domain = message.origin;
        let mut builder = CellBuilder::new();

        info!("Domain:{:?}", domain);
        let id = builder
            .store_uint(32, &BigUint::from(domain))
            .unwrap()
            .build()
            .unwrap();
        info!("Id:{:?}", id);

        let stack = Some(vec![StackItem {
            r#type: "num".to_string(),
            value: domain.to_string(),
        }]);
        info!("Stack:{:?}", stack);

        let function_name = "get_validators_and_threshhold".to_string();
        let response = self
            .provider
            .run_get_method(self.multisig_address.to_hex(), function_name, stack)
            .await;

        if let Ok(response) = response {
            info!(
                "Response runGetMethod for validators_and_threshold: {:?}",
                response
            );
            if let Some(stack_item_validators) = response.stack.get(0) {
                let validators_hex = stack_item_validators.value.trim_start_matches("0x");
                let mut validators = Vec::new();

                for chunk in validators_hex.as_bytes().chunks(64) {
                    if let Ok(hex_str) = std::str::from_utf8(chunk) {
                        if let Ok(val) = H256::from_str(hex_str) {
                            validators.push(val);
                        } else {
                            return Err(ChainCommunicationError::CustomError(
                                "Failed to parse validator address".to_string(),
                            ));
                        }
                    } else {
                        return Err(ChainCommunicationError::CustomError(
                            "Invalid UTF-8 sequence in validator address".to_string(),
                        ));
                    }
                }

                if let Some(stack_item_threshold) = response.stack.get(1) {
                    if let Ok(threshold) = u8::from_str_radix(&stack_item_threshold.value[2..], 16)
                    {
                        info!(
                            "Parsed validators: {:?}, threshold: {:?}",
                            validators, threshold
                        );
                        Ok((validators, threshold))
                    } else {
                        Err(ChainCommunicationError::CustomError(
                            "Failed to parse threshold".to_string(),
                        ))
                    }
                } else {
                    Err(ChainCommunicationError::CustomError(
                        "Missing threshold in stack response".to_string(),
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
}
