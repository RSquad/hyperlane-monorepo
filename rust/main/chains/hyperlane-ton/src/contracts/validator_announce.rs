use crate::client::provider::TonProvider;
use crate::signer::signer::TonSigner;
use crate::traits::ton_api_center::TonApiCenter;
use crate::utils::conversion::ConversionUtils;
use async_trait::async_trait;
use hyperlane_core::{
    Announcement, ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract,
    HyperlaneDomain, HyperlaneProvider, SignedType, TxOutcome, ValidatorAnnounce, H160, H256, U256,
};

use std::fmt::{Debug, Formatter};
use tonlib_core::{
    cell::{ArcCell, BagOfCells, Cell, CellBuilder},
    message::TransferMessage,
    mnemonic::Mnemonic,
    wallet::{TonWallet, WalletVersion},
    TonAddress,
};

pub struct TonValidatorAnnounce {
    address: TonAddress,
    provider: TonProvider,
    signer: TonSigner,
}

impl Debug for TonValidatorAnnounce {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TonValidatorAnnounce")
            .field("workchain:", &self.address.workchain)
            .field("address:", &self.address.to_hex())
            .field("provider:", &self.provider)
            .field("signer", &"<signer omitted>")
            .finish()
    }
}

impl TonValidatorAnnounce {
    pub fn new(provider: TonProvider, address: TonAddress, signer: TonSigner) -> Self {
        Self {
            address,
            provider,
            signer,
        }
    }
}

impl HyperlaneContract for TonValidatorAnnounce {
    fn address(&self) -> H256 {
        let hex = self.address.to_hex();
        let bytes = hex.as_bytes();
        H256::from_slice(bytes)
    }
}

impl HyperlaneChain for TonValidatorAnnounce {
    fn domain(&self) -> &HyperlaneDomain {
        self.provider.domain()
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

#[async_trait]
impl ValidatorAnnounce for TonValidatorAnnounce {
    async fn get_announced_storage_locations(
        &self,
        validators: &[H256],
    ) -> ChainResult<Vec<Vec<String>>> {
        let function_name = "get_announced_storage_locations".to_string();
        let validators_h160: Vec<H160> = validators
            .iter()
            .map(|v| {
                // Assuming H256 to H160 conversion just takes the first 20 bytes
                H160::from_slice(&v.as_bytes()[..20])
            })
            .collect();

        let validators_cell = ConversionUtils::create_address_linked_cells(&validators_h160)
            .map_err(|_| {
                ChainCommunicationError::CustomError(
                    "Failed to create address linked cells".to_string(),
                )
            })?;

        let boc = BagOfCells::from_root(validators_cell)
            .serialize(true)
            .map_err(|e| {
                ChainCommunicationError::CustomError("Failed to create BagOfCells".to_string())
            })?;
        let boc_str = base64::encode(&boc);

        let stack = Some(vec![boc_str]);

        let response = self
            .provider
            .run_get_method(self.address.to_hex(), function_name, stack)
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError("Failed to run get methdod".to_string())
            })?;

        if response.exit_code != 0 {
            return Err(ChainCommunicationError::CustomError(format!(
                "Non-zero exit code in response: {}",
                response.exit_code
            )));
        }

        if let Some(stack_item) = response.stack.get(0) {
            // Assuming `StackItem` has a field `value` that is the base64-encoded cell BOC
            let cell_boc = base64::decode(&stack_item.value).map_err(|_| {
                ChainCommunicationError::CustomError(
                    "Failed to decode cell BOC from response".to_string(),
                )
            })?;

            let cell_boc_decoded = base64::decode(&stack_item.value).map_err(|_| {
                ChainCommunicationError::CustomError(
                    "Failed to decode cell BOC from response".to_string(),
                )
            })?;

            let boc = BagOfCells::parse(&cell_boc_decoded).map_err(|e| {
                ChainCommunicationError::CustomError(format!("Failed to parse BOC: {}", e))
            })?;

            let cell = boc.single_root().map_err(|e| {
                ChainCommunicationError::CustomError(format!("Failed to get root cell: {}", e))
            })?;

            let storage_locations = ConversionUtils::parse_address_storage_locations(&cell)
                .map_err(|e| {
                    ChainCommunicationError::CustomError(format!(
                        "Failed to parse address storage locations: {}",
                        e
                    ))
                })?;

            // Convert HashMap<BigUint, Vec<String>> to Vec<Vec<String>>
            let locations_vec: Vec<Vec<String>> = storage_locations.into_values().collect();

            Ok(locations_vec)
        } else {
            Err(ChainCommunicationError::CustomError(
                "Empty stack in response".to_string(),
            ))
        }
    }

    async fn announce(&self, announcement: SignedType<Announcement>) -> ChainResult<TxOutcome> {
        todo!()
    }

    async fn announce_tokens_needed(&self, announcement: SignedType<Announcement>) -> Option<U256> {
        todo!()
    }
}
