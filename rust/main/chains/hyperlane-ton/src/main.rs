use crate::trait_builder::TonConnectionConf;
use crate::traits::ton_api_center::TonApiCenter;
use hyperlane_core::{HyperlaneDomain, HyperlaneProvider, KnownHyperlaneDomain};
use tonlib::cell::ArcCell;
use tonlib::cell::CellSlice;
use tonlib::client::TonClient;
use tonlib::tl::{TvmSlice, TvmStackEntry};
use tonlib::wallet::WalletVersion;
use tonlib::{
    address::TonAddress,
    cell::{BagOfCells, Cell, CellBuilder},
    client::{TonClientBuilder, TonClientInterface, TonConnection, TonConnectionParams},
    config::{MAINNET_CONFIG, TESTNET_CONFIG},
    contract::{JettonMasterContract, TonContractFactory},
    message::TransferMessage,
    mnemonic::{self, KeyPair, Mnemonic},
    tl::{AccountAddress, AccountState, BlockId, Config, InternalTransactionId, Options},
    wallet::TonWallet,
};
use url::Url;

use crate::client::provider::TonProvider;
use log::info;
use pretty_env_logger::env_logger;
use reqwest::Client;

mod client;
mod trait_builder;
mod traits;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("start");

    Ok(())
}

pub async fn create_mainnet_client() -> TonClient {
    let params = TonConnectionParams {
        config: MAINNET_CONFIG.to_string(),
        ..Default::default()
    };
    TonClient::set_log_verbosity_level(1);
    let client = TonClientBuilder::new()
        .with_connection_params(&params)
        .with_pool_size(2)
        .with_logging_callback()
        //.with_keystore_dir("./var/ton/testnet".to_string())
        //.with_connection_check(ConnectionCheck::Archive)
        .build()
        .await
        .unwrap();

    client
}
