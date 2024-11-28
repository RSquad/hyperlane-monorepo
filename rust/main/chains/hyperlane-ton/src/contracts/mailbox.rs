use async_trait::async_trait;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, Indexed, Indexer, LogMeta, Mailbox, SequenceAwareIndexer,
    TxCostEstimate, TxOutcome, H256, U256,
};
use num_bigint::BigUint;
use std::{
    fmt::{Debug, Formatter},
    num::NonZeroU64,
    ops::RangeInclusive,
    time::SystemTime,
};

use tonlib_core::message::{InternalMessage, TonMessage};
use tonlib_core::{
    cell::{ArcCell, BagOfCells, Cell, CellBuilder},
    message::{CommonMsgInfo, TransferMessage},
    TonAddress,
};

use tracing::info;

use crate::client::provider::TonProvider;
use crate::signer::signer::TonSigner;
use crate::traits::ton_api_center::TonApiCenter;
use crate::utils::conversion::ConversionUtils;

pub struct TonMailbox {
    pub mailbox_address: TonAddress,
    pub provider: TonProvider,
    pub signer: TonSigner,
    pub workchain: i32, // -1 or 0 now
}
impl TonMailbox {
    pub fn new(
        mailbox_address: TonAddress,
        provider: TonProvider,
        workchain: i32,
        signer: TonSigner,
    ) -> Self {
        Self {
            mailbox_address,
            provider,
            workchain,
            signer,
        }
    }
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
            .field("wallet:", &self.signer.address.to_hex())
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

