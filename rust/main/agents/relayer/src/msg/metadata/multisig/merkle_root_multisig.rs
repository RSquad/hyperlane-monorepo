use std::fmt::Debug;

use async_trait::async_trait;
use derive_more::{AsRef, Deref};
use derive_new::new;

use eyre::{Context, Result};
use hyperlane_base::MultisigCheckpointSyncer;
use hyperlane_core::{unwrap_or_none_result, HyperlaneMessage, H256};
use tracing::debug;

use crate::msg::metadata::MessageMetadataBuilder;

use super::base::{MetadataToken, MultisigIsmMetadataBuilder, MultisigMetadata};

#[derive(Debug, Clone, Deref, new, AsRef)]
pub struct MerkleRootMultisigMetadataBuilder(MessageMetadataBuilder);
#[async_trait]
impl MultisigIsmMetadataBuilder for MerkleRootMultisigMetadataBuilder {
    fn token_layout(&self) -> Vec<MetadataToken> {
        vec![
            MetadataToken::CheckpointMerkleTreeHook,
            MetadataToken::MessageMerkleLeafIndex,
            MetadataToken::MessageId,
            MetadataToken::MerkleProof,
            MetadataToken::CheckpointIndex,
            MetadataToken::Signatures,
        ]
    }

    async fn fetch_metadata(
        &self,
        validators: &[H256],
        threshold: u8,
        message: &HyperlaneMessage,
        checkpoint_syncer: &MultisigCheckpointSyncer,
    ) -> Result<Option<MultisigMetadata>> {
        use tracing::info;
        const CTX: &str = "When fetching MerkleRootMultisig metadata";
        info!("fetch_metadata in MerkleRootMultisigMetadataBuilder call!");
        let highest_leaf_index = unwrap_or_none_result!(
            self.highest_known_leaf_index().await,
            debug!("Couldn't get highest known leaf index")
        );
        info!("highest_leaf_index:{:?}", highest_leaf_index);
        let leaf_index = unwrap_or_none_result!(
            self.get_merkle_leaf_id_by_message_id(message.id())
                .await
                .context(CTX)?,
            debug!(
                hyp_message=?message,
                "No merkle leaf found for message id, must have not been enqueued in the tree"
            )
        );
        info!("leaf_index:{:?}", leaf_index);
        let quorum_checkpoint = unwrap_or_none_result!(
            checkpoint_syncer
                .fetch_checkpoint_in_range(
                    validators,
                    threshold as usize,
                    leaf_index,
                    highest_leaf_index,
                    self.origin_domain(),
                    self.destination_domain(),
                )
                .await
                .context(CTX)?,
            debug!(
                leaf_index,
                highest_leaf_index, "Couldn't get checkpoint in range"
            )
        );
        info!("quorum_checkpoint:{:?}", quorum_checkpoint);
        let proof = self
            .get_proof(leaf_index, quorum_checkpoint.checkpoint.checkpoint)
            .await
            .context(CTX)?;
        info!("proof:{:?}", proof);
        Ok(Some(MultisigMetadata::new(
            quorum_checkpoint,
            leaf_index,
            Some(proof),
        )))
    }
}
