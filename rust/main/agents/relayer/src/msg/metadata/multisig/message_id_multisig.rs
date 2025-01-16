use std::fmt::Debug;

use async_trait::async_trait;
use derive_more::{AsRef, Deref};
use derive_new::new;

use eyre::{Context, Result};
use hyperlane_base::MultisigCheckpointSyncer;
use hyperlane_core::{unwrap_or_none_result, HyperlaneMessage, H256};
use tracing::{debug, warn};

use crate::msg::metadata::MessageMetadataBuilder;

use super::base::{MetadataToken, MultisigIsmMetadataBuilder, MultisigMetadata};

#[derive(Debug, Clone, Deref, new, AsRef)]
pub struct MessageIdMultisigMetadataBuilder(MessageMetadataBuilder);

#[async_trait]
impl MultisigIsmMetadataBuilder for MessageIdMultisigMetadataBuilder {
    fn token_layout(&self) -> Vec<MetadataToken> {
        vec![
            MetadataToken::CheckpointMerkleTreeHook,
            MetadataToken::CheckpointMerkleRoot,
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

        info!(
            "fetch_metadata in MessageIdMultisigMetadataBuilder call message:{:?}",
            message
        );
        let message_id = message.id();
        info!("message_id in fetch_metadata:{:?}", message_id);

        const CTX: &str = "When fetching MessageIdMultisig metadata";
        let leaf_index = unwrap_or_none_result!(
            self.get_merkle_leaf_id_by_message_id(message_id)
                .await
                .context(CTX)?,
            debug!(
                hyp_message=?message,
                "No merkle leaf found for message id, must have not been enqueued in the tree"
            )
        );
        info!(
            "leaf_index MessageIdMultisigMetadataBuilder:{:?}",
            leaf_index
        );

        // Update the validator latest checkpoint metrics.
        let _ = checkpoint_syncer
            .get_validator_latest_checkpoints_and_update_metrics(
                validators,
                self.origin_domain(),
                self.destination_domain(),
            )
            .await;

        let quorum_checkpoint = unwrap_or_none_result!(
            checkpoint_syncer
                .fetch_checkpoint(validators, threshold as usize, leaf_index)
                .await
                .context(CTX)?,
            debug!("No quorum checkpoint found")
        );
        info!(
            "quorum_checkpoint MessageIdMultisigMetadataBuilder:{:?}",
            quorum_checkpoint
        );

        if quorum_checkpoint.checkpoint.message_id != message_id {
            info!(
                "Quorum checkpoint message id {} does not match message id {}",
                quorum_checkpoint.checkpoint.message_id, message_id
            );
            if quorum_checkpoint.checkpoint.index != leaf_index {
                info!(
                    "Quorum checkpoint index {} does not match leaf index {}",
                    quorum_checkpoint.checkpoint.index, leaf_index
                );
            }
            return Ok(None);
        }

        Ok(Some(MultisigMetadata::new(
            quorum_checkpoint,
            leaf_index,
            None,
        )))
    }
}
