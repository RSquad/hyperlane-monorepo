use std::{collections::BTreeMap, fmt::Error, fs};

use hyperlane_ton::ConversionUtils;
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
    pub wallet_version: String,
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
        wallet_version: &str,
        mailbox: &str,
        igp: &str,
        validator_announce: &str,
        merkle_tree_hook: &str,
    ) -> Self {
        let mnemonic_vec: Vec<String> = signer_phrase
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        TonAgentConfig {
            name: name.to_string(),
            domain_id,
            metrics_port: 9093,
            mailbox: prepare_address(mailbox),
            interchain_gas_paymaster: prepare_address(igp),
            validator_announce: prepare_address(validator_announce),
            merkle_tree_hook: prepare_address(merkle_tree_hook),
            protocol: "ton".to_string(),
            chain_id: format!("{}", domain_id),
            rpc_urls: vec![AgentUrl {
                http: rpc_url.to_string(),
            }],
            api_key: api_key.to_string(),
            signer: AgentConfigSigner {
                typ: "TonMnemonic".to_string(),
                mnemonic_phrase: mnemonic_vec.join(" "),
                wallet_version: wallet_version.to_string(),
            },

            gas_price: RawTonAmount {
                denom: "ton".to_string(),
                amount: "0.01".to_string(),
            },
            contract_address_bytes: 32,
            index: AgentConfigIndex {
                from: 1,
                chunk: 26543528,
            },
        }
    }
}

fn prepare_address(base64_addr: &str) -> String {
    format!(
        "0x{}",
        hex::encode(
            ConversionUtils::ton_address_to_h256(
                &TonAddress::from_base64_url(base64_addr).unwrap()
            )
            .as_bytes()
        )
    )
}

pub fn generate_ton_config(
    output_name: &str,
    mnemonic: &str,
    wallet_version: &str,
    api_key: &str,
) -> Result<Vec<TonAgentConfig>, Error> {
    let output_path = format!("../../config/{output_name}.json");

    let mnemonic = mnemonic.to_string();
    let addresses = [
        (
            "tontest1",
            777001,
            "EQCyAx6WlFz344IgGCf9XtLawsvEniHXcFqlCxhWGc0UtNmv", // Mailbox
            "EQD-lhO00d-pZDYRP6tzDvuqSIUcCUukZFScP9zOQ1aNHKBh", // IGP
            "EQB6-6wh6JQlZ_vG_Ep6BF10rcZlojzQ2g38Lci7MCnVdk9s", // Validator Announce
            "EQDIcqH1ubCL8eirRGDe3ztAtKgaDoGFw5HU2HO7tvFX4AMZ", // merkle tree hook
        ),
        (
            "tontest2",
            777002,
            "EQDWmnFOuZAjE-H1FbTitj0EoKeakr-wIzQdQGKy-uLXcZBI", // Mailbox
            "EQD-lhO00d-pZDYRP6tzDvuqSIUcCUukZFScP9zOQ1aNHKBh", // IGP
            "EQBpL_niCuX7Yi1QAnTVjqSfXE3NHHXn8Ef6NkxjvYz135c1", // Validator Announce
            "EQDIcqH1ubCL8eirRGDe3ztAtKgaDoGFw5HU2HO7tvFX4AMZ", // merkle tree hook
        ),
    ];

    let ton_chains: Vec<TonAgentConfig> = addresses
        .iter()
        .map(
            |(name, domain_id, mailbox, igp, validator_announce, merkle_tree_hook)| {
                TonAgentConfig::new(
                    name,
                    *domain_id,
                    "https://testnet.toncenter.com/api/",
                    api_key,
                    mnemonic.as_str(),
                    wallet_version,
                    mailbox,
                    igp,
                    validator_announce,
                    merkle_tree_hook,
                )
            },
        )
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
