use async_trait::async_trait;
use derive_new::new;
use log::{debug, info, warn};
use std::error::Error;
use std::ops::Add;
use std::sync::Arc;

use hyperlane_core::{
    BlockInfo, ChainCommunicationError, ChainInfo, ChainResult, ContractLocator, HyperlaneChain,
    HyperlaneCustomErrorWrapper, HyperlaneDomain, HyperlaneDomainProtocol,
    HyperlaneDomainTechnicalStack, HyperlaneDomainType, HyperlaneProvider, HyperlaneProviderError,
    TxnInfo, TxnReceiptInfo, H256, U256,
};
use reqwest::{Client, Response};
use serde_json::json;
use tokio::{sync::RwLock, time::Sleep};

use tonlib::tl::{AccountState, BlockIdExt};
use tonlib::{
    address::{TonAddress, TonAddressParseError},
    client::{
        TonClient, TonClientBuilder, TonClientError, TonClientInterface, TonConnection,
        TonConnectionParams,
    },
};

use crate::types::account_state::AccountStateResponse;
use crate::types::run_get_method::RunGetMethodResponse;
use crate::{
    trait_builder::TonConnectionConf,
    traits::ton_api_center::TonApiCenter,
    types::{message::MessageResponse, transaction::TransactionResponse},
};

