use std::ops::RangeInclusive;

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use hyperlane_core::{
    accumulator::{incremental::IncrementalMerkle, TREE_DEPTH},
    ChainCommunicationError, ChainResult, Checkpoint, HyperlaneChain, HyperlaneContract,
    HyperlaneDomain, HyperlaneProvider, Indexed, Indexer, LogMeta, MerkleTreeHook,
    MerkleTreeInsertion, ReorgPeriod, SequenceAwareIndexer, H256,
};
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use tonlib_core::{
    cell::{
        dict::predefined_readers::{key_reader_uint, val_reader_uint},
        BagOfCells,
    },
    TonAddress,
};
use tracing::{info, warn};

use crate::{
    client::provider::TonProvider, error::HyperlaneTonError, run_get_method::StackValue,
    ton_api_center::TonApiCenter, utils::conversion::ConversionUtils,
};

#[derive(Debug, Clone)]
/// A reference to a MerkleTreeHook contract on some TON chain
pub struct TonMerkleTreeHook {
    /// Domain
    provider: TonProvider,
    /// Contract address
    address: TonAddress,
}

impl TonMerkleTreeHook {
    /// Create a new TonMerkleTreeHook instance
    pub fn new(provider: TonProvider, address: TonAddress) -> ChainResult<Self> {
        Ok(Self { provider, address })
    }
}

impl HyperlaneContract for TonMerkleTreeHook {
    fn address(&self) -> H256 {
        ConversionUtils::ton_address_to_h256(&self.address)
    }
}

impl HyperlaneChain for TonMerkleTreeHook {
    fn domain(&self) -> &HyperlaneDomain {
        &self.provider.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

#[async_trait]
impl MerkleTreeHook for TonMerkleTreeHook {
    async fn tree(&self, _reorg_period: &ReorgPeriod) -> ChainResult<IncrementalMerkle> {
        let count = self.count(&ReorgPeriod::None).await.unwrap();

        let response = self
            .provider
            .run_get_method(self.address.to_string(), "get_tree".to_string(), None)
            .await
            .map_err(|e| HyperlaneTonError::ApiInvalidResponse(format!("Error:{:?}", e)))?;
        if response.exit_code != 0 {
            return Err(ChainCommunicationError::from(
                HyperlaneTonError::ApiRequestFailed("Non-zero exit code in response".to_string()),
            ));
        }

        let stack_item = response.stack.get(0).ok_or_else(|| {
            ChainCommunicationError::from(HyperlaneTonError::ParsingError(
                "Response stack is empty or missing required item".to_string(),
            ))
        })?;

        let value = match &stack_item.value {
            StackValue::String(boc) => boc,
            StackValue::List(list) if list.is_empty() => {
                warn!("Response stack contains empty list");

                let branch = [H256::zero(); TREE_DEPTH];
                return Ok(IncrementalMerkle {
                    branch,
                    count: count as usize,
                });
            }
            _ => {
                return Err(ChainCommunicationError::from(
                    HyperlaneTonError::ParsingError("Unexpected stack value type".to_string()),
                ));
            }
        };

        info!("Parsed value from stack: {:?}", value);

        let cell_boc_decoded = general_purpose::STANDARD.decode(value).map_err(|e| {
            ChainCommunicationError::from(HyperlaneTonError::ParsingError(format!(
                "Failed to decode cell BOC from response{:?}",
                e
            )))
        })?;

        let boc = BagOfCells::parse(&cell_boc_decoded).map_err(|e| {
            ChainCommunicationError::from(HyperlaneTonError::ParsingError(format!(
                "Failed to parse BOC: {}",
                e
            )))
        })?;

        let cell = boc.single_root().map_err(|e| {
            ChainCommunicationError::from(HyperlaneTonError::ParsingError(format!(
                "Failed to get root cell: {}",
                e
            )))
        })?;
        info!("Cell data before parsing: {:?}", cell);

        let mut parser = cell.parser();
        let dict = parser
            .load_dict(256, key_reader_uint, val_reader_uint)
            .map_err(|e| {
                ChainCommunicationError::from(HyperlaneTonError::ParsingError(format!(
                    "Failed to parse dictionary: {}",
                    e
                )))
            })?;

        let mut branch = [H256::zero(); TREE_DEPTH];

        for (depth, value) in dict {
            if let Some(depth_usize) = depth.to_usize() {
                if depth_usize >= TREE_DEPTH {
                    warn!("Unexpected depth: {}, skipping...", depth_usize);
                    continue;
                }

                // let mut value_bytes = [0u8; 32];
                // let value_raw = value.to_bytes_be();
                // value_bytes[32 - value_raw.len()..].copy_from_slice(&value_raw);
                // let value_h256: H256 = H256::from_slice(&value_bytes);

                let value_h256: H256 = H256::from_slice(value.to_bytes_be().as_slice());
                branch[depth_usize] = value_h256;
                info!("Depth: {}, Value: {:?}", depth_usize, value_h256);
            } else {
                warn!("Failed to convert depth to usize: {}", depth);
                continue;
            }
        }

        Ok(IncrementalMerkle {
            branch,
            count: count as usize,
        })
    }

    async fn count(&self, _reorg_period: &ReorgPeriod) -> ChainResult<u32> {
        let response = self
            .provider
            .run_get_method(self.address.to_string(), "get_count".to_string(), None)
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!("run_get_method failed: {e}"))
            })?;

