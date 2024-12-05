use hyperlane_core::H256;
use hyperlane_ton::{ConversionUtils, DebugWalletVersion};
use std::collections::BTreeMap;
use std::fmt::Error;
use std::fs;
use tonlib_core::TonAddress;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AgentUrl {
    pub http: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AgentConfigSigner {
    #[serde(rename = "type")]
    pub typ: String,
    pub mnemonic_phrase: String,
    pub wallet_version: DebugWalletVersion,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RawTonAmount {
    pub denom: String,
    pub amount: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AgentConfigIndex {
    pub from: u32,
    pub chunk: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TonAgentConfigOut {
    pub chains: BTreeMap<String, TonAgentConfig>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TonAgentConfig {
    pub name: String,
    pub domain_id: u32,
    pub metrics_port: u32,
    pub mailbox: String,
    pub interchain_gas_paymaster: String,
    pub validator_announce: String,
    pub merkle_tree_hook: String,
    pub protocol: String,
    pub chain_id: String,
    pub rpc_urls: Vec<AgentUrl>,
    pub api_key: String,
    pub signer: AgentConfigSigner,
    pub gas_price: RawTonAmount,
    pub contract_address_bytes: usize,
    pub index: AgentConfigIndex,
}

impl TonAgentConfig {
    pub fn new(
        name: &str,
        domain_id: u32,
        rpc_url: &str,
        api_key: &str,
        signer_phrase: &str,
        mailbox_address: &TonAddress,
        igp_address: &TonAddress,
        validator_announce_address: &TonAddress,
    ) -> Self {
        let mnemonic_vec: Vec<String> = signer_phrase
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let version = DebugWalletVersion::from_str("V4R2").unwrap();

        TonAgentConfig {
            name: name.to_string(),
            domain_id,
            metrics_port: 9093,
            mailbox: format!(
                "0x{}",
                hex::encode(
                    ConversionUtils::ton_address_to_h256(&mailbox_address)
                        .unwrap()
                        .as_bytes()
                )
            ),
            interchain_gas_paymaster: format!(
                "0x{}",
                hex::encode(
                    ConversionUtils::ton_address_to_h256(&igp_address)
                        .unwrap()
                        .as_bytes()
                )
            ),
            validator_announce: format!(
                "0x{}",
                hex::encode(
                    ConversionUtils::ton_address_to_h256(&validator_announce_address)
                        .unwrap()
                        .as_bytes()
                )
            ),
            merkle_tree_hook: format!("0x{}", hex::encode(H256::zero())),
            protocol: "ton".to_string(),
            chain_id: format!("{}", domain_id),
            rpc_urls: vec![AgentUrl {
                http: rpc_url.to_string(),
            }],
            api_key: api_key.to_string(),
            signer: AgentConfigSigner {
                typ: "TonMnemonic".to_string(),
                mnemonic_phrase: mnemonic_vec.join(" "),
                wallet_version: version,
            },

            gas_price: RawTonAmount {
                denom: "ton".to_string(),
                amount: "0.01".to_string(),
            },
            contract_address_bytes: 32,
            index: AgentConfigIndex {
                from: 1,
                chunk: 25624322,
            },
        }
    }
}

pub fn generate_ton_config(
    output_name: &str,
    mnemonic: &str,
) -> Result<Vec<TonAgentConfig>, Error> {
    let output_path = format!("../../config/{output_name}.json");

    let mnemonic = mnemonic.to_string();
    let addresses = [
        (
            "tontest1",
            777001,
            TonAddress::from_base64_url("EQBmFijEHuueqHQLlu0miQmsx6PXLKuMoEq5NvI1JY08SW30")
                .unwrap(), // Mailbox
            TonAddress::from_base64_url("EQBlC1mxaaAR76QM9LPMvHyd7Lnzni7D0uBhFWvEC1bnFQ3W")
                .unwrap(), // IGP
            TonAddress::from_base64_url("EQCi3mXdLbbSoFRKe3S7s6scEr_-9zwZfhQq54ruyaLqB5yQ")
                .unwrap(), // Validator Announce
        ),
        (
            "tontest2",
            777002,
            TonAddress::from_base64_url("EQCeq8WO5V_sfhjp2uaaW7msEdVTaFFvzeQr_f9Zt3_BdP4q")
                .unwrap(), // Mailbox
            TonAddress::from_base64_url("EQCJ88IkmfIkvelSmnAfpeR4dAjVarRJcv_YYYPNkI0astRO")
                .unwrap(), // IGP
            TonAddress::from_base64_url("EQAKc8EhIduN_bZDyNk6TzLxIfNdAAbzQ572XJLvYrZAHtel")
                .unwrap(), // Validator Announce
        ),
    ];

    let ton_chains: Vec<TonAgentConfig> = addresses
        .iter()
        .map(|(name, domain_id, mailbox, igp, validator_announce)| {
            TonAgentConfig::new(
                name,
                *domain_id,
                "https://testnet.toncenter.com/api/",
                "",
                mnemonic.as_str(),
                mailbox,
                igp,
                validator_announce,
            )
        })
        .collect();

    let mut chains_map = BTreeMap::new();
    for chain in ton_chains.clone() {
        chains_map.insert(chain.name.clone(), chain);
    }

    let ton_config = TonAgentConfigOut { chains: chains_map };

    let json_output = serde_json::to_string_pretty(&ton_config).unwrap();

    fs::write(output_path.as_str(), json_output).unwrap();
    println!("TON configuration written to {}", output_path.as_str());

    Ok(ton_chains)
}
