use log::info;
//use macro_rules_attribute::apply;
use std::{env, fs, thread::sleep, time::Duration};
//use tempfile::tempdir;

use crate::ton::launch_ton_relayer;
use crate::ton::launch_ton_scraper;
use crate::ton::launch_ton_validator;
use crate::ton::types::read_deployed_contracts;
use crate::{
    logging::log,
    program::Program,
    ton::setup::deploy_and_setup_domains,
    ton::types::generate_ton_config,
    ton::utils::build_rust_bins,
    utils::{as_task, concat_path, make_static, stop_child, AgentHandles, TaskHandle},
};

use crate::ton::TonHyperlaneStack;

#[allow(dead_code)]
pub fn run_ton_to_ton_warp_route() {
    info!("Start run_locally() for Ton");
    let domains: Vec<u32> = env::var("DOMAINS")
        .expect("DOMAINS env variable is missing")
        .split(',')
        .map(|d| d.parse::<u32>().expect("Invalid domain format"))
        .collect();
    let origin_token_standard =
        env::var("ORIGIN_TOKEN_STANDARD").expect("Failed to get ORIGIN_TOKEN_STANDARD");
    let destination_token_standart =
        env::var("DESTINATION_TOKEN_STANDARD").expect("Failed to get DESTINATION_TOKEN_STANDARD");
    let validator_key = "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a";

    info!("domains:{:?}", domains);

    deploy_and_setup_domains(&domains, &validator_key);

    for &domain in &domains {
        let domain_str = &format!("{}", domain);
        let deployed_contracts_addresses = read_deployed_contracts(domain_str);
        let mailbox_address = deployed_contracts_addresses
            .get("mailboxAddress")
            .expect("Not found mailbox");
        deploy_warp_route(
            domain,
            origin_token_standard.as_str(),
            destination_token_standart.as_str(),
            mailbox_address.as_str(),
        )
        .expect("Failed to deploy warp route");
    }
    let amount = env::var("AMOUNT")
        .expect("Failed to get amount")
        .parse::<u64>()
        .expect("Failed");

    let recipient = "";
    send_transfer(domains[0], domains[1], amount, recipient).expect("Failed to send transfer");

    info!("deploy_all_contracts and send_dispatch finished!");
    let mnemonic = env::var("MNEMONIC").expect("MNEMONIC env is missing");
    let wallet_version = env::var("WALLET_VERSION").expect("WALLET_VERSION env is missing");
    let api_key = env::var("API_KEY").expect("API_KEY env is missing");

    log!("Building rust...");
    build_rust_bins(&["relayer", "validator", "scraper", "init-db"]);

    info!("current_dir: {}", env::current_dir().unwrap().display());
    let file_name = "ton_config";

    let domains_tuple = (domains[0].to_string(), domains[1].to_string());

    let agent_config = generate_ton_config(
        file_name,
        &mnemonic,
        &wallet_version,
        &api_key,
        (&domains_tuple.0, &domains_tuple.1),
    )
    .unwrap();

    let agent_config_path = format!("../../config/{file_name}.json");

    info!("Agent config path:{}", agent_config_path);
    let relay_chains = vec!["tontest1".to_string(), "tontest2".to_string()];
    let metrics_port = 9090;
    let debug = false;

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

    let persistent_path = "./persistent_data";
    let db_path = format!("{}/db", persistent_path);
    fs::create_dir_all(&db_path).expect("Failed to create persistent database path");

    let validator1 = launch_ton_validator(
        agent_config_path.clone(),
        agent_config[0].clone(),
        metrics_port + 1,
        debug,
        Some(format!("{}1", persistent_path)),
    );

    let validator2 = launch_ton_validator(
        agent_config_path.clone(),
        agent_config[1].clone(),
        metrics_port + 2,
        debug,
        Some(format!("{}2", persistent_path)),
    );

    let validators = vec![validator1, validator2];

    let scraper = launch_ton_scraper(
        agent_config_path.clone(),
        relay_chains.clone(),
        scraper_metrics_port,
        debug,
    );

    info!("Waiting for agents to run for 3 minutes...");
    sleep(Duration::from_secs(300));

    let _ = TonHyperlaneStack {
        validators: validators.into_iter().map(|v| v.join()).collect(),
        relayer: relayer.join(),
        scraper: scraper.join(),
        postgres,
    };
}

use std::process::Command;
use std::str::from_utf8;

pub fn deploy_warp_route(
    domain: u32,
    origin_token_standart: &str,
    destination_token_standart: &str,
    mailbox_address: &str,
) -> Result<String, String> {
    log!("Launching Warp Route deployment...");

    let working_dir = "../../../../altvm_contracts/ton";

    let output = Command::new("yarn")
        .arg("run")
        .arg("deploy:warp")
        .env("DOMAIN", domain.to_string())
        .env("ORIGIN_TOKEN_STANDARD", origin_token_standart)
        .env("DESTINATION_TOKEN_STANDARD", destination_token_standart)
        .env("MAILBOX_ADDRESS", mailbox_address)
        .current_dir(working_dir)
        .output()
        .expect("Failed to execute deploy:warp");

    let stdout = from_utf8(&output.stdout).unwrap_or("[Invalid UTF-8]");
    let stderr = from_utf8(&output.stderr).unwrap_or("[Invalid UTF-8]");

    if !output.status.success() {
        log!("Deploy failed with status: {}", output.status);
        log!("stderr:\n{}", stderr);
        return Err(format!(
            "Deploy failed with status: {}\nstderr:\n{}",
            output.status, stderr
        ));
    }

    log!("Deploy script executed successfully!");
    log!("stdout:\n{}", stdout);

    let deployed_contracts_path = format!("{}/warp-contracts-{}.json", working_dir, domain);

    match fs::read_to_string(&deployed_contracts_path) {
        Ok(content) => Ok(content),
        Err(err) => {
            log!("Failed to read deployed contracts: {}", err);
            Err("Failed to read deployed contracts".into())
        }
    }
}
pub fn send_transfer(
    origin_domain: u32,
    dest_domain: u32,
    amount: u64,
    recipient: &str,
) -> Result<(), String> {
    log!("Launching sendTransfer script...");

    let working_dir = "../../../../altvm_contracts/ton";

    let output = Command::new("yarn")
        .arg("run")
        .arg("send:transfer")
        .env("ORIGIN_DOMAIN", origin_domain.to_string())
        .env("DESTINATION_DOMAIN", dest_domain.to_string())
        .env("AMOUNT", amount.to_string())
        .env("RECIPIENT", recipient)
        .current_dir(working_dir)
        .output()
        .expect("Failed to execute sendTransfer");

    let stdout = from_utf8(&output.stdout).unwrap_or("[Invalid UTF-8]");
    let stderr = from_utf8(&output.stderr).unwrap_or("[Invalid UTF-8]");

    if !output.status.success() {
        log!("sendTransfer failed with status: {}", output.status);
        log!("stderr:\n{}", stderr);
        return Err(format!(
            "sendTransfer failed with status: {}\nstderr:\n{}",
            output.status, stderr
        ));
    }

    log!("sendTransfer script executed successfully!");
    log!("stdout:\n{}", stdout);

    Ok(())
}
