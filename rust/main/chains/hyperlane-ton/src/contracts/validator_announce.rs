use crate::client::provider::TonProvider;
use crate::signer::signer::TonSigner;
use crate::traits::ton_api_center::TonApiCenter;
use crate::types::run_get_method::GetMethodResponse;
use async_trait::async_trait;
use hyperlane_core::{
    Announcement, ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract,
    HyperlaneDomain, HyperlaneProvider, SignedType, TxOutcome, ValidatorAnnounce, H256, U256,
};
use log::warn;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use tonlib::{
    address::TonAddress,
    cell::{ArcCell, BagOfCells, Cell, CellBuilder},
    message::TransferMessage,
    mnemonic::Mnemonic,
    wallet::{TonWallet, WalletVersion},
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
        let response = self
            .provider
            .run_get_method(self.address.to_hex(), function_name, None)
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError("Failed to run get methdod".to_string())
            });

        match response {
            GetMethodResponse::Success(result) => {
                let locations = result
                    .stack
                    .into_iter()
                    .map(|item| {
                        item.to_string()
                            .split(',')
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                    })
                    .collect::<Vec<Vec<String>>>();

                Ok(locations)
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

    async fn announce(&self, announcement: SignedType<Announcement>) -> ChainResult<TxOutcome> {
        todo!()
    }

    async fn announce_tokens_needed(&self, announcement: SignedType<Announcement>) -> Option<U256> {
        todo!()
    }
}
