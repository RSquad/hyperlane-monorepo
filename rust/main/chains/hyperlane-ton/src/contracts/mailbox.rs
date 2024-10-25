use async_trait::async_trait;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, Indexed, Indexer, LogMeta, Mailbox, SequenceAwareIndexer,
    TxCostEstimate, TxOutcome, H256, H512, U256,
};
use num_bigint::BigUint;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::num::NonZeroU64;
use std::ops::RangeInclusive;
use std::pin::Pin;
use std::time::SystemTime;
use tonlib::address::TonAddress;
use tonlib::cell::{ArcCell, BagOfCells, CellBuilder};
use tonlib::message::TransferMessage;
use tracing::{debug, info, instrument, warn};

use crate::client::provider::TonProvider;
use crate::traits::ton_api_center::TonApiCenter;
use crate::utils::conversion::{hyperlane_message_to_cell, metadata_to_cell};

pub struct TonMailbox {
    pub mailbox_address: TonAddress,
    pub provider: TonProvider,
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
impl TonMailbox {
    const DISPATCH_OPCODE: i32 = 0xf8cf866b;
    const PROCESS_OPCODE: i32 = 0xea81949b;
    const PROCESS_INIT: i32 = 0xba35fd5f;
}
#[async_trait]
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

    async fn delivered(&self, id: H256) -> ChainResult<bool> {
        let response = self
            .provider
            .run_get_method(
                self.mailbox_address.to_hex(),
                "get_deliveries".to_string(),
                None,
            )
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!(
                    "Error calling run_get_method: {:?}",
                    e
                ))
            })?;

        let is_delivered = response.stack.iter().any(|item| {
            let stored_id = item.value.as_str();
            stored_id == format!("{:x}", id)
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
        let recipient_result = self
            .provider
            .run_get_method(recipient.to_hex(), "get_recipient".to_string(), None)
            .await;

        match recipient_result {
            Ok(response) => {
                if let Some(stack) = response.stack.first() {
                    if stack.r#type == "cell" {
                        let decoded_value = base64::decode(&stack.value).map_err(|e| {
                            ChainCommunicationError::CustomError(format!(
                                "Failed to decode base64 value from stack: {:?}",
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
                        return Err(ChainCommunicationError::CustomError(format!(
                            "Unexpected data type in stack: expected 'cell', got '{}'",
                            stack.r#type
                        )));
                    }
                } else {
                    return Err(ChainCommunicationError::CustomError(
                        "No data found in the response stack".to_string(),
                    ));
                }
            }
            Err(_) => {
                let mailbox_response = self
                    .provider
                    .run_get_method(
                        self.mailbox_address.to_hex(),
                        "get_recipient_ism".to_string(),
                        Some(vec![format!("0x{}", recipient)]),
                    )
                    .await
                    .map_err(|e| {
                        ChainCommunicationError::CustomError(format!(
                            "Error calling run_get_method for mailbox: {:?}",
                            e
                        ))
                    })?;

                if let Some(stack) = mailbox_response.stack.first() {
                    if stack.r#type == "cell" {
                        // Декодируем значение из base64
                        let decoded_value = base64::decode(&stack.value).map_err(|e| {
                            ChainCommunicationError::CustomError(format!(
                                "Failed to decode base64 value from stack: {:?}",
                                e
                            ))
                        })?;

                        // Проверяем, что декодированное значение имеет длину 32 байта
                        if decoded_value.len() >= 32 {
                            let ism_hash = H256::from_slice(&decoded_value[0..32]);
                            Ok(ism_hash)
                        } else {
                            Err(ChainCommunicationError::CustomError(
                                "Decoded value is too short for H256".to_string(),
                            ))
                        }
                    } else {
                        Err(ChainCommunicationError::CustomError(format!(
                            "Unexpected data type in stack: expected 'cell', got '{}'",
                            stack.r#type
                        )))
                    }
                } else {
                    Err(ChainCommunicationError::CustomError(
                        "No data found in the mailbox response stack".to_string(),
                    ))
                }
            }
        }
    }

    async fn process(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
        tx_gas_limit: Option<U256>,
    ) -> ChainResult<TxOutcome> {
        let message_cell = message.to_cell();
        let metadata_cell = metadata.to_cell();

        let mut writer = CellBuilder::new();
        let msg = writer
            .store_u32(32, Self::PROCESS_OPCODE)?
            .store_u64(64, 1u64)?
            .store_u32(32, Self::PROCESS_INIT)?
            .store_u64(48, 2u64)?
            .store_reference(&ArcCell::new(message_cell))?
            .store_reference(&ArcCell::new(metadata_cell))?
            .build()?;

        let transfer_message = TransferMessage {
            dest: &self.mailbox_address,
            value: BigUint::from(1000000000u32),
            state_init: None,
            data: Some(ArcCell::new(msg.clone())),
        }
        .build()
        .expect("Failed to create transferMessage");

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as u32;

        let seqno = self
            .provider
            .get_account_state(wallet.address)
            .await
            .expect("");

        let seqno = self.provider.get
        let message = wallet
            .create_external_message(
                now + 60,
                seqno,
                vec![ArcCell::new(transfer_message.clone())],
                false,
            )
            .expect("");

        let boc = BagOfCells::from_root(message.clone()).serialize(true)?;
        //let boc_str = base64::encode(boc.clone());

        let response = self
            .provider
            .send_message(base64::encode(boc.clone()))
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!("Failed to send message: {:?}", e))
            })?;

        if response.message_hash.is_empty() {
            Err(ChainCommunicationError::CustomError(
                "Message hash is empty, likely an error occurred".to_string(),
            ))
        } else {
            Ok(TxOutcome {
                transaction_id: H512::from_slice(&hex::decode(response.message_hash)?),
                executed: true,
                gas_used: Default::default(),
                gas_price: Default::default(),
            })
        }
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

#[derive(Debug)]
pub struct TonMailboxIndexer {
    mailbox: TonMailbox,
}

#[async_trait]
impl Indexer<HyperlaneMessage> for TonMailboxIndexer {
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<HyperlaneMessage>, LogMeta)>> {
        todo!()
    }

    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        todo!()
    }
}

#[async_trait]
impl SequenceAwareIndexer<HyperlaneMessage> for TonMailboxIndexer {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let tip = Indexer::<HyperlaneMessage>::get_finalized_block_number(self).await?;

        let count = Mailbox::count(&self.mailbox, None).await?;
        Ok((Some(count), tip))
    }
}
