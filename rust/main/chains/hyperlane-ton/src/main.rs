use anyhow::anyhow;

mod types;

mod client;
use crate::client::provider::TonProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    Ok(())
}
