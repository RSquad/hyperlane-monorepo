use async_trait::async_trait;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, FixedPointNumber, HyperlaneChain, HyperlaneContract,
    HyperlaneCustomErrorWrapper, HyperlaneDomain, HyperlaneMessage, HyperlaneProvider, Indexed,
    Indexer, LogMeta, Mailbox, SequenceAwareIndexer, TxCostEstimate, TxOutcome, H256, H512, U256,
};
use num_bigint::BigUint;
use std::{
    fmt::{Debug, Formatter},
    future::Future,
    num::NonZeroU64,
    ops::RangeInclusive,
    pin::Pin,
    time::{Duration, SystemTime},
};

use tokio::time::sleep;
use tonlib::{
    address::TonAddress,
    cell::{ArcCell, BagOfCells, Cell, CellBuilder},
    message::TransferMessage,
    mnemonic::Mnemonic,
    wallet::{TonWallet, WalletVersion},
};

use tracing::{debug, info, instrument, warn};

use crate::client::provider::TonProvider;
use crate::traits::ton_api_center::TonApiCenter;
use crate::types::transaction::TransactionResponse;
use crate::utils::conversion::{hyperlane_message_to_message, metadata_to_cell};

pub struct TonMailbox {
    pub mailbox_address: TonAddress,
    pub provider: TonProvider,
    pub wallet: TonWallet,
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
        f.debug_struct("Ton mailbox:")
            .field("mailbox address:", &self.mailbox_address.to_hex())
            .field("provider", &self.provider)
            .field("wallet:", &self.wallet.address.to_hex())
            .finish()
    }
}
impl TonMailbox {
    const DISPATCH_OPCODE: u32 = 0xf8cf866bu32;
    const PROCESS_OPCODE: u32 = 0xea81949bu32;
    const PROCESS_INIT: u32 = 0xba35fd5f;
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
            .run_get_method(recipient.to_string(), "get_recipient".to_string(), None)
            .await;

        match recipient_result {
            Ok(response) => {
                if let Some(stack) = response.stack.first() {
                    if stack.r#type == "cell" {
                        let decoded_value = base64::decode(&stack.value)
                            .map_err(|e| {
                                ChainCommunicationError::CustomError(format!(
                                    "Failed to decode base64 value from stack: {:?}",
                                    e
                                ))
                            })
                            .expect("Failed");

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
                        let decoded_value = base64::decode(&stack.value).map_err(|e| {
                            ChainCommunicationError::CustomError(format!(
                                "Failed to decode base64 value from stack: {:?}",
                                e
                            ))
                        })?;

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
        info!("Let's build process");
        let message_t = hyperlane_message_to_message(message).expect("Failed to build");

        let message_cell = message_t.to_cell();

        let metadata_cell = metadata_to_cell(metadata).expect("Failed to get cell");
        info!("Metadata:{:?}", metadata_cell);

        let query_id = 1;
        let block_number = 1;

        let msg = build_message(
            ArcCell::new(message_cell),
            ArcCell::new(metadata_cell),
            query_id,
            block_number,
        )
        .expect("Failed to build message");

        info!("Msg cell:{:?}", msg);

        let transfer_message = TransferMessage {
            dest: self.mailbox_address.clone(),
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

        self.wait_for_transaction(tx.message_hash).await
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

impl TonMailbox {
    async fn wait_for_transaction(&self, message_hash: String) -> ChainResult<TxOutcome> {
        let max_attempts = 5;
        let delay = Duration::from_secs(5);

        for attempt in 1..=max_attempts {
            info!("Attempt {}/{}", attempt, max_attempts);

            match self
                .provider
                .get_transaction_by_message(message_hash.clone(), None, None)
                .await
            {
                Ok(response) => {
                    if response.transactions.is_empty() {
                        log::info!("Transaction not found, retrying...");
                    } else {
                        log::info!(
                            "Transaction found: {:?}",
                            response
                                .transactions
                                .first()
                                .expect("Failed to get first transaction from list")
                        );

                        if let Some(transaction) = response.transactions.first() {
                            let tx_outcome = TxOutcome {
                                transaction_id: H512::zero(), // at least now
                                executed: !transaction.description.aborted,
                                gas_used: U256::from_dec_str(
                                    &transaction.description.compute_ph.gas_used,
                                )
                                .expect("Failed to parse gas used"),
                                gas_price: FixedPointNumber::from(0),
                            };

                            log::info!("Tx outcome: {:?}", tx_outcome);
                            return Ok(tx_outcome);
                        }
                    }
                }
                Err(e) => {
                    info!("Transaction not found, retrying... {:?}", e);
                    if attempt == max_attempts {
                        return Err(ChainCommunicationError::CustomError(
                            "Transaction not found after max attempts".to_string(),
                        ));
                    }
                }
            }

            sleep(delay).await;
        }

        Err(ChainCommunicationError::CustomError("Timeout".to_string()))
    }
}

fn build_message(
    message_cell: ArcCell,
    metadata_cell: ArcCell,
    query_id: u64,
    block_number: u64,
) -> Result<Cell, ChainCommunicationError> {
    let mut writer = CellBuilder::new();
    writer
        .store_u32(32, TonMailbox::PROCESS_OPCODE)
        .expect("Failed to store process opcode")
        .store_u64(64, query_id)
        .expect("Failed to store query_id")
        .store_u32(32, TonMailbox::PROCESS_INIT)
        .expect("Failed to store process init")
        .store_u64(48, block_number)
        .expect("Failed to store block_number")
        .store_reference(&message_cell)
        .expect("Failed to store message")
        .store_reference(&metadata_cell)
        .expect("Failed to store metadata")
        .build()
        .map_err(|e| ChainCommunicationError::CustomError(format!("Cell build failed: {}", e)))
}
