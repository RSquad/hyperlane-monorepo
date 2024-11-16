use async_trait::async_trait;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, InterchainSecurityModule, ModuleType, H256, U256,
};
use log::warn;
use num_bigint::BigUint;
use num_traits::cast::FromPrimitive;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use tonlib_core::message::{CommonMsgInfo, InternalMessage, TonMessage};
use tonlib_core::{
    cell::{ArcCell, BagOfCells},
    message::TransferMessage,
    wallet::TonWallet,
    TonAddress,
};

use tracing::info;

use crate::client::provider::TonProvider;
use crate::contracts::mailbox::TonMailbox;
use crate::signer::signer::TonSigner;
use crate::traits::ton_api_center::TonApiCenter;
use crate::types::run_get_method::GetMethodResponse;
use crate::utils::conversion::ConversionUtils;

pub struct TonInterchainSecurityModule {
    /// The address of the ISM contract.
    pub ism_address: TonAddress,
    /// The provider for the ISM contract.
    pub provider: TonProvider,
    //pub wallet: TonWallet,
    pub signer: TonSigner,
    pub workchain: i32, // -1 or 0
}
impl TonInterchainSecurityModule {
    const VERIFY: u32 = 0x3b3cca17;
    const GET_ISM: u32 = 0x8f32175;
}
impl Debug for TonInterchainSecurityModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ton mailbox:")
            .field("provider", &self.provider)
            .field("wallet:", &self.signer.wallet.address.to_hex())
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
            .await;

        if let Ok(response) = response {
            info!("Response runGetMethod:{:?}", response);
            if let Some(stack_item) = response.stack.get(0) {
                if let Ok(module_type_value) = u32::from_str_radix(&stack_item.value[2..], 16) {
                    info!("Module type value:{:?}", module_type_value);
                    if let Some(module_type) = ModuleType::from_u32(module_type_value) {
                        info!("Module Type:{:?}", module_type);
                        Ok(module_type)
                    } else {
                        warn!("Unknown module type:{:?}", module_type_value);
                        Ok(ModuleType::Unused)
                    }
                } else {
                    Err(ChainCommunicationError::CustomError(
                        "Failed to parse module type value".to_string(),
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

    async fn dry_run_verify(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
    ) -> ChainResult<Option<U256>> {
        info!("Let's build process");
        let message_t =
            ConversionUtils::hyperlane_message_to_message(message).expect("Failed to build");
        info!("Message_t:{:?}", message_t);

        let message_cell = message_t.to_cell();
        info!("message_cell:{:?}", message_cell);

        let metadata_cell = ConversionUtils::metadata_to_cell(metadata).map_err(|e| {
            ChainCommunicationError::CustomError(format!(
                "Failed to convert metadata to cell: {}",
                e
            ))
        })?;
        info!("Metadata:{:?}", metadata_cell);

        let query_id = 1;
        let block_number = 1;

        let msg = crate::contracts::mailbox::build_message(
            TonInterchainSecurityModule::VERIFY,
            ArcCell::new(message_cell),
            ArcCell::new(metadata_cell),
            query_id,
            block_number,
        )
        .expect("Failed to build message");

        info!("Msg cell:{:?}", msg);
        let common_msg_info = CommonMsgInfo::InternalMessage(InternalMessage {
            ihr_disabled: false,
            bounce: false,
            bounced: false,
            src: self.signer.address.clone(),
            dest: self.ism_address.clone(),
            value: BigUint::from(100000000u32),
            ihr_fee: Default::default(),
            fwd_fee: Default::default(),
            created_lt: 0,
            created_at: 0,
        });

        let transfer_message = TransferMessage {
            common_msg_info,
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
            .get_wallet_states(self.signer.address.to_hex())
            .await
            .expect("Failed to get wallet state")
            .wallets[0]
            .seqno as u32;

        let message = self
            .signer
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

        let result = self.provider.wait_for_transaction(tx.message_hash).await;
        match result {
            Ok(gas_estimate) => Ok(Some(gas_estimate.gas_used)),
            Err(e) => Ok(None),
        }
    }
}
