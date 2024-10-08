use crate::client::provider::TonProvider;
use log::info;
use pretty_env_logger::env_logger;
mod client;
mod trait_builder;
mod traits;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("start");
}
