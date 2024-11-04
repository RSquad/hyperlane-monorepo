use anyhow::Error;
use hyperlane_core::ChainCommunicationError::HyperlaneSignerError;
use log::warn;
use tonlib::mnemonic::KeyPair;
use tonlib::{
    address::TonAddress,
    cell::Cell,
    wallet::{TonWallet, WalletVersion},
};

#[derive(Clone)]
pub struct TonSigner {
    pub address: TonAddress,
    private_key: Vec<u8>,
    wallet: TonWallet,
}

impl TonSigner {
    pub fn new(
        key_pair: KeyPair,
        wallet_version: WalletVersion,
    ) -> Result<Self, HyperlaneSignerError> {
        let wallet = TonWallet::derive_default(wallet_version, &key_pair)
            .map_err(|e| HyperlaneSignerError::CustomError(e.to_string()))?;

        Ok(TonSigner {
            address: wallet.address.clone(),
            private_key: key_pair.secret_key,
            wallet,
        })
    }
    pub async fn sign_message(&self, body: &Cell) -> Result<Vec<u8>, Error> {
        let signature = self
            .wallet
            .sign_external_body(body)
            .map_err(|e| HyperlaneSignerError::CustomError(e.to_string()))?;

        Ok(signature.data().to_vec())
    }
}