#[derive(Clone, new)]
pub struct TonProvider {
    pub ton_client: TonClient,
    pub http_client: Client,
    pub connection_conf: TonConnectionConf,
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
    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl HyperlaneProvider for TonProvider {
    async fn get_block_by_hash(&self, hash: &H256) -> ChainResult<BlockInfo> {
        info!("Fetching block by hash: {:?}", hash);

        // need check
        let block_id: BlockIdExt = BlockIdExt {
            workchain: 0,
            shard: 0,
            seqno: 0,
            root_hash: hash.to_string(),
            file_hash: "".to_string(),
        };
        debug!("Constructed BlockIdExt: {:?}", block_id);

        let blocks_header = self.ton_client.get_block_header(&block_id).await;

        match blocks_header {
            Ok(blocks_header) => {
                let block_info: BlockInfo = BlockInfo {
                    hash: H256::from_slice(&blocks_header.id.root_hash.as_bytes()),
                    timestamp: blocks_header.gen_utime as u64,
                    number: blocks_header.id.seqno as u64,
                };
                info!("Successfully fetched block info: {:?}", block_info);
                Ok(block_info)
            }
            Err(e) => {
                warn!("Failed to fetch block by hash: {:?}", e);
                Err(ChainCommunicationError::Other(
                    HyperlaneCustomErrorWrapper::new(Box::new(e)),
                ))
            }
        }
    }

    async fn get_txn_by_hash(&self, hash: &H256) -> ChainResult<TxnInfo> {
        info!("Fetching transaction by hash: {:?}", hash);

        let url = self
            .connection_conf
            .url
            .join("v3/transactions")
            .map_err(|e| {
                warn!("Failed to construct transaction URL: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        debug!("Constructed transaction URL: {}", url);

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
            warn!("Failed to parse address: {:?}", e);
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
        info!("Fetching balance for address: {:?}", address);

        match self.get_account_state(address, false).await {
            Ok(account_state) => {
                if let Some(first_account) = account_state.accounts.get(0) {
                    let balance: U256 =
                        U256::from_dec_str(first_account.balance.as_deref().ok_or_else(|| {
                            ChainCommunicationError::ParseError {
                                msg: "No balance found in the response".to_string(),
                            }
                        })?)?;

                    info!("Successfully retrieved balance: {:?}", balance);
                    Ok(balance)
                } else {
                    warn!("No account found in the response");
                    Err(ChainCommunicationError::CustomError {
                        0: "No account found in the response".to_string(),
                    })
                }
            }
            Err(e) => {
                warn!("Error while getting account state: {:?}", e);
                Err(ChainCommunicationError::CustomError {
                    0: format!("Error while getting account state: {:?}", e),
                })
            }
        }
    }

    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        Ok(None)
    }
}

#[async_trait]
impl TonApiCenter for TonProvider {
    /// Implements a method to retrieve messages from the TON network based on specified filters.
    /// Parameters include message hashes, source and destination addresses,
    /// and time or other filters for querying messages.
    /// Returns the results in a `MessageResponse` format.
    async fn get_messages(
        &self,
        msg_hash: Option<Vec<String>>,
        body_hash: Option<String>,
        source: Option<String>,
        destination: Option<String>,
        opcode: Option<String>,
        start_utime: Option<i64>,
        end_utime: Option<i64>,
        start_lt: Option<i64>,
        end_lt: Option<i64>,
        direction: Option<String>,
        limit: Option<u32>,
        offset: Option<u32>,
        sort: Option<String>,
    ) -> Result<MessageResponse, Box<dyn std::error::Error>> {
        info!("Fetching messages with filters");

        let url = self.connection_conf.url.join("v3/messages").map_err(|e| {
            warn!("Failed to construct messages URL: {:?}", e);
            ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
        })?;

        debug!("Constructed messages URL: {}", url);

        let params: Vec<(&str, String)> = vec![
            ("msg_hash", msg_hash.map(|v| v.join(","))),
            ("body_hash", body_hash),
            ("source", source),
            ("destination", destination),
            ("opcode", opcode),
            ("start_utime", start_utime.map(|v| v.to_string())),
            ("end_utime", end_utime.map(|v| v.to_string())),
            ("start_lt", start_lt.map(|v| v.to_string())),
            ("end_lt", end_lt.map(|v| v.to_string())),
            ("direction", direction),
            ("limit", limit.map(|v| v.to_string())),
            ("offset", offset.map(|v| v.to_string())),
            ("sort", sort),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect();

        debug!("Constructed query parameters for messages: {:?}", params);

        let response = self
            .http_client
            .get(url)
            .bearer_auth(&self.connection_conf.api_key)
            .query(&params)
            .send()
            .await?;

        let response_text = response.text().await;
        match response_text {
            Ok(text) => {
                debug!("Received response text: {:?}", text);

                let message_response: Result<MessageResponse, _> = serde_json::from_str(&text);
                match message_response {
                    Ok(parsed_response) => {
                        info!("Successfully parsed message response");
                        Ok(parsed_response)
                    }
                    Err(e) => {
                        warn!("Error parsing message response: {:?}", e);
                        Err(Box::new(e) as Box<dyn std::error::Error>)
                    }
                }
            }
            Err(e) => {
                warn!("Error retrieving message response text: {:?}", e);
                Err(Box::new(e) as Box<dyn std::error::Error>)
            }
        }
    }
    async fn get_transactions(
        &self,
        workchain: Option<i32>,
        shard: Option<String>,
        seqno: Option<i32>,
        mc_seqno: Option<i32>,
        account: Option<Vec<String>>,
        exclude_account: Option<Vec<String>>,
        hash: Option<String>,
        lt: Option<i64>,
        start_utime: Option<i64>,
        end_utime: Option<i64>,
        start_lt: Option<i64>,
        end_lt: Option<i64>,
        limit: Option<u32>,
        offset: Option<u32>,
        sort: Option<String>,
    ) -> Result<TransactionResponse, Box<dyn Error>> {
        info!("Fetching transactions with filters");

        let url = self
            .connection_conf
            .url
            .join("v3/transactions")
            .map_err(|e| {
                warn!("Failed to construct transactions URL: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        debug!("Constructed transactions URL: {}", url);

        let query_params: Vec<(&str, String)> = vec![
            ("workchain", workchain.map(|v| v.to_string())),
            ("shard", shard),
            ("seqno", seqno.map(|v| v.to_string())),
            ("mc_seqno", mc_seqno.map(|v| v.to_string())),
            ("account", account.map(|v| v.join(","))),
            ("exclude_account", exclude_account.map(|v| v.join(","))),
            ("hash", hash),
            ("lt", lt.map(|v| v.to_string())),
            ("start_utime", start_utime.map(|v| v.to_string())),
            ("end_utime", end_utime.map(|v| v.to_string())),
            ("start_lt", start_lt.map(|v| v.to_string())),
            ("end_lt", end_lt.map(|v| v.to_string())),
            ("limit", limit.map(|v| v.to_string())),
            ("offset", offset.map(|v| v.to_string())),
            ("sort", sort),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect();

        let response = self
            .http_client
            .get(url)
            .bearer_auth(&self.connection_conf.api_key)
            .query(&query_params)
            .send()
            .await?
            .json::<TransactionResponse>()
            .await?;

        info!("Successfully retrieved transaction response");
        Ok(response)
    }

    async fn get_account_state(
        &self,
        address: String,
        include_boc: bool,
    ) -> Result<AccountStateResponse, Box<dyn Error>> {
        info!(
            "Fetching account state for address: {:?}, include_boc: {:?}",
            address, include_boc
        );

        let url = self
            .connection_conf
            .url
            .join("v3/accountStates")
            .map_err(|e| {
                warn!("Failed to construct account state URL: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        let query_params: Vec<(&str, String)> = vec![
            ("address", address),
            ("include_boc", include_boc.to_string()),
        ];

        let response = self
            .http_client
            .get(url)
            .bearer_auth(&self.connection_conf.api_key)
            .query(&query_params)
            .send()
            .await?
            .json::<AccountStateResponse>()
            .await?;

        info!("Successfully retrieved account state response");
        Ok(response)
    }

    async fn run_get_method(
        &self,
        address: String,
        method: String,
        stack: Option<Vec<String>>,
    ) -> Result<RunGetMethodResponse, Box<dyn Error>> {
        info!(
            "Calling get method for address: {:?}, method: {:?}, stack: {:?}",
            address, method, stack
        );

        let url = self
            .connection_conf
            .url
            .join("v3/runGetMethod")
            .map_err(|e| {
                warn!("Failed to construct account state URL: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        info!("Url:{:?}", url);

        let stack_data = stack.unwrap_or_else(|| vec![]);

        let params = json!({
            "address": address,
            "method": method,
            "stack": stack_data
        });
        info!(
            "Constructed runGetMethod request body: {:?}",
            params.to_string()
        );

        let response = self
            .http_client
            .post(url)
            .bearer_auth(&self.connection_conf.api_key)
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&params)
            .send()
            .await?;

        let response_text = response.text().await?;
        info!("Received response text: {:?}", response_text);

        let parsed_response = serde_json::from_str::<RunGetMethodResponse>(&response_text)
            .map_err(|e| {
                warn!("Error parsing JSON response: {:?}", e);
                Box::new(e) as Box<dyn Error>
            })?;

        info!("Successfully run get method request");
        Ok(parsed_response)
    }
}
