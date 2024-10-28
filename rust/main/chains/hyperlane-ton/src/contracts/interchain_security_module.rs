use crate::client::provider::TonProvider;
use crate::contracts::mailbox::TonMailbox;
use crate::traits::ton_api_center::TonApiCenter;
use async_trait::async_trait;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, InterchainSecurityModule, ModuleType, H256, U256,
};
use log::warn;
use num_bigint::BigUint;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;
use tonlib::address::TonAddress;

use crate::types::run_get_method::GetMethodResponse;
use crate::utils::conversion::{hyperlane_message_to_message, metadata_to_cell};
use tonlib::cell::{ArcCell, BagOfCells};
use tonlib::message::TransferMessage;
use tonlib::wallet::TonWallet;
use tracing::info;

pub struct TonInterchainSecurityModule {
    /// The address of the ISM contract.
    pub ism_address: TonAddress,
    /// The provider for the ISM contract.
    pub provider: TonProvider,
    pub wallet: TonWallet,
    pub workchain: i32, // -1 or 0
}

impl Debug for TonInterchainSecurityModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ton mailbox:")
            .field("provider", &self.provider)
            .field("wallet:", &self.wallet.address.to_hex())
            .finish()
    }
}

impl HyperlaneContract for TonInterchainSecurityModule {
    fn address(&self) -> H256 {
        let hex = self.ism_address.to_hex();
        let address = H256::from_slice(hex.as_bytes());
        address
    }
}
impl HyperlaneChain for TonInterchainSecurityModule {
    fn domain(&self) -> &HyperlaneDomain {
        self.provider.domain()
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

#[async_trait]
impl InterchainSecurityModule for TonInterchainSecurityModule {
    async fn module_type(&self) -> ChainResult<ModuleType> {
        let function_name = "get_module_type".to_string();
        let response = self
            .provider
            .run_get_method(self.ism_address.to_hex(), function_name, None)
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError("Failed to run get methdod".to_string())
            });

        match response {
            GetMethodResponse::Success(result) => {
                if let Some(stack_item) = result.stack.get(0) {
                    let module_type_value =
                        u32::from_str_radix(&stack_item.value, 10).map_err(|_| {
                            ChainCommunicationError::CustomError(
                                "Failed to parse module type value".to_string(),
                            )
                        })?;

                    match ModuleType::from_u32(module_type_value) {
                        Some(module_type) => Ok(module_type),
                        None => {
                            warn!("Unknown module type: {}", module_type_value);
                            Ok(ModuleType::Unused)
                        }
                    }
                } else {
                    Err(ChainCommunicationError::CustomError(
                        "No return data for module type".to_string(),
                    ))
                }
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

    async fn dry_run_verify(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
    ) -> ChainResult<Option<U256>> {
        info!("Let's build process");
        let message_t = hyperlane_message_to_message(message).expect("Failed to build");

        let message_cell = message_t.to_cell();

        let metadata_cell = metadata_to_cell(metadata).expect("Failed to get cell");
        info!("Metadata:{:?}", metadata_cell);

        let query_id = 1;
        let block_number = 1;

        let msg = crate::contracts::mailbox::build_message(
            ArcCell::new(message_cell),
            ArcCell::new(metadata_cell),
            query_id,
            block_number,
        )
        .expect("Failed to build message");

        info!("Msg cell:{:?}", msg);

        let transfer_message = TransferMessage {
            dest: self.ism_address.clone(),
            value: BigUint::from(100000000u32),
            state_init: None,
            data: Some(ArcCell::new(msg.clone())),
        }
        .build()
        .expect("Failed to create transferMessage");

        info!("Transfer message:{:?}", transfer_message);

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to build duration_since")
            .as_secs() as u32;

        let seqno = self
            .provider
            .get_wallet_states(self.wallet.address.to_hex())
            .await
            .expect("Failed to get wallet state")
            .wallets[0]
            .seqno as u32;

        let message = self
            .wallet
            .create_external_message(
                now + 60,
                seqno,
                vec![ArcCell::new(transfer_message.clone())],
                false,
            )
            .expect("");

        let boc = BagOfCells::from_root(message.clone())
            .serialize(true)
            .expect("Failed to get boc from root");
        let boc_str = base64::encode(boc.clone());
        info!("create_external_message:{:?}", boc_str);

        let tx = self
            .provider
            .send_message(boc_str)
            .await
            .expect("Failed to get tx");
        info!("Tx hash:{:?}", tx.message_hash);

        //self.wait_for_transaction(tx.message_hash).await;
        todo!()
    }
}
