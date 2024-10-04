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

use crate::types::transaction::TransactionResponse;
use tonlib::tl::{AccountState, BlockIdExt};
use tonlib::{
    address::{TonAddress, TonAddressParseError},
    client::{
        TonClient, TonClientBuilder, TonClientError, TonClientInterface, TonConnection,
        TonConnectionParams,
    },
};

#[derive(Clone, new)]
pub struct TonProvider {
    pub ton_client: TonClient,
    pub http_client: Client,
    pub domain: HyperlaneDomain,
}
impl std::fmt::Debug for TonProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TonProvider")
            .field("client", &self.http_client)
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
    async fn get_block_by_hash(&self, hash: &H256) -> ChainResult<BlockInfo> {
        // need check
        let block_id: BlockIdExt = BlockIdExt {
            workchain: 0,
            shard: 0,
            seqno: 0,
            root_hash: hash.to_string(),
            file_hash: "".to_string(),
        };
        let client = self.ton_client.get_block_header(&block_id).await;

        match client {
            Ok(blocks_header) => {
                let block_info: BlockInfo = BlockInfo {
                    hash: H256::from_slice(&blocks_header.id.root_hash.as_bytes()),
                    timestamp: blocks_header.gen_utime as u64,
                    number: blocks_header.id.seqno as u64,
                };
                Ok(block_info)
            }
            Err(e) => Err(ChainCommunicationError::Other(
                HyperlaneCustomErrorWrapper::new(Box::new(e)),
            )),
        }
    }

    async fn get_txn_by_hash(&self, hash: &H256) -> ChainResult<TxnInfo> {
        // we need to implement something like ConfConnection later
        let url = "";

        let response = self
            .http_client
            .get(url)
            .query(&[("hash", format!("{:?}", hash))])
            .send()
            .await
            .map_err(|e| {
                warn!("Error when sending request to TON API");
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?
            .json::<TransactionResponse>()
            .await
            .map_err(|e| {
                warn!("Error deserializing response from TON API");
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        if let Some(transaction) = response.transactions.first() {
            let txn_info = TxnInfo {
                hash: H256::from_slice(&hex::decode(&transaction.hash).unwrap()),
                gas_limit: U256::from_dec_str(&transaction.description.compute_ph.gas_limit)
                    .unwrap_or_default(),
                max_priority_fee_per_gas: None,
                max_fee_per_gas: None,
                gas_price: None,
                nonce: transaction.lt.parse::<u64>().unwrap_or(0),
                sender: H256::from_slice(&hex::decode(&transaction.account).unwrap()),
                recipient: transaction
                    .in_msg
                    .as_ref()
                    .map(|msg| H256::from_slice(&hex::decode(&msg.destination).unwrap())),
                receipt: Some(TxnReceiptInfo {
                    gas_used: U256::from_dec_str(&transaction.description.compute_ph.gas_used)
                        .unwrap_or_default(),
                    cumulative_gas_used: U256::zero(),
                    effective_gas_price: None,
                }),
            };
            info!("Successfully retrieved transaction: {:?}", txn_info);

            Ok(txn_info)
        } else {
            warn!("No transaction found for the provided hash");
            Err(ChainCommunicationError::Other(
                HyperlaneCustomErrorWrapper::new(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No transactions found",
                ))),
            ))
        }
    }

    async fn is_contract(&self, address: &H256) -> ChainResult<bool> {
        info!("Checking if contract exists at address: {:?}", address);

        let ton_address = TonAddress::from_base64_url(&address.to_string()).map_err(|e| {
            ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
        })?;

        let account_state = self
            .ton_client
            .get_account_state(&ton_address)
            .await
            .map_err(|e| {
                warn!("Failed to get account state: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        match account_state.account_state {
            AccountState::Uninited { .. } => {
                info!("Account is uninitialized.");
                Ok(false)
            }
            _ => {
                info!("Account is initialized.");
                Ok(true)
            }
        }
    }

    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        info!("Try send get_balance request for: {:?}", address);

        let address = TonAddress::from_base64_url(address.as_str());
        match address {
            Ok(address) => {
                let account_state =
                    self.ton_client
                        .get_account_state(&address)
                        .await
                        .map_err(|e| {
                            info!("Error while getting account state: {:?}", e);
                            ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(
                                Box::new(e),
                            ))
                        })?;

                info!("Get account state success, data: {:?}", account_state);

                let balance: i64 = account_state.balance;
                let balance_u256: U256 = U256::from(balance);

                Ok(balance_u256)
            }
            Err(error) => Err(ChainCommunicationError::Other(
                HyperlaneCustomErrorWrapper::new(Box::new(error)),
            )),
        }
    }

    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        Ok(None)
    }
}
