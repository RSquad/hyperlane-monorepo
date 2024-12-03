use crate::client::provider::TonProvider;
use crate::utils::conversion::ConversionUtils;
use async_trait::async_trait;
use hyperlane_core::accumulator::incremental::IncrementalMerkle;
use hyperlane_core::{
    ChainResult, Checkpoint, HyperlaneChain, HyperlaneContract, HyperlaneDomain, HyperlaneProvider,
    Indexed, Indexer, LogMeta, MerkleTreeHook, MerkleTreeInsertion, SequenceAwareIndexer, H256,
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
        todo!()
    }

    async fn count(&self, _lag: Option<std::num::NonZeroU64>) -> ChainResult<u32> {
        todo!()
    }

    async fn latest_checkpoint(
        &self,
        _lag: Option<std::num::NonZeroU64>,
    ) -> ChainResult<Checkpoint> {
        todo!()
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
        // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // Ok(vec![])
        todo!()
    }

    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // Ok(0)
        todo!()
    }
}

#[async_trait]
impl SequenceAwareIndexer<MerkleTreeInsertion> for TonMerkleTreeHookIndexer {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        println!("Merkle tree hook");
        Ok((Some(1), 1))
    }
}
