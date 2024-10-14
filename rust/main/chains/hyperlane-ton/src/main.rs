use crate::trait_builder::TonConnectionConf;
use crate::traits::ton_api_center::TonApiCenter;
use hyperlane_core::{HyperlaneDomain, HyperlaneProvider, KnownHyperlaneDomain, Mailbox, H256};
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
use crate::contracts::mailbox::TonMailbox;
use log::info;
use pretty_env_logger::env_logger;
use reqwest::Client;

mod client;
mod contracts;
mod trait_builder;
mod traits;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("start");

    let ton_client = create_mainnet_client().await;
    let http_client = Client::new();

    let connection_config = TonConnectionConf::new(
        Url::parse("https://testnet.toncenter.com/api/")?,
        "d0e66cd5bae6419130bc8b3b7e9ee6c675678d21be5f30c1b30619b219d27505".to_string(),
    );

    let domain = HyperlaneDomain::Known(KnownHyperlaneDomain::Sepolia);

    let mailbox = TonMailbox {
        mailbox_address: TonAddress::from_base64_url(
            "EQASffmsB4eQl0wJ4QlwD47fPtI68pbClgygrIe8H5y-SUjB",
        )
        .unwrap(),
        provider: TonProvider {
            ton_client,
            http_client,
            connection_conf: connection_config,
            domain,
        },
    };
    // let count = mailbox.count(None).await;
    // info!("Count:{:?}", count);
    //
    // let default_ism = mailbox.default_ism().await;
    // info!("Default ism:{:?}", default_ism);

    let delivered = mailbox.delivered(H256::from_low_u64_be(1)).await;
    info!("Delivered:{:?}", delivered);

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
