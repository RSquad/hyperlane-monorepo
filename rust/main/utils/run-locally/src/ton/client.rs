use log::{info, log};
use macro_rules_attribute::apply;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use crate::config::Config;
use crate::utils::as_task;
use hyperlane_core::{HyperlaneDomain, KnownHyperlaneDomain};

pub fn send_messages_between_chains() {
    info!("Sending messages between TONTEST1 and TONTEST2...");
    // TODO: Add logic to send messages between chains
}
