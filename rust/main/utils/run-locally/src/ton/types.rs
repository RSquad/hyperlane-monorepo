use hyperlane_ton::DebugWalletVersion;
use std::collections::BTreeMap;
use std::fmt::Error;
use std::fs;
use std::hash::Hash;

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
            mailbox: "0x12345".to_string(),
            interchain_gas_paymaster: "0x67890".to_string(),
            validator_announce: "0xabcdef".to_string(),
            merkle_tree_hook: "0xfedcba".to_string(),
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
            index: AgentConfigIndex { from: 1, chunk: 5 },
        }
    }
}

pub fn generate_ton_config(output_name: &str) -> Result<Vec<TonAgentConfig>, Error> {
    let output_path = format!("/../../{output_name}.json");

    let mnemonic = "".to_string();

    let mut ton_chains = vec![
        TonAgentConfig::new(
            "tontest1",
            777001,
            "https://testnet.toncenter.com/api/",
            "",
            mnemonic.as_str(),
        ),
        TonAgentConfig::new(
            "tontest2",
            777002,
            "https://testnet.toncenter.com/api/",
            "",
            mnemonic.as_str(),
        ),
    ];

    let zero_address =
        String::from("0x0000000000000000000000000000000000000000000000000000000000000000");

    for chain in &mut ton_chains {
        chain.mailbox = zero_address.clone();
        chain.interchain_gas_paymaster = zero_address.clone();
        chain.validator_announce = zero_address.clone();
        chain.merkle_tree_hook = zero_address.clone();
    }

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
