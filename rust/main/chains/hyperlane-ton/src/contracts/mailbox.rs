use crate::client::provider::TonProvider;
use hyperlane_core::{
    ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain, HyperlaneMessage,
    HyperlaneProvider, Mailbox, TxCostEstimate, TxOutcome, H256, U256,
};
use std::fmt::{Debug, Formatter};
use std::num::NonZeroU64;
use tonlib::address::TonAddress;

pub struct TonMailbox {
    pub(crate) mailbox_address: TonAddress,
    pub(crate) provider: TonProvider,
}

impl HyperlaneContract for TonMailbox {
    fn address(&self) -> H256 {
        self.mailbox_address.to_bytes().into()
    }
}

impl HyperlaneChain for TonMailbox {
    fn domain(&self) -> &HyperlaneDomain {
        &self.provider.domain()
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

impl Debug for TonMailbox {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Mailbox for TonMailbox {
    async fn count(&self, lag: Option<NonZeroU64>) -> ChainResult<u32> {
        todo!()
    }

    async fn delivered(&self, id: H256) -> ChainResult<bool> {
        todo!()
    }

    async fn default_ism(&self) -> ChainResult<H256> {
        todo!()
    }

    async fn recipient_ism(&self, recipient: H256) -> ChainResult<H256> {
        todo!()
    }

    async fn process(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
        tx_gas_limit: Option<U256>,
    ) -> ChainResult<TxOutcome> {
        todo!()
    }

    async fn process_estimate_costs(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
    ) -> ChainResult<TxCostEstimate> {
        todo!()
    }

    fn process_calldata(&self, message: &HyperlaneMessage, metadata: &[u8]) -> Vec<u8> {
        todo!()
    }
}
