use anyhow::anyhow;

mod providers;
use crate::providers::provider::TonProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    Ok(())
}
