use crate::trait_builder::TonConnectionConf;
use crate::traits::ton_api_center::TonApiCenter;
use hyperlane_core::{
    HyperlaneDomain, HyperlaneMessage, HyperlaneProvider, InterchainSecurityModule,
    KnownHyperlaneDomain, Mailbox, H256,
};
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
use crate::contracts::interchain_security_module::TonInterchainSecurityModule;
use crate::contracts::mailbox::TonMailbox;
use log::info;
use pretty_env_logger::env_logger;
use reqwest::Client;

mod client;
mod contracts;
mod trait_builder;
mod traits;
mod types;
mod utils;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("start");
    let connection_config = TonConnectionConf::new(
        Url::parse("https://testnet.toncenter.com/api/")?,
        "".to_string(),
    );
    let mnemonic = Mnemonic::new(vec!["Hello world!"], &None)?;
    let key_pair = mnemonic.to_key_pair()?;
    let ton_client = create_mainnet_client().await;
    let http_client = Client::new();

    let wallet = TonWallet::derive_default(WalletVersion::V4R2, &key_pair)?;

    let domain = HyperlaneDomain::Known(KnownHyperlaneDomain::Sepolia);

    let provider = TonProvider {
        ton_client,
        http_client,
        connection_conf: connection_config,
        domain,
    };

    let mailbox = TonMailbox {
        workchain: 0,
        mailbox_address: TonAddress::from_base64_url(
            "EQCN3_ItzxJH9gZtslpbCDN5JE5QhHnm_1azzCbUELZfSMfg",
        )
        .unwrap(),
        provider: provider.clone(),
        wallet: wallet.clone(),
    };

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
