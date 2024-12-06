use crate::client::provider::TonProvider;
use crate::run_get_method::StackItem;
use crate::traits::ton_api_center::TonApiCenter;
use crate::ConversionUtils;
use async_trait::async_trait;
use derive_new::new;
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain,
    HyperlaneMessage, HyperlaneProvider, MultisigIsm, H256,
};
use num_bigint::BigUint;
use std::str::FromStr;
use tonlib_core::cell::dict::predefined_readers::{key_reader_u32, val_reader_cell};
use tonlib_core::{
    cell::{BagOfCells, CellBuilder},
    TonAddress,
};
use tracing::info;

#[derive(Clone, Debug, new)]
pub struct TonMultisigIsm {
    provider: TonProvider,
    multisig_address: TonAddress,
}

impl HyperlaneChain for TonMultisigIsm {
    fn domain(&self) -> &HyperlaneDomain {
        &self.provider.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

impl HyperlaneContract for TonMultisigIsm {
    fn address(&self) -> H256 {
        ConversionUtils::ton_address_to_h256(&self.multisig_address).unwrap()
    }
}

#[async_trait]
impl MultisigIsm for TonMultisigIsm {
    async fn validators_and_threshold(
        &self,
        message: &HyperlaneMessage,
    ) -> ChainResult<(Vec<H256>, u8)> {
        info!("validators_and_threshold call");
        let domain = message.origin;
        let mut builder = CellBuilder::new();

        info!("Domain:{:?}", domain);

        let stack = Some(vec![StackItem {
            r#type: "num".to_string(),
            value: domain.to_string(),
        }]);
        info!("Stack:{:?}", stack);

        let function_name = "get_validators_and_threshhold".to_string();
        let response = self
            .provider
            .run_get_method(self.multisig_address.to_hex(), function_name, stack)
            .await
            .expect("Failed response get_validators_and_threshhold");

        let threshold =
            u8::from_str_radix(response.stack.first().unwrap().value.get(2..).unwrap(), 16)
                .expect("");
        info!("threshold:{:?}", threshold);

        let cell = &response.stack.get(1).unwrap().value;

        let root_cell = ConversionUtils::parse_root_cell_from_boc(cell.as_str())
            .expect("Failed to parse_root_cell_from_boc in validators_and_threshold");

        let mut parser = root_cell.parser();
        let dict = parser
            .load_dict(32, key_reader_u32, val_reader_cell)
            .expect("aboba");

        let mut validators: Vec<H256> = vec![];

        for (key, value_cell) in &dict {
            info!("Key:{:?} value_cell:{:?}", key, value_cell);
            let mut validator_address = H256::zero();
            value_cell
                .parser()
                .load_slice(&mut validator_address.0)
                .expect("failed load_slice");

            validators.push(validator_address);
        }
        Ok((validators, threshold))
    }
}
