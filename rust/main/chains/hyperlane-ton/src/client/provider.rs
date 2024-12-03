use async_trait::async_trait;
use derive_new::new;
use log::{debug, info, warn};
use std::error::Error;
use std::str::FromStr;

use hyperlane_core::{
    BlockInfo, ChainCommunicationError, ChainInfo, ChainResult, FixedPointNumber, HyperlaneChain,
    HyperlaneCustomErrorWrapper, HyperlaneDomain, HyperlaneProvider, TxOutcome, TxnInfo,
    TxnReceiptInfo, H256, U256,
};
use reqwest::Client;
use serde_json::json;
use tokio::time::sleep;

use crate::client::error::CustomHyperlaneError;
use crate::{
    trait_builder::TonConnectionConf,
    traits::ton_api_center::TonApiCenter,
    types::{
        account_state::AccountStateResponse, block_response::BlockResponse,
        message::MessageResponse, message::SendMessageResponse,
        run_get_method::RunGetMethodResponse, transaction::TransactionResponse,
        wallet_state::WalletStatesResponse,
    },
    utils::conversion::ConversionUtils,
};
use tonlib_core::TonAddress;

#[derive(Clone, new)]
pub struct TonProvider {
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
        todo!()
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
                recipient: transaction.in_msg.as_ref().and_then(|msg| {
                    match TonAddress::from_base64_url(msg.destination.as_str()) {
                        Ok(ton_address) => ConversionUtils::ton_address_to_h256(&ton_address).ok(),
                        Err(e) => {
                            warn!(
                                "Failed to parse TON address from destination '{}': {:?}",
                                msg.destination, e
                            );
                            None
                        }
                    }
                }),
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

