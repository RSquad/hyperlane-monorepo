use std::ops::RangeInclusive;

use hyperlane_core::{
    accumulator::TREE_DEPTH, HyperlaneDomain, HyperlaneMessage, Indexer, KnownHyperlaneDomain,
    Mailbox, MerkleTreeHook, ReorgPeriod, H256,
};
use reqwest::Client;
use tonlib_core::{wallet::TonWallet, TonAddress};
use tracing::info;
use url::Url;

use crate::{
    ConversionUtils, TonConnectionConf, TonInterchainGasPaymaster, TonInterchainSecurityModule,
    TonMailbox, TonMerkleTreeHook, TonMerkleTreeHookIndexer, TonMultisigIsm, TonProvider,
    TonSigner, TonValidatorAnnounce,
};

pub struct TestContext {
    pub provider: TonProvider,
    pub signer: TonSigner,
    pub mailbox: TonMailbox,
    pub igp: TonInterchainGasPaymaster,
    pub ism: TonInterchainSecurityModule,
    pub multisig: TonMultisigIsm,
    pub validator_announce: TonValidatorAnnounce,
    pub merkle_hook: TonMerkleTreeHook,
    pub merkle_hook_indexer: TonMerkleTreeHookIndexer,
}

impl TestContext {
    pub fn new(
        api_url: &str,
        api_key: &str,
        wallet: &TonWallet,
        mailbox_address: &str,
        igp_address: &str,
        ism_address: &str,
        multisig_address: &str,
        validator_announce: &str,
        merkle_hook_address: &str,
    ) -> Result<Self, anyhow::Error> {
        let http_client = Client::new();
        let connection_config =
            TonConnectionConf::new(Url::parse(api_url)?, api_key.to_string(), 5);
        let domain = HyperlaneDomain::Known(KnownHyperlaneDomain::TonTest1); // It doesn't matter now.

        let provider = TonProvider::new(http_client, connection_config, domain);

        let signer = TonSigner {
            address: wallet.clone().address,
            wallet: wallet.clone(),
        };

        let mailbox = TonMailbox {
            workchain: 0,
            mailbox_address: mailbox_address.parse()?,
            provider: provider.clone(),
            signer: signer.clone(),
        };

        let igp = TonInterchainGasPaymaster {
            igp_address: TonAddress::from_base64_url(igp_address).unwrap(),
            provider: provider.clone(),
            signer: signer.clone(),
            workchain: 0,
        };

        let ism = TonInterchainSecurityModule {
            ism_address: TonAddress::from_base64_url(ism_address).unwrap(),
            provider: provider.clone(),
            workchain: 0,
            signer: signer.clone(),
        };

        let multisig = TonMultisigIsm::new(
            provider.clone(),
            TonAddress::from_base64_url(multisig_address).unwrap(),
        );

        let validator_announce = TonValidatorAnnounce::new(
            TonAddress::from_base64_url(validator_announce).unwrap(),
            provider.clone(),
            signer.clone(),
        );
        let merkle_hook = TonMerkleTreeHook::new(
            provider.clone(),
            TonAddress::from_base64_url(merkle_hook_address).unwrap(),
        )
        .unwrap();

        let merkle_hook_indexer = TonMerkleTreeHookIndexer::new(
            TonAddress::from_base64_url(merkle_hook_address).unwrap(),
            provider.clone(),
        )
        .unwrap();

        Ok(Self {
            provider,
            signer,
            mailbox,
            igp,
            ism,
            multisig,
            validator_announce,
            merkle_hook,
            merkle_hook_indexer,
        })
    }

    pub async fn test_mailbox_default_ism(&self) -> Result<(), anyhow::Error> {
        let default_ism = self.mailbox.default_ism().await.map_err(|e| {
            anyhow::Error::msg(format!("Failed to fetch Mailbox Default ISM: {:?}", e))
        })?;
        println!("Mailbox Default ISM: {:?}", default_ism);
        Ok(())
    }

    pub async fn test_mailbox_recipient_ism(&self) -> Result<(), anyhow::Error> {
        let recipient = ConversionUtils::ton_address_to_h256(
            &TonAddress::from_base64_url("EQCb3n0SkpKTNyNlhKEnndYSG0DQ2nK6za0oFhl5bRr3n4hc")
                .unwrap(),
        );
        let default_ism = self.mailbox.recipient_ism(recipient).await.map_err(|e| {
            anyhow::Error::msg(format!("Failed to fetch Mailbox recipient ISM: {:?}", e))
        })?;
        println!("recipient ISM: {:?}", default_ism);
        Ok(())
    }

    pub async fn test_mailbox_process(&self) -> Result<(), anyhow::Error> {
        let message = HyperlaneMessage {
            version: 7,
            nonce: 0,
            origin: 777001,
            sender: Default::default(),
            destination: 777002,
            recipient: H256::zero(),
            body: vec![],
        };
        let metadata = [0u8; 64];
        let tx = self
            .mailbox
            .process(&message, &metadata, None)
            .await
            .map_err(|e| anyhow::Error::msg(format!("Failed to send process message: {:?}", e)))?;
        println!("TxOutcome:{:?}", tx);

        Ok(())
    }

    pub async fn test_merkle_tree_hook_tree(&self) -> Result<(), anyhow::Error> {
        let tree = self.merkle_hook.tree(&ReorgPeriod::None).await?;
        println!("Incremental Merkle Tree: {:?}", tree);

        assert_eq!(tree.branch.len(), TREE_DEPTH);
        println!("Tree depth is valid.");
        Ok(())
    }

    pub async fn test_merkle_tree_hook_count(&self) -> Result<(), anyhow::Error> {
        let count = self.merkle_hook.count(&ReorgPeriod::None).await?;
        info!("Merkle Tree Count: {}", count);

        assert!(count >= 0, "Merkle Tree count should be non-negative.");
        info!("Merkle Tree count is valid.");

        Ok(())
    }
    pub async fn test_merkle_tree_hook_latest_checkpoint(&self) -> Result<(), anyhow::Error> {
        let checkpoint = self
            .merkle_hook
            .latest_checkpoint(&ReorgPeriod::None)
            .await?;
        info!("Merkle Tree Latest Checkpoint: {:?}", checkpoint);

        assert_ne!(
            checkpoint.root,
            H256::zero(),
            "Checkpoint root should not be zero."
        );
        info!("Checkpoint root is valid.");

        Ok(())
    }

    pub async fn test_merkle_tree_hook_indexer(&self) -> Result<(), anyhow::Error> {
        let logs = self
            .merkle_hook_indexer
            .fetch_logs_in_range(RangeInclusive::new(10, 26370511))
            .await
            .unwrap();

        info!("Events:{:?}", logs);

        Ok(())
    }
}
