use crate::client::provider::TonProvider;
use crate::ton_api_center::TonApiCenter;
use crate::utils::conversion::ConversionUtils;
use async_trait::async_trait;
use hyperlane_core::accumulator::incremental::IncrementalMerkle;
use hyperlane_core::accumulator::{TREE_DEPTH, ZERO_HASHES};
use hyperlane_core::{
    ChainCommunicationError, ChainResult, Checkpoint, HyperlaneChain, HyperlaneContract,
    HyperlaneDomain, HyperlaneProvider, Indexed, Indexer, LogMeta, MerkleTreeHook,
    MerkleTreeInsertion, SequenceAwareIndexer, H256,
};
use std::ops::RangeInclusive;
use tonlib_core::TonAddress;

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
        ConversionUtils::ton_address_to_h256(&self.address).expect("Failed")
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
    async fn tree(&self, _lag: Option<std::num::NonZeroU64>) -> ChainResult<IncrementalMerkle> {
        let count = self.count(_lag).await?;
        let mut branch: [H256; TREE_DEPTH] = Default::default();
        branch
            .iter_mut()
            .enumerate()
            .for_each(|(i, elem)| *elem = ZERO_HASHES[i]);
        Ok(IncrementalMerkle {
            branch,
            count: count as usize,
        })
    }

    async fn count(&self, _lag: Option<std::num::NonZeroU64>) -> ChainResult<u32> {
        let response = self
            .provider
            .run_get_method(self.address.to_string(), "get_count".to_string(), None)
            .await;

        match response {
            Ok(run_get_method) => {
                if let Some(stack_item) = run_get_method.stack.get(0) {
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
            }
            Err(e) => Err(ChainCommunicationError::CustomError(
                "Failed to get response".to_string(),
            )),
        }
    }

    async fn latest_checkpoint(
        &self,
        _lag: Option<std::num::NonZeroU64>,
    ) -> ChainResult<Checkpoint> {
        let response = self
            .provider
            .run_get_method(
                self.address.to_string(),
                "get_latest_checkpoint".to_string(),
                None,
            )
            .await;

        match response {
            Ok(run_get_method) => {
                let stack = run_get_method.stack;

                if stack.len() < 2 {
                    return Err(ChainCommunicationError::CustomError(
                        "Stack does not contain enough elements".to_string(),
                    ));
                }

                let root_item = &stack[0];
                let root_value = &root_item.value;

                let root = u32::from_str_radix(&root_value[2..], 16).map_err(|_| {
                    ChainCommunicationError::CustomError("Failed to parse root value".to_string())
                })?;

                let index_item = &stack[1];
                let index_value = &index_item.value;
                let index = u32::from_str_radix(&index_value[2..], 16).map_err(|_| {
                    ChainCommunicationError::CustomError("Failed to parse index value".to_string())
                })?;

                Ok(Checkpoint {
                    merkle_tree_hook_address: ConversionUtils::ton_address_to_h256(
                        &self.address.clone(),
                    )
                    .unwrap(),
                    mailbox_domain: 777001,
                    root: H256::from_low_u64_be(root as u64),
                    index,
                })
            }
            Err(e) => Err(ChainCommunicationError::CustomError(format!(
                "Failed to get response: {:?}",
                e
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TonMerkleTreeHookIndexer {
    /// The TonMerkleTreeHook
    merkle_tree_hook_address: TonAddress,
}

impl TonMerkleTreeHookIndexer {
    pub fn new(address: TonAddress) -> ChainResult<Self> {
        Ok(Self {
            merkle_tree_hook_address: address,
        })
    }
}

#[async_trait]
impl Indexer<MerkleTreeInsertion> for TonMerkleTreeHookIndexer {
    async fn fetch_logs_in_range(
        &self,
        _range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<MerkleTreeInsertion>, LogMeta)>> {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(vec![])
    }

    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(0)
    }
}

#[async_trait]
impl SequenceAwareIndexer<MerkleTreeInsertion> for TonMerkleTreeHookIndexer {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        println!("Merkle tree hook");
        Ok((Some(1), 1))
    }
}
