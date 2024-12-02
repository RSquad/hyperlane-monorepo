#![allow(dead_code)] // TODO: `rustc` 1.80.1 clippy issue

use crate::config::Config;
use crate::logging::log;
use crate::program::Program;
use crate::ton::client::send_messages_between_chains;
use crate::ton::deploy::deploy_all_contracts;
use crate::utils::{as_task, concat_path, make_static, stop_child, AgentHandles, TaskHandle};

use crate::ton::types::{generate_ton_config, TonAgentConfig};
use hyperlane_base::settings::parser::h_ton::{TonConnectionConf, TonProvider};
use hyperlane_core::{HyperlaneDomain, KnownHyperlaneDomain};
use log::{error, info};
use macro_rules_attribute::apply;
use reqwest::Client;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{env, fs};
use tempfile::tempdir;
use url::Url;

mod client;
mod deploy;
mod types;

const KEY_VALIDATOR1: (&str, &str) = (
    "validator1",
    "trend inflict kit vehicle gown route never damage spawn moon host tissue \
                     section drink creek erupt comic future link neutral seek nerve sugar degree",
);
const KEY_VALIDATOR2: (&str, &str) = (
    "validator2",
    "tell february meat pulp present shuffle round stove ginger kit like crack ill \
                     fence village gain answer route discover egg quiz dignity ocean water",
);
const KEY_RELAYER: (&str, &str) = ("relayer", "coffee foster dentist begin spirit pioneer someone peace bleak story door wasp clerk invest safe negative junk bacon hollow banana nation impact crowd kitchen");

fn default_keys<'a>() -> [(&'a str, &'a str); 3] {
    [KEY_VALIDATOR1, KEY_VALIDATOR2, KEY_RELAYER]
}

pub struct TonHyperlaneStack {
    pub validators: Vec<AgentHandles>,
    pub relayer: AgentHandles,
    pub scraper: AgentHandles,
    pub postgres: AgentHandles,
}

impl Drop for TonHyperlaneStack {
    fn drop(&mut self) {
        for v in &mut self.validators {
            stop_child(&mut v.1);
        }
        stop_child(&mut self.scraper.1);
        stop_child(&mut self.postgres.1);
        stop_child(&mut self.relayer.1);
    }
}

fn run_locally() {
    info!("Start run_locally() for Ton");
    let mnemonic = std::env::var("MNEMONIC").expect("MNEMONIC env is missing");

    let mailbox_address = "EQBMKHZ4kGptW4veOnDZoeVxahcc5ACyq4EIAcI0oqJBwB2v";
    let igp_address = "EQCHfzFW3GBgjUYRrQrnMh7bvDsbSTo3ehWLzKgZEdrQxlWE";
    let recipient_address = "EQBWmHkjpLAwyJ1qQwH9tIfDKiOyEIa_nH29iJon3qduwWBy";
    let multisig_address = "EQDiSTbhD8dbtUQTldaJ3mbkznRQpw1PzhMzo7GJBpopxxoQ";

    info!("current_dir: {}", env::current_dir().unwrap().display());
    let file_name = "ton_config";
    let agent_config = generate_ton_config(file_name, &mnemonic).unwrap();

    let agent_config_path = format!("../../config/{file_name}.json");

    info!("Agent config path:{}", agent_config_path);
    let relay_chains = vec!["tontest1".to_string(), "tontest2".to_string()];
    let metrics_port = 9090;
    let debug = false;

    // let deploy = deploy_all_contracts();

    let scraper_metrics_port = metrics_port + 10;
    info!("Running postgres db...");
    let postgres = Program::new("docker")
        .cmd("run")
        .flag("rm")
        .arg("name", "ton-scraper-postgres")
        .arg("env", "POSTGRES_PASSWORD=47221c18c610")
        .arg("publish", "5432:5432")
        .cmd("postgres:14")
        .spawn("SQL", None);

    sleep(Duration::from_secs(10));

    let relayer = launch_ton_relayer(
        agent_config_path.clone(),
        relay_chains.clone(),
        metrics_port,
        debug,
    );

    let validator1 = launch_ton_validator(
        agent_config_path.clone(),
        agent_config[0].clone(),
        metrics_port + 1,
        debug,
    );
    let validator2 = launch_ton_validator(
        agent_config_path.clone(),
        agent_config[1].clone(),
        metrics_port + 2,
        debug,
    );

    let validators = vec![validator1, validator2];

    let scraper = launch_ton_scraper(
        agent_config_path.clone(),
        relay_chains.clone(),
        scraper_metrics_port,
        debug,
    );

    send_messages_between_chains();

    info!("Waiting for agents to run for 2 minutes...");
    sleep(Duration::from_secs(120));

    let stack_ = TonHyperlaneStack {
        validators: validators.into_iter().map(|v| v.join()).collect(),
        relayer: relayer.join(),
        scraper: scraper.join(),
        postgres,
    };
}