    //
    async fn delivered(&self, id: H256) -> ChainResult<bool> {
        let mut builder = CellBuilder::new();
        let id_bigint = BigUint::from_bytes_be(id.as_bytes());
        let cell = builder
            .store_uint(256, &id_bigint)
            .unwrap()
            .build()
            .unwrap();
        log::info!("cell:{:?}", cell);

        let boc = BagOfCells::from_root(cell).serialize(true).unwrap();
        let boc_str = base64::encode(boc.clone());

        log::info!("Boc:{:?}", boc_str);

        let stack = vec![boc_str];

        let response = self
            .provider
            .run_get_method(
                self.mailbox_address.to_hex(),
                "get_deliveries".to_string(),
                Some(stack),
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
                let ism_address = ConversionUtils::parse_address_from_boc(&stack.value)
                    .await
                    .map_err(|e| {
                        ChainCommunicationError::CustomError(format!(
                            "Failed to parse address from BOC: {:?}",
                            e
                        ))
                    })?;

                let ism_hash = ConversionUtils::ton_address_to_h256(&ism_address).unwrap();
                return Ok(ism_hash);
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
        let recipient_address =
            ConversionUtils::h256_to_ton_address(&recipient, self.workchain).unwrap();

        let recipient_result = self
            .provider
            .run_get_method(
                recipient_address.to_string(),
                "get_recipient".to_string(),
                None,
            )
            .await;

        match recipient_result {
            Ok(response) => {
                if let Some(stack) = response.stack.first() {
                    if stack.r#type == "cell" {
                        let recipient_ism = ConversionUtils::parse_address_from_boc(&stack.value)
                            .await
                            .map_err(|e| {
                                ChainCommunicationError::CustomError(format!(
                                    "Failed to parse address from BOC: {:?}",
                                    e
                                ))
                            })?;

                        let ism_hash =
                            ConversionUtils::ton_address_to_h256(&recipient_ism).unwrap();
                        return Ok(ism_hash);
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
                        let recipient_ism = ConversionUtils::parse_address_from_boc(&stack.value)
                            .await
                            .map_err(|e| {
                                ChainCommunicationError::CustomError(format!(
                                    "Failed to parse address from BOC: {:?}",
                                    e
                                ))
                            })?;

                        let ism_hash =
                            ConversionUtils::ton_address_to_h256(&recipient_ism).unwrap();
                        return Ok(ism_hash);
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
        info!("HyperlaneMessage:{:?}", message);
        info!("Metadata:{:?}", metadata);

        info!("Let's build process");
        let message_t =
            ConversionUtils::hyperlane_message_to_message(message).expect("Failed to build");

        let message_cell = message_t.to_cell();

        let metadata_cell =
            ConversionUtils::metadata_to_cell(metadata).expect("Failed to get cell");
        info!("Metadata:{:?}", metadata_cell);

        let query_id = 1; // it is not currently used in the contract
        let block_number = 1;

        let msg = build_message(
            TonMailbox::PROCESS_OPCODE,
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
            dest: self.mailbox_address.clone(),
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
        .expect("Failed create transfer message");

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

        self.provider.wait_for_transaction(tx.message_hash).await
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
    pub mailbox: TonMailbox,
}

#[async_trait]
impl Indexer<HyperlaneMessage> for TonMailboxIndexer {
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<HyperlaneMessage>, LogMeta)>> {
        let start_block = *range.start();
        let end_block = *range.end();

        let start_block_info = self
            .mailbox
            .provider
            .get_blocks(
                -1,                       //  masterchain (workchain = -1)
                None,                     // shard
                None,                     // Block block seqno
                Some(start_block as i32), // Masterchain block seqno
                None,
                None,
                None,
                None,
                None, // limit
                None,
                None,
            )
            .await
            .expect("Failed to get start block info");
        let end_block_info = self
            .mailbox
            .provider
            .get_blocks(
                -1,                     //  masterchain (workchain = -1)
                None,                   // shard
                None,                   // Block block seqno
                Some(end_block as i32), // Masterchain block seqno
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!(
                    "Failed to get start block info: {:?}",
                    e
                ))
            })?;

        let start_utime = start_block_info.blocks[0]
            .gen_utime
            .parse::<i64>()
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!(
                    "Failed to parse start_utime: {:?}",
                    e
                ))
            })?;
        let end_utime = end_block_info.blocks[0]
            .gen_utime
            .parse::<i64>()
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!("Failed to parse end_utime: {:?}", e))
            })?;

        let message_response = self
            .mailbox
            .provider
            .get_messages(
                None,
                None,
                Some(self.mailbox.mailbox_address.to_hex()),
                Some("null".to_string()),
                Some(TonMailbox::DISPATCH_OPCODE.to_string()),
                Some(start_utime),
                Some(end_utime),
                None,
                None,
                None,
                None,
                None,
                Some("desc".to_string()),
            )
            .await;
        match message_response {
            Ok(messages) => {
                let mut events = vec![];
                for message in messages.messages {
                    let hyperlane_message = HyperlaneMessage {
                        version: 0,
                        nonce: 0,
                        origin: 0,
                        sender: Default::default(),
                        destination: 0,
                        recipient: Default::default(),
                        body: base64::decode(&message.message_content.body)
                            .expect("Invalid base64 body"),
                    };
                    let index_event = Indexed::from(hyperlane_message);
                    let log_meta = LogMeta {
                        address: Default::default(),
                        block_number: 0,
                        block_hash: Default::default(),
                        transaction_id: Default::default(),
                        transaction_index: 0,
                        log_index: Default::default(),
                    };
                    events.push((index_event, log_meta));
                }
                Ok(events)
            }
            Err(e) => Err(ChainCommunicationError::CustomError(format!(
                "Failed to fetch messages in range: {:?}",
                e
            ))),
        }
    }

    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        let response = self
            .mailbox
            .provider
            .get_blocks(
                -1,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(1),
                None,
                None,
            )
            .await
            .expect("Failed to get latest block");

        if let Some(block) = response.blocks.first() {
            Ok(block.seqno as u32)
        } else {
            Err(ChainCommunicationError::CustomError(
                "No blocks found".to_string(),
            ))
        }
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

#[async_trait]
impl Indexer<H256> for TonMailboxIndexer {
    async fn fetch_logs_in_range(
        &self,
        _range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<H256>, LogMeta)>> {
        todo!()
    }

    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        <TonMailboxIndexer as Indexer<HyperlaneMessage>>::get_finalized_block_number(self).await
    }
}
#[async_trait]
impl SequenceAwareIndexer<H256> for TonMailboxIndexer {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        // TODO: implement when ton scraper support is implemented
        info!("Message delivery indexing not implemented");
        let tip = Indexer::<H256>::get_finalized_block_number(self).await?;
        Ok((Some(1), tip))
    }
}

pub(crate) fn build_message(
    opcode: u32,
    message_cell: ArcCell,
    metadata_cell: ArcCell,
    query_id: u64,
    block_number: u64,
) -> Result<Cell, ChainCommunicationError> {
    let mut writer = CellBuilder::new();
    writer
        .store_u32(32, opcode)
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
