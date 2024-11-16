use anyhow::Error;
use hyperlane_core::{ChainCommunicationError, ChainCommunicationError::HyperlaneSignerError};
use log::warn;
use tonlib_core::{
    cell::{ArcCell, BagOfCells, Cell},
    mnemonic::{KeyPair, Mnemonic},
    wallet::{TonWallet, WalletVersion},
    TonAddress,
};

#[derive(Clone)]
pub struct TonSigner {
    pub address: TonAddress,
    private_key: Vec<u8>,
    pub wallet: TonWallet,
}

impl TonSigner {
    pub fn new(key_pair: KeyPair, wallet_version: WalletVersion) -> Result<Self, Error> {
        let wallet =
            TonWallet::derive_default(wallet_version, &key_pair).map_err(|e| Error::new(e))?;

        Ok(TonSigner {
            address: wallet.address.clone(),
            private_key: key_pair.secret_key,
            wallet,
        })
    }
    pub fn from_mnemonic(
        mnemonic_phrase: Vec<String>,
        wallet_version: WalletVersion,
    ) -> Result<Self, Error> {
        let mnemonic_phrase_str: Vec<&str> =
            mnemonic_phrase.iter().map(|item| item.as_str()).collect();
        let mnemonic = Mnemonic::new(mnemonic_phrase_str, &None)
            .expect("Failed to create Mnemonic from phrase")?;

        let key_pair = mnemonic
            .to_key_pair()
            .expect("Failed to generate KeyPair from mnemonic")?;

        Self::new(key_pair, wallet_version)
    }
    pub async fn sign_message(&self, body: &Cell) -> Result<Vec<u8>, Error> {
        let signature = self
            .wallet
            .sign_external_body(body)
            .map_err(|e| Error::new(e))?;

        Ok(signature.data().to_vec())
    }
    pub async fn create_signed_message(
        &self,
        transfer_message: Cell,
        now: u32,
        seqno: u32,
    ) -> Result<String, ChainCommunicationError> {
        let message = self
            .wallet
            .create_external_message(now + 60, seqno, vec![ArcCell::new(transfer_message)], false)
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!("Failed to create message: {}", e))
            })?;

        let boc = BagOfCells::from_root(message)
            .serialize(true)
            .map_err(|e| {
                ChainCommunicationError::CustomError(format!("Failed to serialize BOC: {}", e))
            })?;

        Ok(base64::encode(boc))
    }
}