        ConversionUtils::parse_stack_item_to_u32(&response.stack, 0)
    }
    async fn latest_checkpoint(&self, _reorg_period: &ReorgPeriod) -> ChainResult<Checkpoint> {
        let response = self
            .provider
            .run_get_method(
                self.address.to_string(),
                "get_latest_checkpoint".to_string(),
                None,
            )
            .await
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!("Failed to get response: {:?}", e))
            })?;

        let stack = response.stack;

        if stack.len() < 2 {
            return Err(ChainCommunicationError::CustomError(
                "Stack does not contain enough elements".to_string(),
            ));
        }

        // let root = ConversionUtils::parse_stack_item_to_u32(&stack, 0).map_err(|e| {
        //     ChainCommunicationError::CustomError(format!("Failed to parse root: {:?}", e))
        // })?;
        // let index = ConversionUtils::parse_stack_item_to_u32(&stack, 1).map_err(|e| {
        //     ChainCommunicationError::CustomError(format!("Failed to parse index: {:?}", e))
        // })?;
        let root = stack.get(0).ok_or_else(|| {
            ChainCommunicationError::CustomError(
                "Stack does not contain value at index 0".to_string(),
            )
        })?;
        let index = stack.get(1).ok_or_else(|| {
            ChainCommunicationError::CustomError(
                "Stack does not contain value at index 1".to_string(),
            )
        })?;
        let root_value = match &root.value {
            StackValue::String(val) => {
                BigUint::parse_bytes(val.trim_start_matches("0x").as_bytes(), 16).ok_or_else(
                    || {
                        ChainCommunicationError::CustomError(format!(
                            "Failed to parse BigUint from string: {}",
                            val
                        ))
                    },
                )?
            }
            _ => {
                return Err(ChainCommunicationError::CustomError(
                    "Unexpected stack value type for root".to_string(),
                ));
            }
        };
        let index_value = match &index.value {
            StackValue::String(val) => {
                BigUint::parse_bytes(val.trim_start_matches("0x").as_bytes(), 16)
                    .ok_or_else(|| {
                        ChainCommunicationError::CustomError(format!(
                            "Failed to parse BigUint from string: {}",
                            val
                        ))
                    })?
                    .try_into()
                    .map_err(|_| {
                        ChainCommunicationError::CustomError(
                            "Value is too large for u32".to_string(),
                        )
                    })?
            }
            _ => {
                return Err(ChainCommunicationError::CustomError(
                    "Unexpected stack value type for index".to_string(),
                ));
            }
        };
        Ok(Checkpoint {
            merkle_tree_hook_address: ConversionUtils::ton_address_to_h256(&self.address.clone()),
            mailbox_domain: self.domain().id(),
            root: H256::from_slice(root_value.to_bytes_be().as_slice()),
            index: index_value,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TonMerkleTreeHookIndexer {
    #[allow(dead_code)]
    merkle_tree_hook_address: TonAddress,
    provider: TonProvider,
}

impl TonMerkleTreeHookIndexer {
    pub fn new(address: TonAddress, provider: TonProvider) -> ChainResult<Self> {
        Ok(Self {
            merkle_tree_hook_address: address,
            provider,
        })
    }
}

#[async_trait]
impl Indexer<MerkleTreeInsertion> for TonMerkleTreeHookIndexer {
    async fn fetch_logs_in_range(
        &self,
        _range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<MerkleTreeInsertion>, LogMeta)>> {
        let start_block = *_range.start();
        let end_block = *_range.end();

        let timestamps = self
            .provider
            .fetch_blocks_timestamps(vec![start_block, end_block])
            .await?;

        let start_utime = *timestamps.get(0).ok_or_else(|| {
            ChainCommunicationError::from(HyperlaneTonError::ApiInvalidResponse(
                "Failed to get start_utime".to_string(),
            ))
        })?;
        let end_utime = *timestamps.get(1).ok_or_else(|| {
            ChainCommunicationError::from(HyperlaneTonError::ApiInvalidResponse(
                "Failed to get end_utime".to_string(),
            ))
        })?;

        let messages = self
            .provider
            .get_messages(
                None,
                None,
                Some(self.merkle_tree_hook_address.to_string()),
                Some("null".to_string()),
                None,
                Some(start_utime),
                Some(end_utime),
                None,
                None,
                None,
                None,
                None,
                Some("desc".to_string()),
            )
            .await
            .map_err(|e| {
                ChainCommunicationError::from(HyperlaneTonError::ApiRequestFailed(format!(
                    "Failed to fetch messages in range: {:?}",
                    e
                )))
            })?;

        let events: Vec<(Indexed<MerkleTreeInsertion>, LogMeta)> = messages
            .messages
            .iter()
            .filter_map(|message| {
                let boc = &message.message_content.body;
                let cell = ConversionUtils::parse_root_cell_from_boc(boc)
                    .map_err(|e| {
                        warn!("Failed to parse root cell from BOC: {:?}", e);
                        e
                    })
                    .ok()?;

                let mut parser = cell.parser();

                let message_id = parser
                    .load_uint(256)
                    .map_err(|e| {
                        warn!("Failed to load_uint message_id: {:?}", e);
                        e
                    })
                    .ok()?;
                let message_id_h256 = H256::from_slice(message_id.to_bytes_be().as_slice());

                let index = parser
                    .load_uint(256)
                    .map_err(|e| {
                        warn!("Failed to load_uint index: {:?}", e);
                        e
                    })
                    .ok()?;

                let index_u32 = index
                    .to_u32()
                    .ok_or_else(|| {
                        warn!("Index value is too large for u32");
                    })
                    .ok()?;

                let merkle_tree_insertion = MerkleTreeInsertion::new(index_u32, message_id_h256);

                let log_meta = LogMeta {
                    address: ConversionUtils::ton_address_to_h256(&self.merkle_tree_hook_address),
                    block_number: Default::default(),
                    block_hash: Default::default(),
                    transaction_id: Default::default(),
                    transaction_index: 0,
                    log_index: Default::default(),
                };

                Some((Indexed::new(merkle_tree_insertion), log_meta))
            })
            .collect();

        Ok(events)
    }

    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        self.provider.get_finalized_block().await.map_err(|e| {
            HyperlaneTonError::ApiRequestFailed(format!(
                "Failed to fetch finalized block number for TonMailboxIndexer: {:?}",
                e
            ))
            .into()
        })
    }
}

#[async_trait]
impl SequenceAwareIndexer<MerkleTreeInsertion> for TonMerkleTreeHookIndexer {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        println!("Merkle tree hook");
        Ok((Some(1), 1))
    }
}
