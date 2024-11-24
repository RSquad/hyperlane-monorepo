#![allow(dead_code)] // TODO: `rustc` 1.80.1 clippy issue

use crate::config::Config;
use crate::logging::log;
use crate::program::Program;
use crate::ton::client::send_messages_between_chains;
use crate::ton::deploy::deploy_all_contracts;
use crate::utils::{as_task, concat_path, AgentHandles};
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

const KEY_VALIDATOR1: (&str, &str) = ("validator1", "legend auto stand worry powder...");
const KEY_VALIDATOR2: (&str, &str) = ("validator2", "stomach employ hidden risk fork...");
const KEY_RELAYER: (&str, &str) = ("relayer", "guard evolve region sentence danger...");

fn default_keys<'a>() -> [(&'a str, &'a str); 3] {
    [KEY_VALIDATOR1, KEY_VALIDATOR2, KEY_RELAYER]
}

fn run_locally() {
    info!("Start run_locally() for Ton");
    let agent_config_path =
        concat_path("utils/run-locally/src/ton/configs", "ton_agent_config.toml")
            .to_str()
            .unwrap()
            .to_string();
    info!("Agent config path:{:?}", agent_config_path);
    let relay_chains = vec!["TONTEST1".to_string(), "TONTEST2".to_string()];
    let metrics_port = 9090;
    let debug = true;

    let deploy = deploy_all_contracts();

    let relayer = launch_ton_relayer(
        agent_config_path.clone(),
        relay_chains.clone(),
        metrics_port,
        debug,
    );

    let validator1 = launch_ton_validator(agent_config_path.clone(), metrics_port + 1, debug);
    let validator2 = launch_ton_validator(agent_config_path.clone(), metrics_port + 2, debug);

    send_messages_between_chains()
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

    let relayer = Program::default()
        .bin(relayer_bin)
        .working_dir("../../")
        .env("CONFIG_FILES", agent_config_path)
        .env("RUST_BACKTRACE", "1")
        .hyp_env("RELAYCHAINS", relay_chains.join(","))
        .hyp_env("DB", relayer_base.as_ref().to_str().unwrap())
        .hyp_env("ALLOWLOCALCHECKPOINTSYNCERS", "true")
        .hyp_env("CHAINS_TONTEST1_MAXBATCHSIZE", "5")
        .hyp_env("CHAINS_TONTEST2_MAXBATCHSIZE", "5")
        .hyp_env("TRACING_LEVEL", if debug { "debug" } else { "info" })
        .hyp_env("METRICSPORT", metrics.to_string())
        .spawn("TON_RELAYER", None);

    relayer
}
#[apply(as_task)]
pub fn launch_ton_validator(
    agent_config_path: String,
    metrics_port: u32,
    debug: bool,
) -> AgentHandles {
    let validator_bin = concat_path("../../target/debug", "validator");
    let validator_base = tempdir().unwrap();

    let validator = Program::default()
        .bin(validator_bin)
        .working_dir("../../")
        .env("CONFIG_FILES", agent_config_path)
        .env("RUST_BACKTRACE", "1")
        .hyp_env("DB", validator_base.as_ref().to_str().unwrap())
        .hyp_env("METRICSPORT", metrics_port.to_string())
        .hyp_env("TRACING_LEVEL", if debug { "debug" } else { "info" })
        .spawn("TON_VALIDATOR", None);

    validator
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

        run_locally()
    }
}