#[apply(as_task)]
pub fn launch_ton_relayer(
    agent_config_path: String,
    relay_chains: Vec<String>,
    metrics: u32,
    debug: bool,
) -> AgentHandles {
    let relayer_bin = concat_path("../../target/debug", "relayer");
    let relayer_base = tempdir().unwrap();
    let mut configs_path = std::env::current_dir().unwrap();
    configs_path.push(agent_config_path);
    let config_files_str = configs_path
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let relayer = Program::default()
        .bin(relayer_bin)
        .working_dir("../../")
        .env("CONFIG_FILES", config_files_str)
        .env("RUST_BACKTRACE", "1")
        .env("RUST_LOG", "info")
        .hyp_env("RELAYCHAINS", relay_chains.join(","))
        .hyp_env("DB", relayer_base.as_ref().to_str().unwrap())
        .hyp_env("ALLOWLOCALCHECKPOINTSYNCERS", "true")
        .hyp_env("tontest1", "1")
        .hyp_env("tontest2", "1")
        .hyp_env("TRACING_LEVEL", "info")
        .hyp_env("GASPAYMENTENFORCEMENT", "[{\"type\": \"none\"}]") //
        .hyp_env("METRICSPORT", metrics.to_string())
        .spawn("TON_RLY", None);

    relayer
}
#[apply(as_task)]
pub fn launch_ton_validator(
    agent_config_path: String,
    agent_config: TonAgentConfig,
    metrics_port: u32,
    debug: bool,
) -> AgentHandles {
    let validator_bin = concat_path("../../target/debug", "validator");
    let validator_base = tempdir().expect("Failed to create a temp dir").into_path();
    let validator_base_db = concat_path(&validator_base, "db");

    fs::create_dir_all(&validator_base_db).expect("Failed to create validator base DB directory");
    info!("Validator DB: {:?}", validator_base_db);

    let checkpoint_path = concat_path(&validator_base, "checkpoint");
    let signature_path = concat_path(&validator_base, "signature");

    let validator = Program::default()
        .bin(validator_bin)
        .working_dir("../../")
        .env("CONFIG_FILES", agent_config_path)
        .env(
            "MY_VALIDATOR_SIGNATURE_DIRECTORY",
            signature_path.to_str().unwrap(),
        )
        .env("RUST_BACKTRACE", "1")
        .hyp_env("CHECKPOINTSYNCER_PATH", checkpoint_path.to_str().unwrap())
        .hyp_env("CHECKPOINTSYNCER_TYPE", "localStorage")
        .hyp_env("ORIGINCHAINNAME", agent_config.name)
        .hyp_env("DB", validator_base.to_str().unwrap())
        .hyp_env("METRICSPORT", metrics_port.to_string())
        .hyp_env("VALIDATOR_SIGNER_TYPE", agent_config.signer.typ)
        .hyp_env(
            "VALIDATOR_MNEMONIC_PHRASE",
            agent_config.signer.mnemonic_phrase,
        )
        .hyp_env("SIGNER_SIGNER_TYPE", "TonMnemonic")
        .hyp_env("VALIDATOR_WALLET_VERSION", "V4R2")
        .hyp_env("TRACING_LEVEL", if debug { "debug" } else { "info" })
        .spawn(make_static(format!("TON-VL{}", metrics_port % 2 + 1)), None);

    validator
}
#[apply(as_task)]
#[allow(clippy::let_and_return)]
fn launch_ton_scraper(
    agent_config_path: String,
    chains: Vec<String>,
    metrics: u32,
    debug: bool,
) -> AgentHandles {
    let bin = concat_path("../../target/debug", "scraper");

    info!(
        "Current working directory: {:?}",
        env::current_dir().unwrap()
    );
    // info!(
    //     "Resolved scraper config path: {:?}",
    //     fs::canonicalize("../utils/run-locally/src/ton/configs/ton_scraper_config.json")
    // );
    info!("CHAINSTOSCRAPE env variable: {}", chains.join(","));

    let scraper = Program::default()
        .bin(bin)
        .working_dir("../../")
        .env("CONFIG_FILES", agent_config_path)
        .env("RUST_BACKTRACE", "1")
        .hyp_env("CHAINSTOSCRAPE", chains.join(","))
        .hyp_env("tontest1", "1")
        .hyp_env("tontest2", "1")
        .hyp_env(
            "DB",
            "postgresql://postgres:47221c18c610@localhost:5432/postgres",
        )
        .hyp_env("TRACING_LEVEL", if debug { "info" } else { "warn" })
        .hyp_env("METRICSPORT", metrics.to_string())
        .spawn("TON_SCR", None);

    scraper
}

fn cycle_messages() -> u32 {
    info!("Sending messages between TONTEST1 and TONTEST2...");
    let mut dispatched_messages = 0;

    for i in 0..5 {
        send_messages_between_chains();
        dispatched_messages += 1;
        info!("Dispatched message #{} from TONTEST1 to TONTEST2", i + 1);
    }

    dispatched_messages
}

#[cfg(feature = "ton")]
mod test {
    #[test]
    fn test_run() {
        use crate::ton::run_locally;
        env_logger::init();

        run_locally()
    }
}
