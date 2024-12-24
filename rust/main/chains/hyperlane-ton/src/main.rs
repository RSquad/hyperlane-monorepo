use hyperlane_ton::wrappers::test_context::TestContext;
use tonlib_core::{
    mnemonic::Mnemonic,
    wallet::{TonWallet, WalletVersion},
};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let phrase = vec!["kitchen"];
    let mnemonic = Mnemonic::new(phrase.clone(), &None)?;
    let key_pair = mnemonic.to_key_pair()?;
    let wallet = TonWallet::derive_default(WalletVersion::V4R2, &key_pair)?;

    let test_context = TestContext::new(
        "https://testnet.toncenter.com/api/",
        "",
        &wallet,
        "EQC5xrynw_llDS7czwH70rIeiblbn0rbtk-zjI8erKyIMTN6",
        "EQD-lhO00d-pZDYRP6tzDvuqSIUcCUukZFScP9zOQ1aNHKBh",
        "0QCvsB60DElBwHpHOj26K9NfxGJgzes_5pzwV48QGxHar9r9",
        "EQBxuFfnP5UVFIeWBiZJ9UStEGEVW_DqIgETU36GSkrhWuzD",
        "EQDnXpCHMJsCm7m2GPbFVNGsMdU-BxMsMZ-Vzqc1KTB4Tw2z",
        "EQB5cet7borOx5YT_muDLy7OtfVpENStjvUQKYOGK3p-jWuC",
    )
    .unwrap();

    test_context.test_merkle_tree_hook_tree().await.unwrap();

    Ok(())
}
