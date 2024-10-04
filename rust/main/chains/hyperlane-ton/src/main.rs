use anyhow::anyhow;
use reqwest::Client;
use tonlib::client::TonClient;

mod client;
mod traits;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    Ok(())
}
