use crate::client::provider::TonProvider;
use anyhow::anyhow;
use reqwest::Client;
use tonlib::client::TonClient;
use url::Url;

mod client;
mod trait_builder;
mod traits;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    Ok(())
}
