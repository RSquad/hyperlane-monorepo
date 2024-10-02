use async_trait::async_trait;
use derive_new::new;
use log::{debug, info, warn};
use std::sync::Arc;

use hyperlane_core::{
    BlockInfo, ChainCommunicationError, ChainInfo, ChainResult, ContractLocator, HyperlaneChain,
    HyperlaneCustomErrorWrapper, HyperlaneDomain, HyperlaneDomainProtocol,
    HyperlaneDomainTechnicalStack, HyperlaneDomainType, HyperlaneProvider, HyperlaneProviderError,
    TxnInfo, TxnReceiptInfo, H256, U256,
};
use reqwest::{Client, Response};
use tokio::{sync::RwLock, time::Sleep};

use tonlib::{
    address::{TonAddress, TonAddressParseError},
    cell::{BagOfCells, Cell, CellBuilder, CellSlice},
    client::{TonClient, TonClientBuilder, TonClientInterface, TonConnection, TonConnectionParams},
    config::{MAINNET_CONFIG, TESTNET_CONFIG},
    contract::{JettonMasterContract, TonContractError, TonContractFactory, TonContractInterface},
    message::TransferMessage,
    mnemonic::{self, KeyPair, Mnemonic},
    tl::AccountState::WalletV4,
    tl::{
        AccountAddress, AccountState, BlockId, BlockIdExt, BlocksHeader, Config, FullAccountState,
        InternalTransactionId, Options, TvmSlice,
    },
    types::{TvmStackEntry, TvmSuccess},
    wallet::TonWallet,
};

#[derive(Clone, new)]
pub struct TonProvider {
    pub ton_client: Arc<RwLock<TonClient>>,
    pub client: Arc<RwLock<Client>>,
    pub domain: HyperlaneDomain,
}
impl std::fmt::Debug for TonProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TonProvider")
            .field("client", &self.client)
            .field("domain", &self.domain)
            .finish()
    }
}

impl HyperlaneChain for TonProvider {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }
    /// A provider for the chain
    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl HyperlaneProvider for TonProvider {
    #[doc = " Get block info for a given block hash"]
    #[must_use]
    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    async fn get_block_by_hash(&self, hash: &H256) -> ChainResult<BlockInfo> {
        todo!()
    }

    #[must_use]
    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    async fn get_txn_by_hash(&self, hash: &H256) -> ChainResult<TxnInfo> {
        todo!()
    }

    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    async fn is_contract(&self, address: &H256) -> ChainResult<bool> {
        todo!()
    }

    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        todo!()
    }

    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        todo!()
    }
}