        let ton_address = ConversionUtils::h256_to_ton_address(address, 0).map_err(|e| {
            warn!("Failed to parse address: {:?}", e);
            ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(
                CustomHyperlaneError(format!("Failed to parse address: {:?}", e)),
            )))
        })?;

        let account_state = match self.get_account_state(ton_address.to_string(), true).await {
            Ok(state) => state,
            Err(e) => {
                warn!(
                    "Failed to get account state for address {:?}: {:?}",
                    ton_address, e
                );
                return Err(ChainCommunicationError::Other(
                    HyperlaneCustomErrorWrapper::new(Box::new(CustomHyperlaneError(format!(
                        "Failed to get account state for address {:?}: {:?}",
                        ton_address, e
                    )))),
                ));
            }
        };

        let account = match account_state.accounts.first() {
            Some(account) => account,
            None => {
                warn!(
                    "No account found for the address: {:?}. Assuming it is not a contract.",
                    ton_address
                );
                return Ok(false);
            }
        };

        if account.code_boc.is_some() {
            info!("Address {:?} is a contract.", ton_address);
            Ok(true)
        } else {
            info!("Address {:?} is not a contract.", ton_address);
            Ok(false)
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
    ) -> Result<MessageResponse, Box<dyn Error>> {
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

        info!("Constructed query parameters for messages: {:?}", params);

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
                info!("Received response text: {:?}", text);

                let message_response: Result<MessageResponse, _> = serde_json::from_str(&text);
                match message_response {
                    Ok(parsed_response) => {
                        info!("Successfully parsed message response");
                        Ok(parsed_response)
                    }
                    Err(e) => {
                        warn!("Error parsing message response: {:?}", e);
                        Err(Box::new(e) as Box<dyn Error>)
                    }
                }
            }
            Err(e) => {
                warn!("Error retrieving message response text: {:?}", e);
                Err(Box::new(e) as Box<dyn Error>)
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
    ) -> Result<RunGetMethodResponse, Box<dyn Error + Send + Sync>> {
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
            .await
            .map_err(|e| {
                warn!("Error sending request: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get response text".to_string());
            warn!("Request failed with status: {}, body: {}", status, body);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Request failed with status: {}", status),
            )) as Box<dyn Error + Send + Sync>);
        }

        let response_text = response.text().await.map_err(|e| {
            warn!("Error retrieving response text: {:?}", e);
            Box::new(e) as Box<dyn Error + Send + Sync>
        })?;
        info!("Received response text: {:?}", response_text);

        let parsed_response = serde_json::from_str::<RunGetMethodResponse>(&response_text)
            .map_err(|e| {
                warn!("Error parsing JSON response: {:?}", e);
                Box::new(e) as Box<dyn Error + Send + Sync>
            })?;

        info!("Successfully executed run_get_method request");
        Ok(parsed_response)
    }

    async fn send_message(&self, boc: String) -> Result<SendMessageResponse, Box<dyn Error>> {
        let url = self.connection_conf.url.join("v3/message").map_err(|e| {
            warn!("Failed to construct message URL: {:?}", e);
            Box::new(e) as Box<dyn Error>
        })?;

        let params = json!({
            "boc": boc
        });

        let response = self
            .http_client
            .post(url)
            .bearer_auth(&self.connection_conf.api_key)
            .json(&params)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;

        let send_message_response: SendMessageResponse = response.json().await.map_err(|e| {
            warn!("Error parsing send_message response: {:?}", e);
            Box::new(e) as Box<dyn Error>
        })?;

        Ok(send_message_response)
    }

    async fn get_wallet_states(
        &self,
        mut account: String,
    ) -> Result<WalletStatesResponse, Box<dyn Error>> {
        if account.starts_with("0x") {
            let h256 = H256::from_str(&account[2..]).map_err(|e| {
                warn!("Failed to parse H256 address: {:?}", e);
                Box::new(e) as Box<dyn Error>
            })?;

            match ConversionUtils::h256_to_ton_address(&h256, 0) {
                Ok(ton_address) => account = ton_address.to_string(),
                Err(e) => {
                    warn!("Failed to convert H256 to TonAddress: {:?}", e);
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Conversion failed: {:?}", e),
                    )));
                }
            }
        }
        let mut url = self
            .connection_conf
            .url
            .join("v3/walletStates")
            .map_err(|e| {
                warn!("Failed to construct wallet states URL: {:?}", e);
                Box::new(e) as Box<dyn Error>
            })?;

        url.query_pairs_mut().append_pair("address", &account);
        info!("URL:{:?}", url);

        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed".to_string());
            warn!("Error request: status = {}, body = {}", status, body);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Error",
            )));
        }

        let body = response.text().await?;
        println!("Server response: {}", body);

        let result: WalletStatesResponse = serde_json::from_str(&body).map_err(|e| {
            warn!("Failed deserialization: {:?}", e);
            Box::new(e) as Box<dyn Error>
        })?;

        Ok(result)
    }

    async fn get_transaction_by_message(
        &self,
        msg_hash: String,
        body_hash: Option<String>,
        opcode: Option<String>,
    ) -> Result<TransactionResponse, Box<dyn Error>> {
        info!("Fetching transactions by message");

        let url = self
            .connection_conf
            .url
            .join("v3/transactionsByMessage")
            .map_err(|e| {
                warn!("Failed to construct transactions URL: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        debug!("Constructed transactions URL: {}", url);

        let query_params: Vec<(&str, String)> = vec![
            ("msg_hash", msg_hash),
            ("body_hash", body_hash.unwrap_or_default()),
            ("opcode", opcode.unwrap_or_default()),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .collect();

        let raw_response = self
            .http_client
            .get(url)
            .bearer_auth(&self.connection_conf.api_key)
            .query(&query_params)
            .send()
            .await?
            .text()
            .await?;

        info!("Raw response from server: {}", raw_response);

        let response: TransactionResponse = serde_json::from_str(&raw_response)?;

        info!("Successfully retrieved transaction response");
        Ok(response)
    }

    async fn get_blocks(
        &self,
        workchain: i32,
        shard: Option<String>,
        seqno: Option<i32>,
        mc_seqno: Option<i32>,
        start_utime: Option<i64>,
        end_utime: Option<i64>,
        start_lt: Option<i64>,
        end_lt: Option<i64>,
        limit: Option<u32>,
        offset: Option<u32>,
        sort: Option<String>,
    ) -> Result<BlockResponse, Box<dyn Error>> {
        info!("Fetching blocks by parameters");

        let url = self.connection_conf.url.join("v3/blocks").map_err(|e| {
            warn!("Failed to construct transactions URL: {:?}", e);
            ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
        })?;

        info!("Constructed transactions URL: {}", url);

        let query_params: Vec<(&str, String)> = vec![
            ("workchain", workchain.to_string()),
            ("shard", shard.unwrap_or_default()),
            ("seqno", seqno.map_or("".to_string(), |s| s.to_string())),
            (
                "mc_seqno",
                mc_seqno.map_or("".to_string(), |s| s.to_string()),
            ),
            (
                "start_utime",
                start_utime.map_or("".to_string(), |s| s.to_string()),
            ),
            (
                "end_utime",
                end_utime.map_or("".to_string(), |s| s.to_string()),
            ),
            (
                "start_lt",
                start_lt.map_or("".to_string(), |s| s.to_string()),
            ),
            ("end_lt", end_lt.map_or("".to_string(), |s| s.to_string())),
            ("limit", limit.map_or("10".to_string(), |l| l.to_string())),
            ("offset", offset.map_or("0".to_string(), |o| o.to_string())),
            ("sort", sort.unwrap_or("desc".to_string())),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .collect();

        info!("Query params:{:?}", query_params);

        let raw_response = self
            .http_client
            .get(url)
            .query(&query_params)
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .header("X-API-Key", self.connection_conf.api_key.clone())
            .send()
            .await
            .map_err(|e| {
                warn!("Error sending request to fetch blocks: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?
            .text()
            .await
            .map_err(|e| {
                warn!("Error reading response text while fetching blocks: {:?}", e);
                ChainCommunicationError::Other(HyperlaneCustomErrorWrapper::new(Box::new(e)))
            })?;

        info!("Raw response from server: {}", raw_response);

        let response: BlockResponse = serde_json::from_str(&raw_response)?;

        info!("Successfully retrieved blocks response");
        Ok(response)
    }
}

impl TonProvider {
    pub async fn wait_for_transaction(&self, message_hash: String) -> ChainResult<TxOutcome> {
        let max_attempts = self.connection_conf.max_attempts;
        let delay = self.connection_conf.timeout;

        for attempt in 1..=max_attempts {
            tracing::info!("Attempt {}/{}", attempt, max_attempts);

            match self
                .get_transaction_by_message(message_hash.clone(), None, None)
                .await
            {
                Ok(response) => {
                    if response.transactions.is_empty() {
                        info!("Transaction not found, retrying...");
                    } else {
                        info!(
                            "Transaction found: {:?}",
                            response
                                .transactions
                                .first()
                                .expect("Failed to get first transaction from list")
                        );

                        if let Some(transaction) = response.transactions.first() {
                            let transaction_id = ConversionUtils::base64_to_h512(&transaction.hash)
                                .map_err(|e| {
                                    ChainCommunicationError::CustomError(format!(
                                        "Failed to convert hash to H512: {:?}",
                                        e
                                    ))
                                })?;

                            let tx_outcome = TxOutcome {
                                transaction_id, // at least now
                                executed: !transaction.description.aborted,
                                gas_used: U256::from_dec_str(
                                    &transaction.description.compute_ph.gas_used,
                                )
                                .unwrap_or_else(|_| {
                                    warn!("Failed to parse gas used; defaulting to 0.");
                                    U256::zero()
                                }),
                                gas_price: FixedPointNumber::from(0),
                            };

                            info!("Tx outcome: {:?}", tx_outcome);
                            return Ok(tx_outcome);
                        }
                    }
                }
                Err(e) => {
                    tracing::info!("Transaction not found, retrying... {:?}", e);
                    if attempt == max_attempts {
                        return Err(ChainCommunicationError::CustomError(
                            "Transaction not found after max attempts".to_string(),
                        ));
                    }
                }
            }

            sleep(delay).await;
        }

        Err(ChainCommunicationError::CustomError("Timeout".to_string()))
    }
}
