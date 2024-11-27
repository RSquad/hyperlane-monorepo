use crate::client::provider::TonProvider;
use crate::signer::signer::TonSigner;
use crate::traits::ton_api_center::TonApiCenter;
use async_trait::async_trait;
use derive_new::new;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneProvider, Indexed, Indexer, InterchainGasPaymaster, InterchainGasPayment, LogMeta,
    SequenceAwareIndexer, H256, U256,
};
use log::info;
use std::fmt::{Debug, Formatter};
use std::ops::RangeInclusive;
use std::string::ToString;
use std::time::Duration;
use tokio::time::sleep;
use tonlib_core::TonAddress;

#[derive(Clone)]
pub struct TonInterchainGasPaymaster {
    pub igp_address: TonAddress,
    pub provider: TonProvider,
    pub signer: TonSigner,
    pub workchain: i32,
}
impl TonInterchainGasPaymaster {
    const EVENT_GAS_PAYMENT: &'static str = "event::gas_payment";
    const EVENT_REQUIRED_PAYMENT: &'static str = "event::required_payment";
}

impl HyperlaneContract for TonInterchainGasPaymaster {
    fn address(&self) -> H256 {
        let hex = self.igp_address.to_hex();
        let bytes = hex.as_bytes();
        H256::from_slice(bytes)
    }
}
impl HyperlaneChain for TonInterchainGasPaymaster {
    fn domain(&self) -> &HyperlaneDomain {
        &self.provider.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

impl Debug for TonInterchainGasPaymaster {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TonInterchainGasPaymaster:")
            .field("igp address:", &self.igp_address.to_hex())
            .field("provider", &self.provider)
            .field("wallet:", &self.signer.wallet.address.to_hex())
            .finish()
    }
}
impl InterchainGasPaymaster for TonInterchainGasPaymaster {}

#[derive(Debug, Clone, new)]
pub struct TonInterchainGasPaymasterIndexer {
    provider: TonProvider,
    igp_address: TonAddress,
}

#[async_trait]
impl Indexer<InterchainGasPayment> for TonInterchainGasPaymasterIndexer {
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<InterchainGasPayment>, LogMeta)>> {
        let start_block = *range.start();
        let end_block = *range.end();

        let start_block_info = self
            .provider
            .get_blocks(
                -1,                       //  masterchain (workchain = -1)
                None,                     // shard
                None,                     // block seqno
                Some(start_block as i32), // masterchain seqno
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
        info!("Start block info:{:?}", start_block_info);

        sleep(Duration::from_secs(5)).await;

        let end_block_info = self
            .provider
            .get_blocks(
                -1,                     //  masterchain (workchain = -1)
                None,                   // shard
                None,                   // block seqno
                Some(end_block as i32), // masterchain seqno
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .expect("Failed to get end block info");

        info!("End block info:{:?}", start_block_info);

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
            .provider
            .get_messages(
                None,
                None,
                Some(self.igp_address.to_hex()), // I need check this
                //None,
                Some("null".to_string()),
                Some(TonInterchainGasPaymaster::EVENT_GAS_PAYMENT.to_string()),
                //None,
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
                info!("Messages:{:?}", messages);

                let mut events = vec![];
                for message in messages.messages {
                    let body_data =
                        base64::decode(&message.message_content.body).expect("Invalid base64 body");
                    info!("Body:{:?}", body_data);

                    let interchain_gas_payment = InterchainGasPayment {
                        message_id: H256::from_slice(&body_data[0..32]), //Default::default(),
                        destination: u32::from_be_bytes(body_data[32..36].try_into().unwrap()), //0,
                        payment: U256::from_big_endian(&body_data[36..68]), //Default::default(),
                        gas_amount: U256::from_big_endian(&body_data[68..100]), //Default::default(),
                    };
                    let index_event = Indexed::from(interchain_gas_payment);
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
impl SequenceAwareIndexer<InterchainGasPayment> for TonInterchainGasPaymasterIndexer {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let tip = Indexer::<InterchainGasPayment>::get_finalized_block_number(self).await?;

        Ok((None, tip))
    }
}
